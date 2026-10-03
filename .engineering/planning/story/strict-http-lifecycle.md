---
format: aep.planning-md/3
id: story:strict-http-lifecycle
kind: story
status: implemented
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
  path: conformance/schema-lifecycle
- confidence: inferred
  path: conformance/strict-lifecycle
- confidence: inferred
  path: conformance/typed-lifecycle
- confidence: cited
  path: crates/b10x-mcp-client/Cargo.toml
- confidence: cited
  path: crates/b10x-mcp-client/src/lib.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/schema_worker
- confidence: cited
  path: crates/b10x-mcp-client/src/schema_worker.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_cancellation.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_connection.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_discovery
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_discovery.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_http.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_invocation
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
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T06:29:28Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T06:29:28Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-03T10:00:49Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":4}}}
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
schema validation requests termination and attempts a reap within the separate
teardown deadline. If it cannot complete, retain ownership and refuse worker
reuse until explicit bounded cleanup observes exit. A kill request is not a reap.

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

## Explicit HTTP cancellation checkpoint,2026-10-03

The ESS-controlled outcome and caller_cancelled refusal were modeled and validated
before runtime edits. The domain now projects78 values with35 codec obligations.
Generated enum ordinal changes were reconciled in connection, discovery and
invocation deadline/input classifiers; existing suites pass. Finished outcomes are
constructed directly, preserving opaque peer objects without a decoding roundtrip.

StrictConnection::exchange_cancellable takes an explicit cloneable monotonic
Cancellation signal plus separate operation and teardown absolute deadlines.
Both use existing execution/provider caps. No signal is serializable authority.
Raw exchange behavior remains unchanged. Local pre-send cancellation retains
not_sent and sends nothing. After an attempted send, modern closes its stream;
legacy attempts one notifications/cancelled POST with the actual request ID.
Notification acknowledgement requires empty202 and never proves remote rollback.
The original stream/control history and cancellation response share one byte
ceiling; the notification gets only the remaining allowance. Expired teardown,
nonempty202, failure, bound and acknowledgement timeout are observed refusals.
Initialize is rejected by this business entry point without a cancellation POST.
Awaited cancellation clears pending state; future drop still requires shutdown.

Cancellation is selected at I/O boundaries before unread bytes. A terminal already
parsed before side-reply cancellation remains the original finished exchange.
Pending side I/O is dropped and records caller_cancelled with its actual send
state. Late bytes offered by the peer after cancellation are not fabricated as
observed replies. Explicit reuse is tested separately and uses a fresh request ID.

Lifecycle105passed(101authored),0failed/error/skipped/unsupported, including24new
actual cancellation scenarios and5newnative functions. Cases cover caller/timeout,
pre-send/pre-header states, no initialize notification, modern no-POST, legacy
single POST, offered late replies, reuse, expired teardown, side failures,
terminal precedence and aggregate retained-byte accounting.

Mutation disabled the legacy notification branch:93passed12failed, exactly the
12authored legacy cancellation-control cases,0error/skipped/unsupported. Restored
before final gate. Mutation SHA256
643c8f22ef103be461306397f8b1ac4e5cb5a4b1ea6926541f2f04c2768ad895.
Suite SHA2562f99b63cb6bcc83393b9ac15ed7759342c9beaee15b40bb77b63cbddcfc8bbf6.
Rust1.88fullgate105nativepassed0failed0ignored38summaries, including spec/type/suite
drift, AEP, fmt, locked check/tests, all-target/all-feature Clippy and rustdoc.
Gate SHA256b905851dd25f071adf6fc5713317f6a50c09a26b7ad199e862b5b229cb24ffaf.
Pre-gate Clippy found receiver length and explicit-default style issues; corrected
without suppressions. Constructor partial refusal inventory remains unchanged.

Story stays active: typed invocation cancellation and bounded awaited worker
kill/reap remain, including before/after-business schema validation semantics.
Then actual Connectors inbound/outbound integration and the verified final release.
Consumer ownership questions remain unanswered. No independent review is claimed;
existing workers remain quota-exhausted. Root remains the author/coordinator.

## Bounded schema-worker checkpoint, 2026-10-03

The controlled worker API now implements ESS L11. It distinguishes no owned child,
an observed reap, and retained ownership. A kill request is never proof of reap.
Separate absolute operation and teardown deadlines apply to run_cancellable;
expired teardown requests nonblocking termination and retains the child handle.
Both execution APIs refuse reuse while that handle remains. reap_pending permits
reuse only after an actual wait succeeds. Dropping the controlled future requests
termination and retains ownership; dropping the whole worker promises only kill
on drop. The original run API retains its compatibility cleanup behavior.

The acceptance wording above now explicitly permits retained ownership when a
bounded reap cannot complete. The previous unconditional kill-and-reap phrase
could not truthfully describe an already expired teardown deadline. L11 and its
named outcomes define the stronger observable contract; no exit is fabricated.
The generated model has85 values and37 codec obligations. Existing nine partial
constructor and three authored refusals remain visible and unchanged.

The separate Linux schema-lifecycle selection has15 scenarios,14 authored:
worker-complete, real, caller, timeout, expired-teardown, drop, overflow,
overflow-retained, invalid-reply, exit-failure, before-start, expired-operation,
input-bound and spawn-failure. Actual Rust children record PIDs; the target checks
/proc after cleanup, rejects both reuse APIs without replacing the PID, then
observes a successful new execution after reap. Assertions run after cleanup,
including in mutation runs. Existing real schema tests also exercise the controlled
API: present null, offline references, dialects, exact numbers and private-marker
objects. The HTTP lifecycle suite still runs with strict-http alone; the new
OS-specific selection requires Linux and test-schema-worker, enabled in the gate.

Three consecutive restored runs report15passed0failed/error/skipped/unsupported.
Mutation reported a retained child as reaped without changing termination/reaping:
12passed3failed, exactly worker-drop, worker-expired-teardown and
worker-overflow-retained, each for its state literal. Original source was restored.
Mutation log SHA256 e6dedc462f7cc86ac2c4540c06a5516210fc7240b83f62ec7c5006a4b252af8b.
Suite SHA256 2645fe086c2d1e90e5ec7f0b29d5c2a46b54a7f793651729b75dda54ee2fc791.

Rust1.88 full repository gate exited0:106native test functions passed,0failed,
0ignored,39summaries. Generated types/suites, AEP, fmt, locked check/tests,
all-target/all-feature Clippy and warnings-denied rustdoc pass. Gate SHA256
538d37f56e0c5914abcf3c0b76ebb6b545e5f9980089f2042e74f4d48400d848.
Initial targeted Clippy findings were unnecessary by-value helper arguments and
similar fixture binding names; corrected without suppressions. This is local
source evidence; hosted CI and release are not claimed. Detailed logs, extracted
reports and worker-evidence.json remain under.cache/mcp-next-runtime/lifecycle.

The story remains active. Typed invocation/discovery cancellation still needs to
carry worker and HTTP observations through pre-dispatch and post-response phases,
without erasing a business response or sending cancellation after its terminal.
Actual Connectors inbound/outbound integration and the verified final release
remain required; existing consumer ownership decisions remain unanswered.
The three workers are still quota-exhausted; review here is the coordinator's,
not an independent approval. No source release is claimed by this checkpoint.

## Typed lifecycle checkpoint and library completion, 2026-10-03

Controlled typed tools/resources/prompts and discovery now preserve the original
catalog admission, parameter-header projection, schema semantics and opaque JSON.
Cancellation is checked at each await boundary against a fixed operation deadline
and independent fixed teardown deadline. Interruption before exchange preparation
invents no request id; HTTP interruption retains the actual connection observation.
Input validation, output preflight and post-response validation retain their phase
and actual worker cleanup state. Stopped post-response validation retains the
original exchange and never cancels a terminal business request. Explicit bounded
reap precedes consuming shutdown when a worker exit barrier is required.

A refresh invalidates the selected catalog before its first await. Only a complete
traversal replaces admission. Cancelled/dropped refreshes cannot promote a prefix
or stale catalog; completed earlier pages remain observations. No consumer
credential, authority, rollback, retry, stdio ownership or tenant mapping is invented.
The model now projects 93 values with 45 structural-codec obligations (L1–L14).

Rust 1.88 cargo xtask gate exited 0: 116 native tests, 0 failed, 0 ignored,
42 summary lines. It validates/regenerates ESS and AEP, checks fmt and the locked
default workspace, and runs all-feature tests, all-target Clippy and denied-warning
docs. Constructor 9, HTTP replay 5, strict HTTP 43, connection 32, discovery 111,
invocation 54, HTTP lifecycle 105, schema lifecycle 15 and typed lifecycle 31 are
separate executed ESS selections, not full-system conformance. The original nine
partial synthesis and three authored constructor refusals remain explicit.

Typed lifecycle ran 31 passed, 0 failed/error/skipped/unsupported three times after
restoration. A deliberate author mutation discarding the original response failed
exactly worker-output and worker-output-bound in both revisions: 27 passed, 4 failed,
0 error/skipped/unsupported. It was restored before the full green gate. Earlier
worker retention, cancellation notification, exact progress and control-budget
mutations remain recorded in the preceding checkpoints. These are author controls.

Gate SHA256: 06eee956b87ac193a499d0679892a77191affe2119474b1609f0ef87e648344c
Mutation SHA256: 3ae1aa6abc28584e95bf35a0cae885409bc666780f2249abd0650a9ccfcce40c
Typed suite SHA256: 2490b6dc2a30c35fe43001d3854375afc2b4389481702145700479b8df06dd70
Logs/reports: .cache/mcp-next-runtime/lifecycle/typed-*.
The deterministic ESS clock is not the execution wall clock. Initial fixture
mistakes and omitted constructor --scenarios selection were corrected without
weakening runtime guards or inventories; the gate then passed.

The coordinator's separate read-only review covers controlled admission, fixed
budgets, actual child ownership, terminal retention and catalog invalidation.
Existing workers are quota-exhausted; no independent or human approval is claimed.
This completes this library story's selected lifecycle scope only. Actual Connectors
local inbound and outbound HTTP integration, unresolved cloud caller mapping and
outbound stdio decisions, and the final verified release remain unfinished.
