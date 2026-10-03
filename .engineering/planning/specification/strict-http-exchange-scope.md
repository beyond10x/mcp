---
format: aep.planning-md/3
id: specification:strict-http-exchange-scope
kind: specification
status: draft
title: Source-grounded scope for strict HTTP observations before resource and prompt APIs
relations:
- serves: vision:consumer-owned-mcp-mechanics
- informed_by: specification:governed-compatibility-baseline
revision: 6
---
# Next MCP runtime slice: source-grounded scope

Read-only scope, 2026-10-03. MCP tree observed at `1fcb09186aafdec1e6a6acd6655f0d4f29b361f0`; root is concurrently publishing its foundation. No build, test, model validation, network request or source mutation was performed for this report. Existing partial constructor evidence remains nine executions, four partial-synthesis refusals, three full-authored refusals, conformance inconclusive. Candidate Connectors documents were read from `cb26g-out/adapters/mcp/contracts/client/v1alpha1/invocation.md` and `cb26g-in/adapters/mcp/contracts/server/v1alpha1/projection.md`; they are review candidates, not published/runtime authority.

## Recommendation

The next executable vertical slice should be **one strict, bounded HTTP tool exchange through the caller-supplied HTTP boundary**, with actual negotiated-version, bytes, completion, peer-error and one-dispatch observations. Extend that same seam to explicit resource reads, then prompt retrieval. Do not first add thin `read_resource`/`get_prompt` wrappers and call the Connectors dependency complete: the pinned SDK loses observations the candidate contract requires and can silently retry an ordinary POST.

This is a bounded three-step milestone, not a client rewrite: (1) strict HTTP observation with one existing tool; (2) concrete-resource list/read with bounded discovery; (3) prompt list/get and the two-revision compatibility matrix. No inbound server, stdio owner, OAuth store, external execution family, resource templates, subscriptions or persistence belongs in these three steps. Resource templates must be explicitly marked outside this first subset, not counted as all resource capability coverage.

The first slice is useful independently: it supplies the genuine transport seam for original HTTP obligations and makes the next families reuse demonstrated mechanics. The milestone enables Connectors integration work only after its reviewed contract's exact limits, result/error and no-redispatch requirements are demonstrated. Consumer admission, credential custody and invocation observation remain separate Connectors work.

## What exists versus what must be added

Existing public API: `Connection::{id,snapshot,call,close}`, `connect_http`, `connect_http_with_client`, `connect_stdio` (`client/src/lib.rs:21-184`). `snapshot()` returns `ToolSnapshot`, with frozen order, negotiated-version string and digest. `call()` checks local argument object/serialized size and frozen tool membership, uses the minimum caller/configured timeout, calls `call_tool_once`, and returns `ToolResult` for complete results. InputRequired and Task are explicitly refused. No resources/prompts methods or public family descriptors exist. `b10x-mcp-http` currently reexports only `Connection`, `connect_http`, `HttpTransportConfig`; its injected-client entry point is available in the client crate, not the HTTP facade.

Existing product value homes in `ess/domains/values.yaml` are the six `mcp.tools` types: ConnectionId, ToolDescriptor, ToolSnapshot, ToolCall, ToolResult, HttpTransportConfig. Existing `Limits`, ClientError, exact byte observations and receiver semantics are deliberately UNMAPPED, not modelled by generic Json. HTTP behavior is only the 18 prose obligations in `ess/obligations.md`; constructor evidence does not execute HTTP.

Genuinely new product values needed before corresponding stories: resource descriptor/read request/read result, prompt descriptor/string argument definition/get request/get result, and an explicit bounded exchange observation/error vocabulary if the public API returns complete versus incomplete raw observations. These are immutable values, not entities. No new connection identity, owner, cardinality, state machine, persistent invocation or snapshot relation is required. Selected structured fields need typed homes; retained raw JSON is an additional preservation field, not a substitute for those fields.

## Source-confirmed blockers and narrow remedies

1. **Automatic redispatch is currently enabled.** Both HTTP constructors use SDK default transport config (`client:109-112,140-143`). rmcp 3.2.0 `transport/streamable_http_client.rs:2028-2040,2126-2139` defaults `reinit_on_expired_session: true`: an ordinary POST returning SessionExpired can be sent again after initialization. Strict path must explicitly disable recovery and prove one business POST after an expired-session response. SSE reconnect/resume is a separate behavior; distinguish resume GET from a second business POST and retain incomplete outcome when no final correlated result arrives.
2. **Wire bytes are unavailable above the current typed boundary.** `call:67-69` serializes SDK-decoded data, and `ToolResult::from_raw` counts this serialization. This cannot enforce the candidate's exact response-message byte ceiling or retain a mid-UTF-8 prefix. SDK `StreamableHttpPostResponse::Json` already contains `ServerJsonRpcMessage` (`transport:253-256`); its client trait accepts typed requests (`382-465`). The reqwest implementation sends `.json(&message)` and parses `response.json::<ServerJsonRpcMessage>()`; error HTTP bodies use unbounded `.text()` (`transport/common/reqwest/streamable_http_client.rs:211,278-319`). Add a bounded pre-deserialization HTTP observer/client adapter at that seam, with correlation, terminal-boundary and request-send observations. A forwarding wrapper around already parsed responses is insufficient. Preserve caller-owned network admission by delegating actual I/O to the supplied client/transport facility; do not secretly introduce an independent reqwest route.
3. **SDK decoding loses required distinctions.** `model.rs:3785-3867` uses `Option<Value>` for structuredContent: explicit null and absent both deserialize to None. Unknown top-level result fields are not retained. `ContentBlock` is a closed serde union (`model/content.rs:259`) with no unknown variant. ResourceResult and PromptResult similarly expose selected fields, not arbitrary extension preservation (`model.rs:1737,4180`). Preserve a bounded raw decoded object/bytes before this step; assert null versus absence and unknown content explicitly. Do not call SDK reserialization original wire data.
4. **Peer protocol errors are erased.** `map_service_error` (`client:315-345`) retains auth classes but turns ordinary JSON-RPC error code/message/data into generic Transport. Add typed peer-error observation distinct from safe local diagnostics. Opaque peer data is untrusted; do not interpolate it into a safe log message. Existing clients can keep their existing compatibility projection while the strict API exposes the fuller observation.
5. **Version literals are not strict validation.** Lifecycle selects preferred 2026-07-28 and fallback 2025-11-25 (`client:196-215`). SDK result fields intentionally accept absent modern `resultType`, ttlMs and cacheScope for backward compatibility. The new strict path must validate against the actual negotiated revision before accepting a final result; legacy absence is allowed only for legacy. Current controlled fixture announces modern but returns tools results without modern resultType/cache metadata, so it is not a strict modern acceptance oracle.
6. **Discovery bounds are incomplete.** `prepare` uses `list_all_tools()` (`client:228`), whose SDK implementation loops until no cursor and accumulates all rows (`service/client.rs:1727-1741`); only afterwards does wrapper test max_tools. `Limits.max_pages` has no client read. Replace the selected discovery path with page-by-page calls, reject before fetching beyond max_pages, apply incremental total/descriptor limits, and detect repeated cursors. Define zero limit behavior explicitly. Maintain one overall discovery deadline so bounded pages cannot each consume a fresh full budget.
7. **Frame bounds depend on the transport.** max_frame_bytes is passed only as max_sse_event_size; stdio is not bounded by this wrapper. Custom `StreamableHttpClient` default size-aware methods ignore the limit unless overridden (`transport:396-414,444-465`). SSE raw event size is not the candidate's assembled JSON-RPC message byte definition. Keep both axes explicit. Do not relabel max_result_bytes (serialized library result) as exact wire bytes, or claim a supplied arbitrary client honors a contract it does not implement.
8. **The convenient family APIs have hidden continuation/cache behavior.** Use `RunningService::{read_resource_once,get_prompt_once}` rather than higher-level methods that can drive MRTR rounds (`service/client.rs:1910-1922,1982-2060`). Even `read_resource_once` uses fresh/stale cached results (`1431-1474`). `ClientCacheConfig::default` enables caching AND stale-on-error (`client/cache.rs:54-65`). Explicitly disable response caching for the strict live-exchange profile before discovery/read; a second read with changed result then a failed read must demonstrate actual I/O and no stale success. No authorization-context cache partition can be invented from the unresolved consumer identity decision.

## Minimal ESS-first proposal (prose, not validated YAML)

Retain `mcp.tools` and current constructor suite untouched. Add a separate domain file `ess/domains/http_observations.yaml`, declared by `ess/system.yaml` and the explicit manifest. First declare only immutable observation values required by the strict exchange: a selected protocol revision enum; a complete/incomplete terminal observation enum; an exact byte sequence representation; request-sent versus not-sent observation; typed peer JSON-RPC error with integer code, String message and explicit optional data presence. Do not equate sent with effects applied or unsent with a persistent attempt state. A byte prefix must be Bytes or another compiler-supported exact byte representation, never lossy String. Verify pinned ESS support before selecting syntax. Preserve absent-versus-null with explicit presence plus Json value or a supported tagged representation, not Optional<Json> with null_when_absent.

For the initial real-tool binding, add commands in a separate `ess/domains/http_checks.yaml` describing an owned loopback fixture input and actual return/observation fields. The fixture identifier is a binding input, not a new product entity; no database/view should be fabricated to make an opaque Rust Connection receiver look persistent. The target creates the actual library connection, calls it, independently counts wire methods and returns actual observations. Wrong result bytes, wrong error code, a second POST and false completeness must each produce a red named scenario. Reuse the original HTTP obligations where semantics match, append strict-path cases where intentionally stronger. Exact received-length-known versus prefix-length is explicit: do not pretend overflow reveals the unseen response size.

Before resource implementation add `ess/domains/resources.yaml`: ResourceDescriptor { uri:String, name:String, description presence, mime_type presence, raw:Json }; ResourceRead { uri:String }; ResourceReadResult { ordered typed text/blob contents, revision-appropriate cache fields/presence, retained raw:Json }. Before prompt implementation add `ess/domains/prompts.yaml`: PromptDescriptor { name:String, ordered argument definitions with String name and Boolean required, description presence, raw:Json }; PromptGet { name:String, arguments:Map<String,String> }; PromptResult { ordered role/content messages, description presence, raw:Json }. Reuse or explicitly define typed content variants rather than making every payload Json. Unknown content remains a bounded opaque observation with unsupported status, not a silently successful subset. Metadata and optional extension fields require preservation tests.

Add bounded family-list snapshots only when needed by the selected API: immutable ordered lists tied to existing ConnectionId/protocol text as values, no persistence/retention relation or replacement of ToolSnapshot. Define per-family count/descriptor bounds in an additive family-profile value; existing `Limits` is a public struct with strict serde, so casually adding fields breaks struct literals and serialized compatibility. Pin ESS 0.50.0; validate and attempt projections before AEP stories around new values. If a generator/observer refuses, keep the exact refusal and do not hand-transcribe an unchecked parallel model. This report does not claim those proposed types or receiver bindings already compile.

## Named actual behavior cases for the three steps

Step 1, strict existing-tool exchange:

- `http-strict-current-and-legacy`: real negotiation for both revisions, valid revision-specific complete result; modern missing resultType refused, legacy omission accepted. Unsupported negotiation sends zero business calls.
- `http-strict-wire-size-exact`: exact limit accepted; request envelope one byte over rejected before business dispatch; complete result message one byte over yields bounded incomplete observation; prefix ending within UTF-8 retained exactly. Include whitespace/envelope/metadata so reserialization cannot accidentally pass.
- `http-strict-peer-error-preserved`: arbitrary integer code, message and optional data distinguish peer response from local input error and transport loss; tool isError remains successful protocol completion.
- `http-strict-no-redispatch`: expired session, InputRequired and lost reply each send one business request; no follow-up round or implicit new call. Positive control reaches provider once.
- `http-strict-content-preservation`: structuredContent absent versus null, ordered mixed known content, unknown discriminator and optional extension. Unknown is retained/refused, never dropped as a successful subset.
- `http-strict-incomplete-is-not-success`: valid JSON prefix without correlated final boundary, progress-only SSE, body interruption and caller timeout do not claim completion or no effects; paired terminal-complete control succeeds.
- `http-discovery-page-and-total-bounds`: exact page/item limit, next cursor past limit, repeated cursor, oversized descriptor and cumulative deadline; refused connection exposes no half-built snapshot. Exercise real forwarding client and actual page counters.

Step 2, resource list/read reuses the same exchange observations:

- `http-resource-concrete-list-read`: advertised concrete URIs preserve order/metadata; selected read preserves text/blob representation and URI without fetching embedded links. Missing family is explicit unsupported, not fictitious empty supported content.
- `http-resource-current-cache-fields`: modern required ttlMs/cacheScope retained/validated; legacy response acquires no invented fields.
- `http-resource-no-stale-success`: read A, changed read B, then transport/peer failure; observe three actual requests and the final failure, not A/B returned from cache.
- `http-resource-no-mrtr-followup`: InputRequired produces bounded unsupported observation with exactly one resources/read request.

Step 3, prompts:

- `http-prompt-list-get`: declared required/optional string arguments, descriptions and message order/roles/content survive actual exchanges for both revisions; no numeric/JSON coercion.
- `http-prompt-input-before-dispatch`: missing required, undeclared or wrong-type argument according to the selected strict profile produces no prompts/get; valid paired control succeeds. Define unknown-name/local snapshot membership behavior explicitly before implementation.
- `http-prompt-data-not-action`: returned messages/links produce no tool/resource/other prompt follow-up; InputRequired refused once.

These are proposed acceptance names, not executed counts. Cover exact chosen protocol fixtures and source decisions; do not grow into a general SDK audit.

## File scopes, compatibility and fixture seams

ESS-first scope: `ess/system.yaml`, `ess-inputs.yaml`, new domains named above, new authored scenarios under `conformance/scenarios/http/`, `ess/obligations.md`, `ess/coverage.md`, generated suite at a separate `conformance/http.json`, and narrow gate inventory updates for that suite. Keep existing constructor suite and its refusal inventory intact. Root owns AEP edits after model review.

First production slice: additive `crates/b10x-mcp-client/src/http_observation.rs` and a narrowly named transport adapter module, `client/src/lib.rs` for opt-in strict entry point and policy selection, `b10x-mcp-types/src/lib.rs` or explicit family/observation modules for generated/model-checked new values, `b10x-mcp-http/src/lib.rs` facade reexport only if adopted. Cargo manifests/lock change only for actual dependencies; rmcp stays exactly 3.2.0 unless a measured blocker forces a separately reviewed change. Add `CHANGELOG.md` for wire behavior. Resource/prompt methods can live in explicit client/types modules in following steps, avoiding one monolithic patch.

Compatibility: preserve old signatures, ToolSnapshot shape/digest, tools-only default discovery, strict serde behavior and existing tool client call sites. Do not unconditionally probe resources/prompts on every old connect, which would reject formerly valid tools-only servers and add network behavior. Prefer an explicit strict family-selection/options entry point with existing APIs retained as compatibility projection; do not label old clients strict. Tightening old retry/page behavior can be an intentional documented bug fix, but distinguish it from additive source compatibility and test it. Keep any new cache/recovery policy explicit rather than silently exposing SDK defaults as governed semantics.

Fixture seams: existing `b10x-mcp-testkit/fixtures/mcp_server.rs` has fixed responses for initialize/tools/list/tools/call, substring dispatch, no request counters, pages, discovery response, controllable terminal loss or deadlines. `http/tests/transport.rs` starts it as an owned child and proves one HTTP happy path; failure cleanup is not guarded. Extend testkit with a separately framed controlled HTTP fixture that parses real requests, records bounded per-method/correlation observations and exposes a joined shutdown barrier. Prefer an in-process Rust testkit API/new `src/http_fixture.rs` with exact selected scenarios, no new CLI; a new command-line fixture would require clap derive and cannot quietly retain the standard-library-only rustc build assumption. Do not use rmcp server framing as the independent wire oracle. Add `http/tests/strict_exchange.rs`, then `http/tests/resources.rs` and `http/tests/prompts.rs`; add the ESS target at `http/tests/ess_conformance.rs` calling actual public methods. Mutation controls must change actual return/transport behavior, never mirror expectations in the target. Fixture production and consumer test additions touch shared files, so serialize those implementation units or establish non-overlapping ownership first.

Unresolved outside this scope: Connectors caller-to-Connection assignment, stdio child supervision, external execution family; existing U1 credential-location contradiction; durable registry/custody relations; resource templates/subscriptions; output-schema dialect validation and how its typed supported profile is exposed; Connectors result persistence/provenance. None blocks owned loopback HTTP mechanics. None is resolved by selecting disabled SDK caching, adding immutable value types, or making a contract checker green.
## Refined immutable observation specification — 2026-10-03

The operator explicitly requested refinement of the MCP specifications using ESS,
not edits to the skills. Independent author and design reviewer used pinned ESS0.50.
The new ess/domains/http_observations.yaml declares eleven immutable values, including
exact Bytes, uncertain length, local send and terminal observations, and an opaque
peer error preserving absent versus present-null data. No entity, durable record,
caller relation or public Rust API is introduced. Source SHA256:
330b0e919bf55513108afd7f675d25feac0a8a7f385a2e625bfeefdb723f9205.

Validation: mcp v1 — 4 file(s), 5 scenario(s), valid. Rust type projection produced
11types; schema projection produced28combinedartifacts. Generated Rust uses base64
String for Bytes and serde_json::Number for Integer; structural codecs leave nine
explicit obligations, including base64, integrality and invariants. No handwritten
parallel model is permitted. Production integration of generated types remains nextwork.

Measured expression refusals remain exact: ESS-TYPE-003 cannot select bytes.count
from Bytes and cannot select message_length.value from the tagged union. H1/H2
therefore retain actual byte/length consistency as constructor/observer obligations;
H3/H4 retain peer-field fidelity and truthful correlated instrumentation. No fictional
view or weakened invariant is used to erase them.

Independent design review found zero actual mismatches. A planted removal of peer
data remained structurally valid and was caught by design review; this is a human/
agent review control, not an automated runtime mutation score. Review report SHA256
b98b6c12a69fec343b7450610e453566cff1232cad87172524fcc1d062240e46;
author reportf8c89f166cf00375ead730da8fe9432a06651f452de348629333edaf994f2e36.
Immutable review-result:http-observation-values-design-20261003 and no-op outcome
retain the result. Hardening techniques1–7 were notrun; no HTTP runtime suite exists.

Integrated manifest synthesis still emits the same9constructor scenario bodies,
with only regenerated spec/contract digests changed. It adds two ESS-SYNTH-013
refusals for BoundedWireBytes and OctetCount invariants to the previous four.
Toolchain inventory now names all six; full authored selection still exits1 with
exactly its original three ESS-AUTHOR-001 heads. No existing case/constraint dropped.
Regenerated constructor-suite SHA256:
24f5615c8d3a636e808b40d7c593f3fe3bc9a0ba9a5f6073dc821424c5c9c37e.

HTTP observation conformance execution remains zero. A clean structural model is a
prerequisite for the next strict HTTP runtime slice, not delivery of that slice.
Root Rust1.88 taskcheck is running; its result is recorded separately aftercompletion.

## Integrated gate result — 2026-10-03

RUSTUP_TOOLCHAIN=1.88.0 CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache task check exited 0: 26 native tests passed, zero failed or ignored; formatting, Clippy and documentation passed. Specification validation, exact regenerated constructor inventory and AEP validation passed. The partial suite retains nine unchanged constructor scenario bodies and six explicit synthesis refusals; full authored synthesis retains three explicit refusals. No HTTP runtime conformance is established. Gate log SHA256: 71165edeffdabe0ed080b71c1d68dd0eafee522550aabfc1153fb3fe6b7dcac6.

## Checked draft for bounded exchange inputs — 2026-10-03

While shared-disk exhaustion prevents a new repository build, a scratch ESS proposal extends the reviewed observation model with RequestId, PositiveMilliseconds, ExchangeBudget and ExchangeInput. It separates byte ceilings for the request, response message and SSE event, and remaining execution/provider/connect milliseconds. One already-negotiated business exchange carries selected revision, typed identity and exact encoded request Bytes. It introduces no credentials, endpoint retargeting, entity, durable identity or ownership relation. These are explicit proposed design choices, not claims about current runtime APIs.

Pinned ESS0.50 validated the combined5files; selected Rust generation emitted6types including two reused observation values. Four generated deliverables byte-match a second generation. Eight explicit runtime codec obligations remain; H5 actual monotonic deadline conservation and H6 encoded request/correlation/count coherence still require real behavior. No runtime conformance or independent design review was executed. Existing APIs, Limits, generated constructor suite and refusal inventory remain unchanged. Strict result/refusal design, review and actual generated-data guards precede runtime stories.

Draft source .cache/mcp-next-runtime/exchange-input-draft/spec/ess/domains/http_exchange.yaml SHA2565948a7876e2290b9322aa0d9b3595b88c8ba221c806e0d937d8c16bddd8a9d8b. Report .cache/mcp-next-runtime/exchange-input-draft/report.md SHA256f8bfea4acb6bbf1244db4f9ed55ff27f7a0e44a4a47d0e3e6a5c06296c69b772. This is retained preparation, not promoted product source or completed implementation acceptance.

## Checked draft for strict exchange results — 2026-10-03

The input proposal now has a companion scratch result domain: seven new immutable values for optional HTTP status, independent send/terminal/byte observations, closed local refusal reasons, a complete raw result object, an opaque peer error and a tagged result/refusal union. Complete protocol results are not typed family results or business-success claims. Retention and correlation remain independent, so complete malformed/wrong-id bodies can be retained while still refused. No entity, credential, durable relation or new authority is introduced.

Pinned ESS0.50 validates the combined five files. Generation rooted at ExchangeInput and ExchangeResult emits22types, whose four deliverable files byte-match a repeated projection. The actual generated standalone Rust crate passes cargo check under Rust1.88 (exit0,7.09s). Nineteen runtime codec obligations remain. H7 requires real correlation/exclusive result-error/body fidelity and H8 requires truthful refusal classification/no-redispatch/deadline conservation/safe diagnostics. H1-H6 remain. No HTTP runtime conformance is established.

This remains a proposal: no product model or API promoted, no implementation story closed, no independent review claim. All three delegated agents report account usage exhaustion; the coordinator's separate design pass is disclosed as such. Source .cache/mcp-next-runtime/exchange-result-draft/spec/ess/domains/http_exchange.yaml SHA2561c5b44c655ec6a8584225cafb73d7cd66220e2a6d8d3a6cd4292517e8b9088dc; generated types.rs SHA2565a3ad65666f97c0ab480d1ed336cec28ba2d2656752b2937de4b9cf6c95ef5d9; report SHA2563580c1424c89a1d27c1dccc7c0ca5f61c5536788f0119185bc3342929f72572f. Existing suite and APIs remain unchanged until a reviewed runtime unit adopts the typed model and its guards.


## Implemented strict transport foundation, 2026-10-03

story:strict-http-exchange promotes the additive input/result model and its
22 generated types into the opt-in strict-http profile. H7 now preserves unknown
send knowledge independently of an observed correlated terminal. The receiver
consumes an admitted request and caller-configured HTTP builder, enforces explicit
bounds and an absolute monotonic deadline, and retains actual JSON/SSE message
bytes before typed SDK decoding. Its result remains a transport envelope, not
family acceptance. No existing default client feature selection was broadened.

The separate suite has 27 scenarios, 26 authored across both revision inputs, with
zero synthesis refusals. All 27 executed successfully against real owned HTTP.
The full Rust 1.88 gate passed 38 native tests, zero failed or ignored; it checks
the default workspace and tests/lints/documents all features. Correlation mutation
failed exactly both wrong-id scenarios (25 passed, 2 failed). Four author controls
also exposed and corrected SSE prefix loss, BOM handling, codec-private object-key
reinterpretation and an invalid HTTP status entering the constrained carrier.
Review was a separate pass by the same coordinator; agent quota prevents an
independent-agent claim. review-result:strict-http-exchange-20261003 records this.

The old constructor bodies remain unchanged with regenerated model digests.
Two new ESS invariant-observer refusals for HttpStatus and PositiveMilliseconds
join the six existing partial refusals; all three authored refusals remain.
The strict runtime target is separate and claims no full model conformance.
Default runner timestamps are logical test time; AEP evidence records actual run
observations and hashes instead of importing those timestamps as execution time.

Next delivery remains the actual negotiated strict tool connection, bounded family
discovery, concrete resource operations, prompt operations, and Connectors
integration under its original admission and credential-custody requirements.
The input's revision is not evidence of negotiation. Caller assignment and stdio
ownership blockers remain unanswered. Mixed-feature old SDK decoding does not
inherit the strict receiver's preservation guarantees. No MCP release is cut by
this transport foundation, and the broad MCP goal remains incomplete.
