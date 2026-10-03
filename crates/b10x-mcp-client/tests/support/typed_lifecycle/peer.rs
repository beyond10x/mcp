//! A script-owned HTTP peer with cancellation and actual socket-closure barriers.
use b10x_mcp_client::{
    schema_worker::SchemaWorker, strict_cancellation::Cancellation,
    strict_invocation::InvocationClient,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Notify, oneshot},
    time::{Duration, Instant, timeout},
};

pub struct Reply {
    pub method: &'static str,
    pub body: Value,
    pub stall: bool,
    pub cancel: bool,
}
impl Reply {
    pub fn result(method: &'static str, body: Value) -> Self {
        Self {
            method,
            body,
            stall: false,
            cancel: false,
        }
    }
    pub fn stall(method: &'static str, cancel: bool) -> Self {
        Self {
            method,
            body: Value::Null,
            stall: true,
            cancel,
        }
    }
}
#[derive(Clone)]
struct Capture {
    calls: Arc<Mutex<Vec<Value>>>,
    closed: Arc<AtomicUsize>,
    started: Arc<Notify>,
    signal: Cancellation,
}
pub struct Peer {
    capture: Capture,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<usize, String>>,
}
impl Peer {
    pub async fn started(&self) -> Result<(), String> {
        timeout(Duration::from_secs(3), self.capture.started.notified())
            .await
            .map_err(|_| "no stalled request".to_owned())
    }
    pub async fn finish(self) -> Result<(Vec<Value>, usize), String> {
        let _ = self.stop.send(());
        let unused = self.task.await.map_err(|e| e.to_string())??;
        if unused != 0 {
            return Err(format!("{unused} unused replies"));
        }
        let calls = self
            .capture
            .calls
            .lock()
            .map_err(|_| "capture lock")?
            .clone();
        Ok((calls, self.capture.closed.load(Ordering::SeqCst)))
    }
}
async fn read_request(stream: &mut TcpStream) -> Result<Value, String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= 65536 {
            return Err("request header bound".into());
        }
        head.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let head = std::str::from_utf8(&head).map_err(|e| e.to_string())?;
    let mut headers = serde_json::Map::new();
    for (key, value) in head.lines().filter_map(|l| l.split_once(':')) {
        if headers
            .insert(key.to_ascii_lowercase(), json!(value.trim()))
            .is_some()
        {
            return Err("duplicate header".into());
        }
    }
    let len: usize = headers["content-length"]
        .as_str()
        .ok_or("missing length")?
        .parse()
        .map_err(|_| "invalid length")?;
    if len > 65536 {
        return Err("request body bound".into());
    }
    let mut body = vec![0; len];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    let body: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    Ok(json!({"headers":headers,"body":body}))
}
async fn answer(
    mut stream: TcpStream,
    reply: Reply,
    capture: &Capture,
    request: &Value,
) -> Result<(), String> {
    if reply.stall {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n: waiting\n\n").await.map_err(|e|e.to_string())?;
        capture.started.notify_one();
        if reply.cancel {
            capture.signal.cancel();
        }
        match timeout(Duration::from_secs(3), stream.read_u8())
            .await
            .map_err(|_| "interrupted socket stayed open")?
        {
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                ) => {}
            _ => return Err("interrupted socket did not close".into()),
        }
        capture.closed.fetch_add(1, Ordering::SeqCst);
        return Ok(());
    }
    let notification = reply.method.starts_with("notifications/");
    let body = if notification {
        Vec::new()
    } else {
        serde_json::to_vec(&json!({"jsonrpc":"2.0","id":request["body"]["id"],"result":reply.body}))
            .map_err(|e| e.to_string())?
    };
    let status = if notification { 202 } else { 200 };
    stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.map_err(|e|e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.shutdown().await.map_err(|e| e.to_string())
}
async fn serve(
    listener: TcpListener,
    mut script: VecDeque<Reply>,
    capture: Capture,
    mut stop: oneshot::Receiver<()>,
) -> Result<usize, String> {
    loop {
        let (mut stream, _) = tokio::select! { _ = &mut stop => return Ok(script.len()), r = listener.accept() => r.map_err(|e|e.to_string())? };
        let request = timeout(Duration::from_secs(3), read_request(&mut stream))
            .await
            .map_err(|_| "request timeout")??;
        capture
            .calls
            .lock()
            .map_err(|_| "capture lock")?
            .push(request.clone());
        let reply = script.pop_front().ok_or("request after script exhausted")?;
        if request["body"]["method"] != reply.method {
            return Err(format!(
                "expected {}, got {}",
                reply.method, request["body"]["method"]
            ));
        }
        answer(stream, reply, &capture, &request).await?;
    }
}
pub fn result(mut value: Value, modern: bool) -> Value {
    if modern {
        value["resultType"] = json!("complete");
    }
    value
}
pub fn page(family: &str, rows: Value, modern: bool) -> Value {
    let mut value = result(json!({}), modern);
    value[family] = rows;
    if modern {
        value["ttlMs"] = json!(0);
        value["cacheScope"] = json!("private");
    }
    value
}
pub async fn start(
    modern: bool,
    replies: Vec<Reply>,
    worker: SchemaWorker,
    signal: &Cancellation,
) -> Result<(InvocationClient, Peer), String> {
    let revision = if modern { "2026-07-28" } else { "2025-11-25" };
    let caps = json!({"tools":{},"resources":{},"prompts":{}});
    let mut script = VecDeque::new();
    if modern {
        script.push_back(Reply::result("server/discover",json!({"resultType":"complete","supportedVersions":[revision],"capabilities":caps,"ttlMs":0,"cacheScope":"private"})));
    } else {
        script.push_back(Reply::result("initialize",json!({"protocolVersion":revision,"capabilities":caps,"serverInfo":{"name":"fixture","version":"1"}})));
        script.push_back(Reply::result("notifications/initialized", Value::Null));
    }
    script.extend(replies);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let endpoint = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let capture = Capture {
        calls: Arc::default(),
        closed: Arc::default(),
        started: Arc::default(),
        signal: signal.clone(),
    };
    let (stop, stop_rx) = oneshot::channel();
    let task = tokio::spawn(serve(listener, script, capture.clone(), stop_rx));
    let input = serde_json::from_value(json!({"revision":revision,"client_info":{"name":"fixture","version":"1"},"session_id_octets":64,"budget":{"request_octets":65536,"response_octets":65536,"sse_event_octets":65536,"connect_ms":1000,"provider_ms":3000,"remaining_execution_ms":5000}})).map_err(|e|e.to_string())?;
    let connection = b10x_mcp_client::strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        reqwest::Request::new(
            reqwest::Method::POST,
            endpoint.parse().map_err(|_| "endpoint")?,
        ),
        &input,
        Instant::now() + Duration::from_secs(3),
    )
    .await
    .map_err(|_| "setup refused")?;
    Ok((
        InvocationClient::new(connection, worker),
        Peer {
            capture,
            stop,
            task,
        },
    ))
}
