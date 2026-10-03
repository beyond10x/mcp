//! Independent raw HTTP fixture; every observation comes from the real client.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use b10x_mcp_types::http_exchange::McpHttpExchangeExchangeInput;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::Instant;

const REQUEST: &[u8] = br#"{ "jsonrpc":"2.0", "id":17, "method":"tools/call", "params":{"name":"effect","arguments":{}} }"#;
const SUCCESS: &[u8] = br#"{ "jsonrpc":"2.0", "id":17, "result":{"content":[{"type":"text","text":"wire"}],"structuredContent":null,"extension":{"kept":true}} }"#;

struct Reply {
    body: Vec<u8>,
    status: u16,
    content_type: &'static str,
    response_limit: usize,
    request_limit: usize,
    event_limit: usize,
    missing_tail: bool,
    delay: bool,
}
impl Reply {
    fn selected(case: &str) -> Result<Self, String> {
        let mut reply = Self {
            body: SUCCESS.to_vec(),
            status: 200,
            content_type: "application/json",
            response_limit: 4096,
            request_limit: 4096,
            event_limit: 4096,
            missing_tail: false,
            delay: false,
        };
        match case {
            "json-complete" => {}
            "invalid-http-status" => reply.status = 900,
            "reserved-json-key" => reply.body = br#"{"jsonrpc":"2.0","id":17,"result":{"extension":{"$serde_json::private::Number":"7"}}}"#.to_vec(),
            "reserved-peer-data" => reply.body = br#"{"jsonrpc":"2.0","id":17,"error":{"code":-1,"message":"opaque","data":{"$serde_json::private::Number":"7","$serde_json::private::RawValue":"null"}}}"#.to_vec(),
            "peer-data-absent" => reply.body = br#"{"jsonrpc":"2.0","id":17,"error":{"code":-32099,"message":"opaque"}}"#.to_vec(),
            "peer-data-null" => reply.body = br#"{"jsonrpc":"2.0","id":17,"error":{"code":-32099,"message":"opaque","data":null}}"#.to_vec(),
            "huge-peer-code" => reply.body = br#"{"jsonrpc":"2.0","id":17,"error":{"code":123456789012345678901234567890,"message":"opaque"}}"#.to_vec(),
            "fractional-peer-code" => reply.body = br#"{"jsonrpc":"2.0","id":17,"error":{"code":1.5,"message":"opaque"}}"#.to_vec(),
            "wrong-id" => reply.body = br#"{"jsonrpc":"2.0","id":18,"result":{}}"#.to_vec(),
            "null-id" => reply.body = br#"{"jsonrpc":"2.0","id":null,"result":{}}"#.to_vec(),
            "duplicate-id" => reply.body = br#"{"jsonrpc":"2.0","id":18,"id":17,"result":{}}"#.to_vec(),
            "exclusive-result-error" => reply.body = br#"{"jsonrpc":"2.0","id":17,"result":{},"error":{"code":-1,"message":"opaque"}}"#.to_vec(),
            "response-bound" => reply.response_limit = SUCCESS.len() - 1,
            "zero-response-bound" => reply.response_limit = 0,
            "exact-response-bound" => reply.response_limit = SUCCESS.len(),
            "utf8-bound" => {
                reply.body = "{\"jsonrpc\":\"2.0\",\"id\":17,\"result\":{\"text\":\"€\"}}".as_bytes().to_vec();
                reply.response_limit = reply.body.iter().position(|b| *b == 0xe2).ok_or("fixture UTF8 byte missing")? + 1;
            }
            "request-bound-before-send" => reply.request_limit = REQUEST.len() - 1,
            "body-loss" => reply.missing_tail = true,
            "deadline" => reply.delay = true,
            "session-expired-once" => reply.status = 404,
            "redirect" => reply.status = 307,
            case if case.starts_with("sse-") => reply.sse(case)?,
            _ => return Err("unknown owned fixture case".into()),
        }
        Ok(reply)
    }
    fn sse(&mut self, case: &str) -> Result<(), String> {
        self.content_type = "text/event-stream";
        self.body = b"event: message\ndata: ".to_vec();
        self.body.extend_from_slice(SUCCESS);
        if case != "sse-prefix-incomplete" {
            self.body.extend_from_slice(b"\n\n");
        }
        match case {
            "sse-terminal" | "sse-prefix-incomplete" => {}
            "sse-event-bound" => self.event_limit = 20,
            "sse-event-exact" => self.event_limit = self.body.len(),
            "sse-zero-bound" => self.event_limit = 0,
            "sse-response-bound" => self.response_limit = SUCCESS.len() - 1,
            "sse-crlf" => self.body = String::from_utf8(self.body.clone()).map_err(|e| e.to_string())?.replace('\n', "\r\n").into_bytes(),
            "sse-cr" => self.body = self.body.iter().map(|b| if *b == b'\n' { b'\r' } else { *b }).collect(),
            "sse-multiline" => self.body = b"data: {\"jsonrpc\":\"2.0\",\"id\":17,\ndata: \"result\":{\"text\":\"multiline\"}}\n\n".to_vec(),
            "sse-progress" => {
                let mut prefix = b": comment\n\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{}}\n\n".to_vec();
                prefix.extend_from_slice(&self.body); self.body = prefix;
            }
            "sse-bom" => { let mut prefix = vec![0xef, 0xbb, 0xbf]; prefix.extend_from_slice(b"data: "); prefix.extend_from_slice(SUCCESS); prefix.extend_from_slice(b"\n\n"); self.body = prefix; }
            _ => return Err("unknown SSE fixture".into()),
        }
        Ok(())
    }
}

async fn request(stream: &mut TcpStream, revision: &str) -> Result<(), String> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() == 65_536 {
            return Err("fixture header bound".to_owned());
        }
        header.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let header = std::str::from_utf8(&header).map_err(|e| e.to_string())?;
    if !header.starts_with("POST /mcp HTTP/1.1\r\n") {
        return Err("unexpected HTTP request".into());
    }
    let headers: Vec<_> = header.lines().filter_map(|l| l.split_once(':')).collect();
    if !headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("mcp-protocol-version") && v.trim() == revision)
    {
        return Err("wrong revision on wire".into());
    }
    let lengths: Vec<_> = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .collect();
    if lengths.len() != 1 {
        return Err("fixture requires one length".into());
    }
    let length: usize = lengths[0]
        .1
        .trim()
        .parse()
        .map_err(|_| "invalid fixture length")?;
    if length > 65_536 {
        return Err("fixture request bound".into());
    }
    let mut received = vec![0; length];
    stream
        .read_exact(&mut received)
        .await
        .map_err(|e| e.to_string())?;
    if received != REQUEST {
        return Err("request bytes were regenerated".into());
    }
    let parsed: Value = serde_json::from_slice(&received).map_err(|e| e.to_string())?;
    if parsed["id"] != 17 || parsed["method"] != "tools/call" {
        return Err("wrong RPC request".into());
    }
    Ok(())
}

pub async fn observe(case: &str, revision: &str) -> Result<(Value, usize), String> {
    observe_input(case, revision, |_| {}).await
}

pub async fn observe_input(
    case: &str,
    revision: &str,
    mutate: impl FnOnce(&mut Value),
) -> Result<(Value, usize), String> {
    let reply = Reply::selected(case)?;
    let mut input = json!({
        "revision":revision,"request_id":{"kind":"integer","value":17},"encoded_request":STANDARD.encode(REQUEST),
        "budget":{"request_octets":reply.request_limit,"response_octets":reply.response_limit,"sse_event_octets":reply.event_limit,
            "remaining_execution_ms":3000,"provider_ms":3000,"connect_ms":1000}
    });
    mutate(&mut input);
    let input: McpHttpExchangeExchangeInput =
        serde_json::from_value(input).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let endpoint = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let expected_revision = revision.to_owned();
    let delayed = reply.delay;
    let server = tokio::spawn(async move {
        let service = async {
            loop {
                let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
                request(&mut stream, &expected_revision).await?;
                counted.fetch_add(1, Ordering::SeqCst);
                let length = reply.body.len() + usize::from(reply.missing_tail);
                stream.write_all(format!("HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {length}\r\nLocation: /mcp\r\nConnection: close\r\n\r\n", reply.status, reply.content_type).as_bytes()).await.map_err(|e| e.to_string())?;
                if reply.delay {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                stream
                    .write_all(&reply.body)
                    .await
                    .map_err(|e| e.to_string())?;
                stream.shutdown().await.map_err(|e| e.to_string())?;
            }
        };
        tokio::select! {
            _ = &mut stop_rx => Ok::<(), String>(()),
            result = service => result,
            () = tokio::time::sleep(Duration::from_secs(10)) => Err("owned fixture deadline".into()),
        }
    });
    let action = async {
        let request = reqwest::Request::new(
            reqwest::Method::POST,
            endpoint
                .parse::<reqwest::Url>()
                .map_err(|e| e.to_string())?,
        );
        let deadline = Instant::now() + Duration::from_millis(if delayed { 500 } else { 3000 });
        let result = b10x_mcp_client::strict_http::exchange(
            reqwest::Client::builder().no_proxy(),
            request,
            &input,
            deadline,
        )
        .await
        .map_err(|e| e.to_string())?;
        serde_json::to_value(result).map_err(|e| e.to_string())
    }
    .await;
    let _ = stop_tx.send(());
    server.await.map_err(|e| e.to_string())??;
    Ok((action?, calls.load(Ordering::SeqCst)))
}
