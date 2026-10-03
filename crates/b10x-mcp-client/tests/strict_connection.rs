//! Real revision-fixed HTTP setup; the fixture parses requests independently.
#![cfg(feature = "strict-http")]
#[path = "support/strict_connection.rs"]
mod fixture;

#[tokio::test]
async fn cache_hint_integers_preserve_valid_zero_and_large_values() {
    // The JSON number parser normalizes integral -0 to 0; both satisfy ttl >= 0.
    for (case, number) in [
        ("setup-zero-ttl", "0"),
        ("setup-large-ttl", "123456789012345678901234567890"),
    ] {
        let actual = fixture::observe(case, "2026-07-28").await.unwrap();
        assert_eq!(actual["ready"], true, "{case}: {actual}");
        assert_eq!(
            actual["description"]["cache_hints"]["ttl_ms"].to_string(),
            number
        );
    }
}

#[tokio::test]
async fn setup_uses_each_revisions_actual_wire_sequence() {
    for revision in ["2026-07-28", "2025-11-25"] {
        let actual = fixture::observe("setup-valid", revision).await.unwrap();
        assert_eq!(actual["ready"], true, "{revision}: {actual}");
        assert_eq!(
            actual["calls"],
            if revision == "2026-07-28" { 1 } else { 2 }
        );
    }
}

#[tokio::test]
async fn peer_description_keeps_presence_raw_fields_and_no_selected_version_change() {
    let actual = fixture::observe("setup-valid", "2026-07-28").await.unwrap();
    let description = &actual["description"];
    assert_eq!(description["configured_revision"], "2026-07-28");
    assert_eq!(description["reported_versions"][1], "future");
    assert_eq!(description["capabilities"]["tools"], serde_json::json!({}));
    assert_eq!(
        description["raw_result"]["extension"]["$serde_json::private::Number"],
        "7"
    );
    assert_eq!(description["cache_hints"]["ttl_ms"], 0);
    assert_eq!(
        actual["requests"][0]["body"]["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
        serde_json::json!({})
    );
    assert_eq!(
        actual["requests"][0]["headers"]["mcp-method"],
        "server/discover"
    );
    assert!(!actual["debug"].as_str().unwrap().contains("fixture-secret"));
    let actual = fixture::observe("setup-valid", "2025-11-25").await.unwrap();
    assert!(actual["description"].get("cache_hints").is_none());
    assert!(
        actual["description"]["capabilities"]
            .get("resources")
            .is_none()
    );
    assert_eq!(
        actual["requests"][0]["body"]["params"]["capabilities"],
        serde_json::json!({})
    );
    assert!(
        actual["requests"][0]["headers"]
            .get("mcp-session-id")
            .is_none()
    );
    assert_eq!(
        actual["requests"][1]["headers"]["mcp-session-id"],
        "fixture-secret"
    );
    assert!(actual["requests"][1]["body"].get("id").is_none());
}

#[tokio::test]
async fn setup_refuses_before_a_usable_handle_and_never_falls_back() {
    for revision in ["2026-07-28", "2025-11-25"] {
        for (case, reason, calls) in [
            ("setup-revision-mismatch", "revision_mismatch", 1),
            ("setup-invalid-description", "invalid_description", 1),
            ("setup-peer-error", "peer_error", 1),
            ("setup-ambiguous400", "exchange_refused", 1),
            ("setup-request-bound", "exchange_refused", 0),
            ("setup-invalid-input", "invalid_input", 0),
        ] {
            let actual = fixture::observe(case, revision).await.unwrap();
            assert_eq!(actual["ready"], false, "{case} {revision}: {actual}");
            assert_eq!(actual["calls"], calls, "{case} {revision}");
            assert_eq!(actual["error"]["reason"], reason, "{case} {revision}");
        }
    }
}

#[tokio::test]
async fn legacy_session_and_notification_are_verified_before_ready() {
    for (case, reason, calls) in [
        ("setup-invalid-session", "invalid_session_header", 1),
        ("setup-duplicate-session", "invalid_session_header", 1),
        ("setup-session-bound", "session_header_bound", 1),
        ("setup-notification-refused", "notification_refused", 2),
        ("setup-notification-body", "notification_refused", 2),
        ("setup-missing-info", "invalid_description", 1),
        ("setup-deadline", "deadline_exhausted", 2),
    ] {
        let actual = fixture::observe(case, "2025-11-25").await.unwrap();
        assert_eq!(actual["ready"], false, "{case}: {actual}");
        assert_eq!(actual["calls"], calls);
        assert_eq!(actual["error"]["reason"], reason, "{case}");
    }
    let actual = fixture::observe("setup-sessionless", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(actual["ready"], true);
    assert!(
        actual["requests"][1]["headers"]
            .get("mcp-session-id")
            .is_none()
    );
}

#[tokio::test]
async fn modern_description_checks_its_own_shape_without_sessions() {
    for case in [
        "setup-missing-cache",
        "setup-negative-ttl",
        "setup-unknown-result",
    ] {
        let actual = fixture::observe(case, "2026-07-28").await.unwrap();
        assert_eq!(actual["ready"], false);
        assert_eq!(actual["error"]["reason"], "invalid_description");
    }
    for case in [
        "setup-missing-info",
        "setup-invalid-session",
        "setup-duplicate-session",
    ] {
        let actual = fixture::observe(case, "2026-07-28").await.unwrap();
        assert_eq!(actual["ready"], true, "{case}: {actual}");
        assert_eq!(actual["calls"], 1);
        assert!(
            actual["requests"][0]["headers"]
                .get("mcp-session-id")
                .is_none()
        );
    }
}

#[tokio::test]
async fn raw_exchange_reuses_the_bound_revision_session_and_modern_metadata() {
    for revision in ["2026-07-28", "2025-11-25"] {
        let actual = fixture::observe("setup-reuse", revision).await.unwrap();
        assert_eq!(actual["ready"], true);
        assert_eq!(actual["business"]["kind"], "result");
        let requests = actual["requests"].as_array().unwrap();
        let last = requests.last().unwrap();
        assert_eq!(last["body"]["method"], "tools/call");
        assert_eq!(last["headers"]["mcp-protocol-version"], revision);
        if revision == "2025-11-25" {
            assert_eq!(last["headers"]["mcp-session-id"], "fixture-secret");
        } else {
            assert!(last["headers"].get("mcp-session-id").is_none());
            assert_eq!(last["headers"]["mcp-method"], "tools/call");
            assert_eq!(last["headers"]["mcp-name"], "effect");
            assert_eq!(
                last["body"]["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
                revision
            );
        }
    }
    let actual = fixture::observe("setup-metadata-conflict", "2026-07-28")
        .await
        .unwrap();
    assert_eq!(actual["business_error"], true);
    assert_eq!(actual["calls"], 1);
    let actual = fixture::observe("setup-missing-family", "2025-11-25")
        .await
        .unwrap();
    assert_eq!(actual["business_error"], true);
    assert_eq!(actual["calls"], 2);
    let actual = fixture::observe("setup-unicode-name", "2026-07-28")
        .await
        .unwrap();
    assert_eq!(
        actual["requests"][1]["headers"]["mcp-name"],
        "=?base64?c25vd21hbuKYgw==?="
    );
}
