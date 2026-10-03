//! Real child processes, PID barriers and explicit retained-handle cleanup.
use b10x_mcp_client::{schema_worker::SchemaWorker, strict_cancellation::Cancellation};
use b10x_mcp_types::http_exchange::McpHttpInvocationSchemaRequest as Request;
use serde_json::{Value, json};
use std::path::Path;
use tokio::time::{Duration, Instant};

fn pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.parse().ok()
}
fn gone(pid: u32) -> bool {
    !Path::new("/proc").join(pid.to_string()).exists()
}
async fn wait_pid(path: &Path) -> Result<u32, String> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(pid) = pid(path) {
            return Ok(pid);
        }
        if Instant::now() >= deadline {
            return Err("worker did not record a PID before the startup deadline".into());
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
fn encode<T: serde::Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}
fn request(mode: &str, path: &Path) -> Request {
    serde_json::from_value(json!({"action":"validate","schema":{"mode":mode},"instance":path}))
        .expect("fixture request")
}
fn retained(case: &str) -> bool {
    matches!(case, "expired-teardown" | "drop" | "overflow-retained")
}
fn no_process(case: &str) -> bool {
    matches!(
        case,
        "before-start" | "expired-operation" | "input-bound" | "spawn-failure"
    )
}
async fn execute(
    worker: &mut SchemaWorker,
    request: &Request,
    signal: &Cancellation,
    case: &str,
    path: &Path,
) -> Result<Value, String> {
    let deadline = match case {
        "expired-operation" => Instant::now(),
        "timeout" => Instant::now() + Duration::from_millis(500),
        _ => Instant::now() + Duration::from_secs(3),
    };
    let teardown = if retained(case) {
        Instant::now()
    } else {
        Instant::now() + Duration::from_secs(4)
    };
    let mut run = Box::pin(worker.run_cancellable(request, deadline, signal, teardown));
    if matches!(case, "caller" | "expired-teardown" | "drop") {
        tokio::select! {
            result = &mut run => return encode(result.map_err(|e| format!("early run: {e:?}"))?),
            observed = wait_pid(path) => { observed?; }
        }
        if case == "drop" {
            drop(run);
            let cleanup = worker
                .reap_pending(Instant::now())
                .await
                .map_err(|e| format!("drop cleanup: {e:?}"))?;
            return Ok(json!({"kind":"dropped","value":{"cleanup":cleanup}}));
        }
        signal.cancel();
        signal.cancel();
    }
    encode(run.await.map_err(|e| format!("run: {e:?}"))?)
}

async fn retained_reuse(worker: &mut SchemaWorker, path: &Path) -> Result<(bool, bool), String> {
    let original_pid = pid(path);
    let request = request("complete", path);
    let signal = Cancellation::default();
    let end = Instant::now() + Duration::from_secs(2);
    let controlled = encode(
        worker
            .run_cancellable(&request, end, &signal, end)
            .await
            .map_err(|e| format!("reuse: {e:?}"))?,
    )?;
    let refused_without_spawn = controlled["kind"] == "refused"
        && controlled["value"]["reason"] == "schema_unavailable"
        && original_pid.is_some()
        && pid(path) == original_pid;
    let raw = worker.run(&request, end).await;
    Ok((
        refused_without_spawn,
        raw.is_err_and(|reason| encode(reason).is_ok_and(|reason| reason == "schema_unavailable"))
            && original_pid.is_some()
            && pid(path) == original_pid,
    ))
}

pub async fn observations(case: &str) -> Result<Value, String> {
    if !matches!(
        case,
        "complete"
            | "real"
            | "caller"
            | "timeout"
            | "expired-teardown"
            | "drop"
            | "overflow"
            | "overflow-retained"
            | "invalid-reply"
            | "exit-failure"
            | "before-start"
            | "expired-operation"
            | "input-bound"
            | "spawn-failure"
    ) {
        return Err(format!("unknown worker fixture {case}"));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/mcp-next-runtime/lifecycle/worker-fixtures");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let dir = tempfile::tempdir_in(&root).map_err(|e| e.to_string())?;
    let path = dir.path().join("pid");
    let executable = match case {
        "real" => env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(),
        "spawn-failure" => dir.path().join("missing-worker"),
        _ => env!("CARGO_BIN_EXE_b10x-mcp-schema-fixture").into(),
    };
    let mut worker = SchemaWorker::new(executable, if case == "input-bound" { 0 } else { 65536 })
        .map_err(|e| format!("worker: {e:?}"))?;
    let mode = match case {
        "overflow-retained" => "overflow",
        other => other,
    };
    let input = if case == "real" {
        serde_json::from_value(
            json!({"action":"validate","schema":{"type":"integer","minimum":2},"instance":2}),
        )
        .unwrap()
    } else {
        request(mode, &path)
    };
    let signal = Cancellation::default();
    if case == "before-start" {
        signal.cancel();
    }
    let start = Instant::now();
    let outcome = execute(&mut worker, &input, &signal, case, &path).await;
    let elapsed = start.elapsed();
    let child = pid(&path);
    // Exercise reuse based on the scenario, never on a possibly false cleanup report.
    let attempts = if retained(case) {
        retained_reuse(&mut worker, &path).await
    } else {
        Ok((false, false))
    };
    // Always reap before inspecting assertions/errors, including under report mutations.
    let cleanup = worker
        .reap_pending(Instant::now() + Duration::from_secs(3))
        .await
        .map_err(|e| format!("final cleanup: {e:?}"))?;
    let cleanup = encode(cleanup)?;
    let outcome = outcome?;
    let (reuse_refused, raw_reuse_refused) = attempts?;
    let process_gone = child.is_some_and(gone);
    let mut reused = false;
    if retained(case) {
        let signal = Cancellation::default();
        let end = Instant::now() + Duration::from_secs(3);
        let reply = encode(
            worker
                .run_cancellable(&request("complete", &path), end, &signal, end)
                .await
                .map_err(|e| format!("after reap: {e:?}"))?,
        )?;
        reused = reply["kind"] == "completed"
            && reply["value"]["status"] == "valid"
            && pid(&path).is_some_and(gone);
    }
    let body = &outcome["value"];
    Ok(json!({
        "kind":outcome["kind"], "status":body["status"].as_str().unwrap_or("absent"),
        "cause":body["cause"].as_str().unwrap_or("absent"),
        "reason":body["reason"].as_str().unwrap_or("absent"),
        "state":body["cleanup"]["state"].as_str().unwrap_or("absent"),
        "kill":body["cleanup"]["kill"].as_str().unwrap_or("absent"),
        "failure":body["cleanup"]["failure"].as_str().unwrap_or("absent"),
        "pid_recorded":child.is_some(), "process_gone":process_gone,
        "within_budget":elapsed < if retained(case) || no_process(case) { Duration::from_secs(1) } else { Duration::from_secs(4) },
        "reuse_refused":reuse_refused, "raw_reuse_refused":raw_reuse_refused,
        "explicit_reaped":cleanup["state"] == "reaped", "reused":reused
    }))
}
