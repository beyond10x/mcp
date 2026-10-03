//! Explicit cancellation observations from actual HTTP peers.
#![cfg(feature = "strict-http")]
#[path = "support/strict_lifecycle.rs"]
#[allow(dead_code)]
mod fixture;
use serde_json::json;
const REVISIONS: [&str; 2] = ["2025-11-25", "2026-07-28"];
#[tokio::test]
async fn caller_and_timeout_cancellation_select_the_configured_wire_behavior() {
    for revision in REVISIONS {
        for case in [
            "cancel-caller",
            "cancel-timeout",
            "cancel-before-headers",
            "cancel-late",
            "cancel-reuse",
        ] {
            let actual = fixture::observe_cancel(case, revision).await.unwrap();
            let outcome = &actual["outcome"];
            assert_eq!(outcome["kind"], "cancelled", "{actual}");
            assert_eq!(
                outcome["value"]["interrupted"]["cause"],
                if case == "cancel-timeout" {
                    "deadline_exhausted"
                } else {
                    "caller_cancelled"
                }
            );
            assert_eq!(
                outcome["value"]["interrupted"]["request_id"],
                json!({"kind":"integer","value":2})
            );
            assert!(outcome["value"].get("late_response").is_none());
            let controls: Vec<_> = actual["requests"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["body"]["method"] == "notifications/cancelled")
                .collect();
            assert_eq!(controls.len(), usize::from(revision == "2025-11-25"));
            if revision == "2025-11-25" {
                assert_eq!(outcome["value"]["notification"]["disposition"], "accepted");
                assert_eq!(controls[0]["body"]["params"]["requestId"], 2);
                assert!(controls[0]["body"].get("id").is_none());
                assert_eq!(
                    controls[0]["headers"]["mcp-session-id"],
                    "lifecycle-fixture-session"
                );
            } else {
                assert!(outcome["value"].get("notification").is_none());
            }
            if case == "cancel-before-headers" {
                assert_eq!(
                    outcome["value"]["interrupted"]["exchange"]["send"],
                    "unknown"
                );
            }
            if case == "cancel-reuse" {
                assert_eq!(actual["reused"], true);
            }
        }
    }
}
#[tokio::test]
async fn pre_send_cancel_and_initialize_refusal_emit_no_cancellation() {
    for revision in REVISIONS {
        for case in [
            "cancel-before-send",
            "cancel-initialize",
            "cancel-completed",
        ] {
            let actual = fixture::observe_cancel(case, revision).await.unwrap();
            let requests = actual["requests"].as_array().unwrap();
            assert!(
                !requests
                    .iter()
                    .any(|r| r["body"]["method"] == "notifications/cancelled")
            );
            if case == "cancel-before-send" {
                assert_eq!(actual["outcome"]["kind"], "cancelled");
                assert_eq!(
                    actual["outcome"]["value"]["interrupted"]["exchange"]["send"],
                    "not_sent"
                );
            } else if case == "cancel-initialize" {
                assert_eq!(actual["refused_input"], true);
            } else {
                assert_eq!(actual["outcome"]["kind"], "finished");
                assert_eq!(actual["outcome"]["value"]["kind"], "result");
            }
        }
    }
}
#[tokio::test]
async fn notification_failures_and_expired_teardown_preserve_uncertainty() {
    for (case, reason, controls) in [
        ("cancel-notification-failure", "http_status", 1),
        ("cancel-notification-body", "invalid_response", 1),
        ("cancel-notification-bound", "response_bound", 1),
        ("cancel-notification-timeout", "deadline_exhausted", 1),
        ("cancel-teardown-expired", "deadline_exhausted", 0),
    ] {
        let actual = fixture::observe_cancel(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["outcome"]["kind"], "cancelled");
        let control = &actual["outcome"]["value"]["notification"];
        assert_eq!(control["disposition"], "refused");
        assert_eq!(control["refusal"], reason);
        assert_eq!(
            actual["requests"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["body"]["method"] == "notifications/cancelled")
                .count(),
            controls
        );
        if controls == 0 {
            assert_eq!(control["exchange"]["send"], "not_sent");
        }
    }
}
#[tokio::test]
async fn terminal_precedence_and_pending_side_reply_cancellation_are_observed() {
    for (case, kind) in [
        ("cancel-control", "cancelled"),
        ("cancel-terminal-race", "finished"),
    ] {
        let actual = fixture::observe_cancel(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["outcome"]["kind"], kind, "{actual}");
        let observation = if kind == "finished" {
            &actual["outcome"]["value"]["value"]["observation"]
        } else {
            &actual["outcome"]["value"]["interrupted"]["exchange"]
        };
        let reply = &observation["stream"]["messages"][0]["value"]["reply"];
        assert_eq!(reply["refusal"], "caller_cancelled");
        assert_eq!(reply["exchange"]["send"], "unknown");
        assert_eq!(
            observation["terminal"],
            if kind == "finished" {
                "correlated_terminal"
            } else {
                "incomplete"
            }
        );
    }
}

#[tokio::test]
async fn cancellation_acknowledgement_uses_only_the_remaining_response_budget() {
    let actual = fixture::observe_cancel("cancel-control-bound", "2025-11-25")
        .await
        .unwrap();
    let value = &actual["outcome"]["value"];
    assert_eq!(actual["outcome"]["kind"], "cancelled");
    let notification = &value["notification"];
    assert_eq!(notification["refusal"], "response_bound");
    let retained = notification["exchange"]["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap();
    let request = &value["interrupted"]["exchange"]["stream"]["messages"][0]["value"]["request"];
    let earlier = request["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap();
    assert!(earlier > 0);
    assert_eq!(earlier + retained, 4096);
    assert_eq!(
        notification["exchange"]["response"]["counts"]["limit_octets"],
        retained
    );
}
