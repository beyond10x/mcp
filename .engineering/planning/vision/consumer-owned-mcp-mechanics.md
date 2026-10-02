---
format: aep.planning-md/3
id: vision:consumer-owned-mcp-mechanics
kind: vision
status: draft
title: Reusable MCP mechanics under consumer authority
revision: 1
---
## Outcome

Provide reusable MCP protocol and transport mechanics while each consumer keeps
its own publication, invocation, egress and credential-custody authority. Preserve
named refusals, bounded complete results, frozen discovery and existing tools-only
consumers as selected capabilities are extended.

## Evidence

- README.md:1–16 describes the tools-only Rust workspace, real stdio/HTTP fixture
  tests and preferred/fallback protocol revisions.
- README.md:51–59 assigns consumer authority and injectable HTTP exchanges.
- AGENTS.md:3–18 assigns objectives O1/O5 and forbids interpreting remote server
  annotations as local authority.
- docs/design/0001-consumer-authority.md:3–23 records the current client boundary,
  frozen snapshot and consumer-owned authority.

## Current boundary

This initial plan is derived from the verified baseline
0fdfbafa7c130caa76d74825680d72af3280e21b. Existing tools/HTTP/stdio/OAuth behavior
is shipped source, not a backlog of unimplemented features. Current docs claim
bounds and credential placement that need source/fixture reconciliation during
ESS retrofit; no fresh compatibility run is implied by this artifact.

The operator's Connectors milestone request authorizes necessary sibling MCP work
and verified source publication. No Harness repin, hosted deployment or consumer
policy migration follows. New resources/prompts/server behavior is specified before
implementation; it is not retroactively claimed as existing behavior.
