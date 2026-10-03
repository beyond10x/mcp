//! Stream controls must cross a separate real HTTP request, never replace a result.
#![cfg(feature = "strict-http")]
#[path = "support/strict_lifecycle.rs"]
#[allow(dead_code)] // This target exercises the stream half of the shared real peer.
mod fixture;
use serde_json::json;

#[tokio::test]
async fn legacy_ping_is_answered_and_original_result_survives() {
    for case in [
        "stream-ping",
        "stream-ping-large-id",
        "stream-ping-collision",
        "stream-ping-sessionless",
        "stream-setup-ping",
        "stream-ping-cr",
        "stream-empty-prime",
    ] {
        let actual = fixture::observe_stream(case, "2025-11-25")
            .await
            .unwrap_or_else(|e| panic!("{case}: {e}"));
        assert_eq!(actual["exchange"]["kind"], "result", "{case}: {actual}");
        let requests = actual["requests"].as_array().unwrap();
        let reply: Vec<_> = requests
            .iter()
            .filter(|r| r["body"].get("method").is_none())
            .collect();
        assert_eq!(reply.len(), 1, "{case}: {actual}");
        assert_eq!(reply[0]["body"]["result"], json!({}));
        let expected_id = match case {
            "stream-ping-large-id" => {
                serde_json::from_str("123456789012345678901234567890").unwrap()
            }
            "stream-ping-collision" => json!(2),
            _ => json!("peer-ping"),
        };
        assert_eq!(reply[0]["body"]["id"], expected_id);
        if case == "stream-setup-ping" {
            assert_eq!(
                actual["exchange"]["value"]["result"]["protocolVersion"],
                "2025-11-25"
            );
        } else {
            assert_eq!(actual["exchange"]["value"]["result"]["content"], json!([]));
        }
        assert!(reply[0]["body"].get("error").is_none());
        let messages = actual["exchange"]["value"]["observation"]["stream"]["messages"]
            .as_array()
            .unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["value"]["reply"]["disposition"], "accepted");
    }
}

#[tokio::test]
async fn modern_server_requests_are_refused_without_a_reply() {
    let actual = fixture::observe_stream("stream-modern-server-request", "2026-07-28")
        .await
        .unwrap();
    assert_eq!(actual["exchange"]["kind"], "refused", "{actual}");
    assert_eq!(actual["exchange"]["value"]["reason"], "invalid_response");
    assert_eq!(actual["requests"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn dropping_an_exchange_closes_its_pending_control_before_client_drop() {
    let actual = fixture::observe_stream("stream-control-abandoned", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(actual["exchange"]["abandoned_stream_closed"], true);
    assert_eq!(actual["exchange"]["abandoned_control_closed"], true);
    assert_eq!(actual["exchange"]["reuse_refused"], true);
    assert_eq!(actual["requests"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn a_delayed_control_ack_cannot_hide_an_already_arrived_business_result() {
    let actual = fixture::observe_stream("stream-control-delayed", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(actual["exchange"]["kind"], "result", "{actual}");
    assert_eq!(actual["exchange"]["value"]["result"]["content"], json!([]));
    let reply =
        &actual["exchange"]["value"]["observation"]["stream"]["messages"][0]["value"]["reply"];
    assert_eq!(reply["refusal"], "deadline_exhausted");
    assert_eq!(reply["exchange"]["send"], "unknown");
}

#[tokio::test]
async fn unadvertised_requests_get_errors_and_side_failures_do_not_replace_results() {
    for (case, disposition, reason) in [
        ("stream-unsupported", "accepted", None),
        ("stream-control-failure", "refused", Some("http_status")),
        ("stream-control-body", "refused", Some("invalid_response")),
    ] {
        let actual = fixture::observe_stream(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["exchange"]["kind"], "result", "{case}: {actual}");
        let message = &actual["exchange"]["value"]["observation"]["stream"]["messages"][0];
        assert_eq!(message["value"]["reply"]["disposition"], disposition);
        assert_eq!(message["value"]["reply"]["refusal"].as_str(), reason);
        let reply = actual["requests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["body"].get("method").is_none())
            .unwrap();
        assert_eq!(
            reply["headers"]["authorization"],
            "Bearer fixture-only-credential"
        );
        assert_eq!(
            reply["headers"]["mcp-session-id"],
            "lifecycle-fixture-session"
        );
        assert_eq!(reply["headers"]["mcp-protocol-version"], "2025-11-25");
        assert!(reply["headers"].get("mcp-method").is_none());
        if case == "stream-unsupported" {
            assert_eq!(reply["body"]["error"]["code"], -32601);
            assert!(reply["body"].get("result").is_none());
        }
    }
}

#[tokio::test]
async fn notifications_and_bounds_retain_exact_nonterminal_observations() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    for revision in ["2025-11-25", "2026-07-28"] {
        let actual = fixture::observe_stream("stream-notifications", revision)
            .await
            .unwrap();
        assert_eq!(actual["exchange"]["kind"], "result");
        let message = &actual["exchange"]["value"]["observation"]["stream"]["messages"][0];
        assert_eq!(message["kind"], "notification");
        let raw = STANDARD
            .decode(message["value"]["response"]["bytes"].as_str().unwrap())
            .unwrap();
        assert!(
            String::from_utf8(raw)
                .unwrap()
                .contains("$serde_json::private::Number")
        );
        for (case, reason) in [
            ("stream-total-bound", "response_bound"),
            ("stream-event-bound", "sse_event_bound"),
        ] {
            let actual = fixture::observe_stream(case, revision).await.unwrap();
            assert_eq!(
                actual["exchange"]["value"]["reason"], reason,
                "{case}: {actual}"
            );
            assert_eq!(
                actual["requests"].as_array().unwrap().len(),
                if revision == "2025-11-25" { 3 } else { 2 }
            );
        }
    }
    let actual = fixture::observe_stream("stream-control-bound", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(
        actual["exchange"]["value"]["reason"], "response_bound",
        "{actual}"
    );
    let message = &actual["exchange"]["value"]["observation"]["stream"]["messages"][0]["value"];
    let original = message["request"]["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap();
    let reply = message["reply"]["exchange"]["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap();
    let terminal =
        actual["exchange"]["value"]["observation"]["response"]["counts"]["retained_octets"]
            .as_u64()
            .unwrap();
    assert_eq!(original + reply + terminal, 4096);
}

#[tokio::test]
async fn initialization_never_echoes_invalid_session_headers() {
    for (case, reason) in [
        ("stream-setup-session-invalid", "invalid_session_header"),
        ("stream-setup-session-duplicate", "invalid_session_header"),
        ("stream-setup-session-bound", "session_header_bound"),
    ] {
        let actual = fixture::observe_stream(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["setup_reason"], reason, "{actual}");
        assert_eq!(actual["requests"].as_array().unwrap().len(), 1);
        let reply =
            &actual["exchange"]["value"]["observation"]["stream"]["messages"][0]["value"]["reply"];
        assert_eq!(reply["exchange"]["send"], "not_sent");
        assert_eq!(reply["refusal"], "invalid_response");
    }
}
