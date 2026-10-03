---
format: aep.planning-md/3
id: review-result:strict-http-discovery-20261003
kind: review-result
status: active
title: Coordinator review of bounded selected-family HTTP discovery
relations:
- reviews: story:strict-http-discovery
revision: 1
---
unit: story:strict-http-discovery; working tree over5801adc8092b95da96b74c0975ccd79e2779c719
verdict: nothing additional found in final coordinator read-only pass
cases: native49 ->57; new discovery111scenarios,110authored
origin: introduced0 / pre-existing0 / undecided0 in this final review
wrote-outside-worktree: none
needs-coordinator: source publication and remaining full MCP delivery

## Review boundary

The implementing coordinator performed this separate read-only review because the
existing delegated workers remain quota-exhausted. This is not independent review
or a human approval. Author red/green and mutation observations remain labelled as
such; no test or production edit arose in this final review.

## Scope checked

The actual StrictConnection traversal selects exactly tools/resources/prompts and
uses already advertised capability presence, one endpoint and configured revision.
Limits are nonnegative host-representable values checked before list I/O. The next
page is refused before dispatch past max_pages; rows and compact descriptor bytes
are bounded as accumulated. Exact wire-byte bounds remain the receiver's distinct
axis. Zero pages refuses without a list; a zero item budget admits an empty final
list. Every page shares the same monotonic deadline capped by execution/provider
budgets. Every failure retains actual exchanges and yields no completed catalog.

The pinned pagination chapter explicitly permits an empty cursor and requires
opaque handling. Both empty and repeated values are sent unchanged, with the page
ceiling providing a bound. This corrects the earlier proposed repeated-token
refusal in scope prose. Present null remains malformed under the optional-string
schema. Each family preserves ordered typed descriptors and raw unknown JSON;
unique names/tools/prompts and URIs/resources prevent an ambiguous catalog.
Prompt argument names are unique. Descriptor shapes are checked, not arbitrary
JSON Schema semantic validity or supported-dialect invocation admission.

Raw opaque JSON is assigned directly after structural placeholder decoding so
codec-private marker-shaped objects stay objects. Resource size follows upstream
JSON number, including fractional values; no invented Integer narrowing. Modern
pages require complete resultType/cache hints; legacy gains no synthetic cache.
No retry, recovery, response cache, link fetch, template or subscription is driven.

## Runtime evidence and author controls

Initial stub red:0passed1failed,exit101; real setup was observed but no list calls.
After implementation six native groups passed. An additional author probe found
that generic annotations validation wrongly rejected an unknown prompt extension;
its first run failed. Prompt has no standard annotations field in either selected
schema, so the corrected code preserves it opaquely; both revisions then passed.
That native case and two ESS cases remain. This is a production defect found by
an author probe, not a false positive or an independent adversary finding.

Full Rust1.88 cargo xtask gate exited0:57nativepassed0failed0ignored28summarylines,
including target assertions for discovery111,setup32,strictHTTP43,replay5 and
constructor9. Fmt/default build/allfeature tests/Clippy/docs and deterministic ESS
outputs/inventories passed. Namespace-authoring and Clippy failures were corrected;
no lints suppressed. Product projection43types/27codecobligations; nine partial
model synthesis refusals and three authored refusals remain explicit.

Author mutation changed terminal cursor handling to stop on empty strings. The
actual ESS target failed exactly six empty-cursor scenarios (three families,two
revisions):105passed6failed0error/unsupported/skipped. Restoring the exact source
returned111passed0failed/error/unsupported/skipped. The mutation cannot ship: cmp
verified restoration before the green run. Default report timestamps are the
runner's logical clock; execution evidence is the enclosing actual gate/log record.

Evidence under .cache/mcp-next-runtime/discovery/:
red.log SHA25685b4a03e69a86b29f462c96f44cee3fed11695318baddc3f6d1f98fbbe4e229f.
prompt-extension-red.log SHA25670ab46473212937dc68f754dfdb8c9fb91791c5da012a9748e85791340fadb92.
prompt-extension-green.log SHA2567a7f40f58db998d60e12b77d10c337d344127f7bdf657ff873f2bcf5d0f6aae0.
gate-first.log SHA2569adf7b60b912e4c13c4f689de5e9e867b4252611b8f9610a25684bde29daa9a7.
cursor-mutation.log SHA256e1eb405fd4ba4ee7c1921ead889899febba3dcd1c861737ae772dd4043fe4863.
restored.log SHA256cae1b26c0acc21792e96f09746b93b74a2873bf5c428781ef68bba382dd16dd7.
SuiteSHA256aea9df30389637bae6cb6c5a881bc3f42764fbd69b09147a0ef81dfc604d68c7.

## Remaining scope

Typed call/read/get results, output-schema semantic validation, progress,
cancellation/shutdown and actual Connectors inbound/outbound admission/credential
integration remain unfinished. Open caller/Connection and stdio ownership decisions
are not resolved. No tag, release, consumer pin or full conformance claim follows.
Random sequence, concurrency and exhaustive guard analyses were not run. A bounded
cursor mutation and real-wire matrix establish only the selected behaviors.

```findings
[]
```
