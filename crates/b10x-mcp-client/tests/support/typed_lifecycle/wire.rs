//! Real HTTP cancellation, completed prefixes and dropped refresh invalidation.
use super::{
    encode, end, limits,
    peer::{self, Reply},
    summary,
};
use b10x_mcp_client::{schema_worker::SchemaWorker, strict_cancellation::Cancellation};
use b10x_mcp_types::http_exchange::McpHttpDiscoveryFamily as Family;
use serde_json::{Value, json};
use tokio::time::{Duration, Instant};
fn tool() -> Value {
    json!({"name":"run","inputSchema":{"type":"object"}})
}
fn script(case: &str, modern: bool) -> Vec<Reply> {
    let timeout = case.contains("timeout");
    let drop_refresh = case == "discovery-drop-refresh";
    let mut replies = Vec::new();
    if case.starts_with("discovery-") {
        if drop_refresh {
            replies.push(Reply::result(
                "tools/list",
                peer::page("tools", json!([tool()]), modern),
            ));
        }
        if case != "discovery-first" {
            let mut page = peer::page("tools", json!([tool()]), modern);
            page["nextCursor"] = json!("next");
            replies.push(Reply::result("tools/list", page));
        }
        replies.push(Reply::stall("tools/list", !timeout && !drop_refresh));
    } else {
        let (list, method, rows) = if case.starts_with("resource-") {
            (
                "resources/list",
                "resources/read",
                json!([{"name":"one","uri":"test:one"}]),
            )
        } else if case.starts_with("prompt-") {
            ("prompts/list", "prompts/get", json!([{"name":"ask"}]))
        } else {
            ("tools/list", "tools/call", json!([tool()]))
        };
        replies.push(Reply::result(
            list,
            peer::page(list.split('/').next().unwrap(), rows, modern),
        ));
        replies.push(Reply::stall(method, !timeout));
    }
    if !modern && !drop_refresh && case != "tool-expired-teardown" {
        replies.push(Reply::result("notifications/cancelled", Value::Null));
    }
    replies
}
pub async fn observe(case: &str, modern: bool) -> Result<Value, String> {
    let signal = Cancellation::default();
    let worker = SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(), 65536)
        .map_err(|e| format!("worker: {e:?}"))?;
    let (mut client, peer) = peer::start(modern, script(case, modern), worker, &signal).await?;
    let family = if case.starts_with("resource-") {
        Family::V1
    } else if case.starts_with("prompt-") {
        Family::V0
    } else {
        Family::V2
    };
    if !case.starts_with("discovery-") || case == "discovery-drop-refresh" {
        client
            .discover(family.clone(), &limits(), end())
            .await
            .map_err(|e| format!("initial catalog: {e:?}"))?;
    }
    let operation = if case.contains("timeout") {
        Instant::now() + Duration::from_millis(500)
    } else {
        end()
    };
    let teardown = if case == "tool-expired-teardown" {
        Instant::now()
    } else {
        end()
    };
    let outcome = if case == "discovery-drop-refresh" {
        let limits = limits();
        let mut refresh =
            Box::pin(client.discover_cancellable(family, &limits, operation, &signal, teardown));
        let finished = tokio::select! {
            value=&mut refresh=>Some(value),
            started=peer.started()=>{started?;None}
        };
        drop(refresh);
        match finished {
            Some(value) => encode(value)?,
            None => json!({"kind":"dropped","value":{}}),
        }
    } else if case.starts_with("discovery-") {
        encode(
            client
                .discover_cancellable(family, &limits(), operation, &signal, teardown)
                .await,
        )?
    } else if case.starts_with("resource-") {
        encode(
            client
                .read_resource_cancellable("test:one", operation, &signal, teardown)
                .await,
        )?
    } else if case.starts_with("prompt-") {
        encode(
            client
                .get_prompt_cancellable("ask", json!({}), operation, &signal, teardown)
                .await,
        )?
    } else {
        encode(
            client
                .call_tool_cancellable("run", json!({}), operation, &signal, teardown)
                .await,
        )?
    };
    let mut reuse_refused = false;
    if case.starts_with("discovery-") {
        let fresh = Cancellation::default();
        let attempt = encode(
            client
                .call_tool_cancellable("run", json!({}), end(), &fresh, end())
                .await,
        )?;
        reuse_refused = attempt["kind"] == "refused"
            && attempt["value"]["refusal"]["reason"] == "unsupported_family";
    }
    let items = client.tools().count();
    drop(client);
    let (calls, closed) = peer.finish().await?;
    let mut observed = summary(&outcome, &calls, closed, items, modern);
    observed["reuse_refused"] = json!(reuse_refused);
    Ok(observed)
}
