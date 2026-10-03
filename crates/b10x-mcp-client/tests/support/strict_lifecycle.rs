//! Real HTTP peer: records control requests and observes client socket closure.
use b10x_mcp_client::strict_connection;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, oneshot};
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout};

#[derive(Clone)]
struct Peer {
    case: String,
    requests: Arc<Mutex<Vec<Value>>>,
    started: Arc<Notify>,
    stream_closed: Arc<Notify>,
    pool_closed: Arc<Notify>,
}
async fn read_request(stream: &mut TcpStream) -> Result<Option<Value>, String> {
    let mut head = Vec::new();
    loop {
        let mut byte = [0];
        if stream.read(&mut byte).await.map_err(|e| e.to_string())? == 0 {
            return if head.is_empty() {
                Ok(None)
            } else {
                Err("partial request".into())
            };
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() > 65536 {
            return Err("fixture header bound".into());
        }
    }
    let head = std::str::from_utf8(&head).map_err(|e| e.to_string())?;
    let mut lines = head.lines();
    let line = lines.next().ok_or("missing request line")?;
    let mut parts = line.split_whitespace();
    let method = parts.next().ok_or("missing method")?;
    if parts.next() != Some("/mcp") {
        return Err("wrong endpoint".into());
    }
    let mut headers = serde_json::Map::new();
    for (name, value) in lines.filter_map(|l| l.split_once(':')) {
        if headers
            .insert(name.to_ascii_lowercase(), json!(value.trim()))
            .is_some()
        {
            return Err("duplicate header".into());
        }
    }
    let length: usize = headers
        .get("content-length")
        .and_then(Value::as_str)
        .unwrap_or("0")
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
    let body: Value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).map_err(|e| e.to_string())?
    };
    Ok(Some(json!({"method":method,"headers":headers,"body":body})))
}
fn response(peer: &Peer, request: &Value) -> Result<(u16, Vec<u8>, String), String> {
    if request["method"] == "DELETE" {
        return Ok(match peer.case.as_str() {
            "lifecycle-shutdown-405" => (405, b"delete unavailable".to_vec(), String::new()),
            "lifecycle-shutdown-failure" => (503, b"failure evidence".to_vec(), String::new()),
            "lifecycle-shutdown-redirect" => (
                307,
                b"redirect refused".to_vec(),
                "Location: /redirect\r\n".into(),
            ),
            "lifecycle-shutdown-bound" => (200, vec![b'x'; 8192], String::new()),
            "lifecycle-shutdown-body-timeout" => (200, b"late".to_vec(), String::new()),
            _ => (204, Vec::new(), String::new()),
        });
    }
    let result = match request["body"]["method"].as_str().ok_or("no RPC method")? {
        "server/discover" => {
            json!({"resultType":"complete","supportedVersions":["2026-07-28"],"capabilities":{"tools":{}},"ttlMs":0,"cacheScope":"private"})
        }
        "initialize" => {
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"lifecycle-fixture","version":"1"}})
        }
        "notifications/initialized" => return Ok((202, Vec::new(), String::new())),
        "tools/call" => json!({"resultType":"complete","content":[]}),
        _ => return Err("unexpected RPC method".into()),
    };
    // A modern session header must never be adopted even when the peer sends it.
    let extra = if peer.case == "lifecycle-shutdown-sessionless" {
        ""
    } else {
        "Mcp-Session-Id: lifecycle-fixture-session\r\n"
    };
    Ok((
        200,
        serde_json::to_vec(&json!({"jsonrpc":"2.0","id":request["body"]["id"],"result":result}))
            .map_err(|e| e.to_string())?,
        extra.into(),
    ))
}
async fn connection(mut stream: TcpStream, peer: Peer) -> Result<(), String> {
    while let Some(request) = read_request(&mut stream).await? {
        peer.requests
            .lock()
            .map_err(|_| "fixture lock")?
            .push(request.clone());
        if peer.case == "lifecycle-shutdown-abandoned" && request["body"]["method"] == "tools/call"
        {
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n: ready\n\n").await.map_err(|e| e.to_string())?;
            stream.flush().await.map_err(|e| e.to_string())?;
            peer.started.notify_one();
            let mut byte = [0];
            let closed = timeout(Duration::from_secs(3), stream.read(&mut byte))
                .await
                .map_err(|_| "stream stayed open")?;
            match closed {
                Ok(0) => {}
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
                _ => return Err("stream did not close".into()),
            }
            peer.stream_closed.notify_one();
            return Ok(());
        }
        let (status, body, extra) = response(&peer, &request)?;
        let head = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n",
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        if peer.case == "lifecycle-shutdown-body-timeout" && request["method"] == "DELETE" {
            stream.flush().await.map_err(|e| e.to_string())?;
            let mut byte = [0];
            let closed = timeout(Duration::from_secs(3), stream.read(&mut byte))
                .await
                .map_err(|_| "control stayed open")?;
            if !matches!(closed, Ok(0)) {
                return Err("control did not close".into());
            }
            peer.pool_closed.notify_one();
            return Ok(());
        }
        stream.write_all(&body).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
    }
    peer.pool_closed.notify_one();
    Ok(())
}
async fn serve(
    listener: TcpListener,
    peer: Peer,
    mut stop: oneshot::Receiver<()>,
) -> Result<(), String> {
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            result = tasks.join_next(), if !tasks.is_empty() => {
                result.ok_or("missing fixture task")?.map_err(|e| e.to_string())??;
            }
            _ = &mut stop => break,
            incoming = listener.accept() => {
                let (stream, _) = incoming.map_err(|e| e.to_string())?;
                tasks.spawn(connection(stream, peer.clone()));
            }
        }
    }
    while let Some(result) = tasks.join_next().await {
        result.map_err(|e| e.to_string())??;
    }
    Ok(())
}

fn request_template(endpoint: &str) -> Result<reqwest::Request, String> {
    let mut request =
        reqwest::Request::new(reqwest::Method::POST, endpoint.parse().map_err(|_| "URL")?);
    request.headers_mut().insert(
        "authorization",
        "Bearer fixture-only-credential"
            .parse()
            .map_err(|_| "header")?,
    );
    Ok(request)
}

async fn abandon(
    client: &mut strict_connection::StrictConnection,
    peer: &Peer,
) -> Result<bool, String> {
    {
        let exchange = client.exchange(
            "tools/call",
            json!({"name":"fixture","arguments":{}}),
            Instant::now() + Duration::from_secs(5),
        );
        tokio::pin!(exchange);
        tokio::select! {
            result = &mut exchange => return Err(format!("exchange unexpectedly completed: {}", result.is_ok())),
            () = peer.started.notified() => {},
        }
    }
    timeout(Duration::from_secs(3), peer.stream_closed.notified())
        .await
        .map_err(|_| "abandoned stream not closed")?;
    Ok(client
        .exchange(
            "tools/call",
            json!({"name":"fixture","arguments":{}}),
            Instant::now() + Duration::from_secs(1),
        )
        .await
        .is_err())
}

pub async fn observe(case: &str, revision: &str) -> Result<Value, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let endpoint = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let peer = Peer {
        case: case.into(),
        requests: Arc::default(),
        started: Arc::default(),
        stream_closed: Arc::default(),
        pool_closed: Arc::default(),
    };
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(serve(listener, peer.clone(), stopped));
    let request = request_template(&endpoint)?;
    let input = serde_json::from_value(json!({"revision":revision,"client_info":{"name":"fixture","version":"1"},"session_id_octets":256,"budget":{"request_octets":4096,"response_octets":4096,"sse_event_octets":4096,"remaining_execution_ms":5000,"provider_ms":5000,"connect_ms":1000}})).map_err(|e| e.to_string())?;
    let reusable = request.try_clone().ok_or("fixture template clone")?;
    let input_before = serde_json::to_value(&input).map_err(|e| e.to_string())?;
    let mut client = strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        request,
        &input,
        Instant::now() + Duration::from_secs(5),
    )
    .await
    .map_err(|_| "setup failed")?;
    let abandoned = case == "lifecycle-shutdown-abandoned";
    let reuse_refused = if abandoned {
        abandon(&mut client, &peer).await?
    } else {
        false
    };
    if case == "lifecycle-shutdown-completed" {
        client
            .exchange(
                "tools/call",
                json!({"name":"fixture","arguments":{}}),
                Instant::now() + Duration::from_secs(1),
            )
            .await
            .map_err(|_| "call failed")?;
    }
    let deadline = match case {
        "lifecycle-shutdown-deadline" => Instant::now(),
        "lifecycle-shutdown-body-timeout" => Instant::now() + Duration::from_millis(100),
        _ => Instant::now() + Duration::from_secs(2),
    };
    let observed = if case == "lifecycle-shutdown-invocation" {
        let worker = b10x_mcp_client::schema_worker::SchemaWorker::new(
            std::path::PathBuf::from(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker")),
            4096,
        )
        .map_err(|_| "fixture worker")?;
        b10x_mcp_client::strict_invocation::InvocationClient::new(client, worker)
            .shutdown(deadline)
            .await
    } else {
        client.shutdown(deadline).await
    }
    .map_err(|_| "shutdown failed")?;
    if !(abandoned && revision == "2026-07-28") {
        timeout(Duration::from_secs(3), peer.pool_closed.notified())
            .await
            .map_err(|_| "owned pool not closed")?;
    }
    if case == "lifecycle-shutdown-reconnect" {
        let client = strict_connection::connect(
            reqwest::Client::builder().no_proxy(),
            reusable,
            &input,
            Instant::now() + Duration::from_secs(2),
        )
        .await
        .map_err(|_| "reconnect failed")?;
        client
            .shutdown(Instant::now() + Duration::from_secs(2))
            .await
            .map_err(|_| "second shutdown failed")?;
        timeout(Duration::from_secs(3), peer.pool_closed.notified())
            .await
            .map_err(|_| "second pool not closed")?;
    }
    if serde_json::to_value(&input).map_err(|e| e.to_string())? != input_before {
        return Err("caller configuration mutated".into());
    }
    stop.send(()).map_err(|()| "fixture exited")?;
    timeout(Duration::from_secs(3), task)
        .await
        .map_err(|_| "fixture join timeout")?
        .map_err(|e| e.to_string())??;
    let requests = peer.requests.lock().map_err(|_| "fixture lock")?.clone();
    Ok(
        json!({"shutdown":observed,"requests":requests,"abandoned_stream_closed":abandoned,"reuse_refused":reuse_refused,"pool_closed":true}),
    )
}

pub async fn observations(case: &str, revision: &str) -> Result<Value, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let actual = observe(case, revision).await?;
    let shutdown = &actual["shutdown"];
    let deletion = &shutdown["deletion"];
    let requests = actual["requests"].as_array().ok_or("fixture requests")?;
    let controls: Vec<_> = requests
        .iter()
        .filter(|r| r["method"] == "DELETE")
        .collect();
    let bytes = deletion["exchange"]["response"]["bytes"]
        .as_str()
        .unwrap_or("");
    let bytes = STANDARD.decode(bytes).map_err(|e| e.to_string())?;
    let agreement = controls.iter().all(|r| {
        r["headers"]["mcp-session-id"] == "lifecycle-fixture-session"
            && r["headers"]["mcp-protocol-version"] == "2025-11-25"
            && r["headers"]["authorization"] == "Bearer fixture-only-credential"
            && r["headers"].get("mcp-method").is_none()
            && r["body"].is_null()
    });
    Ok(json!({
        "requests":requests.len(), "deletes":controls.len(),
        "disposition":deletion["disposition"].as_str().unwrap_or("absent"),
        "refusal":deletion["refusal"].as_str().unwrap_or("absent"),
        "retained_octets":bytes.len(),
        "http_status":deletion["exchange"]["http_status"].as_u64().unwrap_or(0),
        "send":deletion["exchange"]["send"].as_str().unwrap_or("absent"),
        "interrupted":shutdown.get("interrupted").is_some(),
        "interrupted_id":shutdown["interrupted"]["request_id"]["value"].as_u64().unwrap_or(0),
        "pool_closed":actual["pool_closed"],
        "abandoned_stream_closed":actual["abandoned_stream_closed"],
        "reuse_refused":actual["reuse_refused"],
        "header_agreement":agreement,
    }))
}
