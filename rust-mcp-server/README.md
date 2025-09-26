# rust-mcp-server

Skeleton Model Context Protocol (MCP) server implemented in Rust. The goal of this crate is to
provide a well-structured starting point for building a fully-featured MCP server: transports,
request handling, shared state, and protocol data types are stubbed out and ready for extension.

## Features

- Tokio-based async runtime with pluggable transport abstraction.
- Working stdio transport that implements MCP-style `Content-Length` framed JSON-RPC.
- Placeholder request router with `initialize` and `ping` handlers and a shared application state
  container.
- Protocol model structs for handshake payloads and server capability declarations.

## Getting Started

```
cargo run --release -- --transport stdio
```

The server currently only supports the stdio transport. Additional transports (for example Unix
sockets) can be added by implementing the `Transport` trait in `src/transport/` and updating the
factory in `transport::create_transport`.

## Extending the Server

- Add new MCP methods by extending `Server::handle_request` / `handle_notification` and implementing
  dedicated handler functions. Common utilities can live in new modules under `src/server/`.
- Register tools and resources by editing `ServerCapabilities` in `Server::new` and enriching the
  protocol model types under `src/protocol.rs`.
- Persist application state via the `AppState` helper in `src/state.rs` using `tokio::sync`
  primitives.

## Development Notes

- Formatting is handled with `cargo fmt`.
- `cargo check` / `cargo clippy` currently require network access to pull dependencies from
  crates.io.
