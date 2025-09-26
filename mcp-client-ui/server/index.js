import express from 'express';
import bodyParser from 'body-parser';
import cors from 'cors';
import { OpenAI } from 'openai';
import { spawn } from 'child_process';
import { appendConversation, listConversations } from './storage.js';

const app = express();
const PORT = process.env.PORT || 4000;

app.use(cors());
app.use(bodyParser.json({ limit: '1mb' }));

app.get('/api/conversations', async (_req, res) => {
  try {
    const conversations = await listConversations();
    res.json({ conversations });
  } catch (err) {
    console.error(err);
    res.status(500).json({ error: 'Failed to read conversations' });
  }
});

app.post('/api/run', async (req, res) => {
  const { apiKey, serverPath, scriptsDir, model = 'gpt-4.1-mini', prompt, tool = 'app.finder' } = req.body || {};

  if (!apiKey || !apiKey.trim()) {
    return res.status(400).json({ error: 'OpenAI API key is required' });
  }
  if (!prompt || !prompt.trim()) {
    return res.status(400).json({ error: 'Prompt is required' });
  }

  res.setHeader('Content-Type', 'application/json');
  res.setHeader('Transfer-Encoding', 'chunked');

  const events = [];
  function writeEvent(type, payload) {
    const entry = { type, ...payload };
    events.push(entry);
    res.write(JSON.stringify(entry) + '\n');
  }

  try {
    const openai = new OpenAI({ apiKey });
    writeEvent('status', { message: 'Generating AppleScript with OpenAI…' });

    const completion = await openai.chat.completions.create({
      model,
      messages: [
        {
          role: 'system',
          content:
            'You write short AppleScript bodies that can run inside a "tell application" block. Respond with AppleScript code only, no explanations.',
        },
        { role: 'user', content: prompt },
      ],
    });

    const choice = completion.choices?.[0]?.message?.content;
    if (!choice) {
      throw new Error('OpenAI response did not contain any content');
    }

    const script = choice.trim();
    writeEvent('script', { script });

    writeEvent('status', { message: 'Launching MCP server…' });
    const { responses, scriptResult } = await runMcpSession({ serverPath, scriptsDir, tool, script });

    responses.forEach((response) => {
      writeEvent('mcp', response);
    });

    writeEvent('complete', { result: scriptResult });
    await appendConversation({ prompt, script, responses, model, tool });
  } catch (err) {
    console.error(err);
    writeEvent('error', { message: err.message });
  } finally {
    res.end();
  }
});

async function runMcpSession({ serverPath, scriptsDir, tool, script }) {
  return new Promise((resolve, reject) => {
    const child = spawn(serverPath, ['--transport', 'stdio', '--scripts-dir', scriptsDir], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });

    const responses = [];
    let stdoutBuffer = Buffer.alloc(0);
    let done = false;

    function finish() {
      if (done) return;
      done = true;
      child.stdout.off('data', onData);
      child.stdin.end();
      child.kill();
      resolve({ responses, scriptResult: responses[responses.length - 1] || null });
    }

    child.on('error', (err) => reject(err));
    child.on('exit', (code) => {
      if (code !== 0) {
        console.warn(`MCP server exited with code ${code}`);
      }
      finish();
    });

    function sendFrame(payload) {
      const frame = JSON.stringify(payload);
      const header = Buffer.from(`Content-Length: ${Buffer.byteLength(frame)}\r\n\r\n`);
      child.stdin.write(header);
      child.stdin.write(frame);
    }

    function readFrames(chunk) {
      stdoutBuffer = Buffer.concat([stdoutBuffer, chunk]);
      const frames = [];
      while (true) {
        const headerEnd = stdoutBuffer.indexOf('\r\n\r\n');
        if (headerEnd === -1) break;
        const header = stdoutBuffer.slice(0, headerEnd).toString('utf8');
        const match = header.match(/Content-Length:\s*(\d+)/i);
        if (!match) {
          throw new Error('Missing Content-Length in MCP response');
        }
        const length = Number(match[1]);
        const frameStart = headerEnd + 4;
        const frameEnd = frameStart + length;
        if (stdoutBuffer.length < frameEnd) break;
        const body = stdoutBuffer.slice(frameStart, frameEnd).toString('utf8');
        frames.push(body);
        stdoutBuffer = stdoutBuffer.slice(frameEnd);
      }
      return frames;
    }

    const onData = (chunk) => {
      try {
        const frames = readFrames(chunk);
        frames.forEach((body) => {
          try {
            const parsed = JSON.parse(body);
            responses.push(parsed);
            if (parsed.id === 2) {
              finish();
            }
          } catch (err) {
            console.error('Failed to parse MCP frame', err, body);
          }
        });
      } catch (err) {
        reject(err);
      }
    };

    child.stdout.on('data', onData);

    try {
      sendFrame({
        jsonrpc: '2.0',
        id: 1,
        method: 'initialize',
        params: {
          client: { name: 'mcp-client-ui', version: '0.1.0' },
          protocol_version: '2024-10-30',
        },
      });

      sendFrame({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: tool,
          arguments: {
            script,
          },
        },
      });
    } catch (err) {
      child.kill();
      reject(err);
      return;
    }

    setTimeout(finish, 5000);
  });
}

app.listen(PORT, () => {
  console.log(`MCP client server listening on http://localhost:${PORT}`);
});
