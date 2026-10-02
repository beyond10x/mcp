---
format: aep.planning-md/3
id: story:constructor-conformance-target
kind: story
status: implemented
title: Execute supported constructor conformance against real MCP methods
relations:
- informed_by: specification:governed-compatibility-baseline
- serves: vision:consumer-owned-mcp-mechanics
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: conformance/constructors.json
- confidence: cited
  path: crates/b10x-mcp-types/Cargo.toml
- confidence: inferred
  path: crates/b10x-mcp-types/tests/ess_conformance.rs
- confidence: cited
  path: ess/README.md
- confidence: cited
  path: ess/coverage.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T21:18:41Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-02T21:18:41Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-02T21:53:48Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Outcome

Execute the supported constructor ESS scenarios against the actual public MCP
methods, while preserving the refused constrained-return/invariant obligations
and every explicitly uncovered input family. This is one partial conformance
slice, not completion of the tools/HTTP retrofit or MCP delivery.

## Evidence and existing typed home

`ess/system.yaml`, `ess/domains/values.yaml` and `constructor_checks.yaml` declare
six existing public values and four real constructor bindings. ESS0.50.0 validates
three files. `ess/coverage.md` maps all eighteen original obligations to the
partial selection. `conformance/scenarios/` retains all eight authored cases;
root `ess-inputs.yaml` selects five that synthesize. No new entity/relation is
introduced by the target. Runtime baseline0fdfbafa matches the source citations.

Actual source is crates/b10x-mcp-types/src/lib.rs:80–105(ConnectionId),179–214
(ToolDescriptor),236–265(ToolSnapshot),307–319(ToolResult). The target may call only
these APIs for this slice and project actual results/errors; expected values and
scenario assertions are not target inputs. No HTTP or credential operation.

## Acceptance

1. The pinned ESS0.50.0 suite from the unchanged declarations and partial manifest
   executes its nine cases against real constructors: four generated and five
   authored (`id-empty`, `id-overlength`, `descriptor-raw-content`, `result-success`,
   `result-tool-error`). Report exact executed/passed/failed/error/unsupported/skipped
   counts; a filter selecting zero cases is a failure.
2. An independently incorrect returned field, injected through a test-owned wrapper
   around the real target, makes a named result scenario fail. Capture red before
   crediting the real target's green run; do not mutate production source or encode
   an expected verdict into the target.
3. Input/returned JSON projection preserves exact supported values and absent/null
   semantics declared by the binding. Reject unsupported observation shapes by name,
   rather than silently round/truncate/coerce. The bounded profiles stay narrow.
4. Full authored synthesis still reports the three constrained-return refusals;
   partial synthesis still retains four refusals. Preserve all nominal constraints,
   all eight case files and all U/C gaps. No claim of complete coverage or a successful
   constrained return observation absent actual support.
5. Target test, affected package formatting/Clippy and repository gate pass; existing
   types/MSRV/public APIs and runtime behavior remain unchanged. No new executable
   CLI is required; all running code Rust, any introduced CLI uses clap derive.

## Scope

- crates/b10x-mcp-types/tests/ess_conformance.rs: inferred new in-process real target,
  count guard and wrong-return control. Reuse documented ESS ConformanceTarget.
- crates/b10x-mcp-types/Cargo.toml: cited package; inferred dev-only pinned ESS dependency.
- Cargo.lock: inferred locked dependency update only for that dev dependency.
- conformance/constructors.json: inferred generated partial suite, never hand-edited.
- ess/README.md and ess/coverage.md: cited draft ownership; execution counts and
  remaining omissions only, never erasing historical refusals or broadening claims.

Root owns every AEP write and source publication. Worker has a separate managed tree,
private defaulttarget, two Cargo jobs, sccache and an8GiB disk reserve. Record full
commands/exits/source/binary identities and one exact scratch directory. Gate
integration/toolchain resolution is a following coordinated slice, not incidental
scope for this worker. No production, public constructor or typed model change.
