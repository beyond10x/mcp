---
format: aep.planning-md/3
id: review-result:specification-gate-followup-20261002
kind: review-result
status: active
title: Specification gate correction followup
relations:
- reviews: story:specification-gate-enrollment
revision: 1
---
unit: specification-gate-enrollment corrected working tree over 1fcb09186aafdec1e6a6acd6655f0d4f29b361f0
verdict: nothing found
cases: executed 5→5, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: full repository gate and lifecycle remain with coordinator

```text
followup review-only diff stat:
 0 files changed
```

The tracked inherited diff remains eight files, 119 insertions and 15 deletions;
three untracked author files are additional inherited changes. The original review's
55 test lines remain unchanged. This followup made no source or planning edits.
The corrected `specification.rs` SHA256 is
`b6b6138e2c25f03b483190127aee7ffe4516380fd07022907b5d1c83ecf3ecb2`.

1. The original dangling-local-executable case was rerun alone first. It existed
before this pass and was already red in the immutable first report. No new case
or mutant was added for this bounded correction review.

Rust 1.88.0, two jobs, sccache; command:

```console
cargo test --locked -p xtask specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path -- --exact --nocapture
```

Exit 0; complete output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/main.rs (target/debug/deps/xtask-91cdc4b340e477d5)

running 1 test
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.02s

```

2. Then the affected suite ran under the same settings:

```console
cargo test --locked -p xtask
```

Exit 0; complete output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/main.rs (target/debug/deps/xtask-91cdc4b340e477d5)

running 5 tests
test specification::tests::missing_new_duplicate_and_empty_refusal_inventory_are_refused ... ok
test specification::tests::archive_digest_is_checked_before_installation ... ok
test specification::tests::wrong_or_ambiguous_tool_version_is_refused ... ok
test specification::tests::suite_drift_and_zero_or_missing_cases_are_refused ... ok
test specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

```

`cargo clippy --locked -p xtask --all-targets -- -D warnings` also passed, exit 0.
The before count of five is the prior review's actual suite count, including its
then-failing case. This pass did not rerun full workspace or constructor conformance.

3. Nothing found in the bounded correction. The earlier finding no longer reproduces.

**Measured:** the unchanged test now observes PATH fallback for a genuinely absent
local tool entry and refusal for an existing dangling symlink. The affected suite
executes all five cases with no failures, ignored cases, or filtered cases.

**Correction inspected:** `tool()` at `crates/xtask/src/specification.rs:108` keeps
explicit `*_BIN` precedence, calls `symlink_metadata` for the local selection,
selects any existing local entry, permits PATH only on `ErrorKind::NotFound`, and
returns other lookup errors. A selected entry that cannot execute is refused by
the existing process-start error path. This covers the reported local leaf-symlink
fallthrough; the review did not expand into ancestor-path policy or hostile concurrent
filesystem replacement.

**Lookup-error coverage limit:** the non-NotFound refusal branch was source reviewed,
not exercised by a new filesystem permission-error or symlink-loop fixture. No claim
of measured coverage for every operating-system error is made. The tested caller is
the same generic resolver used by `check()` for ESS and AEP; the current bootstrap
still installs regular files, and no actual broken installed cache was observed.

4. Retained raw evidence:

| Repository-relative path | SHA256 |
|---|---|
| `.cache/gate-review/followup-case.log` | `e0e3395e7faa1f500c6f66f7e3df01e387d7026e852f5dc55e94aae1010b5890` |
| `.cache/gate-review/followup-suite.log` | `5f70888b2411e9d9d64a829365adddab65ac9cf6849ea00dd4f0b0966c3fda45` |
| `.cache/gate-review/followup-clippy.log` | `3054b0ea5e684866498da8a161a36628a07b1b39ca7aa32bc7d5bda53ffc8876` |
| `.cache/gate-review/report.md` unchanged original | `d8959d1d0acbf3820de2f2fcf65534b5546edbab2a00dea6c6573921c68dc390` |

No explicit output path outside the worktree was written. Cargo used its existing
default target and configured sccache. The review lease is released; the coordinator
retains the tree and owns the full gate, publication, and cleanup.

```findings
[]
```
