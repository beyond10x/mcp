//! Exact progress over actual HTTP peers in both configured protocol revisions.
#![cfg(feature = "strict-http")]
#[path = "support/strict_lifecycle.rs"]
#[allow(dead_code)]
mod fixture;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
const REVISIONS: [&str; 2] = ["2025-11-25", "2026-07-28"];
fn observations(actual: &Value) -> &[Value] {
    actual["exchange"]["value"]["observation"]["stream"]["messages"]
        .as_array()
        .unwrap()
}
#[tokio::test]
async fn progress_preserves_exact_numbers_text_and_extreme_exponents() {
    for revision in REVISIONS {
        for (case, expected) in [
            (
                "stream-progress-exact",
                vec![
                    "-2",
                    "-0.5",
                    "-0.0",
                    "0.0001",
                    "9007199254740992",
                    "9007199254740993",
                    "1e+999999999999999999999999",
                    "2e+999999999999999999999999",
                ],
            ),
            (
                "stream-progress-tiny",
                vec!["1e-999999999999999999999999", "2e-999999999999999999999999"],
            ),
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(actual["exchange"]["kind"], "result", "{actual}");
            assert_eq!(observations(&actual).len(), expected.len());
            for (message, expected) in observations(&actual).iter().zip(expected) {
                assert_eq!(message["kind"], "progress");
                let value = &message["value"];
                assert_eq!(value["progress"].as_number().unwrap().to_string(), expected);
                assert_eq!(value["message"], "untrusted server text");
                assert_eq!(value["disposition"], "accepted");
                let wire = STANDARD
                    .decode(value["exchange"]["response"]["bytes"].as_str().unwrap())
                    .unwrap();
                let original: Value = serde_json::from_slice(&wire).unwrap();
                assert_eq!(original["params"]["progress"], value["progress"]);
                if case == "stream-progress-exact" {
                    assert_eq!(
                        value["total"].as_number().unwrap().to_string(),
                        "1e+999999999999999999999999"
                    );
                } else {
                    assert!(value.get("total").is_none());
                }
            }
        }
    }
}
#[tokio::test]
async fn unmatched_tokens_cannot_poison_the_owned_sequence() {
    for revision in REVISIONS {
        for (case, expected) in [
            (
                "stream-progress-unopted",
                vec!["unmatched_token", "unmatched_token"],
            ),
            (
                "stream-progress-unmatched",
                vec!["unmatched_token", "accepted", "unmatched_token", "accepted"],
            ),
            (
                "stream-progress-token-kind",
                vec!["unmatched_token", "accepted"],
            ),
            ("stream-progress-large-token", vec!["accepted", "accepted"]),
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(actual["exchange"]["kind"], "result");
            let observed: Vec<_> = observations(&actual)
                .iter()
                .map(|v| v["value"]["disposition"].as_str().unwrap())
                .collect();
            assert_eq!(observed, expected);
            if case == "stream-progress-large-token" {
                assert_eq!(
                    observations(&actual)[0]["value"]["token"]["value"].to_string(),
                    "123456789012345678901234567890"
                );
            }
        }
    }
}
#[tokio::test]
async fn equivalent_or_decreasing_progress_refuses_with_the_message_retained() {
    for revision in REVISIONS {
        for case in [
            "stream-progress-equal",
            "stream-progress-exponent-equal",
            "stream-progress-negative-zero",
            "stream-progress-decreasing",
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(actual["exchange"]["kind"], "refused");
            assert_eq!(actual["exchange"]["value"]["reason"], "invalid_response");
            let messages = observations(&actual);
            assert_eq!(messages.len(), 2);
            assert_eq!(messages[0]["value"]["disposition"], "accepted");
            assert_eq!(messages[1]["value"]["disposition"], "non_increasing");
            assert!(
                !messages[1]["value"]["exchange"]["response"]["bytes"]
                    .as_str()
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
#[tokio::test]
async fn invalid_progress_fields_and_invalid_opt_in_do_not_become_progress() {
    for revision in REVISIONS {
        for case in [
            "stream-progress-malformed",
            "stream-progress-bad-total",
            "stream-progress-bad-message",
            "stream-progress-bad-token",
            "stream-progress-invalid-input",
            "stream-progress-null-input",
            "stream-progress-fractional-input",
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(actual["exchange"]["kind"], "refused", "{actual}");
            let value = &actual["exchange"]["value"];
            assert!(value["observation"].get("stream").is_none());
            if case.ends_with("input") {
                assert_eq!(value["reason"], "invalid_input");
                assert_eq!(value["observation"]["send"], "not_sent");
                assert_eq!(
                    actual["requests"].as_array().unwrap().len(),
                    if revision == "2025-11-25" { 2 } else { 1 }
                );
            } else {
                assert_eq!(value["reason"], "invalid_response");
                assert!(
                    !value["observation"]["response"]["bytes"]
                        .as_str()
                        .unwrap()
                        .is_empty()
                );
            }
        }
    }
}
#[tokio::test]
async fn progress_never_extends_the_deadline_or_evades_retention_limits() {
    for revision in REVISIONS {
        for (case, reason) in [
            ("stream-progress-bound", "response_bound"),
            ("stream-progress-deadline", "deadline_exhausted"),
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(actual["exchange"]["kind"], "refused", "{actual}");
            assert_eq!(actual["exchange"]["value"]["reason"], reason);
            let observed = &actual["exchange"]["value"]["observation"];
            let retained: u64 = observations(&actual)
                .iter()
                .map(|v| {
                    v["value"]["exchange"]["response"]["counts"]["retained_octets"]
                        .as_u64()
                        .unwrap()
                })
                .sum();
            let total = retained
                + observed["response"]["counts"]["retained_octets"]
                    .as_u64()
                    .unwrap();
            assert!(total <= 4096);
            assert!(!observations(&actual).is_empty());
            if reason == "response_bound" {
                assert_eq!(total, 4096);
            }
            assert_ne!(actual["exchange"]["value"]["result"], json!({"content":[]}));
        }
    }
}
