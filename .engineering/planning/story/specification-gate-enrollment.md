---
format: aep.planning-md/3
id: story:specification-gate-enrollment
kind: story
status: draft
title: Enroll pinned specification and real conformance checks in the MCP gate
relations:
- depends_on: story:constructor-conformance-target
- informed_by: specification:governed-compatibility-baseline
- serves: vision:consumer-owned-mcp-mechanics
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: crates/xtask/Cargo.toml
- confidence: cited
  path: crates/xtask/src/main.rs
- confidence: inferred
  path: crates/xtask/src/specification.rs
- confidence: cited
  path: ess/README.md
- confidence: inferred
  path: toolchain.json
revision: 10
---
## Outcome

Make the repository's ordinary check validate the authored ESS, expose the exact
remaining synthesis refusals, compare the generated constructor suite and execute
its real target through workspace tests. Enroll AEP validation under an explicit
tool pin. This is tooling around existing modeled constructors, not a new product
entity, protocol capability or claim of complete MCP conformance.

## Evidence and dependency

crates/xtask/src/main.rs:6–38 currently selects gate through raw argv and runs
fmt/tests/Clippy/docs only. AGENTS.md:33–40 states the same existing boundary.
The operator requires Rust/clap derive for executable tooling and task check to
validate/run conformance. story:constructor-conformance-target supplies the real
in-process test and generated conformance/constructors.json before this integrates.

ess/system.yaml and rootess-inputs.yaml already own the declarations and explicit
partial selection. ESS0.50.0 validates3files; full8authored selection refuses3;
partial9scenarios retains4refusals. The pinned CLI's --format json emits only the
suite and no refusal diagnostics in this measured run. A gate using that alone
would hide unsupported obligations. Preserve diagnostic text and exact inventory.

## Acceptance

1. cargo xtask gate and a thin task check entry run structural ESS validation,
   partial-suite regeneration/drift checking, explicit refusal inventory and actual
   constructor conformance via workspace tests, then existing Clippy/docs checks.
   AEP validation is included. No ignored/unsupported case becomes a pass.
2. An explicit repository pin selects ESS0.50.0 and AEP0.68.0, matching observed
   tools; supplied paths or PATH binaries must report exact versions. Wrong/missing
   tools refuse by name without installation, downloading or silently falling back.
   The ESS Rust devdependency remains at the matching released source revision.
3. Gate command parsing uses clap derive. Keep the existing cargo xtask gate entry
   working; any additional command has an explicit documented purpose. No shell
   program, Python or named common/shared/utils/misc/helpers module is introduced.
4. Preserve the exact nine scenario identities and the declared four refusal
   identities, with their codes and subject names, and the full selection's three
   refused authored cases. Any new refusal, missing case or unexpected success is
   a reviewable drift failure, not an automatically accepted changed count.
5. Deciding tests show wrong tool versions, suite drift, a missing/new refusal and
   an empty/missing scenario selection fail. Use genuine gate seams and bounded
   fixtures; do not merely assert copied constants or require live providers.
6. Report actual executed target counts and native gate results separately from
   emitted/refused cases and unresolved obligations. Update AGENTS/README only to
   checks actually enrolled. Full gate passes before each source commit.

## Scope

- crates/xtask/src/main.rs and crates/xtask/Cargo.toml: existing tooling, clapderive
  and checked subprocess/version/drift/refusal handling. No production library change.
- crates/xtask/src/specification.rs: inferred focused gate implementation if useful.
- Cargo.lock: inferred direct-tool dependency metadata only, after target integration.
- toolchain.json: inferred repository-owned executable/revision pin data.
- Taskfile.yml: inferred thin task check invocation of the Rust gate.
- AGENTS.md, README.md and ess/README.md: existing verification/coverage descriptions.

Root owns this following unit and every AEP mutation. Source authority and protocol
review remain independent; no tool installation or upstream ESS implementation is
part of this story. Twojobs,sccache,isolatedtarget,8GiBreserve and exact exitcodes.
