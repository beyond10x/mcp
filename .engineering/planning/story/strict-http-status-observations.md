---
format: aep.planning-md/3
id: story:strict-http-status-observations
kind: story
status: implemented
title: Preserve modern HTTP peer errors and legacy session evidence
relations:
- derived_from: specification:strict-http-exchange-scope
- depends_on: story:strict-http-exchange
- serves: vision:consumer-owned-mcp-mechanics
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/strict-http
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_http.rs
- confidence: cited
  path: crates/b10x-mcp-client/tests/strict_http.rs
- confidence: cited
  path: crates/b10x-mcp-client/tests/strict_http_conformance.rs
- confidence: cited
  path: crates/b10x-mcp-client/tests/support/strict_http.rs
- confidence: cited
  path: ess/coverage.md
- confidence: cited
  path: ess/domains/http_exchange.yaml
- confidence: cited
  path: toolchain.json
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T04:03:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T04:03:55Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-03T04:09:04Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Preserve revision-specific HTTP failures

## Acceptance

The actual strict HTTP target must preserve complete correlated JSON-RPC errors on
modern HTTP 400 and 404, including arbitrary integral code and absent/present-null
or structured data, while refusing malformed, ambiguous, wrong-id or success-shaped
error-status bodies. Every case performs exactly one business POST. Auth statuses,
capacity, loss and deadlines retain their existing distinct observations. Only a
legacy request actually carrying a session header may classify HTTP 404 as
session_expired; modern or sessionless legacy HTTP 404 is http_status unless a
modern correlated error is observed. No fallback, retry or replacement session.

Named scenarios: strict-status-peer-error, strict-status-invalid-envelope,
strict-status-bound, strict-status-loss, strict-not-found-sessionless, each with
revision-qualified authored cases; retain the existing legacy session-expired
case with an actual sent session header. Existing modern session-expired expectation
is a confirmed incorrect oracle: correct it to http_status, retaining the case.
Native cases cover all three modern rejection codes, method-not-found, opaque
unknown codes, data presence and repeated/wrong IDs. The conformance target returns
actual peer code and actual status in addition to existing observations.

## Basis

Connectors af8aefad3c83195068e68e606d174f9b125d091c,
adapters/mcp/contracts/client/v1alpha1/semantics.md sections 2–4 and 9:
one configured revision, no automatic fallback, sessions only on 2025-11-25.
Pinned upstream 2026-07-28 commit 5f5440bb26a62e2cf3440b92da5a667efa03b267,
basic/transports/streamable-http lines 250–275: modern 400 protocol errors and
404 method-not-found. Pinned legacy commit38c84e9f93ad191d9eb26d92b945d17bd0efcaf3,
basic/transports session management: 404 identifies expiry on requests carrying
an assigned session. Session provenance remains the future connection's duty;
this low-level already-selected exchange observes only the header it sends.

## Scope

Cited: ess/domains/http_exchange.yaml existing classifications and envelope model;
client strict_http.rs status classification and bounded receiver; existing native
and conformance targets and owned wire fixture; strict-http suite/inventory;
CHANGELOG and ESS coverage. No new entity or handwritten product model.

## Delivery

User-authorized MCP delivery and specification refinement. Implement with red
wire tests first. Existing three agents are quota-exhausted; coordinator performs
implementation and a separately labelled review, never an independent review claim.
Negotiated connection, metadata/header agreement, pagination and family APIs remain
subsequent work. This correction alone does not finish or release MCP.
