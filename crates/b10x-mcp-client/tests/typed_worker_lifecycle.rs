//! Cancellation at actual worker phase barriers preserves the typed call's truth.
#![cfg(all(feature = "test-schema-worker", target_os = "linux"))]
#[path = "support/typed_lifecycle.rs"]
mod fixture;
#[tokio::test]
async fn worker_phases_keep_process_ownership_and_completed_business_responses() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for (case, phase) in [
            ("worker-input", "input_validation"),
            ("worker-preflight", "output_preflight"),
            ("worker-output", "output_validation"),
        ] {
            let actual = fixture::observations(case, revision).await.unwrap();
            assert_eq!(actual["kind"], "interrupted", "{case}/{revision}: {actual}");
            assert_eq!(actual["cause"], "caller_cancelled");
            assert_eq!(actual["phase"], phase);
            assert_eq!(actual["worker_state"], "reaped");
            assert_eq!(actual["worker_gone"], true);
            assert_eq!(actual["notifications"], 0);
            assert_eq!(actual["exchange_retained"], case == "worker-output");
            assert_eq!(actual["body_preserved"], case == "worker-output");
            assert_eq!(
                actual["business_calls"],
                usize::from(case == "worker-output")
            );
        }
    }
}
#[tokio::test]
async fn expired_and_dropped_worker_teardown_retains_typed_client_ownership() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for case in ["worker-retained", "worker-drop"] {
            let actual = fixture::observations(case, revision).await.unwrap();
            assert_eq!(
                actual["worker_state"], "retained",
                "{case}/{revision}: {actual}"
            );
            assert_eq!(actual["reuse_refused"], true);
            assert_eq!(actual["explicit_reaped"], true);
            assert_eq!(actual["worker_gone"], true);
            assert_eq!(actual["business_calls"], 0);
            assert_eq!(actual["notifications"], 0);
        }
    }
}
#[tokio::test]
async fn output_ipc_capacity_is_unavailable_validation_with_the_response_preserved() {
    for revision in ["2025-11-25", "2026-07-28"] {
        let actual = fixture::observations("worker-output-bound", revision)
            .await
            .unwrap();
        assert_eq!(actual["kind"], "refused", "{actual}");
        assert_eq!(actual["reason"], "schema_unavailable");
        assert_eq!(actual["phase"], "output_validation");
        assert_eq!(actual["worker_state"], "no_child");
        assert_eq!(actual["body_preserved"], true);
        assert_eq!(actual["business_calls"], 1);
        assert_eq!(actual["notifications"], 0);
    }
}
#[tokio::test]
async fn typed_wire_cancellation_keeps_revision_specific_controls_and_ids() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for case in [
            "tool-wire",
            "resource-wire",
            "prompt-wire",
            "resource-timeout-wire",
            "tool-expired-teardown",
        ] {
            let actual = fixture::observations(case, revision).await.unwrap();
            assert_eq!(actual["kind"], "interrupted", "{case}/{revision}: {actual}");
            assert_eq!(actual["phase"], "dispatch");
            assert_eq!(
                actual["cause"],
                if case.contains("timeout") {
                    "deadline_exhausted"
                } else {
                    "caller_cancelled"
                }
            );
            assert_eq!(
                actual["notifications"],
                usize::from(revision == "2025-11-25" && case != "tool-expired-teardown")
            );
            assert_eq!(actual["closed"], 1);
            assert_eq!(actual["attempted"], true);
            assert_eq!(actual["id_agreement"], true);
            assert_eq!(actual["protocol_agreement"], true);
            assert_eq!(actual["business_calls"], 1);
            assert_eq!(actual["worker_state"], "absent");
        }
    }
}
#[tokio::test]
async fn cancelled_or_dropped_discovery_never_promotes_a_partial_or_stale_catalog() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for case in [
            "discovery-first",
            "discovery-later",
            "discovery-timeout-later",
            "discovery-drop-refresh",
        ] {
            let actual = fixture::observations(case, revision).await.unwrap();
            let dropped = case == "discovery-drop-refresh";
            assert_eq!(
                actual["kind"],
                if dropped { "dropped" } else { "interrupted" },
                "{case}/{revision}: {actual}"
            );
            assert_eq!(actual["catalog_items"], 0);
            assert_eq!(actual["reuse_refused"], true);
            assert_eq!(actual["closed"], 1);
            assert_eq!(
                actual["notifications"],
                usize::from(!dropped && revision == "2025-11-25")
            );
            assert_eq!(
                actual["prior_pages"],
                usize::from(!dropped && case != "discovery-first")
            );
            assert_eq!(
                actual["list_calls"],
                if dropped {
                    3
                } else if case == "discovery-first" {
                    1
                } else {
                    2
                }
            );
            assert_eq!(actual["id_agreement"], true);
        }
    }
}
