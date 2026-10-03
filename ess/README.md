# MCP specification draft

This initial source-cited retrofit declares six shipped public value shapes and
four bounded constructor bindings. It also declares eleven immutable HTTP
observation types and eleven strict exchange input/result types. Their generated
structural library is used by the additive strict HTTP receiver; separate real
HTTP binding suites exercise the receiver and the older session-replay policy.
It is a draft, not complete MCP conformance.
The source baseline is `0fdfbafa7c130caa76d74825680d72af3280e21b`; runtime is
identical to the cited `51b9c7969def3dc0fbee9026cd26f4ce5837d62c` inventory.
No product entities, lifecycle state, events or persistent views are invented.

ESS 0.50.0 validates the five specification files. The same installed release
reproduced ESS 0.45.0's refusals during preparation; inspected 0.51.0 source retains
the constrained direct-return and count-invariant boundaries. `toolchain.json`
pins ESS 0.50.0 and AEP 0.68.0, their source revisions, tool archives and the exact
scenario/refusal inventory. The normal gate verifies those selections.

`ess-inputs.yaml` explicitly selects five authored constructor cases. All eight
cases remain under `conformance/scenarios/`: the three successful constrained-return cases
are preserved despite synthesis refusing them. A partial suite previously emitted
nine cases (four generated, five authored), with four remaining refusals. Adding
the HTTP observation domain retains those cases and adds two uncovered type-invariant
refusals. The exchange domain adds two more for HttpStatus and
PositiveMilliseconds, for eight partial-synthesis refusals in total.
`conformance/constructors.json` now executes against the real in-process constructor
target in `crates/b10x-mcp-types/tests/ess_conformance.rs`. Its ESS 0.50.0 runner
reports nine passed, zero failed/error/unsupported/skipped. A wrapper that flips
the actual returned `is_error` field fails the two named authored result cases;
the other seven cases still pass. The runner's report/2 execution status is
`passed`, but its conformance status remains `inconclusive`: coverage is undeclared,
the selection is partial, and the eight synthesis refusals remain unresolved.

[Original obligations](obligations.md) retain eighteen constructor/HTTP names.
[Coverage](coverage.md) records the restricted input profiles, omissions and
unresolved semantics. Constructor checks do not establish HTTP negotiation,
provider effects, persistence, OAuth atomicity or caller/process authority.
Public serde values do not universally enforce constructor invariants. Decoded
JSON is not original wire bytes; `Some(null)` and `None` may collapse at the
explicit optional-value serialization boundary.

The Rust target invokes actual constructors, receives only actual inputs, and
projects returned values/errors. Its ESS libraries are dev-only dependencies pinned
to released 0.50.0 commit `8700d0808e8f3b19711629d8a17afc5281680f58`.
Run `cargo test --locked -p b10x-mcp-types --test ess_conformance -- --nocapture`
to see the actual runner reports for both the real target and wrong-return control.
Any synthesis refusal,
missing native invariant observation, skipped case or undeclared coverage remains
visible; do not weaken nominal constraints or fabricate state to remove it.

`cargo xtask gate` and `task check` validate the specification, regenerate and
compare the partial suite byte-for-byte, check all nine partial refusals and all
three full authored-selection refusals, validate AEP and run the target through
workspace tests. Unexpected synthesis success is drift requiring review, just as
a new refusal or missing case is. The narrower `cargo xtask specification` performs
the document checks only; it does not execute conformance. Tool setup and exact
version selection are documented in the root README and AGENTS.md.

The proposed [HTTP observations](domains/http_observations.yaml) preserve exact
octets, explicit length uncertainty, local send/terminal observations and peer-error
data presence. They introduce no entity, durable record, authority or effects claim.
H1–H4 retain the actual byte-count, cross-field, error-to-message and instrumentation
obligations. Rust type projection succeeds but represents Bytes as padded-base64
String and Integer as `serde_json::Number`; structural decoding does not enforce
base64, integrality or invariants. The strict receiver guards its admitted inputs
and produces observations from actual bytes. The gate regenerates all four data
library artifacts, the separate 43-scenario strict HTTP binding suite and the
32-scenario connection setup suite. Ten connection values add to the original
22 exchange values; 21 runtime codec obligations remain. The new CacheHints
invariant adds one explicit synthesis refusal to the previous eight.
See [coverage](coverage.md) for what this execution does and does not establish.
