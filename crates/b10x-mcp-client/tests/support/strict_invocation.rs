//! Real loopback peer with an ordered script and captured HTTP requests.
use b10x_mcp_client::{schema_worker::SchemaWorker, strict_invocation::InvocationClient};
use b10x_mcp_types::http_exchange::{
    McpHttpDiscoveryCatalog as Catalog, McpHttpDiscoveryFamily as Family,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    time::{Duration, Instant},
};
pub fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
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
pub fn tool(input: Value) -> Value {
    let mut tool = json!({"name":"run"});
    tool["inputSchema"] = input;
    tool
}
pub struct Peer {
    captured: Arc<Mutex<Vec<Value>>>,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<usize, String>>,
}
impl Peer {
    pub async fn finish(self) -> Vec<Value> {
        let _ = self.stop.send(());
        assert_eq!(
            self.task.await.unwrap().unwrap(),
            0,
            "unused scripted replies"
        );
        self.captured.lock().unwrap().clone()
    }
}
// Keep the complete owned fixture lifecycle in one place.
#[allow(clippy::too_many_lines)]
pub async fn start(modern: bool, script: Vec<(&str, Value)>) -> (InvocationClient, Peer) {
    let revision = if modern { "2026-07-28" } else { "2025-11-25" };
    let caps = json!({"tools":{},"resources":{},"prompts":{}});
    let mut replies = VecDeque::new();
    if modern {
        replies.push_back(("server/discover".to_owned(),json!({"resultType":"complete","supportedVersions":[revision],"capabilities":caps,"ttlMs":0,"cacheScope":"private","_meta":{"io.modelcontextprotocol/serverInfo":{"name":"fixture","version":"1"}}})));
    } else {
        replies.push_back(("initialize".to_owned(),json!({"protocolVersion":revision,"capabilities":caps,"serverInfo":{"name":"fixture","version":"1"}})));
        replies.push_back(("notifications/initialized".to_owned(), Value::Null));
    }
    replies.extend(script.into_iter().map(|(m, r)| (m.to_owned(), r)));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let captured = Arc::new(Mutex::new(Vec::new()));
    let recording = captured.clone();
    let (stop, mut stop_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        loop {
            let accepted = tokio::select! { _ = &mut stop_rx => return Ok(replies.len()), r = listener.accept() => r };
            let (mut stream, _) = accepted.map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                if bytes.len() > 65536 {
                    return Err("fixture header bound".into());
                }
                bytes.push(stream.read_u8().await.map_err(|e| e.to_string())?);
            }
            let head = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            let mut headers = serde_json::Map::new();
            for (key, value) in head.lines().filter_map(|l| l.split_once(':')) {
                if headers
                    .insert(key.to_ascii_lowercase(), json!(value.trim()))
                    .is_some()
                {
                    return Err("duplicate header".into());
                }
            }
            let length: usize = headers["content-length"]
                .as_str()
                .ok_or("missing length")?
                .parse()
                .map_err(|_| "invalid length")?;
            if length > 65536 {
                return Err("fixture body bound".into());
            }
            let mut body = vec![0; length];
            stream
                .read_exact(&mut body)
                .await
                .map_err(|e| e.to_string())?;
            let request: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
            recording.lock().map_err(|_| "fixture lock")?.push(
                json!({"headers":headers,"body":request,"raw_body":String::from_utf8_lossy(&body)}),
            );
            let (method, reply) = replies
                .pop_front()
                .ok_or("unexpected request after script exhausted")?;
            if request["method"] != method {
                return Err(format!("expected {method}, observed {}", request["method"]));
            }
            if reply.get("fixture_delay").is_some() {
                tokio::select! { _ = &mut stop_rx => return Ok(replies.len()), () = tokio::time::sleep(Duration::from_secs(1)) => {} }
            }
            if reply.get("fixture_incomplete").is_some() {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 200\r\nConnection: close\r\n\r\n{\"jsonrpc\":").await.map_err(|e|e.to_string())?;
                stream.shutdown().await.map_err(|e| e.to_string())?;
                continue;
            }
            let (status, body) = if method == "notifications/initialized" {
                ("202 Accepted", Vec::new())
            } else if let Some(raw) = reply.get("fixture_literal").and_then(Value::as_str) {
                if request["id"] != 3 {
                    return Err("literal fixture expects call id3".into());
                }
                (
                    "200 OK",
                    raw.replacen("\"id\":1", "\"id\":3", 1).into_bytes(),
                )
            } else if reply.get("fixture_error").is_some() {
                (
                    "200 OK",
                    serde_json::to_vec(
                        &json!({"jsonrpc":"2.0","id":request["id"],"error":reply["fixture_error"]}),
                    )
                    .unwrap(),
                )
            } else {
                (
                    "200 OK",
                    serde_json::to_vec(&json!({"jsonrpc":"2.0","id":request["id"],"result":reply}))
                        .unwrap(),
                )
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream
                .write_all(head.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
            stream.write_all(&body).await.map_err(|e| e.to_string())?;
            stream.shutdown().await.map_err(|e| e.to_string())?;
        }
    });
    let input = serde_json::from_value(json!({"revision":revision,"client_info":{"name":"fixture","version":"1"},"session_id_octets":64,"budget":{"request_octets":65536,"response_octets":65536,"sse_event_octets":65536,"connect_ms":1000,"provider_ms":3000,"remaining_execution_ms":5000}})).unwrap();
    let connection = b10x_mcp_client::strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        reqwest::Request::new(reqwest::Method::POST, url.parse().unwrap()),
        &input,
        deadline(),
    )
    .await
    .unwrap();
    let worker =
        SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(), 65536).unwrap();
    (
        InvocationClient::new(connection, worker),
        Peer {
            captured,
            stop,
            task,
        },
    )
}
pub async fn discover(client: &mut InvocationClient, family: Family) -> Catalog {
    let limits =
        serde_json::from_value(json!({"max_pages":3,"max_items":16,"descriptor_octets":16384}))
            .unwrap();
    client
        .discover(family, &limits, deadline())
        .await
        .unwrap()
        .clone()
}
