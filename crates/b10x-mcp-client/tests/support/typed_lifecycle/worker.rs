//! Real child barriers before dispatch, during preflight and after peer completion.
use super::{encode, end, limits, peer, summary};
use b10x_mcp_client::{
    schema_worker::SchemaWorker, strict_cancellation::Cancellation,
    strict_invocation::InvocationClient,
};
use b10x_mcp_types::http_exchange::McpHttpDiscoveryFamily as Family;
use serde_json::{Value, json};
use std::path::Path;
use tokio::time::{Duration, Instant};
fn pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.parse().ok()
}
async fn wait_pid(path: &Path) -> Result<(), String> {
    let until = Instant::now() + Duration::from_secs(2);
    while pid(path).is_none() {
        if Instant::now() >= until {
            return Err("worker startup deadline".into());
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    Ok(())
}
fn retained(case: &str) -> bool {
    matches!(case, "worker-retained" | "worker-drop")
}
fn descriptor(case: &str, path: &Path) -> Value {
    let mut input = json!({"type":"object","mode":"complete"});
    let mut output = json!({"type":"object"});
    match case {
        "worker-input" | "worker-retained" | "worker-drop" => {
            input["mode"] = json!("sleep");
            input["fixture_pid"] = json!(path);
        }
        "worker-preflight" => {
            output["mode"] = json!("sleep");
            output["fixture_pid"] = json!(path);
        }
        "worker-output" => {
            output["mode"] = json!("validate-sleep");
            output["fixture_pid"] = json!(path);
        }
        "worker-output-bound" => {}
        _ => unreachable!("selected worker case"),
    }
    let mut tool = json!({"name":"run","inputSchema":input});
    if matches!(
        case,
        "worker-preflight" | "worker-output" | "worker-output-bound"
    ) {
        tool["outputSchema"] = output;
    }
    tool
}
async fn action(
    client: &mut InvocationClient,
    signal: &Cancellation,
    case: &str,
    path: &Path,
) -> Result<Value, String> {
    let teardown = if retained(case) {
        Instant::now()
    } else {
        end()
    };
    let mut call =
        Box::pin(client.call_tool_cancellable("run", json!({}), end(), signal, teardown));
    if case == "worker-output-bound" {
        return encode(call.await);
    }
    tokio::select! {
        result = &mut call => return encode(result),
        started = wait_pid(path) => { started?; }
    }
    if case == "worker-drop" {
        drop(call);
        let cleanup = client
            .reap_schema_worker(Instant::now())
            .await
            .map_err(|e| format!("drop cleanup: {e:?}"))?;
        Ok(json!({"kind":"dropped","value":{"worker":cleanup}}))
    } else {
        signal.cancel();
        encode(call.await)
    }
}
pub async fn observe(case: &str, modern: bool) -> Result<Value, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/mcp-next-runtime/lifecycle/typed-fixtures");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let directory = tempfile::tempdir_in(&root).map_err(|e| e.to_string())?;
    let path = directory.path().join("pid");
    let mut script = vec![peer::Reply::result(
        "tools/list",
        peer::page("tools", json!([descriptor(case, &path)]), modern),
    )];
    if matches!(case, "worker-output" | "worker-output-bound") {
        let mut structured =
            json!({"proof":"peer-result","opaque":{"$serde_json::private::Number":"7"}});
        if case == "worker-output-bound" {
            structured["padding"] = json!("x".repeat(4096));
        }
        script.push(peer::Reply::result(
            "tools/call",
            peer::result(json!({"content":[],"structuredContent":structured}), modern),
        ));
    }
    let (executable, bound) = if case == "worker-output-bound" {
        (env!("CARGO_BIN_EXE_b10x-mcp-schema-worker"), 512)
    } else {
        (env!("CARGO_BIN_EXE_b10x-mcp-schema-fixture"), 65536)
    };
    let worker =
        SchemaWorker::new(executable.into(), bound).map_err(|e| format!("worker: {e:?}"))?;
    let signal = Cancellation::default();
    let (mut client, peer) = peer::start(modern, script, worker, &signal).await?;
    client
        .discover(Family::V2, &limits(), end())
        .await
        .map_err(|e| format!("discovery: {e:?}"))?;
    let outcome = action(&mut client, &signal, case, &path).await;
    let original = pid(&path);
    let mut reuse_refused = false;
    if retained(case) {
        let fresh = Cancellation::default();
        let retry = encode(
            client
                .call_tool_cancellable("run", json!({}), end(), &fresh, end())
                .await,
        )?;
        reuse_refused = retry["kind"] == "refused"
            && retry["value"]["refusal"]["reason"] == "schema_unavailable"
            && retry["value"]["worker"]["state"] == "retained"
            && original.is_some()
            && pid(&path) == original;
    }
    // Cleanup is selected by the test case, never by a possibly mutated report.
    let cleanup = client
        .reap_schema_worker(end())
        .await
        .map_err(|e| format!("reap: {e:?}"));
    let items = client.tools().count();
    drop(client);
    let (calls, closed) = peer.finish().await?;
    let cleanup = encode(cleanup?)?;
    let mut observed = summary(&outcome?, &calls, closed, items, modern);
    observed["worker_gone"] =
        json!(original.is_some_and(|id| !Path::new("/proc").join(id.to_string()).exists()));
    observed["reuse_refused"] = json!(reuse_refused);
    observed["explicit_reaped"] = json!(cleanup["state"] == "reaped");
    Ok(observed)
}
