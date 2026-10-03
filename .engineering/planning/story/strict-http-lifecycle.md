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
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: inferred
  path: conformance/strict-lifecycle
- confidence: cited
  path: crates/b10x-mcp-client/Cargo.toml
- confidence: cited
  path: crates/b10x-mcp-client/src/lib.rs
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
  path: crates/b10x-mcp-client/src/strict_progress.rs
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
revision: 9
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

## Stream-control checkpoint, 2026-10-03

The lifecycle selection now executes41 scenarios (39 authored), including22
actual HTTP stream/control scenarios, with no failures/errors/skips/unsupported.
Seven native stream tests additionally cover exact server IDs and reply headers,
original terminal retention, shared byte bounds, invalid initialization session
headers and closing a pending control reply when its exchange future is dropped.
That closure is observed before the connection handle is dropped, while the
operation deadline remains in the future. The model projects77 values with31
structural-codec obligations; constructor partial refusals remain unchanged.

Legacy ping and unadvertised requests receive separate result/error POSTs.
Modern server requests are refused. Ordered notification and request/reply
observations share the existing response byte ceiling with the original body.
Control I/O is driven concurrently inside the exchange-owned futures; it creates
no background control task. Deadline or dropped exchange drops pending replies.
An observed business terminal survives a delayed side acknowledgement.
Bare CR dispatches promptly; empty legacy priming events carry no authority.

Red/green evidence includes the bare-CR deadlock and delayed-acknowledgement
regressions. Disabling the shared control-body retention budget produced40passed
and1failed, exactly mcp.strict_stream_checks/authored/stream-control-bound-2025-11-25,
with no error/skip/unsupported. Source was restored before the final gate.
Shared-budget mutation log SHA256:
80a40d62dd610dfd0cc8d7c87be1d3744368a130b00e844d004efa0f41a35982.
Current lifecycle suite SHA256:
49e7c3df63b05ad45b3a9d72fe3ff0a3a63e41c459b5ab69ce50619bb1b5113c.

This remains a partial checkpoint and the story remains active. Next: exact
opted-in progress, explicit revision-specific cancellation, completion races,
observed late replies and awaited schema-worker cancellation. Then actual
Connectors inbound/outbound integration and the verified release. Existing
consumer ownership decision blockers remain unanswered. No independent review
is claimed: workers are quota-exhausted and the coordinator authored/reviewed.
Detailed evidence and continuation notes are retained under
.cache/mcp-next-runtime/lifecycle; next-progress.md records the next implementation.

Rust1.88 full gate exited0:95 native test functions passed,0failed0ignored,
36 summaries. Specification regeneration, AEP validation, fmt, locked workspace
check/test, all-target/all-feature Clippy and warnings-denied rustdoc passed.
Final gate SHA256:
bee46c9a8420d05a7418242b2e7cc5ef85581b00007fe95b14e2ed59be609c62.
The preceding gate passed tests but failed the receiver's101-line lint; response
format parsing was extracted and the full gate rerun successfully. Builds use
two jobs, no incremental output and dev/test debug=0. The task's old6.8GiB build
output was safely cleaned; source, evidence, archives and other tasks are retained.

## Exact progress checkpoint,2026-10-03

L1/L2 now run over actual HTTP streams in both revisions. Explicit string/integer
params._meta.progressToken opts in. Invalid metadata/tokens refuse before send.
Unknown or absent request tokens produce unmatched observations and cannot
change the owned sequence. Progress and optional total stay exact JSON numbers;
message remains untrusted text. A bounded decimal representation compares sign,
digits and arbitrary-precision exponent without f64 or expanded powers of ten.
Equal spellings1/1.0,100/1e2 and signed zero, plus decreasing values, are retained
and refused. Malformed progress/total/message/token fields refuse with original
message bytes. Total does not impose an invented upper bound on progress.
Cumulative byte limits and the original absolute deadline remain unchanged.

Lifecycle selection80passed (77authored),0failed/error/skipped/unsupported:
38 new authored progress cases cover both revisions. Five native progress tests
assert exact numbers beyond2^53, extreme exponents, raw message agreement,
optional presence, token types, sequence ownership, before-send refusal and
continuous progress ending at the operation deadline. The original acceptance
families map to lifecycle-progress-exact/tiny/large-token,
lifecycle-progress-unmatched/unopted/token-kind, equal/exponent-equal/
negative-zero/decreasing, bound and deadline in mcp.strict_progress_checks.
Malformed input/message cases are additional coverage. All prior selections remain.

Hardening mutation disabled the strict-increase predicate. Result72passed8failed,
exactly equal,exponent-equal,negative-zero,decreasing in both revisions;
0error/skipped/unsupported. Restored source before the full gate.
Mutation SHA25619e9006dbfd1ced222b64bdc498f7eeb581022cd88227a335d7f1d2a42073cb4.
Suite SHA256f8825c868c6a9b609e3e4ff471ab57b4fcbb4ca5a1e0d1ed57a8cb660284212f.

Rust1.88 full gate passed100native functions,0failed0ignored,37summaries.
Specification/type/suite drift, AEP validation, fmt, locked workspace check/test,
all-target/all-feature Clippy and warnings-denied docs pass.
Gate SHA2563146b2405c2efc410b67f06b197c4236d880dba1e3a1ae06e436711592dc6c17.
The first native failure was an over-strict spelling assertion: JSON emits an
explicit plus in positive exponents. It was corrected while retaining exact
numeric and original message checks. Subsequent gate/Clippy findings were
unnecessary by-value parameters, corrected without suppressions. Model remains
77types31codecobligations; constructor partial refusals remain unchanged.

Story remains active. Explicit cancellation, completion races, observed late
responses and awaited schema-worker cancellation remain. Actual Connectors
inbound/outbound integration and verified final release remain required. Existing
consumer ownership decisions remain unanswered. No independent review claimed;
workers remain quota-exhausted. Next owner is the continuing coordinator; details,
logs and next-cancellation.md are in.cache/mcp-next-runtime/lifecycle.
