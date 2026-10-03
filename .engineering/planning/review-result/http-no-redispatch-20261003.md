---
format: aep.planning-md/3
id: review-result:http-no-redispatch-20261003
kind: review-result
status: active
title: Coordinator review of actual HTTP no-redispatch correction
relations:
- reviews: story:http-no-implicit-redispatch
revision: 1
---
unit: story:http-no-implicit-redispatch; working tree over c45e1a74125ac1c4cbba79fa90fe1cb098cb18c8
verdict: nothing found
cases: added 0; no additional execution credited to this review
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: publication and remaining MCP delivery

## Scope and limitation

Separate coordinator read-only review after the implementation gate passed. All
three delegated agents remain unavailable at an account usage limit. The same
coordinator implemented this change; this is not independent-agent approval.
No implementation, test, specification or planning file was changed in this pass.
The reviewed author diff includes two explicit SDK policy selections, the owned
fixture and actual ESS target, five generated/authored scenarios, gate inventory
and pin checks, documentation and planning. The earlier scratch result/input
proposals remain unpromoted; this runtime fix does not adopt those carriers.

## Checked acceptance and callers

Source enumeration finds exactly two StreamableHttpClientTransportConfig
constructors, both explicitly disabling session recovery. Public signatures,
caller-supplied client forwarding, snapshot construction, authentication headers
and other transport settings are unchanged. The source change covers ordinary
POST recovery, including tool calls, without deriving authority from annotations.

The new ESS binding is exercised by the real library, not a fixture's own return.
The target consumes only command inputs; it does not read scenario expectations.
The server parses bounded HTTP headers/body and JSON-RPC independently of rmcp,
requires the selected tool and actual session header, and counts received business
requests and initializations. Connection close must succeed; fixture teardown is
joined before counters are returned. Startup/framing/teardown failures become
TargetError, not expected refusals or fabricated counters. Positive controls
assert returned provider text alongside one-request counts. The first expired
reply permits a subsequent call to succeed, so hidden redispatch cannot hide
behind repeated failure.

Author evidence: initial actual probe had 2passed/2failed, both expired cases
showing2calls/2initializations/success. Fixed probe had4passed. The retained ESS
target executed5passed/0failed/0error/0unsupported/0skipped. Author mutation of SDK
recovery made exactly default-expired and injected-expired fail, with3passed/2failed;
production source was restored. These are the author's observed runs, not new
review executions. Full Rust1.88 gate then passed27native tests including the ESS
runner, formatting, Clippy and documentation. No independent mutation campaign
or added case was performed in this review.

## Limits checked

Coverage explicitly remains inconclusive/unknown outside this five-scenario
selection. Existing constructor inventory and refusals remain unchanged. The
logical runner clock is not called the wall-clock execution time. Modern strict
results, resources/prompts, general disconnect/retry behavior, SSE resume,
credential custody and consumer ownership are not claimed complete. The broader
MCP goal is not discharged by this fix. No confirmed additional finding arose
from this bounded read-only pass.

```findings
[]
```
