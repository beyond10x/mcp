---
format: aep.planning-md/3
id: story:strict-http-invocation
kind: story
status: implemented
title: Deliver typed strict tool resource and prompt invocation
relations:
- serves: vision:consumer-owned-mcp-mechanics
- derived_from: specification:strict-http-exchange-scope
- depends_on: story:strict-http-discovery
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: conformance/constructors.json
- confidence: inferred
  path: conformance/strict-invocation
- confidence: inferred
  path: crates/b10x-mcp-client/src/bin
- confidence: cited
  path: crates/b10x-mcp-client/src/parameter_headers.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/schema_worker.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_connection.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_discovery.rs
- confidence: cited
  path: crates/b10x-mcp-client/src/strict_http.rs
- confidence: inferred
  path: crates/b10x-mcp-client/src/strict_invocation.rs
- confidence: inferred
  path: crates/b10x-mcp-client/tests
- confidence: cited
  path: crates/b10x-mcp-types/src/http_exchange_generated
- confidence: cited
  path: crates/xtask/src/specification.rs
- confidence: cited
  path: ess
- confidence: cited
  path: ess-inputs.yaml
- confidence: inferred
  path: toolchain.json
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T05:09:19Z", actor: "human:timo", revision: 4, executor: "agent:codex-coordinator"}
- {from: "proposed", to: "active", at: "2026-10-03T05:09:19Z", actor: "human:timo", revision: 5, executor: "agent:codex-coordinator"}
- {from: "active", to: "implemented", at: "2026-10-03T06:09:00Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":3,"review_outcome":1}}}
---
# Complete typed strict HTTP tool, resource and prompt invocation

## Acceptance

Run named real-wire scenarios invoke-tool-content, invoke-tool-structured-presence,
invoke-business-error, invoke-output-schema, invoke-input-before-dispatch,
invoke-resource-content, invoke-resource-cache-fields, invoke-no-stale-success,
invoke-prompt-content, invoke-prompt-input, invoke-unknown-content,
invoke-unselected-result, invoke-peer-error, invoke-incomplete, invoke-deadline
and invoke-no-followup for both configured revisions. Reuse the forty literal
Connectors invocation document vectors where they apply, but establish actual
runtime evidence instead of claiming the document checker as conformance.

Private runtime catalogs must originate from actual discovery on the same owned
strict connection. A public deserialized ESS catalog is not that evidence. Caller
selection and grants remain external; no server hint authorizes a side effect.
Reject unknown catalog names and invalid/missing tool arguments or declared prompt
string arguments before dispatch. Preserve exact modern header/metadata agreement.
Validate supported declared input/output JSON Schema semantics, including local
references and exact large JSON integers. Modern defaults to2020-12; unsupported
dialects/retrieval stay named refusals. No schema fetches a URL or local file.

Preserve ordered text/image/audio/embedded-resource/resource-link blocks, raw
unknown fields, original URIs and representations. No link is fetched or prompt
instruction executed. Unknown content/result discriminators are unsupported with
retained bounded exchanges, no partial success or MRTR. Modern requires complete
resultType. Modern structuredContent preserves any JSON including null; legacy
requires an object. isError remains peer business data, not a protocol error or
rollback claim. Resource results require modern cache metadata without enabling
cache/stale fallback. Peer errors, malformed peer output, local caller input and
incomplete transport stay distinguishable; no automatic redispatch occurs.

## Isolated schema validation

Use a one-shot Rust worker with clap derive, a caller-admitted absolute executable
path, empty environment, no shell, bounded stdin/stdout and an absolute deadline.
The parent kills and reaps it on timeout/protocol failure; cancellation initiates
termination via kill-on-drop and awaited completion proves reap. Never claim an
async timeout preempts a synchronous validator. Offline JSON Schema2020-12 is the
selected profile; retain exact numbers and vocabulary semantics. Worker results
carry only a safe status, no schema/instance/error text. Test schema-valid,
schema-invalid, schema-reference-offline, schema-large-number, schema-present-null,
schema-worker-timeout and schema-worker-excess-output with actual owned children.
This worker is internal validation, not an MCP stdio server; the pending outbound
stdio supervision decision is unchanged.

## Model and scope

ESS mcp.http_invocation validates first:17 immutable values join the existing43.
S1–S2/I1–I6 retain runtime, process and model consistency obligations. Text/blob
resource fields are not falsely exclusive; preserve both when present. Cited:
strict_connection/discovery, generated types, ESS manifest and generation gate.
Inferred:strict invocation/result modules, schema worker/binary, its optional
jsonschema+clap dependencies, real HTTP/process fixtures, native and ESS targets,
conformance suite and gate inventory, README/CHANGELOG/coverage. One implementation
writer; unavailable quota-exhausted workers require disclosed coordinator review.

## Remaining full delivery

Progress/server-request handling, cancellation/shutdown and actual Connectors
inbound/outbound policy/credential integration still follow. No durable custody,
caller/Connection assignment, external process binding or release is inferred.

## Required modern parameter-header projection

Pinned modern streamable-http.mdx lines356–508 requires support for x-mcp-header,
not merely Mcp-Method/Mcp-Name. Four additional immutable values validate before
this scope extension (64total generated types). Invalid annotations exclude that
tool from the usable tools projection while retaining raw discovery and a typed
rejection; valid siblings remain usable. Test nonempty HTTP token names,
case-insensitive uniqueness, only string/integer/boolean, and only properties
chains from the schema root. Instances under const/enum/examples are data, not
schema annotations. Annotated integer values must be within ±(2^53−1). Extract
exact argument paths, omit missing values, encode unsafe/sentinel strings and
send only Mcp-Param-prefixed headers agreeing with the JSON body. Legacy does not
gain modern header semantics. Named scenarios invoke-parameter-headers,
invoke-parameter-header-refusal and invoke-parameter-header-missing cover actual
wire requests, invalid-tool exclusion with a valid sibling, and zero-dispatch
unsafe input. This is required runtime behavior, not an optional future feature.
