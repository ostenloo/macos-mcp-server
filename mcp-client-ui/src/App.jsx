import { useCallback, useEffect, useRef, useState } from 'react';

const LOCAL_STORAGE_KEY = 'mcp-ui-conversation';

function App() {
  const [apiKey, setApiKey] = useState(localStorage.getItem('OPENAI_API_KEY') || '');
  const [serverPath, setServerPath] = useState('../rust-mcp-server/target/debug/rust-mcp-server');
  const [scriptsDir, setScriptsDir] = useState('../AppScripts');
  const [model, setModel] = useState('gpt-4.1-mini');
  const [prompt, setPrompt] = useState('Return the name of the front Finder window.');
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState(null);
  const [conversation, setConversation] = useState(() => {
    const saved = localStorage.getItem(LOCAL_STORAGE_KEY);
    if (!saved) return [];
    try {
      return JSON.parse(saved);
    } catch (err) {
      console.error('Failed to parse saved conversation', err);
      return [];
    }
  });
  const outputRef = useRef(null);

  useEffect(() => {
    localStorage.setItem('OPENAI_API_KEY', apiKey);
  }, [apiKey]);

  useEffect(() => {
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(conversation));
  }, [conversation]);

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [conversation]);

  const handleClearConversation = useCallback(() => {
    setConversation([]);
  }, []);

  const runInteraction = useCallback(async () => {
    if (!apiKey.trim()) {
      setError('OpenAI API key is required.');
      return;
    }

    setIsRunning(true);
    setError(null);

    const controller = new AbortController();
    try {
      const response = await fetch('/api/run', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ apiKey, serverPath, scriptsDir, model, prompt }),
        signal: controller.signal,
      });

      if (!response.ok || !response.body) {
        throw new Error(`Server returned ${response.status}`);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      const newMessages = [];

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        let newlineIndex;
        while ((newlineIndex = buffer.indexOf('\n')) !== -1) {
          const chunk = buffer.slice(0, newlineIndex).trim();
          buffer = buffer.slice(newlineIndex + 1);
          if (!chunk) continue;
          try {
            const event = JSON.parse(chunk);
            newMessages.push(event);
            setConversation((prev) => [...prev, event]);
          } catch (err) {
            console.error('Failed to parse chunk', err, chunk);
          }
        }
      }

      if (buffer.trim()) {
        try {
          const event = JSON.parse(buffer.trim());
          setConversation((prev) => [...prev, event]);
        } catch (err) {
          console.error('Failed to parse trailing chunk', err, buffer);
        }
      }
    } catch (err) {
      if (err.name !== 'AbortError') {
        console.error(err);
        setError(err.message);
      }
    } finally {
      setIsRunning(false);
    }

    return () => controller.abort();
  }, [apiKey, serverPath, scriptsDir, model, prompt]);

  return (
    <div className="app-container">
      <div className="card">
        <h1>MCP Client UI</h1>
        <p style={{ marginBottom: 24, color: 'rgba(148, 163, 184, 0.8)' }}>
          Generate AppleScript with OpenAI and execute it through the Rust MCP server.
        </p>

        <div className="row">
          <div>
            <label>OpenAI API Key</label>
            <input
              type="password"
              placeholder="sk-..."
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
          <div>
            <label>Model</label>
            <input value={model} onChange={(e) => setModel(e.target.value)} />
          </div>
        </div>

        <div className="row">
          <div>
            <label>Server path</label>
            <input value={serverPath} onChange={(e) => setServerPath(e.target.value)} />
          </div>
          <div>
            <label>Scripts directory</label>
            <input value={scriptsDir} onChange={(e) => setScriptsDir(e.target.value)} />
          </div>
        </div>

        <div>
          <label>Prompt</label>
          <textarea value={prompt} onChange={(e) => setPrompt(e.target.value)} />
        </div>

        <div className="meta">
          <button onClick={runInteraction} disabled={isRunning}>
            {isRunning ? 'Running…' : 'Run'}
          </button>
          <button onClick={handleClearConversation} disabled={isRunning || conversation.length === 0}>
            Clear conversation
          </button>
          <span>{conversation.length} message(s) stored locally</span>
        </div>

        {error && <div className="error">{error}</div>}

        <div className="conversation" ref={outputRef}>
          {conversation.map((message, index) => (
            <div key={index} className="message">
              <h4>{message.type ?? 'event'}</h4>
              <pre style={{ margin: 0 }}>{JSON.stringify(message, null, 2)}</pre>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default App;
