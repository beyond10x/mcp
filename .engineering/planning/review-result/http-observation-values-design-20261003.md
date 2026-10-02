---
format: aep.planning-md/3
id: review-result:http-observation-values-design-20261003
kind: review-result
status: active
title: ESS design review of strict HTTP immutable observations
relations:
- reviews: specification:strict-http-exchange-scope
revision: 1
---
# HTTP observation values — bounded ESS design review

No new actionable model mismatch found in the eleven-value proposal. This is a
source design review, not approval or HTTP conformance evidence. H1–H4 and the
nine generated runtime obligations remain open. The existing nine constructor
executions do not exercise this domain.

Reviewed MCP base `62e522298bcc6e116c2f6b82bed60b34ca62e16a` plus the sole proposed
source `ess/domains/http_observations.yaml`, SHA256
`330b0e919bf55513108afd7f675d25feac0a8a7f385a2e625bfeefdb723f9205`.
No tracked or proposed source, model, AEP artifact or generated file was edited.
All writes are in assigned `.cache/http-values-review`. No Cargo, runtime target,
network operation, commit or external publication was run.

Paths below are portable: `mcp/` and `connectors/` identify the two repositories.
The root scope report is `mcp/.cache/mcp-next-runtime/report.md`; it is a design
input, not published product authority. The author report supplies the mapping
and explicitly separates proposed values from future behavior.

## Validation first

Used the pinned executable, verified as `ess 0.50.0`:

`ess specify validate --path .cache/http-values/model`

Exit 0, verbatim output:

```text
mcp v1 — 4 file(s), valid
```

The contained combined model's observation domain is byte-identical to the
proposed source (`cmp` exit 0). The new source is not yet enrolled in the root
manifest; this review does not relabel root validation as its validation.

## Design-to-model and reverse mapping

Each declaration has a design purpose; none introduces an entity, relation,
persistent identity, ownership rule or lifecycle.

| Design statement and exact quote | Model declaration and exact quote | Assessment |
|---|---|---|
| `mcp/.cache/mcp-next-runtime/report.md:34`: “a selected protocol revision enum” | `mcp/ess/domains/http_observations.yaml:11`: `variants: ["2026-07-28", "2025-11-25"]` | Both selected revisions; no invented fallback. |
| Scope report:34: “Do not equate sent with effects applied or unsent with a persistent attempt state.” | Domain:16: `variants: [not_sent, send_observed, unknown]`; :19: “proves neither full delivery, peer receipt nor business effects.” | Local send observation remains distinct from business effect knowledge; H4 owns truthful instrumentation. |
| `connectors/adapters/mcp/contracts/client/v1alpha1/invocation.md:95`: “Completion requires an observed transport end for a single-body response” | Domain:24: `variants: [correlated_terminal, incomplete]`; :26: “not business success or” and :27: “schema validity.” | Terminal classification is only one prerequisite, not a success/result discriminator. H4 retains correlation. |
| Invocation contract:86: “at most the admitted prefix”; :87: “A byte prefix may end inside a UTF-8 code” | Domain:59: `{name: bytes, type: Bytes}`; :64: `counts.retained_octets <= counts.limit_octets` | Exact byte carrier and inclusive bound; OctetCount, RetentionCounts and BoundedWireBytes serve this rule. H1 explicitly retains actual decoded-byte equality. |
| Invocation contract:92: “Implementations must not infer” and :93: “the full remote output length from an overflow prefix.” | Domain:45–46: `exact: mcp.http_observations.OctetCount` and `at_least: mcp.http_observations.OctetCount` | Length knowledge is explicit; H2 retains selected-count consistency. |
| Invocation contract:87: “incomplete observation” | Domain:52: `variants: [whole_message, prefix]`; :54: “no retained prefix can be returned as a complete result.” | RetentionKind separates full retention from prefix; zero counts/limits remain representable. |
| Invocation contract:117: “Preserve integer code, message and optional data as peer data, source peer and response stage.” | Domain:90–92 declares Integer code, String message, PeerData data; :93 says “never safe local diagnostic prose or a closed error”. | PeerError preserves an open integer vocabulary and untrusted payload; H3 owns correspondence to actual bytes. |
| Scope report:34: “Preserve absent-versus-null with explicit presence plus Json value or a supported tagged representation” | Domain:82–83: `absent: mcp.http_observations.AbsentPeerData` and `present: Json` | PeerData's tagged cases distinguish absence from present null; AbsentPeerData is only the required union payload. |

The singleton absence payload and the small count/retention types have explicit
representation purposes. No extra runtime entity or fabricated view is needed.
A later implementation should consume generated types and enforce their recorded
obligations, rather than create a second hand-transcribed model.

H1/H2 are acknowledged model-expression gaps with the author's retained ESS
refusals; H3/H4 are future observer obligations. This review does not classify
those disclosed, scoped deferrals as newly discovered missing implementation.
The domain currently has no aggregate exchange result and makes no aggregate
consistency guarantee. In the future actual target, complete malformed bytes must
remain distinguishable from terminal loss (invocation contract:102–104); enum
names alone do not prove that distinction.

## Generated codec obligations

Inspected generated `types.rs`, `source.schema.json` and `types-report.json`.
The structural Rust carrier uses String for Bytes and serde_json::Number for
Integer. It does not itself enforce padded base64, integrality, nonnegative
counts or the retained-count bound. The report records all nine obligations:
base64 pattern; BoundedWireBytes invariant; MessageLength oneOf and untagged
selection; OctetCount integer and invariant; PeerData oneOf and untagged selection;
PeerError code integer. Discriminated branches retain required `value` fields,
including present Json null. This is source inspection, not an executed decoder
round-trip or claim that arbitrary-precision integers fit a host integer type.

## Planted omission control — separate from actual findings

On a contained scratch copy, removed only PeerError's `data` field. The original
proposal and author scratch were unchanged. Command:

`ess specify validate --path .cache/http-values-review/planted-model`

Exit 0, verbatim output:

```text
mcp v1 — 4 file(s), valid
```

The design-review procedure then identifies:

| # | classification | design (quoted) | planted spec (quoted) | note |
|---|---|---|---|---|
| CONTROL-1 | missing | `connectors/adapters/mcp/contracts/client/v1alpha1/invocation.md:117`: “Preserve integer code, message and optional data as peer data, source peer and response stage.” | `mcp/.cache/http-values-review/planted-model/http_observations.yaml:91`: `- {name: message, type: String}`; no data field follows | The typed error no longer exposes the promised peer data or its presence. |

Returning to the unchanged actual source, domain:92 supplies that missing field
and PeerData:78–85 supplies the tagged presence distinction. Control outcome:
one planted `missing`, zero actual mismatches. This was an agent source-review
control, not a new automated verifier, mutation score or runtime failure.
`planted-findings.json` preserves the control separately; actual `findings.json`
is empty. The first two validations alone would not detect this design omission.

## Counts and limits

Actual classifications: missing 0; contradicts 0; stale mapping 0; spec-only 0;
unclear 0. Planted-only classifications: missing 1, all others 0.

Read the complete scope report and invocation contract. The selected immutable
observation declarations and their mapping were reviewed in both directions.
Transport implementation, deadlines, one-dispatch behavior, discovery bounds,
resource/prompt types, aggregate result classification, authority and credential
custody are future or excluded scopes, not claims verified by this pass.
Techniques 1–7 were not run. Technique 8 was applied as the explicitly requested
early design review; this new domain has no green runtime conformance suite.

No authored paths outside this worktree. Root owns retained scratch and cleanup;
this review releases only its own lease.

## Identities

- Author report: `f8c89f166cf00375ead730da8fe9432a06651f452de348629333edaf994f2e36`.
- Root scope report: `ec0a2489d8e6e5d526d62b3bee060e202eb9b8c9213afaf17ddec63c879e6320`.
- Connectors invocation contract: `346606033482b34ef76884a6d3dc090e9ac76b7e2f8dffe5195acee300b68b7e`.
- Generated Rust: `b1972834519212b10497a339872375a30f0bb9476d78bb95f57397b76dd38175`.
- Types report: `7ca7b6b4f298dfffdb1102452c2cb59307cfa7682e223c0c6ca03fcda2cdb61a`.

Raw control/validation identities are retained in `hashes.txt` beside this report.

```json
{"findings":[],"planted_control_findings":"planted-findings.json","runtime_conformance_executed":0}
```

```findings
[]
```
