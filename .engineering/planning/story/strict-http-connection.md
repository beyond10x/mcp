---
format: aep.planning-md/3
id: story:strict-http-connection
kind: story
status: implemented
title: Establish a revision-fixed strict HTTP connection
relations:
- serves: vision:consumer-owned-mcp-mechanics
- derived_from: specification:strict-http-exchange-scope
- depends_on: story:strict-http-status-observations
scope:
- confidence: inferred
  path: conformance/strict-connection
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_connection.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_http.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests/strict_connection.rs
- confidence: cited
  path: crates/b10x-mcp-types/src/http_exchange_generated
- confidence: cited
  path: crates/xtask/src/specification.rs
- confidence: cited
  path: ess
- confidence: cited
  path: ess-inputs.yaml
- confidence: inferred
  path: toolchain.json
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T04:17:55Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T04:17:55Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-03T04:37:40Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}, executor: "agent:codex-coordinator"}
---
# Establish one revision-fixed strict HTTP connection

## Acceptance

Execute named real-wire scenarios setup-modern, setup-legacy-session,
setup-legacy-sessionless, setup-revision-mismatch, setup-invalid-description,
setup-peer-error, setup-ambiguous400, setup-invalid-session, setup-session-bound,
setup-notification-refused, setup-deadline and setup-request-bound. Modern sends
server/discover only; legacy sends initialize then one empty202 initialized
notification. Both preserve one configured revision, endpoint and empty client
capabilities. Reject every invalid response before returning a usable handle.
Preserve typed family capability presence, unknown reported strings/raw fields,
optional server identity/instructions and modern cache hints without enabling a
cache. Session values are private, redacted/zeroized and never serialized.

Use a reusable client built from the admitted builder once, bound to one request
template. A subsequent raw family exchange proves reuse of selected revision,
metadata/headers and a real assigned legacy session. Raw exchange returns only
transport observations: typed family validation and bounded list discovery remain
subsequent work. No retry, era fallback, repeated initialize or endpoint change.
One monotonic deadline covers setup phases including the notification. Byte limits
and malformed generated carriers refuse before dispatch wherever observable.

## Basis and model

ESS mcp.http_connection validates before this story: SetupInput, ImplementationInfo,
ServerCapabilities, PeerDescription, CacheHints and SetupRefusal are immutable
values generated alongside existing strict exchange types. C1–C3 remain explicit
runtime obligations. No persistent identity, ownership or consumer-custody relation.
Connectors af8aefad client/v1alpha1/semantics.md sections1–4 and pinned upstream
2026-07-28/2025-11-25 are the wire authority, not rmcp Auto defaults.

## Scope

Cited: ess domains/manifest, existing strict receiver, generated types and generation
gate. Inferred additions: client strict_connection module, independently framed
connection fixture/native cases, separate ESS setup target/suite, gate inventory.
Existing default APIs and ToolSnapshot shape/digest remain unchanged; strict-http
feature only. No consumer pin or release claim from this unit.

## Boundary assumptions

The caller admits endpoint and network builder, including TLS/DNS/proxy policy.
It must not hide MCP protocol/session/routing headers in builder defaults: reqwest
has no public method to inspect/remove those defaults. Explicit request headers
are validated and owned protocol headers are set from this setup. This is a named
consumer-port obligation, not a claim that opaque builder configuration is proved.

## Review

User authorized continued MCP delivery. Existing workers quota-exhausted; root
performs author and separate disclosed coordinator review. No independent claim.
