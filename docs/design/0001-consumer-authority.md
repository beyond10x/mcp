# 0001 — MCP is transport, not authority

## Decision

This repository owns MCP client compatibility and the mechanics needed to reach a server. It does
not own permission to publish or invoke what that server describes.

The client negotiates `2026-07-28` by discovery and falls back to initialization at `2025-11-25`.
Version 0.1 supports `tools/list` and `tools/call` over stdio and Streamable HTTP. Resources,
prompts, elicitation, sampling, roots, tasks, and multi-round tool results are refused by feature
name until a later design admits them.

Discovery produces a bounded immutable snapshot. A consumer may publish only an intersection of
that snapshot and a local authority document. The snapshot remains fixed for a Harness run or a
Connectors catalog revision; a server notification can start review of a later snapshot but cannot
change the active authority surface.

Server annotations are retained in the raw descriptor for review and replay. They are never
interpreted as effect, risk, approval, grant, idempotency, or egress policy. Harness supplies tool
envelopes and its approval gate. Connectors supplies catalog metadata, grants, egress policy, and
hosted credential custody.

## Credentials

Configuration names sources, never values. Stdio receives only explicitly inherited environment
names and never invokes a shell. HTTP requires TLS except on loopback. OAuth uses resource metadata,
authorization-server or OIDC discovery, pre-registration or Client ID Metadata Documents or DCR,
authorization code with PKCE, resource indicators, issuer checks, refresh, and challenged scope
upgrades through the pinned SDK implementation. Stored credentials are bound to both resource and
authorization-server issuer.

The standalone CLI owns local XDG custody. Hosted consumers implement the same OAuth storage and
HTTP traits against their own secret stores and egress controls.

## Failure and bounds

Unknown content is preserved as JSON. Frame, descriptor, page, argument, result, and deadline limits
are named refusals. No result is shortened into something that could be mistaken for a complete MCP
result. Errors exposed to consumers do not contain request bodies, authorization headers, tokens,
authorization codes, or server prose.
