---
format: aep.planning-md/3
id: story:http-no-implicit-redispatch
kind: story
status: implemented
title: Prevent implicit tool redispatch after MCP HTTP session expiry
relations:
- informed_by: specification:strict-http-exchange-scope
- serves: vision:consumer-owned-mcp-mechanics
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: conformance/http-replay/
- confidence: inferred
  path: crates/b10x-mcp-client/Cargo.toml
- confidence: cited
  path: crates/b10x-mcp-client/src/lib.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests/http_replay.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests/support/http_replay.rs
- confidence: inferred
  path: crates/xtask/src/specification.rs
- confidence: inferred
  path: ess/coverage.md
- confidence: inferred
  path: toolchain.json
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T02:16:17Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T02:16:17Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-03T02:29:31Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":3,"review_outcome":1}}}
---
## Problem and source evidence

Both public HTTP constructors in crates/b10x-mcp-client/src/lib.rs select rmcp3.2's default reinit_on_expired_session=true. The pinned SDK automatically reinitializes and repeats an ordinary POST after HTTP404 when a session was attached. That can repeat a tool with effects without a caller making a new operation.

An actual owned loopback probe at baseline c45e1a74125ac1c4cbba79fa90fe1cb098cb18c8 reproduced the defect through connect_http and connect_http_with_client: one Connection::call produced two independently counted tools/call requests, two initializations and success. Both positive controls produced one call and one initialization. Rust1.88 baseline probe: 2passed/2failed, exit101, actual requests parsed independently of rmcp and fixture joined before reading counters. This is real HTTP evidence, not contract-case validation.

## Acceptance

conformance/http-replay/spec/system.yaml and checks.yaml declare the existing MCP system's bounded test binding before this story. Pinned ESS0.50 validates both files. HttpConstructor selects existing public entry points; ObserveSessionCall observes actual wire counts and actual returned result. No product entity, persistent state, connection ownership or new public value is introduced.

The named authored conformance scenarios default-expired, injected-expired, default-success and injected-success must run through the actual library. Expired cases observe one business request, one initialization and refusal; controls observe one request/initialization and actual-call text. Fixture shutdown is bounded and joined before zero/additional-request claims. The target must not read expected observations. A mutation enabling recovery must fail expired cases while controls continue to pass. The repository gate validates and reproduces this separate bounded suite, and runs its actual target.

Set the explicit SDK policy on both constructors; keep public signatures, tool snapshots and the caller-supplied HTTP boundary. Document the intentional behavior change in CHANGELOG.md. No retry may be inferred from tool annotations. Full strict result observation, modern-family compatibility, resources/prompts, authentication repair, caller assignment and stdio ownership remain separate work.

## Scope

Cited production surface: crates/b10x-mcp-client/src/lib.rs. Inferred implementation/verification surfaces: crates/b10x-mcp-client/tests/http_replay.rs and support fixture, crates/b10x-mcp-client/Cargo.toml, Cargo.lock, conformance/http-replay/, crates/xtask/src/specification.rs, CHANGELOG.md and ess/coverage.md. Reuse existing pinned ESS dev dependencies when adding the target. Single coordinator owns these paths; no parallel worker shares this tree.

The operator has already authorized delivery of MCP. This bounded runtime correction implements that requirement. Agent usage exhaustion currently prevents an independent delegated reviewer; any coordinator review pass must be identified as such, not reported as independent review.

## Scope clarification before gate wiring

The existing gate's expected suite inventories live in toolchain.json. Include that inferred surface for the new five-scenario HTTP selection, alongside the existing constructor inventory; do not hardcode a separate shadow inventory. Check both types and client ESS dev-dependency source pins against the same toolchain revision. The new bounded HTTP spec/suite is separate from the unchanged constructor selection and does not claim full MCP conformance.
