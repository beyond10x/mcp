//! Wire-observed cancellation, control acknowledgements and terminal precedence.
use super::{Peer, request_template, response, serve};
use b10x_mcp_client::{strict_cancellation::Cancellation, strict_connection};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::{Instant, timeout},
};

async fn closed(stream: &mut TcpStream) -> Result<(), String> {
    match timeout(Duration::from_secs(3), stream.read_u8())
        .await
        .map_err(|_| "cancelled socket stayed open")?
    {
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        _ => Err("cancelled socket did not close".into()),
    }
}
pub async fn business(mut stream: TcpStream, peer: &Peer, request: &Value) -> Result<(), String> {
    let (_, body, _) = response(peer, request)?;
    let finished = peer.case == "cancel-completed" || request["body"]["id"] != json!(2);
    if finished {
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stream.write_all(&body).await.map_err(|e| e.to_string())?;
        return Ok(());
    }
    if peer.case != "cancel-before-headers" {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n: waiting\n\n").await.map_err(|e|e.to_string())?;
    }
    if matches!(
        peer.case.as_str(),
        "cancel-control" | "cancel-control-bound" | "cancel-terminal-race"
    ) {
        stream
            .write_all(b"data: {\"jsonrpc\":\"2.0\",\"id\":\"peer\",\"method\":\"ping\"}\n\n")
            .await
            .map_err(|e| e.to_string())?;
        if peer.case == "cancel-terminal-race" {
            stream
                .write_all(
                    format!(
                        "data: {}\n\n",
                        std::str::from_utf8(&body).map_err(|e| e.to_string())?
                    )
                    .as_bytes(),
                )
                .await
                .map_err(|e| e.to_string())?;
        }
    } else if peer.case != "cancel-timeout" {
        peer.cancellation.cancel();
        peer.cancellation.cancel(); // Repeated requests remain one signal.
    }
    if peer.case == "cancel-late" {
        // These bytes are offered after cancellation, not claimed observed.
        let _ = stream
            .write_all(
                format!(
                    "data: {}\n\n",
                    std::str::from_utf8(&body).map_err(|e| e.to_string())?
                )
                .as_bytes(),
            )
            .await;
    }
    closed(&mut stream).await?;
    peer.stream_closed.notify_one();
    Ok(())
}
pub async fn control(mut stream: TcpStream, peer: &Peer, request: &Value) -> Result<(), String> {
    if request["body"].get("method").is_none() {
        peer.cancellation.cancel();
        closed(&mut stream).await?;
        return Ok(());
    }
    if peer.case == "cancel-notification-timeout" {
        closed(&mut stream).await?;
        return Ok(());
    }
    let (status, body) = match peer.case.as_str() {
        "cancel-notification-failure" => (503, b"cancellation failed".to_vec()),
        "cancel-notification-body" => (202, b"not empty".to_vec()),
        "cancel-notification-bound" | "cancel-control-bound" => (202, vec![b'x'; 8192]),
        _ => (202, Vec::new()),
    };
    let head = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let _ = stream.write_all(&body).await;
    Ok(())
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
        cancellation: Cancellation::new(),
        requests: Arc::default(),
        started: Arc::default(),
        stream_closed: Arc::default(),
        pool_closed: Arc::default(),
        answered: Arc::default(),
    };
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve(listener, peer.clone(), stopped));
    let input = serde_json::from_value(json!({"revision":revision,"client_info":{"name":"cancel-fixture","version":"1"},"session_id_octets":256,
        "budget":{"request_octets":4096,"response_octets":4096,"sse_event_octets":4096,"remaining_execution_ms":5000,"provider_ms":5000,"connect_ms":1000}})).map_err(|e|e.to_string())?;
    let mut client = strict_connection::connect(
        reqwest::Client::builder().no_proxy(),
        request_template(&endpoint)?,
        &input,
        Instant::now() + Duration::from_secs(2),
    )
    .await
    .map_err(|_| "setup")?;
    if case == "cancel-before-send" {
        peer.cancellation.cancel();
    }
    let now = Instant::now();
    let operation = if case == "cancel-timeout" {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(1)
    };
    let teardown = match case {
        "cancel-teardown-expired" => now,
        "cancel-notification-timeout" => now + Duration::from_millis(200),
        _ => now + Duration::from_secs(2),
    };
    let result = client
        .exchange_cancellable(
            if case == "cancel-initialize" {
                "initialize"
            } else {
                "tools/call"
            },
            json!({"name":"fixture","arguments":{}}),
            now + operation,
            &peer.cancellation,
            teardown,
        )
        .await;
    let (outcome, refused_input) = match result {
        Ok(outcome) => (
            serde_json::to_value(outcome).map_err(|e| e.to_string())?,
            false,
        ),
        Err(_) if case == "cancel-initialize" => (Value::Null, true),
        Err(_) => return Err("controlled exchange failed".into()),
    };
    if case == "cancel-completed" {
        peer.cancellation.cancel();
    }
    let reused = if case == "cancel-reuse" {
        let result = client
            .exchange(
                "tools/call",
                json!({"name":"fixture","arguments":{}}),
                Instant::now() + Duration::from_secs(1),
            )
            .await
            .map_err(|_| "reuse refused")?;
        serde_json::to_value(result).map_err(|e| e.to_string())?["kind"] == "result"
    } else {
        false
    };
    if !matches!(
        case,
        "cancel-before-send" | "cancel-initialize" | "cancel-completed"
    ) {
        timeout(Duration::from_secs(3), peer.stream_closed.notified())
            .await
            .map_err(|_| "stream closure not observed")?;
    }
    drop(client);
    stop.send(()).map_err(|()| "peer stopped")?;
    timeout(Duration::from_secs(4), server)
        .await
        .map_err(|_| "peer join timeout")?
        .map_err(|e| e.to_string())??;
    let requests = peer.requests.lock().map_err(|_| "fixture lock")?.clone();
    Ok(json!({"outcome":outcome,"requests":requests,"refused_input":refused_input,"reused":reused}))
}

pub fn observations(actual: &Value) -> Result<Value, String> {
    let outcome = &actual["outcome"];
    let requests = actual["requests"].as_array().ok_or("requests")?;
    let controls: Vec<_> = requests
        .iter()
        .filter(|r| r["body"]["method"] == "notifications/cancelled")
        .collect();
    let interrupted = &outcome["value"]["interrupted"];
    let notification = &outcome["value"]["notification"];
    let observation = &interrupted["exchange"];
    let mut before = observation["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap_or(0);
    for message in observation["stream"]["messages"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let value = &message["value"];
        let wires = match message["kind"].as_str() {
            Some("notification") => vec![value],
            Some("progress") => vec![&value["exchange"]],
            Some("server_request") => vec![&value["request"], &value["reply"]["exchange"]],
            _ => return Err("unknown stream observation".into()),
        };
        for wire in wires {
            before += wire["response"]["counts"]["retained_octets"]
                .as_u64()
                .ok_or("retained count")?;
        }
    }
    let after = notification["exchange"]["response"]["counts"]["retained_octets"]
        .as_u64()
        .unwrap_or(0);
    Ok(
        json!({"kind":outcome["kind"].as_str().unwrap_or("absent"),"cause":interrupted["cause"].as_str().unwrap_or("absent"),
        "controls":controls.len(),"requests":requests.len(),"notification":notification["disposition"].as_str().unwrap_or("absent"),
        "control_reason":notification["refusal"].as_str().unwrap_or("absent"),"late_observed":outcome["value"].get("late_response").is_some(),
        "id_agreement":controls.iter().all(|r| r["body"]["params"]["requestId"] == interrupted["request_id"]["value"]),
        "header_agreement":controls.iter().all(|r|r["headers"]["authorization"]=="Bearer fixture-only-credential" && r["headers"]["mcp-protocol-version"]=="2025-11-25" && r["headers"].get("mcp-method").is_none()),
        "within_budget":before+after<=4096,"refused_input":actual["refused_input"],"reused":actual["reused"]}),
    )
}
