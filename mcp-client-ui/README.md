# MCP Client UI (React)

A browser-based interface for driving the Rust MCP server with OpenAI-generated AppleScript.

## Features

- React + Vite frontend for editing parameters (API key, prompt, model, paths).
- Streaming status updates and MCP responses over an HTTP streaming endpoint.
- Conversation history persisted in `localStorage` and appended to `server/data/conversations.json` on the server.

## Prerequisites

- Node.js 18+
- Rust MCP server built (`cargo build --release` in `rust-mcp-server`).
- OpenAI API key with access to the specified model.

## Setup

```bash
cd mcp-client-ui
npm install
```

## Running the stack

Start the Express bridge (spawns the Rust MCP server for each request):

```bash
npm run server
```

In another terminal, run the React dev server (proxied to the Express app):

```bash
npm run dev
```

Open http://localhost:5173 to access the UI. Enter your OpenAI API key, modify the prompt as needed, and click **Run** to stream results. Conversations are cached locally and in `server/data/conversations.json`.

## Notes

- The API key is never persisted server-side; it is forwarded only for the lifetime of the request.
- Update `serverPath`/`scriptsDir` in the UI if your binaries live elsewhere.
- To reset history, use the **Clear conversation** button or delete the JSON file and clear browser storage.
