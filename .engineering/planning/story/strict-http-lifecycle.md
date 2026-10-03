---
format: aep.planning-md/3
id: story:strict-http-lifecycle
kind: story
status: active
title: Deliver revision-specific strict HTTP lifecycle
relations:
- serves: vision:consumer-owned-mcp-mechanics
- derived_from: specification:strict-http-exchange-scope
- depends_on: story:strict-http-invocation
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: inferred
  path: conformance/strict-lifecycle
- confidence: cited
  path: crates/b10x-mcp-client/src/schema_worker.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_connection.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_http.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_invocation.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_lifecycle.rs
- confidence: cited
  path: crates/b10x-mcp-client/tests
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
- {from: "draft", to: "proposed", at: "2026-10-03T06:29:28Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T06:29:28Z", actor: "human:timo", revision: 5}
---
# Revision-specific strict HTTP lifecycle

## Acceptance

Implement selected lifecycle behavior over actual HTTP peers for2026-07-28 and
2025-11-25. ESS mcp.http_lifecycle validates before this story is created. Its
immutable observations do not manufacture request or credential authority.

Named conformance cases lifecycle-progress, lifecycle-unmatched-progress,
lifecycle-non-increasing-progress, lifecycle-observation-bound and
lifecycle-progress-deadline prove opt-in token ownership, exact numeric/text
retention, strict increase, bounded observation history and a fixed maximum.
Unknown tokens are recorded without becoming related progress. No progress is a
partial business result. Native tests must exercise arbitrary-precision values.

lifecycle-legacy-ping and lifecycle-legacy-unsupported-request observe separate
empty-result/error POSTs, preserving the originating request outcome and the
actual server id. lifecycle-modern-server-request proves refusal and no reply.
No standalone GET, resumption, implicit business retry or MRTR is added.

lifecycle-cancel, lifecycle-timeout, lifecycle-completion-race and
lifecycle-late-response prove modern stream closure versus legacy cancellation
notification on the wire; initialize is never cancelled that way. A bounded
explicit teardown deadline is distinct from the operation deadline. A failed
notification is recorded, not claimed sent. Actual observed precedence decides
completion; only observed late responses are retained. Awaited cancellation of
schema validation kills and reaps the owned child while preserving this deadline.

lifecycle-shutdown-modern, lifecycle-shutdown-sessionless,
lifecycle-shutdown-delete, lifecycle-shutdown-405, lifecycle-shutdown-failure,
lifecycle-shutdown-bound, lifecycle-shutdown-deadline and
lifecycle-shutdown-abandoned prove no modern/sessionless DELETE, exactly one
legacy session DELETE,405 permitted, bounded failure evidence, no retry, actual
socket closure and retained unknown outcomes for abandoned futures. Explicit
shutdown consumes the runtime handle; caller-owned setup configuration is reusable.
No successful result or rollback is invented by cancellation/shutdown.

## Authority and scope

Connectors client/v1alpha1/semantics.md sections7–11 at af8aefad and pinned
protocol chapters2026-07-28 at5f5440bb /2025-11-25 at38c84e9f. The pinned upstream
progress MUST increase rule is stricter than the authored display's non-decreasing
wording; both hold by recording/refusing equal updates. Runtime guards L1–L7 stay
explicit because structural generated values cannot prove socket/process effects.

Cited surfaces: ESS model/manifest; generated types and regeneration gate;
strict_http receiver, strict_connection, strict_invocation and schema_worker;
existing native/ESS fixture conventions; README/CHANGELOG/coverage. Inferred
surfaces: lifecycle implementation module, native/ESS lifecycle target and suite,
toolchain inventory. One writer. Worker quota exhaustion requires disclosed
coordinator review; it supplies no independent approval. The full story remains
active until every named scenario and the repository gate pass, even if a portion
of the implementation is ready earlier.

## Remaining full delivery

Actual Connectors inbound/outbound integration and the final verified release
remain required. Existing caller-to-Connection and outbound-stdio ownership
blockers are unchanged. This library lifecycle work does not answer them.

## Verified shutdown checkpoint,2026-10-03

Shutdown is implemented locally, not the full story. The domain projects73 total
values with31 existing structural-codec obligations. The18-scenario shutdown
selection (17 authored) executes actual peers with0failure/error/skip/unsupported.
Six native lifecycle tests add exact bytes/header assertions. Rust1.88 full gate
passes88 native test functions,0failed0ignored,35 summary lines. Gate log SHA256
44bb97b8f9f25d331fa4964b22918b4057a7de1ef8425644d63e48406554261f.
A deliberate405-as-accepted mutation gives17passed1failed, exactly the authored
legacy405 case,0error/skip/unsupported; the original implementation is restored.
Mutation log SHA256
21a8dc02dd82c78740b37f3893a82fbf4b4b68cc4692eb44aa5c0bdf3e7c9844.
Suite SHA256
e9a62c8c767b870a6ed0a5b27aa5a9f5f2958a35ab6872df163a272675f46a56.
Evidence stays under.cache/mcp-next-runtime/lifecycle. First gate passed all tests
and stopped at fixture function-length lint; second gate is the verified result.
The earlier constructor nine partial and three authored refusals are unchanged.

No review is claimed independent: the quota-exhausted workers did not review
this checkpoint. This story remains active. Next implement the internal bounded
SSE event/control seam, real legacy ping/error POSTs, opted-in exact progress,
and cancellation/race/schema-worker teardown. Model L1–L6 coverage is explicitly
partial; no source release or consumer adoption is established by this checkpoint.
