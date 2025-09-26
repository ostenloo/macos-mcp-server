# MCP Client UI (React)

A browser-based interface for driving the MCP client, which in turn invokes the MCP server with OpenAI-generated AppleScript.

## Features

- React + Vite frontend for editing parameters (API key, prompt, model, paths).
- Express bridge that proxies a run request into the MCP client binary.
- Streaming status updates and client/server output over an HTTP streaming endpoint.
- Conversation history persisted in `localStorage` and appended to `server/data/conversations.json` on the server.

## Prerequisites

- Node.js 18+
- MCP client built (`cargo build --release` in `mcp-client`).
- MCP server built (`cargo build --release` in `mcp-server`).
- OpenAI API key with access to the specified model.

## Setup

```bash
cd mcp-client-ui
npm install
```

## Running the stack

Start the Express bridge (spawns the MCP client for each request, which then launches the server over stdio):

```bash
npm run server
```

In another terminal, run the React dev server (proxied to the Express app):

```bash
npm run dev
```

Open http://localhost:5173 to access the UI. Enter your OpenAI API key, adjust the paths for the MCP client/server binaries or script directory as needed, and click **Run** to stream the client output. Conversations are cached locally and in `server/data/conversations.json`.

## Notes

- The API key is never persisted server-side; it is forwarded only for the lifetime of the request via the `OPENAI_API_KEY` environment variable.
- Update `clientPath`/`serverPath`/`scriptsDir` in the UI if your binaries live elsewhere.
- To reset history, use the **Clear conversation** button or delete the JSON file and clear browser storage.
