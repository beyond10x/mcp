//! Actual progress notification bodies and a continuously writing HTTP peer.
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Instant, sleep_until},
};

pub fn params(case: &str) -> Value {
    let mut params = json!({"name":"fixture","arguments":{}});
    if !case.starts_with("stream-progress-") || case == "stream-progress-unopted" {
        return params;
    }
    let token = match case {
        "stream-progress-large-token" => number("123456789012345678901234567890"),
        "stream-progress-token-kind" => json!("7"),
        "stream-progress-invalid-input" => json!(false),
        "stream-progress-null-input" => Value::Null,
        "stream-progress-fractional-input" => number("1.5"),
        _ => json!("owned-token"),
    };
    params["_meta"] = json!({"progressToken":token});
    params
}
fn number(text: &str) -> Value {
    serde_json::from_str(text).expect("literal fixture number")
}
fn values(case: &str) -> Vec<Value> {
    let values: &[&str] = match case {
        "stream-progress-exact" => &[
            "-2",
            "-0.5",
            "-0.0",
            "0.0001",
            "9007199254740992",
            "9007199254740993",
            "1e999999999999999999999999",
            "2e999999999999999999999999",
        ],
        "stream-progress-tiny" => &["1e-999999999999999999999999", "2e-999999999999999999999999"],
        "stream-progress-equal" => &["1", "1.0"],
        "stream-progress-exponent-equal" => &["100", "1e2"],
        "stream-progress-negative-zero" => &["-0.0", "0"],
        "stream-progress-decreasing" => &["2", "1"],
        "stream-progress-unmatched" => &["100", "1", "500", "2"],
        "stream-progress-bound" => &["1", "2", "3"],
        _ => &["1", "2"],
    };
    values.iter().map(|v| number(v)).collect()
}
fn notification(token: &Value, progress: &Value) -> Value {
    json!({"jsonrpc":"2.0","method":"notifications/progress","params":{
        "progressToken":token,"progress":progress,"message":"untrusted server text"}})
}
fn messages(case: &str, request: &Value) -> Vec<Value> {
    let token = request["body"]["params"]["_meta"]
        .get("progressToken")
        .cloned()
        .unwrap_or(json!("unsolicited"));
    values(case)
        .into_iter()
        .enumerate()
        .map(|(index, progress)| {
            let token = if case == "stream-progress-unmatched" && index % 2 == 0 {
                json!("other")
            } else {
                token.clone()
            };
            let mut event = notification(&token, &progress);
            let params = &mut event["params"];
            match case {
                "stream-progress-token-kind" if index == 0 => params["progressToken"] = json!(7),
                "stream-progress-malformed" => params["progress"] = Value::Null,
                "stream-progress-bad-total" => params["total"] = json!("2"),
                "stream-progress-bad-message" => params["message"] = json!(2),
                "stream-progress-bad-token" => params["progressToken"] = number("1.5"),
                "stream-progress-bound" => params["message"] = json!("x".repeat(1500)),
                "stream-progress-exact" => params["total"] = number("1e999999999999999999999999"),
                _ => {}
            }
            event
        })
        .collect()
}
async fn frame(stream: &mut TcpStream, event: &Value) -> Result<(), String> {
    stream
        .write_all(format!("data: {event}\n\n").as_bytes())
        .await
        .map_err(|e| e.to_string())
}
pub async fn reply(
    mut stream: TcpStream,
    case: &str,
    request: &Value,
    body: &[u8],
) -> Result<(), String> {
    if case == "stream-progress-deadline" {
        return continuous(&mut stream, request).await;
    }
    for event in messages(case, request) {
        if frame(&mut stream, &event).await.is_err() {
            return Ok(()); // The receiver may already have refused this message.
        }
    }
    let terminal = format!(
        "data: {}\n\n",
        std::str::from_utf8(body).map_err(|e| e.to_string())?
    );
    let _ = stream.write_all(terminal.as_bytes()).await;
    Ok(())
}
async fn continuous(stream: &mut TcpStream, request: &Value) -> Result<(), String> {
    let token = request["body"]["params"]["_meta"]["progressToken"].clone();
    let end = Instant::now() + Duration::from_secs(3);
    let mut counter = 0;
    loop {
        tokio::select! {
            biased;
            result = stream.read_u8() => return match result {
                Err(e) if matches!(e.kind(), std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset) => Ok(()),
                _ => Err("progress stream did not close".into()),
            },
            () = sleep_until(end) => return Err("progress extended the operation deadline".into()),
            () = tokio::time::sleep(Duration::from_millis(15)) => {
                frame(stream, &notification(&token, &json!(counter))).await?;
                counter += 1;
            }
        }
    }
}

pub fn observations(actual: &Value) -> Result<Value, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let exchange = &actual["exchange"];
    let wire = &exchange["value"]["observation"];
    let requests = actual["requests"].as_array().ok_or("requests")?;
    let requested = requests
        .iter()
        .find(|r| r["body"]["method"] == "tools/call")
        .map(|r| &r["body"]["params"]["_meta"]["progressToken"]);
    let messages = wire["stream"]["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut retained = wire["response"]["counts"]["retained_octets"]
        .as_u64()
        .ok_or("retained")?;
    let mut exact = true;
    let mut owned = true;
    let mut accepted = false;
    let mut unmatched = false;
    let mut non_increasing = false;
    for message in &messages {
        if message["kind"] != "progress" {
            return Err("expected progress message".into());
        }
        let value = &message["value"];
        let raw = &value["exchange"]["response"];
        retained += raw["counts"]["retained_octets"]
            .as_u64()
            .ok_or("message retained")?;
        let bytes = STANDARD
            .decode(raw["bytes"].as_str().ok_or("bytes")?)
            .map_err(|e| e.to_string())?;
        let original: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        exact &= ["progress", "total", "message"]
            .iter()
            .all(|k| original["params"].get(k) == value.get(k))
            && original["params"]["progressToken"] == value["token"]["value"];
        match value["disposition"].as_str().ok_or("disposition")? {
            "accepted" => {
                accepted = true;
                owned &= requested == Some(&value["token"]["value"]);
            }
            "unmatched_token" => unmatched = true,
            "non_increasing" => non_increasing = true,
            _ => return Err("unknown disposition".into()),
        }
    }
    Ok(
        json!({"kind":exchange["kind"],"reason":exchange["value"]["reason"].as_str().unwrap_or("absent"),
        "requests":requests.len(),"send":wire["send"],"progress_seen":!messages.is_empty(),
        "accepted_seen":accepted,"unmatched_seen":unmatched,"non_increasing_seen":non_increasing,
        "exact_values":exact,"accepted_owns_token":owned,"within_budget":retained<=4096,
        "own_payload":exchange["value"]["result"].get("content").is_some()}),
    )
}
