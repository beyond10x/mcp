# MCP specification draft

This initial source-cited retrofit declares six shipped public value shapes and
four bounded constructor bindings. It is a draft, not complete MCP conformance.
The source baseline is `0fdfbafa7c130caa76d74825680d72af3280e21b`; runtime is
identical to the cited `51b9c7969def3dc0fbee9026cd26f4ce5837d62c` inventory.
No product entities, lifecycle state, events or persistent views are invented.

ESS 0.50.0 validates the three specification files. The same installed release
reproduced ESS 0.45.0's refusals during preparation; inspected 0.51.0 source retains
the constrained direct-return and count-invariant boundaries. A repository tool
pin and executable gate enrollment are the next work, not established by the
current structural result.

`ess-inputs.yaml` explicitly selects five authored constructor cases. All eight
cases remain under `conformance/scenarios/`: the three successful constrained-return cases
are preserved despite synthesis refusing them. A partial suite previously emitted
nine cases (four generated, five authored), with four remaining refusals.
`conformance/constructors.json` now executes against the real in-process constructor
target in `crates/b10x-mcp-types/tests/ess_conformance.rs`. Its ESS 0.50.0 runner
reports nine passed, zero failed/error/unsupported/skipped. A wrapper that flips
the actual returned `is_error` field fails the two named authored result cases;
the other seven cases still pass. The runner's report/2 execution status is
`passed`, but its conformance status remains `inconclusive`: coverage is undeclared,
the selection is partial, and the four synthesis refusals remain unresolved.

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

The current `cargo xtask gate` runs the target through workspace tests. Pinned ESS
structural validation, synthesis drift/refusal inventory and AEP validation are
still pending separate gate enrollment.
