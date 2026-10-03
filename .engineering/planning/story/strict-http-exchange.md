---
format: aep.planning-md/3
id: story:strict-http-exchange
kind: story
status: implemented
title: Execute one bounded HTTP exchange with exact response observations
relations:
- informed_by: specification:strict-http-exchange-scope
- serves: vision:consumer-owned-mcp-mechanics
scope:
- confidence: inferred
  path: .gitignore
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: conformance/constructors.json
- confidence: inferred
  path: conformance/strict-http/
- confidence: inferred
  path: crates/b10x-mcp-client/
- confidence: inferred
  path: crates/b10x-mcp-types/
- confidence: inferred
  path: crates/xtask/src/main.rs
- confidence: inferred
  path: crates/xtask/src/specification.rs
- confidence: inferred
  path: ess-inputs.yaml
- confidence: inferred
  path: ess/
- confidence: inferred
  path: toolchain.json
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T02:59:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T02:59:14Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-03T03:41:14Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

Implement the strict bounded HTTP exchange required by specification:strict-http-exchange-scope, as the transport foundation for tool, resource and prompt APIs. ess/domains/http_exchange.yaml declares the additive immutable input and result values before this story. ESS0.50 validates five product files. Generated types come from that model; enforce its reported runtime obligations at the receiving boundary. Existing Connection and ToolSnapshot APIs remain compatibility surfaces.

The receiver keeps exact bounded response-message bytes before SDK decoding, correlates the final JSON-RPC id, preserves arbitrary integer peer codes and absent/present-null data, and distinguishes whole-body retention from protocol completion. Unknown send knowledge remains unknown even beside a valid correlated terminal. No result/error acceptance follows from a parseable prefix, wrong id, ambiguous envelope, progress-only stream or body loss. One consumer-supplied HTTP builder retains network admission, proxy, DNS and TLS settings; explicit no-redirect/no-retry/no-transparent-decompression policies prevent hidden redispatch and altered message bytes. The caller supplies the admitted request and negotiated revision. A monotonic absolute caller deadline and the declared remaining budgets cap all work. No initialization, authentication repair, replay, cache or follow-up request occurs here.

## Acceptance

Named conformance scenarios: strict-json-complete, strict-peer-data-absent, strict-peer-data-null, strict-wrong-id, strict-exclusive-result-error, strict-response-bound, strict-request-bound-before-send, strict-body-loss, strict-deadline, strict-session-expired-once, strict-sse-terminal, strict-sse-prefix-incomplete, strict-sse-event-bound. Each runs through actual owned loopback HTTP, independently parses/counts requests, invokes the real strict exchange, and joins fixture shutdown before asserting observations. Both supported revisions are inputs. Family-specific modern result fields, negotiation, discovery and resource/prompt APIs remain required following work; a transport CompleteResult is not family or business success.

## Scope and execution

Single coordinator owns this serial runtime unit in cb26f-mcp, branch unit/strict-http-exchange-20261003, scratch .cache/mcp-next-runtime/strict-exchange and build target/. Root scope includes product ESS, generated types and guard/transport modules, loopback fixtures, separate conformance selection and repository gate inventory, manifests/lock, coverage and changelog. Intended publication is one bot source commit plus exact candidate PR merge after the full gate; existing MCP delivery authorization applies. Agent account quota prevents further dispatch; implementation and separate review passes are by the same coordinator and cannot be represented as independent. Original ownership decision-blockers in Connectors remain open. Only one story is introduced, so the decomposition comparison panel is inapplicable.
