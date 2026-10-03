---
format: aep.planning-md/3
id: story:strict-http-discovery
kind: story
status: implemented
title: Discover bounded strict tool, resource and prompt lists
relations:
- serves: vision:consumer-owned-mcp-mechanics
- derived_from: specification:strict-http-exchange-scope
- depends_on: story:strict-http-connection
scope:
- confidence: inferred
  path: conformance/strict-discovery
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_connection.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_discovery.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests/strict_discovery.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests/support/strict_discovery.rs
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
- {from: "draft", to: "proposed", at: "2026-10-03T04:46:09Z", actor: "human:timo", revision: 4, executor: "agent:codex-coordinator"}
- {from: "proposed", to: "active", at: "2026-10-03T04:46:09Z", actor: "human:timo", revision: 5, executor: "agent:codex-coordinator"}
- {from: "active", to: "implemented", at: "2026-10-03T04:56:33Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}, executor: "agent:codex-coordinator"}
---
# Discover bounded complete tool, resource and prompt lists

## Acceptance

Execute real-wire ESS scenarios list-complete, list-empty-cursor, list-repeated-cursor,
list-page-limit, list-item-limit, list-descriptor-limit, list-zero-pages,
list-empty-zero-items, list-duplicate, list-invalid-descriptor, list-invalid-page,
list-null-cursor, list-missing-cache, list-unselected-result, list-late-error,
list-deadline and list-unsupported-family. Select each of tools/resources/prompts
explicitly and both protocol revisions, with exact generated inventory at delivery.
The independently framed HTTP peer observes literal cursor, metadata, IDs and calls.
No failed page returns accumulated rows as a complete catalog. Empty string and
repeated cursors remain opaque and are sent unchanged; max_pages bounds traversal.
This source-grounded rule supersedes the prior scope's proposed repeated-token
refusal (modern pagination chapter forbids decisions based on token contents).

Check host-representable nonnegative limits before I/O, cumulative pages/items and
per-descriptor compact JSON bytes. Existing transport bounds remain exact message
bytes. Zero pages sends no list; zero items accepts only a valid empty list. All
pages share one monotonic deadline capped by connection execution/provider budgets.
Absent capability is unsupported with zero list requests. Names/URIs are unique
across the complete selected family; prompt argument names are unique. Schema root
shapes and known descriptor field types are validated, with raw unknown fields,
optional presence, resource size JSON numbers and opaque schemas preserved. Modern
pages require complete resultType and valid cache hints; legacy gains no fake cache.
Missing cursor ends; present null is malformed according to both pinned schemas.

## Model and boundaries

Validated ESS mcp.http_discovery precedes this story:11 immutable values generated
with the existing model (43total). D1–D5 name runtime consistency obligations.
No entity, persisted snapshot, credential custody or consumer authority relation.
A returned catalog is a complete descriptor observation, not invocation admission:
JSON Schema semantic/dialect validation and typed call/read/get remain next work.
No cache, subscriptions/templates, automatic retries/repair or remote link fetching.
Preserve existing default APIs and ToolSnapshot shape/digest.

## Scope

Cited:ess/domains/http_discovery.yaml,ess/system.yaml,ess-inputs.yaml,generatedtypes,
strict_connection.rs and generation gate. Inferred:new strict_discovery module,
owned real HTTP fixture/native/ESS target, strict-discovery suite and gate inventory;
CHANGELOG/README/coverage. One implementation writer; quota-exhausted delegated
workers mean disclosed author controls and a separate coordinator review.
