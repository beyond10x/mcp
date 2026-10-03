//! Real HTTP peer: records control requests and observes client socket closure.
#[path = "strict_cancellation.rs"]
mod cancellation;
#[path = "strict_progress.rs"]
mod progress;
use b10x_mcp_client::strict_connection;
pub use cancellation::observe as observe_cancel;
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
    cancellation: b10x_mcp_client::strict_cancellation::Cancellation,
    case: String,
    requests: Arc<Mutex<Vec<Value>>>,
    started: Arc<Notify>,
    stream_closed: Arc<Notify>,
    pool_closed: Arc<Notify>,
    answered: Arc<Notify>,
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
    if request["body"].get("method").is_none() && request["method"] == "POST" {
        return Ok(match peer.case.as_str() {
            "stream-control-failure" => (503, b"side failure".to_vec(), String::new()),
            "stream-control-body" => (202, b"x".to_vec(), String::new()),
            "stream-control-bound" => (202, vec![b'x'; 8192], String::new()),
            _ => (202, Vec::new(), String::new()),
        });
    }
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
    let extra = if peer.case.ends_with("sessionless") {
        ""
    } else if peer.case == "stream-setup-session-invalid" {
        "Mcp-Session-Id: invalid session\r\n"
    } else if peer.case == "stream-setup-session-duplicate" {
        "Mcp-Session-Id: one\r\nMcp-Session-Id: two\r\n"
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
        if peer.case.starts_with("cancel-") {
            if request["body"]["method"] == "tools/call" {
                return cancellation::business(stream, &peer, &request).await;
            }
            if request["body"]["method"] == "notifications/cancelled"
                || request["body"].get("method").is_none()
            {
                return cancellation::control(stream, &peer, &request).await;
            }
        }
        if matches!(
            peer.case.as_str(),
            "stream-control-delayed" | "stream-control-abandoned"
        ) && request["body"].get("method").is_none()
        {
            peer.answered.notify_one();
            peer.started.notify_one();
            let mut byte = [0];
            let closed = timeout(Duration::from_secs(3), stream.read(&mut byte))
                .await
                .map_err(|_| "delayed control stayed open")?;
            if !matches!(closed, Ok(0)) {
                return Err("delayed control did not close".into());
            }
            peer.pool_closed.notify_one();
            return Ok(());
        }
        if peer.case.starts_with("stream-")
            && (request["body"]["method"] == "tools/call"
                || (peer.case.starts_with("stream-setup-")
                    && request["body"]["method"] == "initialize"))
        {
            return stream_reply(stream, &peer, &request).await;
        }
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
        if request["method"] == "POST" && request["body"].get("method").is_none() {
            peer.answered.notify_one();
        }
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
        cancellation: b10x_mcp_client::strict_cancellation::Cancellation::new(),
        case: case.into(),
        requests: Arc::default(),
        started: Arc::default(),
        stream_closed: Arc::default(),
        pool_closed: Arc::default(),
        answered: Arc::default(),
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
    if case.starts_with("cancel-") {
        return cancellation::observations(&observe_cancel(case, revision).await?);
    }
    if case.starts_with("stream-progress-") {
        return progress::observations(&observe_stream(case, revision).await?);
    }
    if case.starts_with("stream-") {
        return stream_observations(case, revision).await;
    }
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

async fn stream_observations(case: &str, revision: &str) -> Result<Value, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let actual = observe_stream(case, revision).await?;
    let exchange = &actual["exchange"];
    let observation = &exchange["value"]["observation"];
    let messages = observation["stream"]["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let requests = actual["requests"].as_array().ok_or("recorded requests")?;
    let replies: Vec<_> = requests
        .iter()
        .filter(|r| r["method"] == "POST" && r["body"].get("method").is_none())
        .collect();
    let mut retained = observation["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap_or(0);
    let mut id_agreement = true;
    let mut disposition = "absent";
    let mut side_refusal = "absent";
    let mut notifications = 0;
    for message in &messages {
        let wire = if message["kind"] == "server_request" {
            let reply = &message["value"]["reply"];
            disposition = reply["disposition"].as_str().ok_or("control disposition")?;
            side_refusal = reply["refusal"].as_str().unwrap_or("absent");
            retained += reply["exchange"]["response"]["counts"]["retained_octets"]
                .as_u64()
                .ok_or("control retained")?;
            &message["value"]["request"]
        } else {
            notifications += 1;
            &message["value"]
        };
        retained += wire["response"]["counts"]["retained_octets"]
            .as_u64()
            .ok_or("message retained")?;
        if message["kind"] == "server_request" && !replies.is_empty() {
            let bytes = STANDARD
                .decode(wire["response"]["bytes"].as_str().ok_or("message bytes")?)
                .map_err(|e| e.to_string())?;
            let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            id_agreement &= replies.iter().any(|r| r["body"]["id"] == raw["id"]);
        }
    }
    let header_agreement = replies.iter().all(|r| {
        r["headers"]["authorization"] == "Bearer fixture-only-credential"
            && r["headers"]["mcp-protocol-version"] == "2025-11-25"
            && r["headers"].get("mcp-method").is_none()
            && r["headers"].get("last-event-id").is_none()
            && if case.ends_with("sessionless") {
                r["headers"].get("mcp-session-id").is_none()
            } else {
                r["headers"]["mcp-session-id"] == "lifecycle-fixture-session"
            }
    });
    let payload = &exchange["value"]["result"];
    Ok(
        json!({"kind":exchange["kind"].as_str().unwrap_or("absent"),"reason":exchange["value"]["reason"].as_str().unwrap_or("absent"),
        "setup_reason":actual["setup_reason"],"requests":requests.len(),"replies":replies.len(),"messages":messages.len(),
        "notifications":notifications,"disposition":disposition,"side_refusal":side_refusal,
        "id_agreement":id_agreement,"header_agreement":header_agreement,"retained_bound":retained<=4096,
        "own_payload":payload.get("content").is_some() || payload.get("capabilities").is_some()}),
    )
}

fn stream_event(peer: &Peer, request: &Value) -> Value {
    let id = match peer.case.as_str() {
        "stream-ping-large-id" => {
            serde_json::from_str("123456789012345678901234567890").expect("fixture integer")
        }
        "stream-ping-collision" => request["body"]["id"].clone(),
        _ => json!("peer-ping"),
    };
    if matches!(
        peer.case.as_str(),
        "stream-notifications" | "stream-total-bound"
    ) {
        let message = if peer.case == "stream-total-bound" {
            "x".repeat(1500)
        } else {
            "server text".into()
        };
        json!({"jsonrpc":"2.0","method":"notifications/message","params":{"message":message,"opaque":{"$serde_json::private::Number":"7"}}})
    } else {
        json!({"jsonrpc":"2.0","id":id,"method":if peer.case == "stream-unsupported" {"sampling/createMessage"} else {"ping"}})
    }
}
async fn stream_reply(mut stream: TcpStream, peer: &Peer, request: &Value) -> Result<(), String> {
    let (_, body, extra) = response(peer, request)?;
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n{extra}\r\n"
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    if peer.case.starts_with("stream-progress-") {
        return progress::reply(stream, &peer.case, request, &body).await;
    }
    if peer.case == "stream-empty-prime" {
        stream
            .write_all(b"id: ignored-cursor\r\nretry: 0\r\ndata:\r\n\r\n")
            .await
            .map_err(|e| e.to_string())?;
    }
    let event = stream_event(peer, request);
    let ending = if peer.case == "stream-ping-cr" {
        "\r\r"
    } else {
        "\r\n\r\n"
    };
    let frame = format!("data: {event}{ending}");
    let count = if peer.case == "stream-total-bound" {
        3
    } else {
        1
    };
    for _ in 0..count {
        stream
            .write_all(frame.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    stream.flush().await.map_err(|e| e.to_string())?;
    let no_reply = matches!(
        peer.case.as_str(),
        "stream-modern-server-request"
            | "stream-event-bound"
            | "stream-total-bound"
            | "stream-control-abandoned"
    );
    if no_reply {
        let mut byte = [0];
        match timeout(Duration::from_secs(3), stream.read(&mut byte))
            .await
            .map_err(|_| "stream not closed")?
        {
            Ok(0) => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
            _ => return Err("stream did not close".into()),
        }
        peer.stream_closed.notify_one();
        return Ok(());
    }
    if event.get("id").is_some() && !peer.case.starts_with("stream-setup-session-") {
        timeout(Duration::from_secs(3), peer.answered.notified())
            .await
            .map_err(|_| "control reply not received")?;
    }
    let terminal = format!(
        "data: {}\n\n",
        std::str::from_utf8(&body).map_err(|e| e.to_string())?
    );
    // An over-bound control response may have already closed the original stream.
    let sent = stream.write_all(terminal.as_bytes()).await;
    if peer.case != "stream-control-bound" {
        sent.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn observe_stream(case: &str, revision: &str) -> Result<Value, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let endpoint = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let peer = Peer {
        cancellation: b10x_mcp_client::strict_cancellation::Cancellation::new(),
        case: case.into(),
        requests: Arc::default(),
        started: Arc::default(),
        stream_closed: Arc::default(),
        pool_closed: Arc::default(),
        answered: Arc::default(),
    };
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(serve(listener, peer.clone(), stopped));
    let input = serde_json::from_value(json!({"revision":revision,"client_info":{"name":"fixture","version":"1"},
        "session_id_octets":if case == "stream-setup-session-bound" {4} else {256},
        "budget":{"request_octets":4096,"response_octets":4096,"sse_event_octets":if case == "stream-event-bound" {40} else {4096},
        "remaining_execution_ms":5000,"provider_ms":5000,"connect_ms":1000}})).map_err(|e| e.to_string())?;
    let setup = strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        request_template(&endpoint)?,
        &input,
        Instant::now() + Duration::from_secs(1),
    )
    .await;
    let (exchange, setup_reason) = match setup {
        Ok(mut client) => {
            let value = if case == "stream-control-abandoned" {
                let reuse_refused = abandon(&mut client, &peer).await?;
                // The five-second exchange deadline has not elapsed and the
                // client still exists: dropping the exchange must close its reply.
                timeout(Duration::from_millis(250), peer.pool_closed.notified())
                    .await
                    .map_err(|_| "abandoned control not closed before client drop")?;
                json!({"abandoned_stream_closed":true,"abandoned_control_closed":true,"reuse_refused":reuse_refused})
            } else if case.starts_with("stream-setup-") {
                json!({"kind":"result","value":{"result":client.description().raw_result,"observation":client.description().exchange_observation}})
            } else {
                serde_json::to_value(
                    client
                        .exchange(
                            "tools/call",
                            progress::params(case),
                            Instant::now()
                                + if case == "stream-progress-deadline" {
                                    Duration::from_millis(200)
                                } else {
                                    Duration::from_secs(1)
                                },
                        )
                        .await
                        .map_err(|_| "exchange failed")?,
                )
                .map_err(|e| e.to_string())?
            };
            drop(client);
            (value, "absent".to_owned())
        }
        Err(refusal) => {
            let refusal = serde_json::to_value(refusal).map_err(|e| e.to_string())?;
            (
                refusal["exchange"].clone(),
                refusal["reason"].as_str().unwrap_or("absent").to_owned(),
            )
        }
    };
    stop.send(()).map_err(|()| "fixture exited")?;
    timeout(Duration::from_secs(4), task)
        .await
        .map_err(|_| "fixture join timeout")?
        .map_err(|e| e.to_string())??;
    let requests = peer.requests.lock().map_err(|_| "fixture lock")?.clone();
    Ok(json!({"exchange":exchange,"setup_reason":setup_reason,"requests":requests}))
}
