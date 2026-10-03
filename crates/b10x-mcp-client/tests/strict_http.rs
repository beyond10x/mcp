//! Raw HTTP regression matrix, before family-specific result validation.
#![cfg(feature = "strict-http")]
#[path = "support/strict_http.rs"]
mod fixture;

use base64::{Engine as _, engine::general_purpose::STANDARD};

#[tokio::test]
async fn modern_http_errors_preserve_peer_envelopes_without_redispatch() {
    for (case, code, status, presence) in [
        ("status-peer-error", "-32022", 400, "present"),
        ("status-header-mismatch", "-32020", 400, "absent"),
        ("status-capability", "-32021", 400, "present"),
        ("status-method-not-found", "-32601", 404, "absent"),
        (
            "status-unknown-code",
            "123456789012345678901234567890",
            400,
            "absent",
        ),
    ] {
        let (actual, calls) = fixture::observe(case, "2026-07-28").await.unwrap();
        assert_eq!(calls, 1, "{case}");
        assert_eq!(actual["kind"], "peer_error", "{case}");
        assert_eq!(actual["value"]["error"]["code"].to_string(), code);
        assert_eq!(actual["value"]["error"]["data"]["presence"], presence);
        assert_eq!(actual["value"]["observation"]["http_status"], status);
        assert_eq!(
            actual["value"]["observation"]["terminal"],
            "correlated_terminal"
        );
        if case == "status-peer-error" {
            assert_eq!(
                actual["value"]["error"]["data"]["value"]["supported"][0],
                "2025-11-25"
            );
        }
        if case == "status-capability" {
            assert!(actual["value"]["error"]["data"]["value"].is_null());
        }
    }
}

#[tokio::test]
async fn session_expiry_requires_legacy_revision_and_a_sent_session_header() {
    for (case, revision, reason) in [
        ("session-expired-once", "2025-11-25", "session_expired"),
        ("session-expired-once", "2026-07-28", "http_status"),
        ("not-found-sessionless", "2025-11-25", "http_status"),
        ("not-found-sessionless", "2026-07-28", "http_status"),
    ] {
        let (actual, calls) = fixture::observe(case, revision).await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["value"]["reason"], reason, "{case} {revision}");
        assert_eq!(actual["value"]["observation"]["http_status"], 404);
    }
}

#[tokio::test]
async fn error_statuses_keep_bounds_and_do_not_promote_invalid_or_success_bodies() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for (case, reason) in [
            ("status-success", "http_status"),
            ("status-invalid-envelope", "http_status"),
            ("status-ambiguous", "http_status"),
            ("status-duplicate-id", "http_status"),
            ("status-bound", "response_bound"),
            ("status-loss", "transport_failure"),
            ("status-deadline", "deadline_exhausted"),
            ("status-unauthorized", "authorization_required"),
            ("status-forbidden", "authorization_required"),
        ] {
            let (actual, calls) = fixture::observe(case, revision).await.unwrap();
            assert_eq!(calls, 1);
            assert_eq!(actual["kind"], "refused");
            assert_eq!(actual["value"]["reason"], reason, "{case} {revision}");
            assert_eq!(actual["value"]["observation"]["terminal"], "incomplete");
        }
    }
    let (actual, calls) = fixture::observe("status-peer-error", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["value"]["reason"], "http_status");
}

#[tokio::test]
async fn exact_json_and_peer_data_survive_real_http() {
    for revision in ["2025-11-25", "2026-07-28"] {
        let (actual, calls) = fixture::observe("json-complete", revision).await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["kind"], "result");
        assert_eq!(
            actual["value"]["observation"]["terminal"],
            "correlated_terminal"
        );
        assert!(
            actual["value"]["result"]
                .as_object()
                .unwrap()
                .contains_key("structuredContent")
        );
        assert_eq!(actual["value"]["result"]["extension"]["kept"], true);
        for (case, presence) in [
            ("peer-data-absent", "absent"),
            ("peer-data-null", "present"),
        ] {
            let (actual, calls) = fixture::observe(case, revision).await.unwrap();
            assert_eq!(calls, 1);
            assert_eq!(actual["kind"], "peer_error");
            assert_eq!(actual["value"]["error"]["code"], -32099);
            assert_eq!(actual["value"]["error"]["data"]["presence"], presence);
            if presence == "present" {
                assert!(actual["value"]["error"]["data"]["value"].is_null());
            }
        }
    }
}

#[tokio::test]
async fn real_http_refusals_keep_actual_counts_and_terminal_knowledge() {
    for (case, reason, calls) in [
        ("wrong-id", "invalid_response", 1),
        ("exclusive-result-error", "invalid_response", 1),
        ("response-bound", "response_bound", 1),
        ("request-bound-before-send", "request_bound", 0),
        ("body-loss", "transport_failure", 1),
        ("deadline", "deadline_exhausted", 1),
        ("session-expired-once", "session_expired", 1),
        ("sse-prefix-incomplete", "invalid_response", 1),
        ("sse-event-bound", "sse_event_bound", 1),
    ] {
        let (actual, count) = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(count, calls, "{case}");
        assert_eq!(actual["kind"], "refused", "{case}");
        assert_eq!(actual["value"]["reason"], reason, "{case}");
        assert_eq!(
            actual["value"]["observation"]["terminal"], "incomplete",
            "{case}"
        );
    }
}

#[tokio::test]
async fn sse_requires_a_complete_event_boundary() {
    let (actual, calls) = fixture::observe("sse-terminal", "2026-07-28")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["kind"], "result");
    assert_eq!(
        actual["value"]["observation"]["terminal"],
        "correlated_terminal"
    );
}

#[tokio::test]
async fn incomplete_sse_retains_received_data_without_inventing_a_terminal() {
    let (actual, calls) = fixture::observe("sse-prefix-incomplete", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    let response = &actual["value"]["observation"]["response"];
    let bytes = STANDARD
        .decode(response["bytes"].as_str().unwrap())
        .unwrap();
    assert!(
        !bytes.is_empty(),
        "received SSE data vanished because its line was incomplete"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"],
        17
    );
    assert_eq!(response["retention"], "prefix");
    assert_eq!(response["message_length"]["kind"], "at_least");
    assert_eq!(response["counts"]["retained_octets"], bytes.len());
    assert_eq!(actual["value"]["observation"]["terminal"], "incomplete");
}

#[tokio::test]
async fn exact_byte_bounds_and_non_utf8_prefixes_are_observations() {
    let (exact, _) = fixture::observe("exact-response-bound", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(exact["kind"], "result");
    let response = &exact["value"]["observation"]["response"];
    let bytes = STANDARD
        .decode(response["bytes"].as_str().unwrap())
        .unwrap();
    assert_eq!(response["message_length"]["value"], bytes.len());
    assert_eq!(response["counts"]["limit_octets"], bytes.len());
    assert!(
        bytes.starts_with(b"{ \"jsonrpc\""),
        "wire whitespace was lost"
    );
    for case in ["zero-response-bound", "utf8-bound"] {
        let (actual, calls) = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["value"]["reason"], "response_bound");
        let response = &actual["value"]["observation"]["response"];
        let bytes = STANDARD
            .decode(response["bytes"].as_str().unwrap())
            .unwrap();
        assert_eq!(response["counts"]["retained_octets"], bytes.len());
        assert_eq!(response["retention"], "prefix");
        if case == "zero-response-bound" {
            assert!(bytes.is_empty());
        } else {
            assert_eq!(bytes.last(), Some(&0xe2));
            assert!(std::str::from_utf8(&bytes).is_err());
        }
    }
}

#[tokio::test]
async fn invalid_structural_carriers_are_refused_before_io() {
    use serde_json::json;
    for (pointer, value) in [
        ("/budget/request_octets", json!(-1)),
        ("/budget/response_octets", json!(0.5)),
        ("/budget/sse_event_octets", json!(-1)),
        ("/budget/provider_ms", json!(0)),
        ("/budget/connect_ms", json!(-5)),
        ("/budget/remaining_execution_ms", json!(1.5)),
        ("/request_id/value", json!(17.5)),
        ("/request_id/value", json!(18)),
        ("/encoded_request", json!("not canonical base64")),
    ] {
        let (actual, calls) = fixture::observe_input("json-complete", "2025-11-25", |input| {
            *input.pointer_mut(pointer).unwrap() = value;
        })
        .await
        .unwrap();
        assert_eq!(calls, 0, "{pointer}");
        assert_eq!(actual["value"]["reason"], "invalid_input", "{pointer}");
        assert_eq!(
            actual["value"]["observation"]["send"], "not_sent",
            "{pointer}"
        );
    }
}

#[tokio::test]
async fn peer_integers_and_ambiguous_envelopes_keep_their_distinctions() {
    for case in ["duplicate-id", "null-id", "fractional-peer-code"] {
        let (actual, calls) = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["value"]["reason"], "invalid_response", "{case}");
    }
    let (actual, calls) = fixture::observe("huge-peer-code", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(
        actual["value"]["error"]["code"].to_string(),
        "123456789012345678901234567890"
    );
    let (actual, calls) = fixture::observe("redirect", "2025-11-25").await.unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["value"]["reason"], "http_status");
}

#[tokio::test]
async fn sse_framing_variants_and_independent_bounds() {
    for case in [
        "sse-crlf",
        "sse-cr",
        "sse-multiline",
        "sse-progress",
        "sse-event-exact",
        "sse-bom",
    ] {
        let (actual, calls) = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["kind"], "result", "{case}: {actual}");
    }
    for (case, reason) in [
        ("sse-zero-bound", "sse_event_bound"),
        ("sse-response-bound", "response_bound"),
    ] {
        let (actual, calls) = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual["value"]["reason"], reason, "{case}");
    }
}

#[tokio::test]
async fn opaque_peer_objects_do_not_become_codec_private_numbers() {
    let (actual, calls) = fixture::observe("reserved-json-key", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["kind"], "result");
    let extension = &actual["value"]["result"]["extension"];
    assert!(
        extension.is_object(),
        "opaque object was reinterpreted as a codec-private number: {extension}"
    );
    assert_eq!(extension["$serde_json::private::Number"], "7");
    let (actual, calls) = fixture::observe("reserved-peer-data", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["kind"], "peer_error");
    let data = &actual["value"]["error"]["data"]["value"];
    assert!(data.is_object());
    assert_eq!(data["$serde_json::private::Number"], "7");
    assert_eq!(data["$serde_json::private::RawValue"], "null");
}

#[tokio::test]
async fn invalid_http_status_cannot_enter_the_constrained_carrier() {
    let (actual, calls) = fixture::observe("invalid-http-status", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(actual["value"]["reason"], "invalid_response");
    assert!(actual["value"]["observation"].get("http_status").is_none());
}
