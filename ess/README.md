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
nine cases (four generated, five authored), with four remaining refusals. No case
has executed against an MCP target yet. Do not count generated cases as passes.

[Original obligations](obligations.md) retain eighteen constructor/HTTP names.
[Coverage](coverage.md) records the restricted input profiles, omissions and
unresolved semantics. Constructor checks do not establish HTTP negotiation,
provider effects, persistence, OAuth atomicity or caller/process authority.
Public serde values do not universally enforce constructor invariants. Decoded
JSON is not original wire bytes; `Some(null)` and `None` may collapse at the
explicit optional-value serialization boundary.

The planned Rust target must invoke actual constructors, receive only actual
inputs, and project only returned values/errors. A deliberately incorrect result
must make a named case fail before execution is credited. Any synthesis refusal,
missing native invariant observation, skipped case or undeclared coverage remains
visible; do not weaken nominal constraints or fabricate state to remove it.

The current repository gate is still `cargo xtask gate`; it has not yet acquired
ESS or AEP checks. The first adoption checkpoint preserves that fact explicitly.
