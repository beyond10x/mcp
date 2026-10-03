//! Independently framed, owned HTTP fixture for the expired-session regression.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use b10x_mcp_client::{connect_http, connect_http_with_client};
use b10x_mcp_types::{ConnectionId, HttpTransportConfig, Limits, ToolCall};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

async fn reply(stream: &mut TcpStream, status: &str, body: Option<Value>) -> Result<(), String> {
    let body = body
        .map_or_else(|| Ok(Vec::new()), |v| serde_json::to_vec(&v))
        .map_err(|e| e.to_string())?;
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nMcp-Session-Id: owned-fixture\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.shutdown().await.map_err(|e| e.to_string())
}

async fn exchange(
    mut stream: TcpStream,
    expire: bool,
    calls: &AtomicUsize,
    initializations: &AtomicUsize,
) -> Result<(), String> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() == 65_536 {
            return Err("fixture header limit".into());
        }
        header.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let header = std::str::from_utf8(&header).map_err(|e| e.to_string())?;
    let method = header
        .split_ascii_whitespace()
        .next()
        .ok_or("missing HTTP method")?;
    if method == "GET" {
        return reply(&mut stream, "405 Method Not Allowed", None).await;
    }
    if method == "DELETE" {
        return reply(&mut stream, "200 OK", None).await;
    }
    if method != "POST" {
        return Err("unexpected HTTP method".into());
    }
    let headers: Vec<_> = header
        .lines()
        .filter_map(|line| line.split_once(':'))
        .collect();
    if headers
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case("transfer-encoding"))
    {
        return Err("fixture requires Content-Length framing".into());
    }
    let lengths: Vec<_> = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .collect();
    if lengths.len() != 1 {
        return Err("fixture requires one Content-Length".into());
    }
    let length = *lengths[0].as_ref().map_err(ToString::to_string)?;
    if length > 65_536 {
        return Err("fixture body limit".into());
    }
    let mut body = vec![0; length];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    let request: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or("missing RPC method")?;
    if !request
        .as_object()
        .ok_or("request is not an object")?
        .contains_key("id")
    {
        return reply(&mut stream, "202 Accepted", None).await;
    }
    let id = request["id"].clone();
    let result = match method {
        "initialize" => {
            initializations.fetch_add(1, Ordering::SeqCst);
            json!({"protocolVersion":"2025-11-25", "capabilities":{"tools":{}}, "serverInfo":{"name":"owned-fixture","version":"1"}})
        }
        "tools/list" => json!({"tools":[{"name":"effect","inputSchema":{"type":"object"}}]}),
        "tools/call" => {
            if request["params"]["name"] != "effect" || !headers.iter().any(|(key, value)| key.eq_ignore_ascii_case("mcp-session-id") && value.trim() == "owned-fixture") {
                return Err("business request must use selected tool and negotiated session".into());
            }
            let previous = calls.fetch_add(1, Ordering::SeqCst);
            if expire && previous == 0 {
                return reply(&mut stream, "404 Not Found", Some(json!({"error":"expired"}))).await;
            }
            json!({"content":[{"type":"text","text":"actual-call"}],"isError":false})
        }
        _ => return reply(&mut stream, "200 OK", Some(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}))).await,
    };
    reply(
        &mut stream,
        "200 OK",
        Some(json!({"jsonrpc":"2.0","id":id,"result":result})),
    )
    .await
}

/// Return actual business/initialize counters and actual result fields after joined teardown.
pub async fn observe(injected: bool, expire: bool) -> Result<(usize, usize, bool, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let config = HttpTransportConfig {
        url: format!(
            "http://{}/mcp",
            listener.local_addr().map_err(|e| e.to_string())?
        ),
        headers: BTreeMap::new(),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let initializations = Arc::new(AtomicUsize::new(0));
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let server_calls = calls.clone();
    let server_initializations = initializations.clone();
    let server = tokio::spawn(async move {
        loop {
            let accepted = tokio::select! {
                _ = &mut stop_rx => return Ok::<_, String>(()),
                accepted = listener.accept() => accepted.map_err(|e| e.to_string())?,
            };
            tokio::time::timeout(
                Duration::from_secs(3),
                exchange(accepted.0, expire, &server_calls, &server_initializations),
            )
            .await
            .map_err(|_| "fixture read deadline".to_owned())??;
        }
    });
    let action = tokio::time::timeout(Duration::from_secs(12), async {
        let id = ConnectionId::new("fixture").map_err(|e| e.to_string())?;
        let limits = Limits {
            request_timeout: Duration::from_secs(5),
            ..Limits::default()
        };
        let mut connection = if injected {
            let client = reqwest::Client::builder()
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?;
            connect_http_with_client(id, &config, None, limits, client).await
        } else {
            connect_http(id, &config, None, limits).await
        }
        .map_err(|e| e.to_string())?;
        let result = connection
            .call(
                &ToolCall {
                    name: "effect".into(),
                    arguments: json!({}),
                },
                None,
            )
            .await;
        let succeeded = result.is_ok();
        let text = result
            .ok()
            .and_then(|r| r.raw["content"][0]["text"].as_str().map(str::to_owned))
            .unwrap_or_default();
        connection
            .close()
            .await
            .map_err(|e| format!("fixture client close: {e}"))?;
        Ok::<_, String>((succeeded, text))
    })
    .await;
    let _ = stop_tx.send(());
    // Do not inspect counters while the fixture could still process a retry.
    server.await.map_err(|e| e.to_string())??;
    let (succeeded, text) = action.map_err(|_| "client action deadline".to_owned())??;
    Ok((
        calls.load(Ordering::SeqCst),
        initializations.load(Ordering::SeqCst),
        succeeded,
        text,
    ))
}
