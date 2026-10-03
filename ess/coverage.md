# Scope against the original18 obligations

This is a **partial constructor-binding draft**, not the full MCP API contract. Original named obligations remain in `obligations.md`. Six public value shapes are retained; two additional types are explicitly binding-only bounded input profiles. Neither excluded input shapes nor emitted scenarios are counted as passing.

The supported selection now executes against actual `ConnectionId::new`,
`ToolDescriptor::from_raw` and `ToolResult::from_raw`: nine passed (four generated,
five authored), zero failed/error/unsupported/skipped. This increases actual
execution from zero to nine; it does not resolve any synthesis refusal. Full
authored selection still refuses `id-valid`, `id-max-length` and `snapshot-literal`;
partial synthesis emits the original four refusals plus two for the HTTP
observation type invariants and two for HttpStatus and PositiveMilliseconds.
The snapshot command has no execution
credit. Report/2 says execution `passed`, conformance `inconclusive`, coverage
`unknown`. The wrong-return control fails exactly `result-success` and
`result-tool-error` by flipping the actual return field, without reading expectations.

The observation binding admits JSON null, boolean, text, containers and exact signed
64-bit integers without a floating-point intermediate. Other numbers are explicitly
unsupported. Observation bounds are 1 MiB serialized bytes, depth 128 and 65,536
nodes; overflow is refused without truncation. Optional fields use the declared
`null_when_absent` serialization, so `None` and `Some(null)` both project as null;
the retained raw JSON still distinguishes absent `outputSchema` from explicit null.
Three native projection tests check these boundaries separately from the nine
ESS scenarios; they are not additional specification coverage.

| Original obligation | Successor coverage / remaining work |
| --- | --- |
| connection-id-admits-only-bounded-ascii | Four authored cases: valid a_0, valid64, empty, overlength65. Only empty/overlength compile into partial suite; success cases refused by constrained-response observer. Alphabet excludes forbidden characters before invocation; those cases and serde path are NOT covered. |
| descriptor-preserves-raw-constructor-input | One authored bounded-profile raw-content case compiles: annotations, _meta and extension retained. Arbitrary Json, invalid output schema and non-string description outside profile. |
| descriptor-refuses-shape-and-byte-overflow | Not represented; general JSON-shape and serialized-byte predicates remain C2. |
| descriptor-camel-alias-precedence | Not represented. |
| snapshot-constructor-count-duplicate-order-digest | New narrower empty-vector literal/digest case authored but refused by constrained ConnectionId response. No nonempty/count/duplicate coverage claimed (C3). |
| call-validation-object-before-bound | No command/scenario in this slice. |
| result-constructor-json-and-error-marker | Two bounded text-result cases compile, for exact camel boolean true/false. Scalar/null/snake/nonboolean/alias precedence remain C4. |
| result-constructor-byte-bound | Not represented. |
| public-serde-values-do-not-run-constructors | Preserved as original obligation; no executable case added. |
| http-current-negotiation-freezes-ordered-tools | Deferred actual HTTP target; constructor protocol literal proves no negotiation. |
| http-snapshot-stays-frozen-across-call | Deferred actual receiver/lifecycle observation. |
| http-invalid-arguments-and-unknown-tool-send-zero-calls | Deferred actual HTTP counters and positive controls. |
| http-excess-argument-bytes-send-zero-calls | Deferred actual HTTP counters and positive controls. |
| http-complete-result-and-tool-error-are-returned | Constructor result cases do not establish SDK/HTTP behavior; deferred. |
| http-result-bound-is-post-provider | Deferred provider-effect observation; no no-effect claim. |
| http-config-refuses-before-client-exchange | Deferred counting forwarding-client fixture. |
| http-discovery-tool-count-refusal | Deferred real list exchange. |
| http-call-deadline-is-minimum-and-not-remote-rollback | Deferred actual timing/dispatch/cleanup. |

UNMAPPED U1–U9 from the earlier draft remain in values.yaml: credential source conflict; max_pages/frame enforcement; live lifecycle; registry/custody cascade and multiplicity; snapshot persistence/authority; OAuth one-use/issuer/concurrency; version/capability coverage; consumer ownership decisions; exact Limits/error binding. No operator decision is resolved here.

New C1–C6: profile-excluded ID alphabet cases; general descriptor JSON/byte validation; nonempty snapshot/count/duplicates; arbitrary result JSON/aliases/byte limits; actual HTTP handle/effect observation; measured ESS constrained-response and .count observer refusals. Scope expansion must retain genuine SUT inputs, source-grounded branches and independently authored observations. Never turn an excluded case into a passing count.

The initial HTTP observation-only domain had **zero executed conformance scenarios**.
Its eleven values validate and project structurally. Partial synthesis now also
reports `ESS-SYNTH-013` for `mcp.http_observations.BoundedWireBytes` and
`mcp.http_observations.OctetCount`; no view observes their invariants. The constructor
suite's scenario bodies are unchanged; only its regenerated specification and
contract digests change. H1–H4 cover decoded byte length, length/retention consistency,
peer-error preservation and truthful send/terminal correlation. Generated codec
obligations also require base64 validation and exact integral numbers. Type validation,
projection and the independent design-review control establish none of these runtime
properties. A planted removal of peer data remained structurally valid and was caught
by design review, illustrating that distinction.

## Bounded HTTP session-replay selection

`conformance/http-replay/spec` describes a separate test-binding selection for the
same MCP system. It synthesizes five scenarios (four authored), with zero synthesis
refusals. `crates/b10x-mcp-client/tests/http_replay.rs` executes all five against the
real default and caller-supplied HTTP constructors. Each scenario creates an owned
loopback server that independently parses HTTP and JSON-RPC, counts actual tool
requests and initializations, and joins teardown before returning observations.

The first retained target run passed five, with zero failed, error, unsupported or
skipped scenarios. Re-enabling SDK session recovery made exactly `default-expired`
and `injected-expired` fail: the actual client repeated the business POST and
initialized again. The generated case and both successful-call controls still
passed. The corrected constructors explicitly disable that recovery policy. The
repository gate regenerates and compares this suite and runs its target; neither
the original constructor suite nor its refusal inventory is replaced.

This evidence establishes the expired-session no-redispatch behavior exercised over
the legacy session transport. It does not establish modern strict-result behavior,
resources/prompts, exact wire bounds, SSE resume, authentication recovery or any
consumer custody/ownership decision. Report/2 says execution `passed`, conformance
`inconclusive`, coverage `unknown`. Its default deterministic runner clock is a
logical test clock, not the wall-clock time of execution; dated execution evidence
comes from the enclosing gate/run record. The strict HTTP observation obligations
are not discharged by this narrow HTTP suite.

## Strict raw HTTP exchange selection

`conformance/strict-http/spec` binds the actual additive
`b10x_mcp_client::strict_http::exchange` receiver to an independently framed owned
HTTP endpoint with the explicit `strict-http` Cargo feature enabled. Its suite contains 43 scenarios (42 authored), with zero synthesis
refusals; all 43 executed successfully. Each authored condition runs with both
supported negotiated-revision inputs. This tests the transport envelope, not
negotiation or a modern typed family result. The target consumes only fixture
inputs and returns actual request counts and receiver fields after joined teardown.

The receiver, connection, discovery, invocation and lifecycle project 93 ESS model types. The four generated artifacts are
regenerated and compared by the gate. Their 45 structural-codec obligations remain
visible in `types-report.json`. Numeric input guards require nonnegative byte
limits representable on this host and positive millisecond values representable as
u64; a nonrepresentable input is refused before I/O. Canonical base64, exact request
identity/byte bounds and duplicate-free JSON are checked before dispatch. Retained
bytes, observed lengths, whole/prefix and terminal knowledge come from the receiver.
Only a whole correlated exclusive JSON-RPC result or integer-code peer error can
produce a completed envelope. Opaque JSON is not decoded through codec-private
object markers. Structurally decoding a forged carrier establishes none of these
facts, and generated `Debug` is not safe logging.

Native boundary tests additionally exercise exact/zero limits, mid-UTF-8 prefixes,
integer peer codes beyond u64, duplicate/null identifiers, malformed input carriers,
disabled redirects, LF/CRLF/CR framing, multiline data, notifications, a leading
UTF-8 BOM and independent SSE-event/message bounds. Author tests first caught and
then verified corrections for lost incomplete SSE data, a missed BOM-prefixed data
field and an opaque object reinterpreted as a codec-private number. These are
author regression controls, not an independent adversary review.

The caller supplies the admitted request and a builder carrying its network policy.
The explicit strict profile replaces redirect/retry/decompression policies and
connect timing; request/input/absolute-deadline timing is selected explicitly.
It builds one client per exchange. It cannot retroactively inspect an arbitrary
already-built client's policy. Before response headers, send knowledge is unknown
once execution has begun; a local pre-dispatch refusal is not_sent. Neither a
timeout nor an incomplete observation claims rollback, cancellation at the peer,
or permission to retry. HTTP error bodies are bounded too.

The feature leaves the default dependency selection unchanged. Enabling it unifies
arbitrary-precision JSON in a consumer binary; an older SDK decoding path in that
binary does not inherit the strict receiver's opaque-object preservation. The gate
checks the default workspace and tests, lints and documents all features. The strict
test target without its feature executes zero cases and is not acceptance evidence.

The strict suite now has 43 scenarios (42 authored). Its additional error-status
cases exercise modern correlated HTTP 400/404 peer errors, legacy status refusals,
actual session-header presence, wrong identifiers, bounds and body loss. The old
modern session-expired expectation was incorrect and is retained as an explicit
http_status refusal. Native tests also cover modern header/capability/version
errors, unknown large integer codes, duplicate IDs, auth statuses and deadlines.
These checks do not establish session provenance or initialization.

The separate strict-connection suite executes 32 scenarios (31 authored) against
actual setup and subsequent raw exchanges. It verifies modern discovery without
initialization or session adoption, legacy initialization and an accepted empty202
notification, exact revision selection, capability presence, bounded private
session headers and one deadline across setup. The fixture captures real method,
metadata/header and session observations; no failed setup yields a usable handle.
A revision-check mutation fails exactly both mismatch scenarios (30 passed,2failed).
This is author mutation evidence, not an independent review. The network builder's
absence of hidden protocol-header defaults remains a named consumer-port obligation.

The later selections below cover typed invocation, schema validation and selected
progress/cancellation/shutdown behavior. Consumer admission and credential integration
and the open caller/stdio ownership decisions remain unfinished. The constructor scenario bodies remain unchanged
with regenerated model digests; their nine partial and three authored refusals
remain. Report/2 still reports inconclusive conformance and unknown coverage.

## Bounded selected-family discovery

The separate `strict-discovery` selection binds the public discovery function to
an independently framed HTTP fixture. Its111 scenarios (110authored) cover both
revisions and all three list families: complete two-page lists, exact descriptor
bounds, cumulative item/page limits, zero-page and empty zero-item behavior,
empty/repeated opaque cursors, duplicate identities, invalid fields, later peer
errors, absent capability and a provider deadline shared across pages. Modern-only
cases check cache hints and selected resultType. No failed list exposes accumulated
rows as a complete catalog. Native cases also inspect actual IDs, metadata, unknown
JSON private-marker fields, fractional resource size and prompt argument fields.

An author regression caught prompt annotations being validated as another family's
known field. Prompt has no declared annotations field in either pinned schema;
the corrected boundary preserves it as an opaque extension. Both revision cases
retain the literal value. This is author red/green evidence, not independent review.
The111-case target and the mutation control are recorded with their actual run
results in AEP; a suite's existence alone establishes no execution result.

D1–D5 retain runtime limits, schema selection, homogeneous ordered descriptors,
unique keys, faithful raw fields and shared-deadline obligations. A catalog is an
observation, never authorization to invoke. Arbitrary JSON Schema semantics and
supported dialect admission belong to invocation, even when descriptor root shapes
are valid. Resource URI strings are validated but preserved without normalization or
fetching. Descriptor-byte bounds count compact JSON encoding; page observations
retain exact wire bytes under the separate transport budget. A page-limit refusal's
actual count is the next required page ordinal, not an unseen remote page total.

## Typed invocation and isolated schema validation

`conformance/strict-invocation` executes 54 scenarios (53 authored) over real HTTP
and the one-shot Rust schema worker. All 54 pass with no errors, unsupported cases
or skips. Authored assertions read returned typed values, refusal observations and
captured business requests. They cover all three families, revision-specific
result selection, null versus absent structured output, input/output schema checks,
business errors, unknown content, incomplete responses, deadlines, failed refreshes
and modern parameter headers. No default-returning target could satisfy both the
successful-result and refusal assertions. The synthesized outcome-only scenario is
structural coverage; it adds no independent behavioral assurance.

Temporarily accepting an output-schema mismatch made exactly two authored scenarios
fail, one per revision (50 passed, two failed); restoring the rejection returned
52 passes. Suppressing parameter headers failed the one header-agreement scenario
(51 passed, one failed). Two subsequent scenarios lock down a corrected
pre-dispatch request-limit classification: caller input stays distinct from peer
or transport refusal, with the actual not-sent observation retained.
Native regression coverage also distinguishes validation-worker capacity after a
business response from caller input, retaining the complete response as evidence.
Native tests additionally execute actual worker timeout/reaping and
excess-output handling, offline references, exact large integers, declared header
locations and safe-integer limits, all five content variants and both resource
representations. An author regression caught opaque private-number-marker objects
being reinterpreted at worker IPC decoding; raw object-preserving decoding fixes it.

Thirty pinned Connectors complete-response vectors are replayed against actual
setup, discovery and invocation. Their result/error fields and response bytes are
preserved, with only the request id adjusted for preceding exchanges. Ten document
input/framing vectors are explicitly excluded from this semantic replay, as detailed
in the fixture provenance. They are not ten additional runtime passes.

S1–S2 and I1–I8 distinguish generated carriers from executed schema semantics,
process cleanup, result validation, private catalog provenance and header projection.
The caller admits the worker executable and owns effect authority. Awaited worker
failure proves kill/reap; dropped futures only initiate termination. No schema
retrieval, caching, link fetching, prompt execution, grants or automatic replay is
introduced. Logical timestamps in ESS reports belong to its deterministic runner;
the surrounding test log records the actual execution. Full lifecycle coverage
and Connectors consumer integration remain open.

## Partial strict HTTP lifecycle implementation

The new `mcp.http_lifecycle` domain and nonrecursive `WireObservation` add thirteen
immutable observation types; controlled worker and typed-call outcomes bring the current
projection to 93 values with 45 structural-codec obligations. L1–L14 record runtime
obligations separately. Progress and explicit connection cancellation now execute
over actual streams below; type generation alone establishes neither.

The first shutdown checkpoint exercised18 scenarios,
17 authored, against actual connections. The peer parses every request and waits
for the owned sockets to close before it joins. The selection covers modern and
sessionless no-DELETE, legacy one-DELETE,405,503 and307, bounded retained bodies,
expired and mid-response deadlines, abandoned streams and retained request IDs,
completed calls, the invocation-client entry point and fresh setup with unchanged
caller configuration. Native tests additionally check exact failure bytes and
control headers. There is no server rollback or guaranteed remote termination
claim. No DELETE response is promoted to a correlated business terminal.

The stream-control checkpoint had41 scenarios (39 authored), including22 authored stream
cases. They cover actual legacy ping/error POSTs, exact large/colliding server ids,
sessionless and initialization-time replies, invalid/duplicate/over-bound session
headers that are not echoed, modern server-request refusal, ordered notifications,
cumulative/event/control-body bounds, bare CR delimiters, ignored legacy priming
events and delayed control acknowledgements. A regression first failed because CR
waited for another byte; another failed because a delayed acknowledgement hid a
business result already on the socket. The controlled receiver now dispatches CR
promptly and receives controls concurrently with a shared atomic retention budget.
Seven native stream tests also verify socket closure after dropping an exchange
with a pending control reply, before dropping the connection handle. Control
futures belong to the exchange directly; no spawned control task outlives it.
The one-shot raw receiver retains its original compatibility selection.

The progress checkpoint had80 scenarios (77 authored), adding38 actual progress cases
across both revisions. These exercise explicit token opt-in, absent/unknown tokens,
string versus integer identity, arbitrary-precision tokens, exact increasing
numbers, huge exponents, optional total/message fields, malformed fields and
invalid request metadata. Equal numeric values with different spellings, signed
zero and decreasing values are retained and refused. Unmatched tokens cannot
change the owned sequence. Five native tests additionally assert exact numeric
values, original message bytes, report order, before-send refusal and cumulative
retention. Continuous progress must end at the fixed operation deadline; the
peer observes actual closure. Neither progress nor a total is a business result.

The selection now has105 scenarios (101 authored), adding24 explicit connection
cancellation cases. They cover before-send cancellation, unknown send state before
headers, caller cancellation and timeout, modern no-POST versus legacy one-POST,
failure/nonempty/over-bound acknowledgements, expired teardown, notification timeout,
initialize refusal, reuse after an awaited cancellation, pending control replies
and an already observed terminal winning the race. Offered late bytes remain
unobserved after closure. Five native tests assert actual request IDs/headers,
unknown effects, prior terminal state and the aggregate response ceiling across
interrupted stream history plus the cancellation acknowledgement. The peer joins
only after observing the cancelled sockets close. No rollback or remote termination
is inferred from a cancellation acknowledgement.

The separate Linux schema-worker selection has 15 scenarios (14 authored). Actual
Rust processes establish the PID barrier and `/proc` absence after an observed
reap. Expired teardown retains ownership and refuses both execution APIs until
explicit cleanup succeeds. A dropped controlled future requests termination and
retains its child handle. A kill request alone never proves reap. Falsifying the
retained-child report causes the three intended retention scenarios to fail.

The Linux typed lifecycle selection adds 31 scenarios (30 authored), covering input
validation, output-schema preflight, post-response validation, retained/dropped
worker ownership, post-response IPC capacity, HTTP cancellation and deadlines,
expired notification teardown, and interrupted/dropped discovery in both revisions.
It invokes the public typed client against actual HTTP peers and Rust workers.
An observed business response survives stopped validation; no cancellation POST
is sent for that terminal. Prior discovery exchanges remain observations, and
cancelled or dropped refreshes cannot promote a partial or stale catalog. Native
tests also exercise controlled tools/resources/prompts, modern parameter headers,
argument admission, exact numbers and opaque private-marker objects. A peer
writing headers does not prove the client observed them before cancellation;
an attempted send can truthfully remain unknown in the client observation.

Both process selections require Linux and `test-schema-worker`, enabled by the
repository gate. Other platform/feature combinations do not execute those selections;
the existing HTTP lifecycle selection remains available with `strict-http` alone.
These are selected lifecycle measurements, not a claim of full-system conformance.
Connectors consumer integration and the final release remain separate unfinished work.
