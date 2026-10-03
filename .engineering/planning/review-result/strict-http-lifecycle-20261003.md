---
format: aep.planning-md/3
id: review-result:strict-http-lifecycle-20261003
kind: review-result
status: active
title: Coordinator review of strict HTTP and typed lifecycle
relations:
- reviews: story:strict-http-lifecycle
revision: 1
---
unit: story:strict-http-lifecycle; working tree over 8350fa6ee9615316e15832833e5912f6b78127b9
verdict: nothing additional found in final coordinator read-only pass
cases: 116 native tests; HTTP lifecycle 105, schema lifecycle 15, typed lifecycle 31 ESS scenarios
origin: introduced 0 / pre-existing 0 / undecided 0 in this final pass
wrote-outside-worktree: none
needs-coordinator: publish library candidate; implement remaining Connectors integration and final release

## Review boundary

The implementing coordinator reviewed the final controlled invocation/discovery
paths and their interaction with existing connection cancellation and retained
schema workers. Existing workers remain quota-exhausted. This is not independent
review or human approval; mutations are separately labeled author hardening.
No implementation or test edits were made during this read-only pass.

## Scope and observations

Control constructs one capped operation and teardown deadline. Admission stops
before business dispatch without fictional request ids. Existing private catalog
checks and modern parameter headers are reused. Schema input validation, output
preflight and output validation preserve their phase and actual child ownership.
The new bounded worker cleanup API is explicit; dropping a handle does not prove
exit. Controlled post-response failures attach the original exchange and do not
send a cancellation notification for an already terminal request.

Discovery invalidates admission before its first await. It uses the same parser,
limits and duplicate checks as raw discovery. Only a complete catalog is stored;
interrupted traversal returns prior observations and current-page cancellation,
never a usable prefix. An observed final page wins a later cancellation signal.
Direct generated constructors preserve opaque private-marker JSON objects.

The 31-case typed selection uses actual TCP peers and Rust workers. Peer counters,
request IDs/protocol headers, observed socket closure, PID barriers and process
absence after explicit reap are measured independently of expected output.
The 105 HTTP lifecycle and 15 worker lifecycle selections remain separate. The
full gate passed 116 native tests with no failed or ignored cases. Original
constructor synthesis limitations remain disclosed; no full conformance claim.

Author response-retention mutation: 27 passed / 4 failed. The four failed cases
are post-response output interruption and output IPC capacity in both revisions.
Restoration passes 31/31 three times and then the full gate. Earlier mutations
check retained-child truthfulness, legacy cancellation, exact progress increase
and cumulative control-byte bounds. Detailed evidence and SHA256 digests are in
the story's appended typed checkpoint and .cache/mcp-next-runtime/lifecycle.

## Remaining delivery

Connectors inbound/outbound authority and credential integration and the final
release remain unfinished. Local inbound and outbound HTTP can proceed without
answering multi-caller cloud assignment or outbound stdio process ownership.
No ownership decision is inferred. Cross-platform process teardown, exhaustive
random exploration and full MCP conformance are not established by this review.

```findings
[]
```
