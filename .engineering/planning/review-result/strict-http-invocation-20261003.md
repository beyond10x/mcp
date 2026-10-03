---
format: aep.planning-md/3
id: review-result:strict-http-invocation-20261003
kind: review-result
status: active
title: Coordinator review of typed strict HTTP invocation
relations:
- reviews: story:strict-http-invocation
revision: 1
---
unit: story:strict-http-invocation; working tree over5a854ba7bbfdacc63f447df1f00ade0aeedec723
verdict: nothing additional found in final coordinator read-only pass
cases: full Rust1.88 gate81nativepassed0failed0ignored;54ESSinvocation scenarios
origin: introduced0 / pre-existing0 / undecided0 in this final review
wrote-outside-worktree: none
needs-coordinator: source publication and remaining full MCP integration/lifecycle/release

## Review boundary

The implementing coordinator performed this separate review because the existing
delegated workers remain quota-exhausted. This is not independent review or human
approval. Author probes and mutations remain labeled as author evidence. The final
read-only pass made no implementation or test edits.

## Scope checked

Twenty-one invocation values were modeled/generated before their story scope,
including required modern parameter-header projection. Total64 generated types
retain31 structural-codec obligations. Four generated artifacts match regeneration.

Invocation owns StrictConnection and obtains selected catalogs through that handle.
Failed refresh invalidates old admission. Modern invalid header definitions leave
the usable projection; raw pages and typed rejections survive. Public returned
catalogs cannot mutate internal admission. No server hint supplies consumer authority.

The admitted absolute-path Rust schema worker uses empty environment, no shell,
bounded IPC and the invocation deadline. JSON Schema2020-12 executes offline;
unsupported root dialects/retrieval refuse. Awaited failures kill/reap; dropped
futures initiate termination without claiming an observed reap barrier. The fixture
worker is explicitly test-feature gated. Header projection distinguishes schema
locations from const/enum/examples data, exact paths and safe integer values;
only Mcp-Param headers are produced, with sensitive values and safe encoding.

Typed results preserve order, opaque fields, URI and text/blob representations,
structured null versus absence, business errors and peer error observations.
Unsupported content never returns a successful prefix. Modern complete selection
and resource cache fields are validated without enabling cache. Schema mismatches
stay malformed output. Pre-dispatch request bounds map to caller input; worker
capacity after a response maps to unavailable validation with the response retained.
No automatic retry, URL retrieval or prompt execution is introduced.

## Executed evidence

Rust1.88 cargo xtask gate exited0:81 native tests,0failed0ignored,33summarylines.
All54 invocation scenarios passed (53authored), alongside constructor9/replay5/
strictHTTP43/connection32/discovery111. Fmt, default workspace check, all-feature
tests, Clippy and docs passed. Nine partial product synthesis refusals and three
authored refusals remain explicit. Test-only long fixture functions have narrow
readability lint allowances; runtime checks and the gate remain enforced.

Author probes found and corrected worker IPC reinterpretation of private-number
marker objects (4pass1fail ->5pass), pre-dispatch request-bound classification and
post-response worker-capacity classification. Each has a retained regression.
Actual worker timeout/reap and excess-output tests exercise owned children.

Output-schema bypass failed exactly both revision mismatch scenarios in the then
52-case suite:50pass2fail; restoration returned52pass. Suppressing parameter headers
failed exactly the modern header-agreement scenario:51pass1fail. Both mutations
were restored before the54-case green full gate. These are author controls.

Thirty pinned Connectors complete-response vectors ran through real setup,
discovery and invocation. Only id1->3 changes; fields and response bytes are checked.
Ten document input/framing vectors are explicitly excluded, not reported as passes.
The initial replay test read a nonexistent observation key; correcting that test
oracle to response.bytes required no production change.

Evidence under.cache/mcp-next-runtime/invocation:
gate-third.log SHA2565fbea7a3180c5e7099c5f5622a0b4b0468aa620b67862baf36652372c5b7f1b6
output-mutation.log SHA256410b947606ca7227ed2117d75a10f6a83ed388391377daac4195daaa8bed0679
header-mutation.log SHA256b3941c30d9609251e66a3f4988ac331cf19bcbbd64a5c7e5c3ec8e3d6eb1ea12
suite SHA256c4c642077b9ed88d993aef364f9437d4b2288a60a3e62464f99520c11e4adf82

ESS report timestamps use the deterministic runner clock; enclosing tool runs/logs
establish actual execution. No independent or human approval is fabricated.

## Remaining full delivery

Progress/server requests, cancellation/shutdown and actual Connectors inbound and
outbound authority/credential integration remain unfinished. Pending caller mapping,
MCP stdio ownership and external-family decisions are not inferred. This unit does
not complete full MCP or justify the final release tag. Random exploration,
exhaustive vocabulary/regex audits and cross-platform reap are not claimed.

```findings
[]
```
