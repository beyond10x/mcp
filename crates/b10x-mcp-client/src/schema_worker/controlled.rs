//! Cancellation retains process ownership until an actual bounded wait succeeds.
use super::{Reason, Reply, Request, SchemaWorker};
use crate::strict_cancellation::{self, Cancellation};
use b10x_mcp_types::http_exchange::{
    McpHttpLifecycleControlledSchemaResult as Outcome, McpHttpLifecycleWorkerCleanup as Cleanup,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Child,
    time::{Instant, timeout_at},
};

pub(super) struct OwnedChild {
    child: Child,
    kill: &'static str,
    failure: Option<&'static str>,
    reaped: bool,
}
impl OwnedChild {
    fn request_kill(&mut self) {
        if !self.reaped && self.kill != "requested" {
            self.kill = if self.child.start_kill().is_ok() {
                "requested"
            } else {
                "failed"
            };
        }
    }
    fn observation(&self) -> Result<Cleanup, Reason> {
        let mut value =
            json!({"state":if self.reaped {"reaped"} else {"retained"},"kill":self.kill});
        if let Some(failure) = self.failure {
            value["failure"] = json!(failure);
        }
        model(value)
    }
}
struct Guard<'a>(&'a mut Option<OwnedChild>);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(owned) = self.0.as_mut() {
            owned.request_kill();
        }
    }
}
fn model<T: DeserializeOwned>(value: Value) -> Result<T, Reason> {
    serde_json::from_value(value).map_err(|_| Reason::V5)
}
fn no_child() -> Result<Cleanup, Reason> {
    model(json!({"state":"no_child","kill":"not_attempted"}))
}
fn refused(reason: &Reason, cleanup: &Cleanup) -> Result<Outcome, Reason> {
    model(json!({"kind":"refused","value":{"reason":reason,"cleanup":cleanup}}))
}
fn interrupted(cause: &str, cleanup: &Cleanup) -> Result<Outcome, Reason> {
    model(json!({"kind":"interrupted","value":{"cause":cause,"cleanup":cleanup}}))
}
pub(super) async fn cleanup(
    slot: &mut Option<OwnedChild>,
    deadline: Instant,
) -> Result<Cleanup, Reason> {
    let Some(owned) = slot.as_mut() else {
        return no_child();
    };
    if !owned.reaped {
        owned.request_kill();
        owned.failure = if Instant::now() >= deadline {
            Some("deadline_exhausted")
        } else {
            match timeout_at(deadline, owned.child.wait()).await {
                Ok(Ok(_)) => {
                    owned.reaped = true;
                    None
                }
                Ok(Err(_)) => Some("wait_failed"),
                Err(_) => Some("deadline_exhausted"),
            }
        };
    }
    let report = owned.observation()?;
    if owned.reaped {
        slot.take();
    }
    Ok(report)
}

pub(super) async fn run(
    worker: &mut SchemaWorker,
    request: &Request,
    deadline: Instant,
    cancellation: &Cancellation,
    teardown: Instant,
) -> Result<Outcome, Reason> {
    if let Some(owned) = &worker.pending {
        return refused(&Reason::V5, &owned.observation()?);
    }
    if cancellation.requested() {
        return interrupted("caller_cancelled", &no_child()?);
    }
    if Instant::now() >= deadline {
        return interrupted("deadline_exhausted", &no_child()?);
    }
    let bytes = match serde_json::to_vec(request) {
        Ok(bytes) if bytes.len() <= worker.max_input_bytes => bytes,
        _ => return refused(&Reason::V2, &no_child()?),
    };
    if cancellation.requested() {
        return interrupted("caller_cancelled", &no_child()?);
    }
    if Instant::now() >= deadline {
        return interrupted("deadline_exhausted", &no_child()?);
    }
    let child = match worker.spawn() {
        Ok(child) => child,
        Err(reason) => return refused(&reason, &no_child()?),
    };
    worker.pending = Some(OwnedChild {
        child,
        kill: "not_attempted",
        failure: None,
        reaped: false,
    });
    let guard = Guard(&mut worker.pending);
    let owned = guard.0.as_mut().ok_or(Reason::V5)?;
    let result = strict_cancellation::until(
        Some(cancellation),
        timeout_at(deadline, transact(owned, &bytes)),
    )
    .await;
    match result {
        Ok(Ok(Ok(reply))) if Instant::now() < deadline => {
            guard.0.take();
            model(json!({"kind":"completed","value":reply}))
        }
        result => {
            let report = cleanup(guard.0, teardown).await?;
            match result {
                Err(_) => interrupted("caller_cancelled", &report),
                Ok(Err(_) | Ok(Ok(_))) => interrupted("deadline_exhausted", &report),
                Ok(Ok(Err(reason))) => refused(&reason, &report),
            }
        }
    }
}

async fn transact(owned: &mut OwnedChild, bytes: &[u8]) -> Result<Reply, Reason> {
    let mut stdin = owned.child.stdin.take().ok_or(Reason::V5)?;
    let mut stdout = owned.child.stdout.take().ok_or(Reason::V5)?;
    stdin.write_all(bytes).await.map_err(|_| Reason::V5)?;
    drop(stdin);
    let mut answer = Vec::new();
    loop {
        let byte = match stdout.read_u8().await {
            Ok(byte) => byte,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return Err(Reason::V5),
        };
        if answer.len() >= 256 {
            return Err(Reason::V5);
        }
        answer.push(byte);
    }
    let status = owned.child.wait().await.map_err(|_| Reason::V5)?;
    owned.reaped = true;
    if !status.success() {
        return Err(Reason::V5);
    }
    serde_json::from_slice(&answer).map_err(|_| Reason::V5)
}
