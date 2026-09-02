# MCP

Reusable Rust client support for the Model Context Protocol. The workspace provides a tools-only
client, stdio and Streamable HTTP transports, OAuth, a strict named local registry, and a standalone
`b10x-mcp` operator CLI.

The client prefers MCP `2026-07-28` and falls back to `2025-11-25`. Consumers retain authority:
Harness supplies envelopes and approvals; Connectors supplies catalog, grants, egress, and hosted
credential custody. MCP server annotations never grant either consumer anything.

```console
cargo xtask gate
cargo run -p b10x-mcp-cli -- connections list
```

The default registry is `$XDG_CONFIG_HOME/b10x/mcp.toml` (falling back to the XDG location below
`HOME`). OAuth material lives separately under `$XDG_STATE_HOME/b10x/mcp`, with owner-only
permissions. A minimal registry looks like this:

```toml
[connections.local_files]
transport = "stdio"
program = "/absolute/path/to/mcp-server"
args = ["--stdio"]
cwd = "/absolute/working/directory"
inherit-env = []

[connections.remote]
transport = "http"
url = "https://mcp.example.com/mcp"

[connections.remote.auth]
kind = "oauth"
resource-url = "https://mcp.example.com/mcp"
redirect-uri = "http://127.0.0.1:38123/callback"
client-name = "b10x MCP client"
scopes = []
application-type = "native"
```

Use `b10x-mcp auth login remote`, then `b10x-mcp tools snapshot remote`. A bearer token may instead
come from an explicitly named environment variable or JSON file/pointer; inline credentials and
ambient credential discovery are not part of the schema.

## Consumer boundary

The library returns lossless descriptors and results. It does not convert MCP annotations into
permissions, risk, idempotency, grants, or approvals. An embedding consumer must review and attach
those facts itself. See [the boundary design](docs/design/0001-consumer-authority.md).

Hosts with their own egress and secret boundary can use `connect_http_with_client`; the supplied
client performs every HTTP exchange while the foundation retains protocol negotiation, discovery,
snapshotting, bounds, and calls.

## License

Apache-2.0.
