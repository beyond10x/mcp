---
format: aep.planning-md/3
id: review-result:adoption-values-20261002
kind: review-result
status: active
title: Source review of partial MCP value specification
relations:
- reviews: specification:governed-compatibility-baseline
revision: 1
---
Review: bounded source/spec review of the first MCP adoption draft
Base: 0fdfbafa7c130caa76d74825680d72af3280e21b; reviewed additions uncommitted
Scope: six public value shapes, four constructor bindings, eight authored cases, partial manifest and scope prose
Reviewer execution: zero tests, builds, synthesis runs or network calls; source/log inspection only
Inherited evidence: gate exit 0, 15 tests passed; validation exit 0; full synthesis exit 1; partial synthesis exit 0, nine emitted cases and four refusals; zero target executions
Result: nothing found in this bounded review; no approval or independence claim

The declarations match the inspected public source at this checkpoint. `ConnectionId` alone receives nominal alphabet/length invariants (`ess/domains/values.yaml:8-15`), supported by both its private representation and constructor-backed serde conversion (`crates/b10x-mcp-types/src/lib.rs:80-100,122-127`). Descriptor, snapshot, call and result fields remain permissive public serde values: no universal constructor-only object, name, digest, duplicate or raw/flag coherence requirement was added (`values.yaml:17-54`; source types:162-176,221-232,269-275,295-318). HTTP configuration describes the materialized value and leaves default-on-input and transport validation distinct (`values.yaml:56-63`; types:40-48; client:138-150,256-298).

The optional-value projection is explicitly lossy in its one named respect: absent output schema and `Some(JSON null)` both serialize to null. `values.yaml:24-27` and `ess/README.md:25-27` disclose that boundary. The authored descriptor case supplies no output schema and expects null; it does not claim to distinguish the two Rust values. Raw JSON is selected-codec data, not original bytes. General JSON numeric/boundary behavior is outside these bounded constructor inputs and awaits a target; this review does not certify an unimplemented projection.

Four binding definitions invoke existing methods, with actual raw arguments documented separately from expected returns (`ess/domains/constructor_checks.yaml:31-97`). The allowed ASCII input profiles make byte/count agreement explicit. Invalid lengths still reach `ConnectionId::new`; forbidden-alphabet inputs are honestly omitted. Descriptor/result profiles remain far below the actual default 65,536/262,144 byte limits (`types:66-76,179-214,307-318`). Empty snapshot input and its literal protocol are not represented as negotiated compatibility. No product entity, lifecycle, view, ownership or event was invented (`ess/system.yaml`, `values.yaml:65-89`, constructor checks:99-104).

The partial manifest selects exactly the five authored cases named in `ess-inputs.yaml:8-13`. The valid-ID, maximum-ID and snapshot cases are retained as authored files, and the full synthesis log reports those exact three constrained-return refusals. The retained partial suite contains four generated and five authored cases, including a 65-character generated overlength input. Its four refusals are the two constrained direct returns and the two count-invariant observers. `ess/README.md:15-19` and `ess/coverage.md:7-24` correctly separate authored, emitted, refused and executed counts. No MCP target exists or has executed here. The eighteen obligations are labeled provisional prose (`ess/obligations.md:3,37`), with HTTP effects and timing still deferred; the cited HTTP source supports the described admission/post-provider distinction (`client:46-70,128-150,224-247`).

Delta inspected: only the new ESS declarations, manifest, scenarios and scope prose; runtime source was not changed. A read-only diff confirms the reviewed types/client sources are identical between the cited inventory commit and current base. No tracked files, planning artifacts, implementation, suite or test were edited by this reviewer. The only reviewer output is this report. The reviewer-owned worktree lease is released at handoff; the root agent retains ownership of implementation and next steps.

Retained evidence SHA256:

- `validate-final.txt`: `e2f4b2434f8ce310c2fa86661c4fc2f1f7e0e7f65608d8e20be2d640fe102db8`
- `full-synthesis.log`: `5477c8ca754bb1b18ffbeba0f3102e30a0096866983fc5dba0f4a8252cf3a75a`
- `partial-synthesis.log`: `765c9178b5c94b6754fe6157603f0843bf095290a8b20354624a2b9f227935b4`
- `partial-suite.json`: `a53a492f82fec31fa14005cba55cf55cc183f0b85811f58476dfde0b6ec4cab7`
- `ess/domains/values.yaml`: `878bc843ff871ee7981b2d54b104ed00f16fe6a9d47304882994bd0536461b84`
- `ess/domains/constructor_checks.yaml`: `c6060223e30dcc1cd31e56b7172f589ead866b27410e01531cfd77534226b941`
- `ess-inputs.yaml`: `667f9114b0916a085ed83c948f94c172a87b4d88fb6d21cc0237271ca6631f96`

```yaml
findings: []
```
