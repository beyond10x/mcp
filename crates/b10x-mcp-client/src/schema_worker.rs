//! A caller-admitted one-shot executable isolates synchronous schema validation.
mod controlled;
use b10x_mcp_types::http_exchange::{
    McpHttpInvocationRefusalReason as Reason, McpHttpInvocationSchemaReply as Reply,
    McpHttpInvocationSchemaRequest as Request,
};
use std::{path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::{Instant, timeout_at},
};
/// Decode the closed worker request while preserving opaque schema and instance
/// objects, including keys used privately by `serde_json`'s number representation.
pub fn decode_request(bytes: &[u8]) -> Option<Request> {
    use b10x_mcp_types::http_exchange::EssPresence;
    let serde_json::Value::Object(mut fields) = crate::strict_http::parse(bytes)? else {
        return None;
    };
    let action = serde_json::from_value(fields.remove("action")?).ok()?;
    let schema = fields.remove("schema")?;
    let instance = fields
        .remove("instance")
        .map_or(EssPresence::Absent, EssPresence::Present);
    fields.is_empty().then_some(Request {
        action,
        schema,
        instance,
    })
}
/// Trusted worker executable and an explicit maximum IPC request size.
///
/// The caller admits the executable and its containing directory. This component
/// neither searches PATH nor turns a remote schema into executable configuration.
pub struct SchemaWorker {
    path: PathBuf,
    max_input_bytes: usize,
    pending: Option<controlled::OwnedChild>,
}
impl SchemaWorker {
    /// Admit an absolute executable path and explicit byte ceiling; no path search.
    pub fn new(path: PathBuf, max_input_bytes: usize) -> Result<Self, Reason> {
        if !path.is_absolute() {
            return Err(Reason::V2);
        }
        Ok(Self {
            path,
            max_input_bytes,
            pending: None,
        })
    }
    /// Run with explicit cancellation and an independent absolute reap deadline.
    ///
    /// Awaited interruption reports actual kill/reap observations. If reaping
    /// cannot finish by the teardown deadline, this handle retains the child
    /// and refuses reuse until `reap_pending` succeeds. Dropping this future
    /// requests termination and retains ownership; it never claims a reap.
    pub async fn run_cancellable(
        &mut self,
        request: &Request,
        deadline: Instant,
        cancellation: &crate::strict_cancellation::Cancellation,
        teardown_deadline: Instant,
    ) -> Result<b10x_mcp_types::http_exchange::McpHttpLifecycleControlledSchemaResult, Reason> {
        controlled::run(self, request, deadline, cancellation, teardown_deadline).await
    }
    /// Retry bounded cleanup of a retained child; no schema or peer request runs.
    /// A successful reap permits reuse. Dropping the worker itself offers only
    /// kill-on-drop, without an observed process-exit barrier.
    pub async fn reap_pending(
        &mut self,
        deadline: Instant,
    ) -> Result<b10x_mcp_types::http_exchange::McpHttpLifecycleWorkerCleanup, Reason> {
        controlled::cleanup(&mut self.pending, deadline).await
    }
    fn spawn(&self) -> Result<tokio::process::Child, Reason> {
        Command::new(&self.path)
            .arg("--max-input-bytes")
            .arg(self.max_input_bytes.to_string())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Reason::V5)
    }
    /// Execute one schema operation with bounded IPC and an absolute deadline.
    ///
    /// An awaited timeout/failure kills and reaps the owned child. Dropping this
    /// future initiates kill-on-drop; it does not establish an observed reap barrier.
    /// Peer schemas, instances and validator diagnostics never enter local errors.
    pub async fn run(&mut self, request: &Request, deadline: Instant) -> Result<Reply, Reason> {
        if self.pending.is_some() {
            return Err(Reason::V5);
        }
        if Instant::now() >= deadline {
            return Err(Reason::V0);
        }
        let bytes = serde_json::to_vec(request).map_err(|_| Reason::V2)?;
        if bytes.len() > self.max_input_bytes {
            return Err(Reason::V2);
        }
        if Instant::now() >= deadline {
            return Err(Reason::V0);
        }
        let mut child = self.spawn()?;
        let transaction = async {
            let mut stdin = child.stdin.take().ok_or(Reason::V5)?;
            let mut stdout = child.stdout.take().ok_or(Reason::V5)?;
            stdin.write_all(&bytes).await.map_err(|_| Reason::V5)?;
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
            let status = child.wait().await.map_err(|_| Reason::V5)?;
            if !status.success() {
                return Err(Reason::V5);
            }
            serde_json::from_slice(&answer).map_err(|_| Reason::V5)
        };
        let result = match timeout_at(deadline, transaction).await {
            Ok(result) => result,
            Err(_) => Err(Reason::V0),
        };
        if result.is_err() {
            let _ = child.start_kill();
            child.wait().await.map_err(|_| Reason::V5)?;
        }
        if result.is_ok() && Instant::now() >= deadline {
            return Err(Reason::V0);
        }
        result
    }
}
