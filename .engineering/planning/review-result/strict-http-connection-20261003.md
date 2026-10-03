---
format: aep.planning-md/3
id: review-result:strict-http-connection-20261003
kind: review-result
status: active
title: Coordinator review of revision-fixed strict HTTP setup
relations:
- reviews: story:strict-http-connection
revision: 1
---
unit: story:strict-http-connection; working tree over b348cb24845146762abe9e5b6712876cd1dec9a8
verdict: nothing additional found in final coordinator read-only pass
cases: native 41 -> 49 groups; connection ESS 32; existing strict HTTP ESS 43
origin: introduced 0 / pre-existing 0 / undecided 0 in this final review
wrote-outside-worktree: none
needs-coordinator: source publication and remaining full MCP delivery

## Review boundary

The implementing coordinator performed this separate review. The existing workers
are quota-exhausted; this is not independent adversary evidence. No production
change arose from the final review. Earlier author mutation and red/green controls
remain author observations, not an independent approval.

## Observed behavior

Modern setup performs server/discover with agreeing per-request metadata and
headers. Legacy setup verifies the configured revision, validates an optional
unique bounded visible-ASCII session, and requires an empty HTTP202 acknowledgement
of notifications/initialized. Both use one caller deadline across setup phases,
one admitted endpoint and one reusable client. Failure returns no usable handle.
The private session uses SecretString, sensitive request headers and redacted
handle Debug; descriptions retain raw untrusted peer JSON and typed observations.
Modern session headers are ignored. Family presence gates later raw exchanges;
these observations are not typed family-result acceptance or authority grants.

The independent HTTP parser observes actual calls, bytes and headers, with joined
fixture teardown. The ESS target consumes inputs, never scenario expectations.
Revision mismatch, invalid descriptions, sessions, cache fields, metadata conflict,
request limits, peer errors and notification/deadline failures are exercised.
Opaque reqwest builder defaults cannot be inspected; the explicit C4 caller-port
precondition is documented, not claimed as library enforcement.

Initial executable stub red: zero passed / one failed, exit101. Disabling both
revision guards later produced exactly two failed setup cases and thirty passes;
restoration passed all32. The final extra numeric probe initially expected lexical
-0 preservation in a typed Integer and failed (actual0); this was an incorrect
oracle, not a production defect. Corrected expectations verify successful setup
with zero and an integer beyond u64, without changing production code.

Rust1.88 full gate exited0:49 native passed,0failed/ignored. Both setup32 and strict
HTTP43 target assertions passed, alongside existing replay5 and constructor9.
Fmt/default build/all-feature tests/Clippy/docs, deterministic ESS outputs and AEP
passed. Nine partial model synthesis refusals (including CacheHints invariant) and
three authored refusals remain explicit; 32 generated values carry21 structural
codec obligations. Neither structural projections nor these selected runtime
cases establish full MCP conformance. Initial lint failures were corrected without
suppressions. Retained logs: .cache/mcp-next-runtime/connection/{red-runtime,
revision-mutation,ttl-red,ttl-green,gate-final}.log.

Final gate SHA256 a0c7259209db628912afbb0189937736a1cce991a8e0d2411f8b5c15bedc3396.
Revision mutation SHA256 43a06044d57cd7f47972fe9db5830bf5991a59cc1b7b9b29ade4ba2107705d90.
Initial runtime red SHA256 77129a4ab1ce5503581455d0558acbcdc57e838a7effbb7d9ceae0bbc94fcb82.

## Remaining delivery

Bounded list discovery, typed tools/resources/prompts, progress/cancellation/shutdown,
inbound/outbound consumer integration and the MCP release remain unfinished.
No consumer ownership or process-supervision decision is inferred. No cache,
paging, automatic fallback, retry, recovery or family-result acceptance is claimed.
Random sequences, concurrency models and exhaustive formal guard analysis were not
run; this immutable-value/setup unit used actual wire scenarios and one targeted
revision-guard mutation only.

```findings
[]
```
