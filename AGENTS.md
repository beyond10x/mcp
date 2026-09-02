# AGENTS.md — mcp

## Serves

- **O1 — governed reach.** A consumer can discover and call an MCP tool without trusting the
  server's annotations as authority, and every bound refuses by name.
- **O5 — the generic agent platform.** Harness and Connectors share one MCP client implementation
  while retaining their own policy and credential-custody boundaries.

## Boundaries

- This repository owns MCP client protocol compatibility, standard transports, OAuth mechanics,
  local named connections, and reusable conformance fixtures.
- It owns no agent loop, connector catalog, grant, approval policy, durable hosted secret store, or
  model-provider wire.
- `b10x-mcp-types` performs no I/O, reads no clock, and carries no credential value.
- Server annotations are untrusted hints. A consumer supplies all authority and effect policy.
- Unknown result content is preserved. No frame, schema, argument, result, or OAuth response is
  silently truncated.
- Anything that runs is Rust. No crate or module is named `common`, `shared`, `utils`, `misc`, or
  `helpers`.

## Credentials

- A credential value has no `Display`, has redacted `Debug`, and is zeroized on drop.
- Configurations contain credential sources, never credential values.
- OAuth client credentials are bound to the authorization-server issuer and MCP resource.
- Stdio receives only explicitly named environment variables and never runs through a shell.

## Contracts and gate

Released contract directories are immutable. A wire-visible change cuts a new directory and enters
`CHANGELOG.md`. Run `cargo xtask gate` before every commit.

