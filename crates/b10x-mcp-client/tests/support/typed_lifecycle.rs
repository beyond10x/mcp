//! Actual typed-client lifecycle observations, consumed by native and ESS tests.
#[path = "typed_lifecycle/peer.rs"]
mod peer;
#[path = "typed_lifecycle/wire.rs"]
mod wire;
#[path = "typed_lifecycle/worker.rs"]
mod worker;
use b10x_mcp_types::http_exchange::McpHttpDiscoveryListLimits as Limits;
use serde_json::{Value, json};
use tokio::time::{Duration, Instant};
fn limits() -> Limits {
    serde_json::from_value(json!({"max_pages":3,"max_items":16,"descriptor_octets":16384})).unwrap()
}
fn end() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
fn encode(value: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}
fn summary(outcome: &Value, calls: &[Value], closed: usize, items: usize, modern: bool) -> Value {
    let body = &outcome["value"];
    let notifications: Vec<_> = calls
        .iter()
        .filter(|c| c["body"]["method"] == "notifications/cancelled")
        .collect();
    let last = calls.iter().rev().find(|c| c["body"].get("id").is_some());
    let exchange = body
        .get("exchange")
        .or_else(|| body.get("refusal").and_then(|v| v.get("exchange")));
    let revision = if modern { "2026-07-28" } else { "2025-11-25" };
    json!({
        "kind":outcome["kind"],"phase":body["phase"].as_str().unwrap_or("absent"),
        "cause":body["cause"].as_str().unwrap_or("absent"),
        "reason":body["refusal"]["reason"].as_str().or_else(||body["reason"].as_str()).unwrap_or("absent"),
        "worker_state":body["worker"]["state"].as_str().unwrap_or("absent"),
        "worker_kill":body["worker"]["kill"].as_str().unwrap_or("absent"),
        "business_calls":calls.iter().filter(|c|matches!(c["body"]["method"].as_str(),Some("tools/call"|"resources/read"|"prompts/get"))).count(),
        "list_calls":calls.iter().filter(|c|c["body"]["method"].as_str().is_some_and(|m|m.ends_with("/list"))).count(),
        "notifications":notifications.len(),"closed":closed,"catalog_items":items,
        "exchange_retained":exchange.is_some(),"prior_pages":body["exchanges"].as_array().map_or(0,Vec::len),
        "body_preserved":exchange.is_some_and(|e|e["value"]["result"]["structuredContent"]["proof"]=="peer-result" && e["value"]["result"]["structuredContent"]["opaque"]["$serde_json::private::Number"]=="7"),
        "control_reason":body["cancellation"]["notification"]["refusal"].as_str().unwrap_or("absent"),
        "attempted":matches!(body["cancellation"]["interrupted"]["exchange"]["send"].as_str(),Some("unknown"|"send_observed")),
        "id_agreement":notifications.iter().all(|n|last.is_some_and(|r|n["body"]["params"]["requestId"]==r["body"]["id"])),
        "protocol_agreement":calls.iter().filter(|c|!matches!(c["body"]["method"].as_str(),Some("initialize"|"server/discover"|"notifications/initialized"))).all(|c|c["headers"]["mcp-protocol-version"]==revision && (!modern || c["headers"]["mcp-method"]==c["body"]["method"])),
        "worker_gone":false,"reuse_refused":false,"explicit_reaped":false
    })
}
pub async fn observations(case: &str, revision: &str) -> Result<Value, String> {
    let modern = match revision {
        "2026-07-28" => true,
        "2025-11-25" => false,
        _ => return Err("unsupported revision".into()),
    };
    match case {
        "worker-input"
        | "worker-preflight"
        | "worker-output"
        | "worker-retained"
        | "worker-drop"
        | "worker-output-bound" => worker::observe(case, modern).await,
        "tool-wire"
        | "resource-wire"
        | "prompt-wire"
        | "resource-timeout-wire"
        | "tool-expired-teardown"
        | "discovery-first"
        | "discovery-later"
        | "discovery-timeout-later"
        | "discovery-drop-refresh" => wire::observe(case, modern).await,
        _ => Err("unknown typed lifecycle case".into()),
    }
}
