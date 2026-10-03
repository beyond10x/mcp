# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `strict_connection::connect` under `strict-http` establishes one explicitly
  configured revision using modern discovery or legacy initialization and its
  acknowledgement. It returns a reusable raw-exchange handle and an ESS-generated
  peer description, preserves unknown reported metadata, privately retains an
  assigned legacy session, and shares one deadline across setup. It performs no
  fallback, cache, paging or typed family-result acceptance. Caller-supplied HTTP
  builders must not hide MCP protocol/session/routing headers in their defaults.
- An opt-in `strict-http` feature with a `b10x_mcp_client::strict_http::exchange` boundary for one
  revision-selected, caller-admitted HTTP POST. It retains bounded original JSON/SSE
  message bytes, correlation and completion observations, and opaque integer
  peer errors with absent versus present-null data. It disables redirects,
  retries and transparent decompression on the caller-supplied HTTP builder,
  and conserves an absolute monotonic deadline. It performs no initialization,
  family-result validation, resource/prompt workflow or credential repair.
  The generated ESS carriers are structural codecs; use the receiver to obtain
  validated observations and do not log their business-data-bearing `Debug`.
- `b10x_mcp_testkit::controlled_server_source` and `build_controlled_server` compile a
  standard-library MCP server that answers `initialize`, `tools/list` and `tools/call` with the same
  bytes over stdio and over Streamable HTTP. It is the reusable conformance fixture this repository's
  boundaries already claimed.
- `b10x-mcp-stdio` and `b10x-mcp-http` each drive their transport against that server over a real
  pipe and a real loopback socket: version negotiation, the frozen snapshot with its uninterpreted
  annotations, one tool call, and the named refusal each transport owns. Both crates carried zero
  tests before this, so the only exercise of either transport lived in a consumer's suite in another
  repository.

### Changed

- Strict HTTP preserves complete correlated modern JSON-RPC errors returned with
  HTTP 400 or 404, including opaque peer data, without fallback or redispatch.
  A 404 is classified as session expiry only for a legacy request carrying a
  session header. Modern and sessionless legacy 404s prove no session expiry.
- With `strict-http` enabled, JSON numbers retain arbitrary precision for peer-error observations.
  Strict decoding preserves legal codec-private-looking object keys as objects;
  it rejects duplicate members instead of silently choosing their last value.
  Default clients retain their previous dependency feature selection. Cargo
  unifies JSON features in a consumer binary that opts in; existing SDK decoding
  paths in that binary do not gain the strict receiver's raw-object guarantees.
- HTTP clients explicitly disable automatic session recovery that can resend an
  ordinary tool request after a session-expired response. Both the default and
  caller-supplied HTTP client entry points return the failure; a caller decides
  whether to start a new operation. Tool annotations do not authorize a replay.
- `AGENTS.md` no longer states that released contract directories are immutable. This repository has
  no contract directory and the gate checks none; the gate's four steps are named instead.

## [0.1.2] - 2026-09-10

- Declare the MCP library, CLI, and consumer-authority design as a public documentation surface.
- Mark operator commands as executable input for the unified documentation renderer.
- Publish the consumer-authority design at its canonical generated documentation route.
- Make ordinary source publication independent of Atlas admission and checkout freshness,
  using standalone bot delivery while preserving MCP's own checks and contracts.

This source release was cut without running gates, tests or binary packaging, at the operator's
request.

## [0.1.1] - 2026-09-02

### Added

- An injected Streamable HTTP client boundary for hosts that own egress policy, credential custody,
  and transport observability while reusing the same MCP lifecycle and bounds.

## [0.1.0] - 2026-09-02

### Added

- A tools-only MCP client with `2026-07-28` discovery and `2025-11-25` initialization fallback.
- Stdio and Streamable HTTP transports, OAuth support, a named local registry, and `b10x-mcp`.

[Unreleased]: https://github.com/beyond10x/mcp/compare/0.1.2...HEAD
[0.1.2]: https://github.com/beyond10x/mcp/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/beyond10x/mcp/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/beyond10x/mcp/releases/tag/0.1.0
