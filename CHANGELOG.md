# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

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
