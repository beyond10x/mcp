---
format: aep.planning-md/3
id: review-result:strict-http-status-20261003
kind: review-result
status: active
title: Coordinator review of revision-specific HTTP error observations
relations:
- reviews: story:strict-http-status-observations
revision: 1
---
unit: story:strict-http-status-observations; working tree over 25e74bac1e3110cde18d4d3a1f62ce39262cd047
verdict: nothing additional found in final coordinator read-only pass
cases: native 10 -> 13 groups; ESS 27 -> 43; author red 11 passed / 2 failed
origin: introduced 0 / pre-existing 0 / undecided 0 in this final review
wrote-outside-worktree: none
needs-coordinator: source publication and remaining full MCP delivery

## Review boundary

The coordinator implemented this change and performed this separate read-only
review. All three delegated agents still report quota exhaustion through October9.
This is not independent adversary evidence. No tests or production files changed
in this review. The initial failing run and final gate are author observations.

## Scope and evidence

The class is HTTP status interpretation across configured protocol revisions.
Modern 400/404 can carry exclusive correlated JSON-RPC errors. The receiver retains
actual bounded bytes, valid arbitrary integral error codes, untrusted message and
absent/present-null data, and reports the actual HTTP status alongside the peer
error. Success, malformed, duplicate-id and wrong-id bodies under error statuses
cannot be promoted to success. Authentication failures remain distinct. Capacity,
body loss and deadlines still take their existing observation paths. No new request,
repair, fallback, cache or endpoint retarget is introduced.

A 404 reports session_expired only for legacy requests with an explicit session
header on the supplied request. A modern header does not create a protocol session.
The low-level admitted-request caller remains responsible for valid session origin;
this API cannot prove initialization or inspect hidden builder default headers.
If the builder adds an otherwise invisible session header, classification remains
conservative http_status. The future strict connection must own that header port.

The fixture verifies actual session-header presence, exact request bytes and
revision, counts actual business POSTs and joins shutdown. The ESS target reads
only inputs and maps actual peer code/status and observations; expected values
remain authored scenario data. All16 added scenarios execute. The old modern
session-expired expectation is corrected with pinned-source justification; the
legacy counterpart now actually sends the fixture session header. No case removed.

The full Rust1.88 gate exited0:41 native passed,0failed/ignored, plus actual strict
43scenario assertions. Formatting, default feature build, all-feature tests,
Clippy, documentation, ESS projections/inventories and AEP validation passed.
The first gate failed on a duplicated fixture match arm; merged patterns corrected
it without a lint suppression. Constructor bodies are unchanged; generated model
fields are unchanged and only their source/schema digests move. Eight partial and
three full-authored synthesis refusals remain; no new full conformance claim.

Next connection work must follow the actual Connectors revision-fixed contract,
not rmcp Auto defaults. Modern has no initialize/session; legacy verifies one
configured revision. Bounded discovery, strict metadata/header agreement, family
APIs, inbound/outbound integration and release remain unfinished.

```findings
[]
```
