//! Real selected-family list traversal, with literal externally observed outcomes.
#![cfg(feature = "strict-http")]
#[path = "support/strict_discovery.rs"]
mod fixture;
#[tokio::test]
async fn complete_lists_keep_both_pages_and_opaque_fields() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for family in ["tools", "resources", "prompts"] {
            let actual = fixture::observe("list-complete", family, revision)
                .await
                .unwrap();
            assert_eq!(actual["complete"], true, "{family}/{revision}: {actual}");
            assert_eq!(
                actual["catalog"]["descriptors"].as_array().unwrap().len(),
                2
            );
            assert_eq!(actual["catalog"]["pages"].as_array().unwrap().len(), 2);
        }
    }
}

fn lists(actual: &serde_json::Value) -> Vec<&serde_json::Value> {
    actual["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["body"]["method"].as_str().unwrap().ends_with("/list"))
        .collect()
}
#[tokio::test]
async fn cursors_metadata_ids_and_raw_descriptors_are_actual_wire_observations() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for family in ["tools", "resources", "prompts"] {
            let actual = fixture::observe("list-empty-cursor", family, revision)
                .await
                .unwrap();
            assert_eq!(actual["complete"], true, "{actual}");
            let requests = lists(&actual);
            assert_eq!(requests.len(), 2);
            assert_eq!(requests[0]["body"]["id"], 2);
            assert_eq!(requests[1]["body"]["id"], 3);
            assert!(requests[0]["body"]["params"].get("cursor").is_none());
            assert_eq!(requests[1]["body"]["params"]["cursor"], "");
            for request in requests {
                assert_eq!(request["headers"]["mcp-protocol-version"], revision);
                if revision == "2026-07-28" {
                    assert_eq!(request["headers"]["mcp-method"], format!("{family}/list"));
                    assert_eq!(
                        request["body"]["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
                        revision
                    );
                }
            }
            let row = &actual["catalog"]["descriptors"][0]["value"];
            assert_eq!(row["name"], "one");
            match family {
                "tools" => assert_eq!(row["raw"]["extension"]["$serde_json::private::Number"], "7"),
                "resources" => {
                    assert_eq!(row["uri"], "test:one");
                    assert_eq!(row["size"], 2.5);
                }
                _ => {
                    assert_eq!(row["arguments"][0]["required"], true);
                    assert_eq!(
                        row["arguments"][0]["raw"]["extra"]["$serde_json::private::Number"],
                        "7"
                    );
                }
            }
            if revision == "2026-07-28" {
                assert_eq!(actual["catalog"]["pages"][0]["cache_hints"]["ttl_ms"], 0);
            } else {
                assert!(actual["catalog"]["pages"][0].get("cache_hints").is_none());
            }
        }
    }
}
#[tokio::test]
async fn limits_and_bad_later_pages_never_return_a_partial_catalog() {
    for revision in ["2025-11-25", "2026-07-28"] {
        for family in ["tools", "resources", "prompts"] {
            for (case, reason, calls) in [
                ("list-zero-pages", "page_limit", 0),
                ("list-invalid-limits", "invalid_limits", 0),
                ("list-page-limit", "page_limit", 1),
                ("list-repeated-cursor", "page_limit", 2),
                ("list-item-limit", "item_limit", 2),
                ("list-descriptor-limit", "descriptor_limit", 1),
                ("list-duplicate", "duplicate_identity", 2),
                ("list-invalid-descriptor", "invalid_descriptor", 1),
                ("list-invalid-page", "invalid_page", 1),
                ("list-null-cursor", "invalid_page", 1),
                ("list-late-error", "exchange_refused", 2),
                ("list-unsupported-family", "unsupported_family", 0),
            ] {
                let actual = fixture::observe(case, family, revision).await.unwrap();
                assert_eq!(
                    actual["complete"], false,
                    "{case}/{family}/{revision}: {actual}"
                );
                assert_eq!(actual["refusal"]["reason"], reason, "{actual}");
                assert_eq!(lists(&actual).len(), calls, "{actual}");
                assert_eq!(
                    actual["refusal"]["exchanges"].as_array().unwrap().len(),
                    calls,
                    "{actual}"
                );
                assert!(actual.get("catalog").is_none());
                if case == "list-repeated-cursor" {
                    assert_eq!(
                        lists(&actual)[1]["body"]["params"]["cursor"],
                        "opaque/cursor?☃"
                    );
                }
                if case == "list-late-error" {
                    assert_eq!(
                        actual["refusal"]["exchanges"][1]["value"]["error"]["code"],
                        -32603
                    );
                }
            }
        }
    }
}
#[tokio::test]
async fn exact_descriptor_bound_and_empty_zero_item_list_are_valid() {
    for family in ["tools", "resources", "prompts"] {
        for (case, items) in [("list-exact-descriptor", 1), ("list-empty-zero-items", 0)] {
            let actual = fixture::observe(case, family, "2026-07-28").await.unwrap();
            assert_eq!(actual["complete"], true, "{actual}");
            assert_eq!(lists(&actual).len(), 1);
            assert_eq!(
                actual["catalog"]["descriptors"].as_array().unwrap().len(),
                items
            );
        }
    }
}
#[tokio::test]
async fn modern_result_selection_and_descriptor_shapes_are_checked() {
    for family in ["tools", "resources", "prompts"] {
        for (case, reason) in [
            ("list-missing-cache", "invalid_page"),
            ("list-unselected-result", "unsupported_result"),
        ] {
            let actual = fixture::observe(case, family, "2026-07-28").await.unwrap();
            assert_eq!(actual["refusal"]["reason"], reason, "{actual}");
            assert_eq!(lists(&actual).len(), 1);
        }
    }
    for (case, family) in [
        ("list-invalid-schema", "tools"),
        ("list-invalid-argument", "prompts"),
    ] {
        let actual = fixture::observe(case, family, "2025-11-25").await.unwrap();
        assert_eq!(
            actual["refusal"]["reason"], "invalid_descriptor",
            "{actual}"
        );
        assert_eq!(lists(&actual).len(), 1);
    }
}
#[tokio::test]
async fn provider_deadline_is_shared_across_pages() {
    for revision in ["2025-11-25", "2026-07-28"] {
        let actual = fixture::observe("list-deadline", "tools", revision)
            .await
            .unwrap();
        assert_eq!(
            actual["refusal"]["reason"], "deadline_exhausted",
            "{actual}"
        );
        assert_eq!(lists(&actual).len(), 2);
        assert!(actual.get("catalog").is_none());
    }
}

#[tokio::test]
async fn prompt_extensions_do_not_inherit_other_families_known_fields() {
    for revision in ["2025-11-25", "2026-07-28"] {
        let actual = fixture::observe("list-prompt-extension", "prompts", revision)
            .await
            .unwrap();
        assert_eq!(actual["complete"], true, "{actual}");
        assert_eq!(
            actual["catalog"]["descriptors"][0]["value"]["raw"]["annotations"],
            serde_json::json!(["opaque extension"])
        );
    }
}
