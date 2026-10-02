# Scope against the original18 obligations

This is a **partial constructor-binding draft**, not the full MCP API contract. Original named obligations remain in `obligations.md`. Six public value shapes are retained; two additional types are explicitly binding-only bounded input profiles. Neither excluded input shapes nor emitted scenarios are counted as passing.

The supported selection now executes against actual `ConnectionId::new`,
`ToolDescriptor::from_raw` and `ToolResult::from_raw`: nine passed (four generated,
five authored), zero failed/error/unsupported/skipped. This increases actual
execution from zero to nine; it does not resolve any synthesis refusal. Full
authored selection still refuses `id-valid`, `id-max-length` and `snapshot-literal`;
partial synthesis still emits four refusals. The snapshot command has no execution
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
