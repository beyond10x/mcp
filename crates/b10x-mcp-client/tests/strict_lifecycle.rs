//! Lifecycle acceptance through an actual local HTTP peer.
#![cfg(feature = "strict-http")]
#[path = "support/strict_lifecycle.rs"]
mod fixture;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

#[tokio::test]
async fn shutdown_selects_delete_only_for_legacy_sessions() {
    for (case, revision, deletes) in [
        ("lifecycle-shutdown-modern", "2026-07-28", 0),
        ("lifecycle-shutdown-sessionless", "2025-11-25", 0),
        ("lifecycle-shutdown-delete", "2025-11-25", 1),
        ("lifecycle-shutdown-405", "2025-11-25", 1),
    ] {
        let actual = fixture::observe(case, revision).await.unwrap();
        let requests = actual["requests"].as_array().unwrap();
        let controls: Vec<_> = requests
            .iter()
            .filter(|r| r["method"] == "DELETE")
            .collect();
        assert_eq!(controls.len(), deletes, "{case}: {actual}");
        assert!(actual["shutdown"].get("interrupted").is_none());
        assert_eq!(actual["pool_closed"], true);
        if let Some(control) = controls.first() {
            assert_eq!(control["body"], Value::Null);
            assert_eq!(
                control["headers"]["mcp-session-id"],
                "lifecycle-fixture-session"
            );
            assert_eq!(control["headers"]["mcp-protocol-version"], revision);
            assert_eq!(
                control["headers"]["authorization"],
                "Bearer fixture-only-credential"
            );
            assert!(control["headers"].get("mcp-method").is_none());
            let deletion = &actual["shutdown"]["deletion"];
            assert_eq!(
                deletion["disposition"],
                if case.ends_with("405") {
                    "delete_not_allowed"
                } else {
                    "accepted"
                }
            );
            assert!(deletion.get("refusal").is_none());
        } else {
            assert!(actual["shutdown"].get("deletion").is_none());
        }
    }
}

#[tokio::test]
async fn shutdown_retains_failure_bytes_and_never_redirects_or_retries() {
    for (case, status, reason, bytes) in [
        (
            "lifecycle-shutdown-failure",
            503,
            "http_status",
            b"failure evidence".as_slice(),
        ),
        (
            "lifecycle-shutdown-redirect",
            307,
            "http_status",
            b"redirect refused".as_slice(),
        ),
    ] {
        let actual = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["requests"].as_array().unwrap().len(), 3, "{actual}");
        let deletion = &actual["shutdown"]["deletion"];
        assert_eq!(deletion["disposition"], "refused");
        assert_eq!(deletion["refusal"], reason);
        assert_eq!(deletion["exchange"]["http_status"], status);
        assert_eq!(
            deletion["exchange"]["response"]["bytes"],
            STANDARD.encode(bytes)
        );
        assert_eq!(deletion["exchange"]["send"], "send_observed");
    }
}

#[tokio::test]
async fn shutdown_bounds_and_deadlines_are_observed_not_assumed() {
    for (case, reason, sent) in [
        ("lifecycle-shutdown-bound", "response_bound", true),
        ("lifecycle-shutdown-deadline", "deadline_exhausted", false),
        (
            "lifecycle-shutdown-body-timeout",
            "deadline_exhausted",
            true,
        ),
    ] {
        let actual = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(
            actual["requests"].as_array().unwrap().len(),
            if sent { 3 } else { 2 }
        );
        let deletion = &actual["shutdown"]["deletion"];
        assert_eq!(deletion["refusal"], reason, "{case}: {actual}");
        assert_eq!(
            deletion["exchange"]["send"],
            if sent { "send_observed" } else { "not_sent" }
        );
        let bytes = STANDARD
            .decode(deletion["exchange"]["response"]["bytes"].as_str().unwrap())
            .unwrap();
        assert!(bytes.len() <= 4096);
        if reason == "response_bound" {
            assert_eq!(bytes, vec![b'x'; 4096]);
        }
    }
}

#[tokio::test]
async fn abandoned_stream_is_closed_and_identity_survives_shutdown() {
    for revision in ["2026-07-28", "2025-11-25"] {
        let actual = fixture::observe("lifecycle-shutdown-abandoned", revision)
            .await
            .unwrap();
        assert_eq!(actual["abandoned_stream_closed"], true, "{actual}");
        assert_eq!(actual["reuse_refused"], true);
        assert_eq!(
            actual["shutdown"]["interrupted"],
            json!({"request_id":{"kind":"integer","value":2},"cause":"future_dropped"})
        );
        let calls = actual["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["body"]["method"] == "tools/call")
            .count();
        assert_eq!(calls, 1);
    }
}

#[tokio::test]
async fn completed_exchange_is_not_reported_interrupted() {
    for revision in ["2026-07-28", "2025-11-25"] {
        let actual = fixture::observe("lifecycle-shutdown-completed", revision)
            .await
            .unwrap();
        assert!(actual["shutdown"].get("interrupted").is_none());
    }
}

#[tokio::test]
async fn invocation_shutdown_and_reconnection_keep_caller_configuration() {
    for revision in ["2026-07-28", "2025-11-25"] {
        for case in [
            "lifecycle-shutdown-invocation",
            "lifecycle-shutdown-reconnect",
        ] {
            let actual = fixture::observations(case, revision).await.unwrap();
            let setups = if case.ends_with("reconnect") { 2 } else { 1 };
            assert_eq!(
                actual["requests"],
                setups * if revision == "2025-11-25" { 3 } else { 1 }
            );
            assert_eq!(actual["pool_closed"], true);
        }
    }
}
