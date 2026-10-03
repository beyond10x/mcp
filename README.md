# MCP

Reusable Rust client support for the Model Context Protocol. The workspace provides a tools-only
client, stdio and Streamable HTTP transports, OAuth, a strict named local registry, and a standalone
`b10x-mcp` operator CLI.

Both transports are exercised here rather than only in a consumer:
`crates/b10x-mcp-stdio/tests/transport.rs` and `crates/b10x-mcp-http/tests/transport.rs` drive a real
pipe and a real loopback socket against the controlled server `b10x-mcp-testkit` compiles from
standard-library source, and assert the negotiated version, the frozen snapshot and one tool call on
each.

The client prefers MCP `2026-07-28` and falls back to `2025-11-25`. Consumers retain authority:
Harness supplies envelopes and approvals; Connectors supplies catalog, grants, egress, and hosted
credential custody. MCP server annotations never grant either consumer anything.

```bash
cargo xtask bootstrap-tools # explicit Linux x86_64 setup, into .cache/tools
cargo xtask gate
cargo run -p b10x-mcp-cli -- connections list
```

The gate requires the ESS 0.50.0 and AEP 0.68.0 tools pinned in
[`toolchain.json`](toolchain.json). `bootstrap-tools` verifies release archive
SHA256 digests before installing the exact binary members; it requires `curl` and
`tar`. It does not change globally installed tools. Alternatively set `ESS_BIN`
and `AEP_BIN` to matching executables, or provide those versions on PATH when no
repository-local tools exist. Explicit paths take priority, then `.cache/tools`,
then PATH; a wrong selected version fails without fallback or automatic download.

`task check` invokes the same gate. It validates ESS and AEP, checks the exact
partial constructor suite and remaining synthesis refusals, then runs formatting,
workspace tests (including the real constructor target), Clippy and Rustdoc.
`cargo xtask specification` performs only the specification checks. Passing those
checks does not establish full MCP conformance; see [coverage](ess/coverage.md).

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

The opt-in `strict-http` feature adds `strict_connection::connect` in
`b10x-mcp-client`. Supply a bodyless admitted POST template, an HTTP builder and
ESS-generated `http_exchange::McpHttpConnectionSetupInput` to establish one configured revision. Modern setup uses
discovery; legacy setup verifies initialization and its acknowledgement, retaining
any assigned session privately. The handle exposes the peer description and one
raw family exchange at a time. `strict_discovery::discover` adds bounded complete
tool, resource or prompt catalogs using generated `McpHttpDiscoveryFamily` and
`McpHttpDiscoveryListLimits`. Pages share one deadline; empty cursors remain valid
continuations. Catalogs preserve raw metadata and do not grant invocation authority
or validate arbitrary JSON Schema semantics. `strict_invocation::InvocationClient`
owns that connection and obtains its own selected catalogs before typed tool
calls, resource reads and prompt retrieval. It preserves ordered content, opaque
fields, business errors and structured-output presence. Invalid modern parameter
header annotations exclude the affected tool; valid annotations mirror arguments
into safely encoded `Mcp-Param-*` headers.

Supply `schema_worker::SchemaWorker` with an admitted absolute path to the
`b10x-mcp-schema-worker` binary and an explicit IPC byte ceiling. Build that binary
with `cargo build --locked -p b10x-mcp-client --features strict-http --bin
b10x-mcp-schema-worker`. Tool input and declared output schemas execute offline
JSON Schema 2020-12 in a one-shot child sharing the invocation deadline. Unsupported
dialects or external references refuse before business dispatch. Awaited failures
kill and reap the worker; dropping the future initiates termination without an
observed reap barrier. This internal validator does not select MCP stdio ownership.

Await `shutdown(teardown_deadline)` on the strict connection or invocation client
to consume the handle and close its pool. A legacy session receives one DELETE
attempt;405 is permitted and other failures retain bounded observations. Modern
and sessionless connections send no DELETE. The explicit teardown deadline is
capped by the existing execution/provider limits and does not extend a business
operation's deadline. Dropping an exchange closes its local stream but proves no
legacy cancellation or rollback. The handle then refuses another exchange until
shutdown, retaining the abandoned request ID with unknown effects. Reconnect from
the caller's unchanged configuration. Strict connections retain ordered SSE
messages and separate control observations under `exchange.observation.stream`.
Legacy ping receives an empty result POST; unadvertised server methods receive
method-not-found. Modern server requests are refused. Shared byte limits cover
the original response and all retained messages/control bodies. Control I/O runs
concurrently within the exchange future under the same deadline; pending control
futures are dropped before return or when the exchange is dropped. An earlier business
terminal survives a delayed control acknowledgement. Progress tracking and
explicit cancellation controls are still under implementation.

The caller supplies authority and network policy;
the builder must contain no hidden MCP protocol/session/routing header defaults.
The existing tools-only constructors retain their separate compatibility behavior.

## License

Apache-2.0.

<!-- b10x-docs:start -->
## Documentation

[MCP documentation](https://beyond10x.github.io/docs/mcp/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->
