---
format: aep.planning-md/3
id: review-result:constructor-target-20261002
kind: review-result
status: active
title: Constructor target adversary with exact projection limits
relations:
- reviews: story:constructor-conformance-target
revision: 1
---
unit: story:constructor-conformance-target; cb26f-target working tree over d36abd82d1beeb2302b339db9cadc434e11dae6f, identities in source.sha256
verdict: nothing found
cases: executed 5→6, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none intentional; existing Cargo/rustup/sccache caches and managed lease metadata only
needs-coordinator: integrate the added test and retain this report; full repository gate after integration

1. Review-only diffstat (`git --no-pager diff --no-index --stat .cache/constructor-review/ess_conformance.before.rs crates/b10x-mcp-types/tests/ess_conformance.rs`):

```text
 .../b10x-mcp-types/tests/ess_conformance.rs        | 36 ++++++++++++++++++++++
 1 file changed, 36 insertions(+)
```

All 356 original test-file lines are unchanged; only one test was appended. No implementation, dependency, contract, generated suite or planning edits. Inherited tracked diff from the implementor is separately:

```text
 Cargo.lock                       | 220 +++++++++++++++++++++++++++++++++++++--
 crates/b10x-mcp-types/Cargo.toml |   4 +
 ess/README.md                    |  24 +++--
 ess/coverage.md                  |  19 ++++
 4 files changed, 254 insertions(+), 13 deletions(-)
```

Inherited untracked additions are conformance/constructors.json and crates/b10x-mcp-types/tests/ess_conformance.rs. They are the reviewed subject, not adversary changes. Verified the six inherited hashes before editing against source manifest SHA f8e7bd438887ac477527e3898d4c1f12de075fa2d4b17a696c5f98e7b65641d0; implementing report SHA 67a16214973ece14f118521e6af25c1fc5ae488513792a2440a367aa342339b6. Review source manifest records the changed test and unchanged other five files.

Public derivative: one exact assigned-checkout prefix was replaced with <managed-target-tree> in captured Cargo output. No personal cache prefixes occurred and no substantive content was changed. The unchanged private original report SHA256 is ca0b80063eb2f51f5a89d2a2c8cac7b7c650703305784350260ca6a6d937ac9a.

2. Added case before any test execution

crates/b10x-mcp-types/tests/ess_conformance.rs:359, adversary_projection_exact_limits_preserve_values_and_name_first_refusal: exact serialized byte limit including escaping, exact node limit including the root, and exact depth limit preserve complete literal values; the first value beyond each limit returns the named Unsupported observation. Green now. No red occurred. These are constructed projection inputs, reachable through the test binding's project function; they do not represent additional admitted ESS scenarios or broader product input coverage.

First command, exit 0; verbatim output:

```console
CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo +1.88.0 test --locked -p b10x-mcp-types --test ess_conformance adversary_projection_exact_limits_preserve_values_and_name_first_refusal -- --exact --nocapture
   Compiling b10x-mcp-types v0.1.2 (<managed-target-tree>/crates/b10x-mcp-types)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.02s
     Running tests/ess_conformance.rs (target/debug/deps/ess_conformance-a17f63597d2c8668)

running 1 test
test adversary_projection_exact_limits_preserve_values_and_name_first_refusal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.07s

```

3. Affected suite after the new case existed

The header's before count 5 is inherited from the implementor's recorded target suite; no suite was run before adding the attack. After count 6 is this pass's actual affected-suite execution. The inherited whole-repository gate's 20 tests were not rerun here and are not the header denominator. ESS execution remains 9 passed, 0 failed/error/unsupported/skipped, with conformance inconclusive and undeclared coverage. Wrong-return control still produces 7 passed and 2 named failed ESS scenarios; those deliberate failures are assertions inside a passing native control test, not findings or extra conformance passes.

Command, exit 0; verbatim output:

```console
CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo +1.88.0 test --locked -p b10x-mcp-types --test ess_conformance -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/ess_conformance.rs (target/debug/deps/ess_conformance-a17f63597d2c8668)

running 6 tests
test projection_preserves_exact_integers_and_refuses_other_numbers ... ok
test projection_retains_nulls_and_declared_optional_collapse ... ok
COUNT_REPORT_BEGIN
{
  "completed_at": 1700000001900,
  "conformance_status": "inconclusive",
  "counts": {
    "error": 0,
    "failed": 0,
    "passed": 9,
    "skipped": 0,
    "total": 9,
    "unsupported": 0
  },
  "coverage": {
    "knowledge": "unknown"
  },
  "execution_status": "passed",
  "format": "ess-conformance-report/2",
  "implementation": "b10x-mcp-types constructors 0.1.2",
  "outcomes": {
    "error": [],
    "failed": [],
    "passed": [
      "mcp.constructor_checks.MakeConnectionId/outcome/empty",
      "mcp.constructor_checks.MakeConnectionId/outcome/too-long",
      "mcp.constructor_checks.MakeToolDescriptor/outcome/returned",
      "mcp.constructor_checks.MakeToolResult/outcome/returned",
      "mcp.constructor_checks/authored/descriptor-raw-content",
      "mcp.constructor_checks/authored/id-empty",
      "mcp.constructor_checks/authored/id-overlength",
      "mcp.constructor_checks/authored/result-success",
      "mcp.constructor_checks/authored/result-tool-error"
    ],
    "skipped": [],
    "unsupported": []
  },
  "policy": "complete-selection/1",
  "producer_profile": "rust-scenario-status/1",
  "spec_digest": "2656cf17271a670d6bbb1b69ff961ea54d597ad82d4139bfa4e3fa6b1e18c66c",
  "specification": "mcp/v1",
  "suite": {
    "digest": "sha256:a53a492f82fec31fa14005cba55cf55cc183f0b85811f58476dfde0b6ec4cab7",
    "digest_profile": "sha256-json-bytes/1",
    "version": "ess-conformance/28"
  }
}
COUNT_REPORT_END
COUNT_REPORT_BEGIN
{
  "completed_at": 1700000001900,
  "conformance_status": "failed",
  "counts": {
    "error": 0,
    "failed": 2,
    "passed": 7,
    "skipped": 0,
    "total": 9,
    "unsupported": 0
  },
  "coverage": {
    "knowledge": "unknown"
  },
  "execution_status": "failed",
  "format": "ess-conformance-report/2",
  "implementation": "wrong-return control 0.1.2",
  "outcomes": {
    "error": [],
    "failed": [
      "mcp.constructor_checks/authored/result-success",
      "mcp.constructor_checks/authored/result-tool-error"
    ],
    "passed": [
      "mcp.constructor_checks.MakeConnectionId/outcome/empty",
      "mcp.constructor_checks.MakeConnectionId/outcome/too-long",
      "mcp.constructor_checks.MakeToolDescriptor/outcome/returned",
      "mcp.constructor_checks.MakeToolResult/outcome/returned",
      "mcp.constructor_checks/authored/descriptor-raw-content",
      "mcp.constructor_checks/authored/id-empty",
      "mcp.constructor_checks/authored/id-overlength"
    ],
    "skipped": [],
    "unsupported": []
  },
  "policy": "complete-selection/1",
  "producer_profile": "rust-scenario-status/1",
  "spec_digest": "2656cf17271a670d6bbb1b69ff961ea54d597ad82d4139bfa4e3fa6b1e18c66c",
  "specification": "mcp/v1",
  "suite": {
    "digest": "sha256:a53a492f82fec31fa14005cba55cf55cc183f0b85811f58476dfde0b6ec4cab7",
    "digest_profile": "sha256-json-bytes/1",
    "version": "ess-conformance/28"
  }
}
COUNT_REPORT_END
RUN_REPORT_BEGIN
{
  "suite": {
    "suite_version": "ess-conformance/28",
    "system": "mcp",
    "specification_version": "v1",
    "spec_digest": "2656cf17271a670d6bbb1b69ff961ea54d597ad82d4139bfa4e3fa6b1e18c66c",
    "contract_digest": "d13b267bad7c8df2fc9ddb0834811f8cfa50660c23fc2a046529b3c9075569cc"
  },
  "implementation": {
    "name": "b10x-mcp-types constructors",
    "version": "0.1.2"
  },
  "started_at": 1700000000000,
  "completed_at": 1700000001900,
  "status": "passed",
  "scenarios": [
    {
      "scenario": "mcp.constructor_checks.MakeConnectionId/outcome/empty",
      "purpose": "`mcp.constructor_checks.MakeConnectionId` answers `empty` for an input that satisfies that branch's guard",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/empty",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeConnectionId/outcome/too-long",
      "purpose": "`mcp.constructor_checks.MakeConnectionId` answers `too-long` for an input that satisfies that branch's guard",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/too-long",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeToolDescriptor/outcome/returned",
      "purpose": "`mcp.constructor_checks.MakeToolDescriptor` answers `returned` for an input no other branch's guard claims",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolDescriptor/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolDescriptor",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeToolResult/outcome/returned",
      "purpose": "`mcp.constructor_checks.MakeToolResult` answers `returned` for an input no other branch's guard claims",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/descriptor-raw-content",
      "purpose": "Real bounded from_raw preserves untrusted annotation, metadata and extension contents.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolDescriptor/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolDescriptor",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/id-empty",
      "purpose": "Empty input reaches the actual constructor and returns its exact Configuration message.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/empty",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/id-overlength",
      "purpose": "Allowed alphabet with65 bytes reaches the actual constructor and is refused.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/too-long",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/result-success",
      "purpose": "Real bounded result constructor preserves the complete selected raw value and false marker.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/result-tool-error",
      "purpose": "Tool-level error is data in a successful real constructor return.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    }
  ]
}
RUN_REPORT_END
RUN_REPORT_BEGIN
{
  "suite": {
    "suite_version": "ess-conformance/28",
    "system": "mcp",
    "specification_version": "v1",
    "spec_digest": "2656cf17271a670d6bbb1b69ff961ea54d597ad82d4139bfa4e3fa6b1e18c66c",
    "contract_digest": "d13b267bad7c8df2fc9ddb0834811f8cfa50660c23fc2a046529b3c9075569cc"
  },
  "implementation": {
    "name": "wrong-return control",
    "version": "0.1.2"
  },
  "started_at": 1700000000000,
  "completed_at": 1700000001900,
  "status": "failed",
  "scenarios": [
    {
      "scenario": "mcp.constructor_checks.MakeConnectionId/outcome/empty",
      "purpose": "`mcp.constructor_checks.MakeConnectionId` answers `empty` for an input that satisfies that branch's guard",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/empty",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeConnectionId/outcome/too-long",
      "purpose": "`mcp.constructor_checks.MakeConnectionId` answers `too-long` for an input that satisfies that branch's guard",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/too-long",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeToolDescriptor/outcome/returned",
      "purpose": "`mcp.constructor_checks.MakeToolDescriptor` answers `returned` for an input no other branch's guard claims",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolDescriptor/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolDescriptor",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks.MakeToolResult/outcome/returned",
      "purpose": "`mcp.constructor_checks.MakeToolResult` answers `returned` for an input no other branch's guard claims",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/descriptor-raw-content",
      "purpose": "Real bounded from_raw preserves untrusted annotation, metadata and extension contents.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolDescriptor/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolDescriptor",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/id-empty",
      "purpose": "Empty input reaches the actual constructor and returns its exact Configuration message.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/empty",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/id-overlength",
      "purpose": "Allowed alphabet with65 bytes reaches the actual constructor and is refused.",
      "status": "passed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeConnectionId/too-long",
          "status": "passed"
        },
        {
          "code": "ESS-CF-ERROR",
          "about": "error mcp.constructor_checks.Configuration",
          "status": "passed"
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/result-success",
      "purpose": "Real bounded result constructor preserves the complete selected raw value and false marker.",
      "status": "failed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "failed",
          "diagnostic": {
            "code": "ESS-CF-PAYLOAD",
            "scenario": "mcp.constructor_checks/authored/result-success",
            "source": [
              {
                "kind": "command",
                "name": "mcp.constructor_checks.MakeToolResult"
              }
            ],
            "input": null,
            "expected": [
              "actual typed return satisfies its complete schema and authored literals"
            ],
            "observed": [
              "response field result differs from its declared literal"
            ]
          }
        }
      ],
      "duration_ms": 100
    },
    {
      "scenario": "mcp.constructor_checks/authored/result-tool-error",
      "purpose": "Tool-level error is data in a successful real constructor return.",
      "status": "failed",
      "checks": [
        {
          "code": "ESS-CF-OUTCOME",
          "about": "outcome mcp.constructor_checks.MakeToolResult/returned",
          "status": "passed"
        },
        {
          "code": "ESS-CF-PAYLOAD",
          "about": "direct response mcp.constructor_checks.MakeToolResult",
          "status": "failed",
          "diagnostic": {
            "code": "ESS-CF-PAYLOAD",
            "scenario": "mcp.constructor_checks/authored/result-tool-error",
            "source": [
              {
                "kind": "command",
                "name": "mcp.constructor_checks.MakeToolResult"
              }
            ],
            "input": null,
            "expected": [
              "actual typed return satisfies its complete schema and authored literals"
            ],
            "observed": [
              "response field result differs from its declared literal"
            ]
          }
        }
      ],
      "duration_ms": 100
    }
  ]
}
RUN_REPORT_END
test supported_constructors_conform ... ok
test wrong_actual_return_fails_named_scenarios ... ok
test projection_refuses_bounds_without_truncating ... ok
test adversary_projection_exact_limits_preserve_values_and_name_first_refusal ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

```

Subsequent `cargo +1.88.0 fmt --all --check` and `CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache cargo +1.88.0 clippy --locked -p b10x-mcp-types --test ess_conformance -- -D warnings` both exit 0; full logs retained in fmt.log and clippy.log. `git diff --check` exits 0. Default assigned-tree target only. Last free-space observation was 13 GiB, above the 8 GiB reserve.

4. Judgement findings

Nothing found.

5. Attacks and limits

Exact projection limits and first overflow were exercised; no rounding, truncation or false refusal at the tested limits.
Source compared generated/authored return documents against actual constructor calls and input binding; target does not read expected values, and literal authored result cases detect the wrong-return control.
Source checked count guards: zero total, extra cases, non-passing terminal counts and missing named control failures cannot satisfy current assertions.
Optional None/Some(null) collapse is explicitly declared, and the existing native test observes that raw JSON still differs; no claim of universal serde constructor enforcement.
Malformed shape accessors return named Unsupported; snapshot remains unsupported and unexecuted. This pass added no malformed-request execution case.
Synthesis drift/gate enrollment remains the following story; no claimed full MCP retrofit, network conformance or resolution of nominal-return/count-observer refusals.

6. Outside-worktree writes

None intentionally selected. Cargo/rustup/sccache used their already configured global caches, and worktree hook commands maintained only this pass's own lease. All explicit scratch is .cache/constructor-review inside the assigned tree, including the before-source snapshot, logs, report and manifests. No external target directory, artifact, temporary probe or repository was created.

```findings
[]
```
