---
format: aep.planning-md/3
id: review-result:specification-gate-20261002
kind: review-result
status: active
title: 'Specification gate adversary: dangling selected tool fallback'
relations:
- reviews: story:specification-gate-enrollment
revision: 1
---
unit: specification-gate-enrollment working tree over 1fcb09186aafdec1e6a6acd6655f0d4f29b361f0
verdict: CONFIRMED
cases: executed 4→5, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: resolve missing local executable fallback before rerunning the gate

```text
review-only diff against supplied specification.rs snapshot:
 crates/xtask/src/specification.rs | 55 +++++++++++++++++++++++++++++++++++++++
 1 file changed, 55 insertions(+)
```

The only review edit is inside the existing `#[cfg(test)]` module, explicitly permitted
by the brief. Byte comparison of the production prefix against the supplied snapshot
passed. The tracked inherited diff against the base was eight files, 119 insertions
and 15 deletions, including coordinator-owned AEP work; the untracked inherited
`Taskfile.yml`, `toolchain.json`, and `specification.rs` are additional author files.
None of those inherited production or planning changes is a review edit. The ten
source-manifest digests matched before adding the case.

1. The new Unix case at `crates/xtask/src/specification.rs:314` calls the actual
   `tool()` resolver with a present PATH compiler and a matching version. It first
   confirms that an absent local entry permits PATH selection, then creates a dangling
   local symlink and asserts refusal. A child test process removes `RUSTC_BIN`, so no
   global environment mutation races other cases. The compiler is just the executable
   fixture for this generic resolver; no fake ESS output or downloaded test tool is used.
   The assertion failed on the first execution. No suite ran before this case existed.

First command (Rust 1.88.0, two jobs, sccache):

```console
cargo test --locked -p xtask specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path -- --exact --nocapture
```

Exit status: 101. Test output below is verbatim from `running 1 test` onward; compiler
progress carrying the machine-local checkout path is omitted from this public report.

```text
running 1 test

thread 'specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path' panicked at crates/xtask/src/specification.rs:325:13:
isolated tool-resolution case failed: 
running 1 test
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... FAILED

failures:

failures:
    specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.03s



thread 'specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path' panicked at crates/xtask/src/specification.rs:361:9:
dangling selected local executable silently fell back to PATH
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... FAILED

failures:

failures:
    specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p xtask --bin xtask`
```

2. After that first isolated execution, the affected suite ran with the same toolchain
   and build settings. The inherited count of four tests is from the implementor's
   supplied brief; this pass did not obtain it from a preliminary run. The child
   execution exercises the same added logical case and is not counted twice.

```console
cargo test --locked -p xtask
```

Exit status: 101. Complete output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running unittests src/main.rs (target/debug/deps/xtask-91cdc4b340e477d5)

running 5 tests
test specification::tests::archive_digest_is_checked_before_installation ... ok
test specification::tests::missing_new_duplicate_and_empty_refusal_inventory_are_refused ... ok
test specification::tests::wrong_or_ambiguous_tool_version_is_refused ... ok
test specification::tests::suite_drift_and_zero_or_missing_cases_are_refused ... ok
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... FAILED

failures:

---- specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path stdout ----

thread 'specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path' panicked at crates/xtask/src/specification.rs:325:13:
isolated tool-resolution case failed: 
running 1 test
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... FAILED

failures:

failures:
    specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.05s



thread 'specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path' panicked at crates/xtask/src/specification.rs:361:9:
dangling selected local executable silently fell back to PATH
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

error: test failed, to rerun pass `-p xtask --bin xtask`
```

`cargo clippy --locked -p xtask --all-targets -- -D warnings` passed, exit 0.
The full workspace gate and constructor conformance were not rerun in this pass.
Inherited author results (25 full-gate tests passing, actual partial target 9/9,
and wrong-return control 7 passed / 2 failed) remain author results, not executions
of this review.

3. Finding:

| File:line | Verdict | Origin | Finding |
|---|---|---|---|
| `crates/xtask/src/specification.rs:113` | CONFIRMED | introduced | A dangling repository-local tool symlink is treated as absent and silently falls back to PATH, contrary to the selected-tool missing/failure refusal boundary. |

**Measured:** the added case reaches `tool()` with an existing dangling directory
entry, receives the PATH executable successfully, and fails the refusal assertion at
line 361; both its first isolated run and the subsequent affected suite exit 101.

**Reachable caller:** both `cargo xtask specification` and `cargo xtask gate` call
`check()`, which calls this same resolver for ESS and AEP. With no explicit `*_BIN`,
`.cache/tools/<name>` is the documented local selection surface. A user-provided
symlink there can outlive its target; if PATH contains the matching version, this
branch silently executes it. This fixture constructs that local-cache condition;
no broken symlink was observed in the current installed cache. `bootstrap-tools`
itself installs regular files and does not create this condition. The defect is a
narrow boundary case, not an observed failure of current CI bootstrap or a bypass
of the tool version check. Story acceptance 2 requires missing tools to refuse
without silent fallback. Preserve fallback for an actually absent directory entry
while refusing dangling entries and metadata-access errors.

**Origin:** this resolver is new in the reviewed unit; the base had no
`crates/xtask/src/specification.rs`. The production branch creating the behavior is
in this unit's source, so the origin is introduced. No base checkout or mutation of
production source was performed.

4. Other bounded review observations:

- The four inherited xtask guard cases remained green during the affected suite.
- Source-pin comparisons, exact suite bytes/identities, refusal inventory, archive
  digest-before-extraction, member extraction, and explicit bootstrap wiring were
  read. No additional finding was established from those source reads; they are not
  claims of full end-to-end bootstrap coverage.
- No mutant was run, no live provider was contacted, and no dependency, AEP,
  production, commit, or publication edit was made.

5. Retained raw evidence (all repository-relative, under assigned scratch):

| File | SHA256 |
|---|---|
| `.cache/gate-review/new-case.log` | `9bc1fd01f3381486bc127f28545839eaa04ef3a18c85855777ea590d977af859` |
| `.cache/gate-review/xtask-suite.log` | `07c27c14095b5cc91e82fcc135caa1c80fca0f3939cf843607e53816fe5941be` |
| `.cache/gate-review/clippy.log` | `885e6377e5bc3325c979777d979806da99b78c6ac48a07352c04d6d490429830` |
| `.cache/gate-review/specification-before.rs` | `7b55a2e9f5a7ac597762d59385fcf7837240be69f618ce6fea0f8c1f8b82b413` |
| `crates/xtask/src/specification.rs` after the review case | `be60423e5057e04645b8b20ec9b9f31bc5223bd34c79d55c73b011e6a0fdc87b` |

The review-only patch, initial source-manifest check, and inherited diff stat are
retained beside these logs. No explicit output path outside the assigned worktree
was written; Cargo used its existing default target and configured sccache. The
coordinator retains the tree and owns any eventual publication or cleanup.

```findings
- file: crates/xtask/src/specification.rs
  line: 113
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A dangling repository-local tool symlink is treated as absent and silently falls back to PATH, contrary to the selected-tool missing/failure refusal boundary.
```
