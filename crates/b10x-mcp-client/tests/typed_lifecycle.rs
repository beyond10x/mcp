//! Controlled typed calls preserve admission and real peer observations.
#![cfg(feature = "strict-http")]
#[path = "support/strict_invocation.rs"]
mod fixture;
use b10x_mcp_client::strict_cancellation::Cancellation;
use b10x_mcp_types::http_exchange::{
    McpHttpDiscoveryFamily as Family, McpHttpDiscoveryListLimits as Limits,
};
use fixture::{deadline, discover, page, result, start, tool};
use serde_json::{Value, json};
fn limits() -> Limits {
    serde_json::from_value(json!({"max_pages":3,"max_items":16,"descriptor_octets":16384})).unwrap()
}
#[tokio::test]
async fn controlled_tools_keep_parameter_headers_and_opaque_content() {
    for modern in [false, true] {
        let schema = json!({"type":"object","properties":{"region":{"type":"string","x-mcp-header":"Region"}}});
        let mut descriptor = tool(schema);
        descriptor["outputSchema"] = json!({"type":"object","required":["count"]});
        descriptor["extension"] = json!({"$serde_json::private::Number":"7"});
        let large: Value = serde_json::from_str("123456789012345678901234567890").unwrap();
        let structured = json!({"count":large,"opaque":{"$serde_json::private::Number":"7"}});
        let (mut client, peer) = start(modern, vec![
            ("tools/list", page("tools", json!([descriptor]), modern)),
            ("tools/call", result(json!({"content":[{"type":"text","text":"kept","opaque":{"$serde_json::private::Number":"7"}}],"structuredContent":structured}), modern)),
        ]).await;
        let signal = Cancellation::default();
        let catalog = serde_json::to_value(
            client
                .discover_cancellable(Family::V2, &limits(), deadline(), &signal, deadline())
                .await,
        )
        .unwrap();
        assert_eq!(catalog["kind"], "completed");
        assert_eq!(
            catalog["value"]["descriptors"][0]["value"]["raw"]["extension"]["$serde_json::private::Number"],
            "7"
        );
        let outcome = serde_json::to_value(
            client
                .call_tool_cancellable(
                    "run",
                    json!({"region":"eu"}),
                    deadline(),
                    &signal,
                    deadline(),
                )
                .await,
        )
        .unwrap();
        assert_eq!(outcome["kind"], "completed", "{outcome}");
        assert_eq!(outcome["value"]["structured_content"], structured);
        assert_eq!(
            outcome["value"]["exchange"]["result"]["content"][0]["opaque"]["$serde_json::private::Number"],
            "7"
        );
        let calls = peer.finish().await;
        let call = calls.last().unwrap();
        assert_eq!(call["body"]["method"], "tools/call");
        assert_eq!(call["body"]["params"]["arguments"]["region"], "eu");
        if modern {
            assert_eq!(call["headers"]["mcp-param-region"], "eu");
        } else {
            assert!(call["headers"].get("mcp-param-region").is_none());
        }
    }
}
#[tokio::test]
async fn controlled_resource_and_prompt_use_their_owned_catalogs() {
    for modern in [false, true] {
        let (mut client, peer) = start(modern, vec![
            ("resources/list", page("resources", json!([{"uri":"test:one","name":"one"}]), modern)),
            ("resources/read", page("contents", json!([{"uri":"test:one","text":"hello"}]), modern)),
            ("prompts/list", page("prompts", json!([{"name":"ask","arguments":[{"name":"topic","required":true}]}]), modern)),
            ("prompts/get", result(json!({"messages":[{"role":"user","content":{"type":"text","text":"untrusted instructions"}}]}), modern)),
        ]).await;
        let signal = Cancellation::default();
        discover(&mut client, Family::V1).await;
        let resource = serde_json::to_value(
            client
                .read_resource_cancellable("test:one", deadline(), &signal, deadline())
                .await,
        )
        .unwrap();
        assert_eq!(resource["kind"], "completed", "{resource}");
        assert_eq!(
            resource["value"]["exchange"]["result"]["contents"][0]["text"],
            "hello"
        );
        discover(&mut client, Family::V0).await;
        let invalid = serde_json::to_value(
            client
                .get_prompt_cancellable("ask", json!({}), deadline(), &signal, deadline())
                .await,
        )
        .unwrap();
        assert_eq!(invalid["kind"], "refused");
        assert_eq!(invalid["value"]["refusal"]["reason"], "invalid_input");
        assert!(invalid["value"]["refusal"].get("exchange").is_none());
        let prompt = serde_json::to_value(
            client
                .get_prompt_cancellable(
                    "ask",
                    json!({"topic":"a"}),
                    deadline(),
                    &signal,
                    deadline(),
                )
                .await,
        )
        .unwrap();
        assert_eq!(prompt["kind"], "completed", "{prompt}");
        assert_eq!(
            prompt["value"]["exchange"]["result"]["messages"][0]["content"]["text"],
            "untrusted instructions"
        );
        assert_eq!(peer.finish().await.len(), if modern { 5 } else { 6 });
    }
}
#[tokio::test]
async fn pre_cancelled_typed_calls_allocate_no_exchange_or_worker_observation() {
    for modern in [false, true] {
        let (mut client, peer) = start(modern, vec![]).await;
        let signal = Cancellation::default();
        signal.cancel();
        let outcomes = [
            serde_json::to_value(
                client
                    .call_tool_cancellable("unadmitted", json!({}), deadline(), &signal, deadline())
                    .await,
            )
            .unwrap(),
            serde_json::to_value(
                client
                    .read_resource_cancellable("unadmitted", deadline(), &signal, deadline())
                    .await,
            )
            .unwrap(),
            serde_json::to_value(
                client
                    .get_prompt_cancellable(
                        "unadmitted",
                        json!({}),
                        deadline(),
                        &signal,
                        deadline(),
                    )
                    .await,
            )
            .unwrap(),
        ];
        for outcome in outcomes {
            assert_eq!(outcome["kind"], "interrupted", "{outcome}");
            assert_eq!(outcome["value"]["phase"], "admission");
            assert_eq!(outcome["value"]["cause"], "caller_cancelled");
            for key in ["worker", "exchange", "cancellation"] {
                assert!(outcome["value"].get(key).is_none());
            }
        }
        assert_eq!(peer.finish().await.len(), if modern { 1 } else { 2 });
    }
}
#[tokio::test]
async fn pre_cancelled_refresh_invalidates_the_previous_tool_catalog() {
    for modern in [false, true] {
        let (mut client, peer) = start(
            modern,
            vec![(
                "tools/list",
                page("tools", json!([tool(json!({"type":"object"}))]), modern),
            )],
        )
        .await;
        discover(&mut client, Family::V2).await;
        assert_eq!(client.tools().count(), 1);
        let signal = Cancellation::default();
        signal.cancel();
        let refresh = serde_json::to_value(
            client
                .discover_cancellable(Family::V2, &limits(), deadline(), &signal, deadline())
                .await,
        )
        .unwrap();
        assert_eq!(refresh["kind"], "interrupted", "{refresh}");
        assert_eq!(refresh["value"]["cause"], "caller_cancelled");
        assert_eq!(refresh["value"]["exchanges"], json!([]));
        assert!(refresh["value"].get("cancellation").is_none());
        assert_eq!(client.tools().count(), 0);
        let fresh = Cancellation::default();
        let call = serde_json::to_value(
            client
                .call_tool_cancellable("run", json!({}), deadline(), &fresh, deadline())
                .await,
        )
        .unwrap();
        assert_eq!(call["kind"], "refused");
        assert_eq!(call["value"]["refusal"]["reason"], "unsupported_family");
        assert_eq!(peer.finish().await.len(), if modern { 2 } else { 3 });
    }
}
