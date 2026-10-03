//! Owned HTTP fixture for actual strict connection setup.
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::Instant;

async fn read_request(stream: &mut TcpStream) -> Result<Value, String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 65536 {
            return Err("fixture header limit".into());
        }
        head.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let head = std::str::from_utf8(&head).map_err(|e| e.to_string())?;
    if !head.starts_with("POST /mcp HTTP/1.1\r\n") {
        return Err("wrong endpoint/method".into());
    }
    let mut headers = serde_json::Map::new();
    for (k, v) in head.lines().filter_map(|l| l.split_once(':')) {
        if headers
            .insert(k.to_ascii_lowercase(), json!(v.trim()))
            .is_some()
        {
            return Err("duplicate request header".into());
        }
    }
    let length: usize = headers
        .get("content-length")
        .and_then(Value::as_str)
        .ok_or("missing request length")?
        .parse()
        .map_err(|_| "invalid request length")?;
    if length > 65536 {
        return Err("fixture body limit".into());
    }
    let mut body = vec![0; length];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    let body: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    Ok(json!({"headers":headers,"body":body}))
}

fn reply(case: &str, method: &str) -> Result<(u16, Vec<u8>, &'static str), String> {
    if method == "notifications/initialized" {
        return Ok(match case {
            "setup-notification-refused" => (400, Vec::new(), ""),
            "setup-notification-body" => (202, b"x".to_vec(), ""),
            _ => (202, Vec::new(), ""),
        });
    }
    let (mut result, mut extra, id) = match method {
        "server/discover" => (
            json!({"resultType":"complete","supportedVersions":["2026-07-28","future"],"capabilities":{"tools":{},"resources":{},"prompts":{},"unknown":{"kept":true}},"ttlMs":0,"cacheScope":"private","_meta":{"io.modelcontextprotocol/serverInfo":{"name":"fixture-server","version":"1"}},"extension":{"$serde_json::private::Number":"7"}}),
            "",
            1,
        ),
        "initialize" => (
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"fixture-server","version":"1"}}),
            "Mcp-Session-Id: fixture-secret\r\n",
            1,
        ),
        "tools/call" => (
            json!({"resultType":"complete","content":[{"type":"text","text":"called"}]}),
            "",
            2,
        ),
        _ => return Err("unexpected fixture method".into()),
    };
    if method != "tools/call" {
        if method == "server/discover" && case == "setup-reuse" {
            extra = "Mcp-Session-Id: modern-do-not-adopt\r\n";
        }
        match case {
            "setup-revision-mismatch" => { result["protocolVersion"] = json!("other"); result["supportedVersions"] = json!(["other"]); },
            "setup-invalid-description" => result["capabilities"]["tools"] = Value::Null,
            "setup-missing-cache" => { result.as_object_mut().ok_or("fixture result")?.remove("ttlMs"); },
            "setup-missing-info" => { result.as_object_mut().ok_or("fixture result")?.remove("serverInfo"); result.as_object_mut().ok_or("fixture result")?.remove("_meta"); },
            "setup-negative-ttl" => result["ttlMs"] = json!(-1),
            "setup-unknown-result" => result["resultType"] = json!("input_required"),
            "setup-peer-error" => return Ok((if method == "server/discover" {400} else {200},br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32022,"message":"opaque","data":{"supported":["other"]}}}"#.to_vec(),"")),
            "setup-ambiguous400" => return Ok((400,b"ambiguous".to_vec(),"")),
            "setup-invalid-session" => extra = "Mcp-Session-Id: bad session\r\n",
            "setup-duplicate-session" => extra = "Mcp-Session-Id: fixture-secret\r\nMcp-Session-Id: duplicate\r\n",
            "setup-sessionless" => extra = "",
            _ => {},
        }
    }
    let mut body = serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"result":result}))
        .map_err(|e| e.to_string())?;
    if method == "server/discover" && matches!(case, "setup-zero-ttl" | "setup-large-ttl") {
        let number = if case == "setup-zero-ttl" {
            "-0"
        } else {
            "123456789012345678901234567890"
        };
        body = String::from_utf8(body)
            .map_err(|e| e.to_string())?
            .replace("\"ttlMs\":0", &format!("\"ttlMs\":{number}"))
            .into_bytes();
    }
    Ok((200, body, extra))
}

async fn serve(
    listener: TcpListener,
    case: String,
    recording: Arc<Mutex<Vec<Value>>>,
    mut stop: oneshot::Receiver<()>,
) -> Result<(), String> {
    let service = async {
        loop {
            let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
            let request = read_request(&mut stream).await?;
            let method = request["body"]["method"]
                .as_str()
                .ok_or("no method")?
                .to_owned();
            let count = {
                let mut calls = recording.lock().map_err(|_| "fixture lock")?;
                calls.push(request);
                calls.len()
            };
            if count > 8 {
                return Err("fixture exchange count bound".into());
            }
            let (status, body, extra) = reply(&case, &method)?;
            if case == "setup-deadline" {
                tokio::time::sleep(Duration::from_millis(if method == "server/discover" {
                    1400
                } else {
                    650
                }))
                .await;
            }
            stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nContent-Type: application/json\r\n{extra}Connection: close\r\n\r\n",body.len()).as_bytes()).await.map_err(|e| e.to_string())?;
            stream.write_all(&body).await.map_err(|e| e.to_string())?;
            stream.shutdown().await.map_err(|e| e.to_string())?;
        }
    };
    tokio::select! { _ = &mut stop => Ok(()), result = service => result, () = tokio::time::sleep(Duration::from_secs(10)) => Err("fixture deadline".into()) }
}

pub async fn observe(case: &str, revision: &str) -> Result<Value, String> {
    let mut raw_input = json!({
        "revision":revision,"client_info":{"name":"fixture-client","version":"1"},"session_id_octets":128,
        "budget":{"request_octets":4096,"response_octets":4096,"sse_event_octets":4096,"remaining_execution_ms":3000,"provider_ms":3000,"connect_ms":1000}
    });
    match case {
        "setup-session-bound" => raw_input["session_id_octets"] = json!(0),
        "setup-request-bound" => raw_input["budget"]["request_octets"] = json!(0),
        "setup-invalid-input" => raw_input["budget"]["connect_ms"] = json!(0),
        _ => {}
    }
    let input = serde_json::from_value(raw_input).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let endpoint = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let captured = Arc::new(Mutex::new(Vec::<Value>::new()));
    let (stop_tx, stop_rx) = oneshot::channel();
    let server = tokio::spawn(serve(listener, case.to_owned(), captured.clone(), stop_rx));
    let deadline =
        Instant::now() + Duration::from_millis(if case == "setup-deadline" { 1000 } else { 3000 });
    let result = b10x_mcp_client::strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        reqwest::Request::new(
            reqwest::Method::POST,
            endpoint.parse().map_err(|_| "fixture URL")?,
        ),
        &input,
        deadline,
    )
    .await;
    let actual = match result {
        Ok(mut connection) => {
            let mut actual = json!({"ready":true,"description":connection.description(),"debug":format!("{connection:?}")});
            if matches!(
                case,
                "setup-reuse"
                    | "setup-metadata-conflict"
                    | "setup-missing-family"
                    | "setup-unicode-name"
            ) {
                let method = if case == "setup-missing-family" {
                    "prompts/get"
                } else {
                    "tools/call"
                };
                let mut params = json!({"name":if case=="setup-unicode-name" {"snowman☃"} else {"effect"},"arguments":{}});
                if case == "setup-metadata-conflict" {
                    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":"other"});
                }
                match connection
                    .exchange(method, params, Instant::now() + Duration::from_secs(3))
                    .await
                {
                    Ok(result) => {
                        actual["business"] =
                            serde_json::to_value(result).map_err(|e| e.to_string())?;
                    }
                    Err(_) => actual["business_error"] = json!(true),
                }
            }
            actual
        }
        Err(error) => json!({"ready":false,"error":error}),
    };
    let _ = stop_tx.send(());
    server.await.map_err(|e| e.to_string())??;
    let calls = captured.lock().map_err(|_| "fixture lock")?.clone();
    let mut actual = actual;
    actual["calls"] = json!(calls.len());
    actual["requests"] = json!(calls);
    Ok(actual)
}
