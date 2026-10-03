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

The receiver uses 22 ESS-generated model types. The four generated artifacts are
regenerated and compared by the gate. Their 19 structural-codec obligations remain
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

Remaining work includes revision-fixed connection setup, typed revision/family checks,
bounded tool/resource/prompt discovery, resource and prompt operations, reusable
strict connections, consumer admission and credential integration, and the open
caller/stdio ownership decisions. The constructor scenario bodies remain unchanged
with regenerated model digests; their eight partial and three authored refusals
remain. Report/2 still reports inconclusive conformance and unknown coverage.
