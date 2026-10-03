---
format: aep.planning-md/3
id: review-result:strict-http-exchange-20261003
kind: review-result
status: active
title: Coordinator review of bounded raw HTTP exchange and compatibility profile
relations:
- reviews: story:strict-http-exchange
revision: 1
---
unit: story:strict-http-exchange; working tree over 47620e9089dbac880245bfca343860e2172cbf33
verdict: nothing additional found in final coordinator read-only pass
cases: final native groups10; actual ESS27passed; no new execution credited to this review
origin: introduced0 / pre-existing0 / undecided0 in this final pass
wrote-outside-worktree: none
needs-coordinator: bot publication and remaining MCP integration

## Review boundary

The same coordinator implemented and reviewed this change. All three delegated
agents were rechecked and still report account usage exhaustion. This is not an
independent-agent approval. The final review inspected the actual receiver,
fixture, target, generation gate, feature graph and scope claims; it did not mutate
production or tests. Earlier author tests and their fixes are disclosed below.

## Actual behavior and generated model

The additive strict-http feature owns one already-negotiated POST. Request bytes
come from the declared canonical base64 input without reserialization. Numeric
limits, positive times, host representability, real id correspondence and an
exclusive request envelope are checked before network dispatch. The caller owns
the admitted endpoint, headers and HTTP builder. Explicit no-redirect/no-retry and
no-decompression policies plus the selected connect deadline are installed on that
builder; no unrelated HTTP client, credential source, initialization, result cache,
authentication repair or follow-up exchange exists here. Send knowledge remains
unknown before observed response headers, and no local refusal implies rollback.

The receiver retains exact bounded original message bytes, observes real body or
SSE message boundaries, and only accepts an exclusive correlated result/error.
Raw JSON objects are parsed without reinterpreting serde-private-looking keys;
opaque results and present peer data are not decoded again through generated
structural codecs. Peer data absence and explicit null are distinct. SSE retains
incomplete data, counts raw event and assembled-message bounds separately, accepts
LF/CRLF/CR, multiline fields and a leading BOM, and does not promote EOF into a
missing event delimiter. HTTP statuses outside the declared100..599 range are
refused without inventing an admitted status value.

Twenty-two data types come from ESS0.50; all four generated artifacts are checked
byte-for-byte by the gate. Nineteen structural-codec obligations remain explicit;
the public codecs alone cannot validate forged observations. No manual parallel
model or fabricated persistent entity was introduced. The H7 refinement preserves
independence of send and terminal knowledge. The default dependency graph retains
its prior JSON feature selection; strict-http explicitly opts in. Cargo feature
unification means older SDK decoding paths in a mixed-feature consumer do not gain
the strict receiver's raw-object guarantees; this limit is documented.

## Acceptance evidence read

The native fixture independently parses actual HTTP and counts real business
requests. It verifies exact request bytes and the supplied revision. The ESS target
reads only command inputs, obtains actual library output, joins fixture teardown,
and returns actual counts and carrier fields. Startup, observation and teardown
failures are target errors. The27-scenario suite has26authored cases across two
revision inputs and zero synthesis refusals; all27passed. This is transport-envelope
coverage, not proof of real negotiation or typed modern-family semantics.

Author failing cases caught incomplete SSE data loss, a skipped BOM-prefixed data
field, an opaque object converted to a codec-private number, and unconstrained
HTTP status900. Each was corrected and its native case now passes. An author
mutation disabling correlation made exactly both wrong-id ESS cases fail:
25passed/2failed, with0error/unsupported/skipped. The exact source was restored.
The final Rust1.88 gate passed38native tests,0failed/ignored, and default build,
all-feature execution, formatting, Clippy, docs, ESS generation and AEP validation.
These runs are author evidence, not an independent hardening campaign.

## Remaining scope

The original constructor case bodies remain unchanged with regenerated digests.
Its original six partial refusals remain; HttpStatus and PositiveMilliseconds add
two explicit invariant-observer refusals. Three full authored refusals remain.
Report2's deterministic logical timestamp is not a wall-clock execution record.
Conformance remains inconclusive and coverage unknown outside the bound selection.

Real negotiation, family/revision acceptance, bounded discovery, resource and
prompt operations, reusable strict connections, Connectors admission/custody and
caller/stdio ownership decisions remain open. This source unit neither cuts the
requested MCP release nor completes that broader delivery goal.

```findings
[]
```
