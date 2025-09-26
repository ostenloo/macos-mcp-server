import express from 'express';
import bodyParser from 'body-parser';
import cors from 'cors';
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
  const {
    apiKey,
    clientPath,
    serverPath,
    scriptsDir,
    model = 'gpt-4.1-mini',
    prompt,
  } = req.body || {};

  if (!apiKey || !apiKey.trim()) {
    return res.status(400).json({ error: 'OpenAI API key is required' });
  }
  if (!clientPath || !clientPath.trim()) {
    return res.status(400).json({ error: 'Path to the MCP client binary is required' });
  }
  if (!serverPath || !serverPath.trim()) {
    return res.status(400).json({ error: 'Path to the MCP server binary is required' });
  }
  if (!scriptsDir || !scriptsDir.trim()) {
    return res.status(400).json({ error: 'AppleScript export directory is required' });
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

  const stdoutLines = [];
  const stderrLines = [];

  function streamToEvents(stream, type, collector) {
    stream.setEncoding('utf8');
    let buffer = '';

    const flush = () => {
      if (!buffer) return;
      const line = buffer.replace(/\r$/, '');
      collector.push(line);
      writeEvent(type, { line });
      buffer = '';
    };

    stream.on('data', (chunk) => {
      buffer += chunk;
      let newlineIndex;
      while ((newlineIndex = buffer.indexOf('\n')) !== -1) {
        const line = buffer.slice(0, newlineIndex).replace(/\r$/, '');
        buffer = buffer.slice(newlineIndex + 1);
        collector.push(line);
        writeEvent(type, { line });
      }
    });

    stream.on('end', flush);
    stream.on('close', flush);
  }

  try {
    writeEvent('status', { message: 'Launching MCP client…' });

    const args = [
      '--server-path',
      serverPath,
      '--scripts-dir',
      scriptsDir,
      '--model',
      model,
      '--prompt',
      prompt,
    ];

    let child;
    try {
      child = spawn(clientPath, args, {
        env: { ...process.env, OPENAI_API_KEY: apiKey },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
    } catch (err) {
      throw new Error(`Failed to spawn MCP client: ${err.message}`);
    }

    if (!child.stdout || !child.stderr) {
      child.kill();
      throw new Error('Failed to capture MCP client output streams');
    }

    writeEvent('status', { message: `Running ${clientPath} ${args.join(' ')}` });

    streamToEvents(child.stdout, 'stdout', stdoutLines);
    streamToEvents(child.stderr, 'stderr', stderrLines);

    const exitInfo = await new Promise((resolve, reject) => {
      child.on('error', reject);
      child.on('close', (code, signal) => {
        resolve({ code, signal });
      });
    });

    const exitMessage = exitInfo.signal
      ? `MCP client terminated by signal ${exitInfo.signal}`
      : `MCP client exited with code ${exitInfo.code ?? 0}`;
    writeEvent('status', { message: exitMessage });

    writeEvent('complete', {
      exitCode: exitInfo.code ?? 0,
      signal: exitInfo.signal || null,
    });

    await appendConversation({
      prompt,
      model,
      clientPath,
      serverPath,
      scriptsDir,
      exitCode: exitInfo.code ?? 0,
      signal: exitInfo.signal || null,
      stdout: stdoutLines,
      stderr: stderrLines,
      events,
    });
  } catch (err) {
    console.error(err);
    writeEvent('error', { message: err.message });
  } finally {
    res.end();
  }
});

app.listen(PORT, () => {
  console.log(`MCP client server listening on http://localhost:${PORT}`);
});
