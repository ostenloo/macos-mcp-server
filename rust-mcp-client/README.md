# rust-mcp-client

Sample Rust MCP client that:

1. Uses OpenAI to convert a natural-language automation request into an AppleScript snippet.
2. Spawns the `rust-mcp-server` binary over stdio.
3. Initializes the MCP session and invokes the Finder tool with the generated script.

## Prerequisites

- `rust-mcp-server` has been built (`cargo build --release` or `--debug`).
- `OPENAI_API_KEY` is exported in your shell. Use the key you provided (avoid committing it):

```bash
export OPENAI_API_KEY="sk-..."
```

## Running the client

```bash
cd rust-mcp-client
cargo run -- \
  --server-path ../rust-mcp-server/target/debug/rust-mcp-server \
  --scripts-dir ../AppScripts \
  --model gpt-4.1-mini \
  --prompt "Return the name of the front Finder window."
```

The program prints the generated AppleScript, the `initialize` response, and the result of the `tools/call` request. Adjust the prompt to target other applications; the available tool names come from the PDFs/TXT files in `AppScripts`.

> **Security note:** never hard-code your API key in source control. This client expects the key in the `OPENAI_API_KEY` environment variable.
