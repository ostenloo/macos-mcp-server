import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

const LOCAL_STORAGE_KEY = 'mcp-ui-conversation';
const DEFAULT_CLIENT_PATH = '/Users/austinliu/macos-mcp-server/mcp-client/target/debug/mcp-client';
const DEFAULT_SERVER_PATH = '/Users/austinliu/macos-mcp-server/mcp-server/target/debug/mcp-server';
const DEFAULT_SCRIPTS_DIR = '/Users/austinliu/macos-mcp-server/AppScripts/text';

function App() {
  const [apiKey, setApiKey] = useState(localStorage.getItem('OPENAI_API_KEY') || '');
  const [clientPath, setClientPath] = useState(DEFAULT_CLIENT_PATH);
  const [serverPath, setServerPath] = useState(DEFAULT_SERVER_PATH);
  const [scriptsDir, setScriptsDir] = useState(DEFAULT_SCRIPTS_DIR);
  const [model, setModel] = useState('gpt-4.1-mini');
  const [prompt, setPrompt] = useState('');
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState(null);
  const [showSettings, setShowSettings] = useState(false);
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

  const latestStatus = useMemo(() => {
    for (let i = conversation.length - 1; i >= 0; i -= 1) {
      const entry = conversation[i];
      if (entry.type === 'status' && entry.message) {
        return typeof entry.message === 'string'
          ? entry.message
          : JSON.stringify(entry.message);
      }
    }
    return null;
  }, [conversation]);

  const latestError = useMemo(() => {
    for (let i = conversation.length - 1; i >= 0; i -= 1) {
      const entry = conversation[i];
      if (entry.type === 'error') {
        if (typeof entry.message === 'string') {
          return entry.message;
        }
        if (entry.message) {
          try {
            return JSON.stringify(entry.message);
          } catch (err) {
            console.error('Failed to stringify error message', err, entry.message);
          }
        }
        return 'An error occurred';
      }
    }
    return null;
  }, [conversation]);

  const statusVariant = useMemo(() => {
    if (conversation.some((entry) => entry.type === 'error')) {
      return 'error';
    }
    if (isRunning) {
      return 'active';
    }
    if (conversation.some((entry) => entry.type === 'complete')) {
      return 'complete';
    }
    if (latestStatus) {
      return 'idle';
    }
    return conversation.length > 0 ? 'idle' : 'hidden';
  }, [conversation, isRunning, latestStatus]);

  const statusText = useMemo(() => {
    if (statusVariant === 'error') {
      return latestError || latestStatus || 'Error encountered';
    }
    if (statusVariant === 'complete') {
      return latestStatus || 'Run complete';
    }
    if (statusVariant === 'active') {
      return latestStatus || 'Working…';
    }
    return latestStatus || 'Ready';
  }, [statusVariant, latestError, latestStatus]);

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
    if (!prompt.trim()) {
      setError('Prompt is required.');
      return;
    }

    setIsRunning(true);
    setError(null);

    const controller = new AbortController();
    try {
      const response = await fetch('/api/run', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ apiKey, clientPath, serverPath, scriptsDir, model, prompt }),
        signal: controller.signal,
      });

      if (!response.ok || !response.body) {
        throw new Error(`Server returned ${response.status}`);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

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
  }, [apiKey, clientPath, prompt, scriptsDir, serverPath, model]);

  const handleSubmit = useCallback(
    (event) => {
      event.preventDefault();
      runInteraction();
    },
    [runInteraction]
  );

  return (
    <div className="app-container">
      <div className="card">
        <h1>MCP Client UI</h1>
        <p style={{ marginBottom: 24, color: 'rgba(148, 163, 184, 0.8)' }}>
          Generate AppleScript with OpenAI and execute it through the MCP server.
        </p>

        <div className="settings">
          <button
            type="button"
            className="button-secondary settings__toggle"
            onClick={() => setShowSettings((prev) => !prev)}
          >
            {showSettings ? 'Hide settings' : 'Show settings'}
          </button>

          {showSettings && (
            <div className="settings__panel">
              <div className="row">
                <div>
                  <label htmlFor="openai-key">OpenAI API Key</label>
                  <input
                    id="openai-key"
                    type="password"
                    placeholder="sk-..."
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                  />
                </div>
                <div>
                  <label htmlFor="model">Model</label>
                  <input id="model" value={model} onChange={(e) => setModel(e.target.value)} />
                </div>
              </div>

              <div className="row">
                <div>
                  <label htmlFor="client-path">MCP client path</label>
                  <input
                    id="client-path"
                    value={clientPath}
                    onChange={(e) => setClientPath(e.target.value)}
                  />
                </div>
                <div>
                  <label htmlFor="server-path">MCP server path</label>
                  <input
                    id="server-path"
                    value={serverPath}
                    onChange={(e) => setServerPath(e.target.value)}
                  />
                </div>
              </div>

              <div>
                <label htmlFor="scripts-dir">Scripts directory</label>
                <input
                  id="scripts-dir"
                  value={scriptsDir}
                  onChange={(e) => setScriptsDir(e.target.value)}
                />
              </div>
            </div>
          )}
        </div>

        <div className="conversation" ref={outputRef}>
          {conversation.map((message, index) => (
            <div key={index} className={`message message--${message.type ?? 'event'}`}>
              <h4>{message.type ?? 'event'}</h4>
              <pre className="message__body">{JSON.stringify(message, null, 2)}</pre>
            </div>
          ))}
        </div>

        {statusVariant !== 'hidden' && (
          <div className={`status-indicator status-indicator--${statusVariant}`}>
            <span className="status-indicator__dot" aria-hidden="true" />
            <span className="status-indicator__text">{statusText}</span>
          </div>
        )}

        {error && <div className="error">{error}</div>}

        <form className="prompt-panel" onSubmit={handleSubmit}>
          <label htmlFor="prompt-input">Prompt</label>
          <textarea
            id="prompt-input"
            value={prompt}
            placeholder="Describe the automation you want to run…"
            onChange={(e) => setPrompt(e.target.value)}
          />
          <div className="prompt-actions">
            <button type="submit" disabled={isRunning || !prompt.trim()}>
              {isRunning ? 'Running…' : 'Send'}
            </button>
            <button
              type="button"
              className="button-secondary"
              onClick={handleClearConversation}
              disabled={isRunning || conversation.length === 0}
            >
              Clear conversation
            </button>
            <span className="prompt-info">{conversation.length} message(s) stored locally</span>
          </div>
        </form>
      </div>
    </div>
  );
}

export default App;
