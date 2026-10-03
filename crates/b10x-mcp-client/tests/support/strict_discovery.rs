//! An independent bounded HTTP peer; replies use literal scenario data and IDs.
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::Instant,
};
async fn read(stream: &mut TcpStream) -> Result<Value, String> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= 65536 {
            return Err("fixture header bound".into());
        }
        bytes.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let head = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    if !head.starts_with("POST /mcp HTTP/1.1\r\n") {
        return Err("fixture endpoint/method".into());
    }
    let mut headers = serde_json::Map::new();
    for (key, value) in head.lines().filter_map(|line| line.split_once(':')) {
        if headers
            .insert(key.to_ascii_lowercase(), json!(value.trim()))
            .is_some()
        {
            return Err("fixture duplicate header".into());
        }
    }
    let len: usize = headers["content-length"]
        .as_str()
        .ok_or("fixture length")?
        .parse()
        .map_err(|_| "fixture length")?;
    if len > 65536 {
        return Err("fixture body bound".into());
    }
    let mut body = vec![0; len];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    let body: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    Ok(json!({"body":body,"headers":headers}))
}
fn descriptor(family: &str, second: bool) -> Value {
    let name = if second { "two" } else { "one" };
    match family {
        "tools" => {
            json!({"name":name,"inputSchema":{"type":"object"},"extension":{"$serde_json::private::Number":"7"}})
        }
        "resources" => {
            json!({"name":name,"uri":if second{"test:two"}else{"test:one"},"size":2.5,"extension":{"kept":true}})
        }
        _ => {
            json!({"name":name,"arguments":[{"name":"input","required":true,"extra":{"$serde_json::private::Number":"7"}}],"extension":{"kept":true}})
        }
    }
}
fn page(case: &str, family: &str, revision: &str, index: usize) -> Value {
    let mut row = descriptor(family, index > 0 && case != "list-duplicate");
    if case == "list-prompt-extension" {
        row["annotations"] = json!(["opaque extension"]);
    }
    if case == "list-invalid-descriptor" {
        row["name"] = Value::Null;
    }
    if case == "list-invalid-schema" && family == "tools" {
        row["inputSchema"]["type"] = json!("array");
    }
    if case == "list-invalid-argument" && family == "prompts" {
        row["arguments"][0]["required"] = json!("yes");
    }
    let mut result = json!({family:[row]});
    if case == "list-empty-zero-items" {
        result[family] = json!([]);
    }
    if revision == "2026-07-28" {
        result["resultType"] = json!("complete");
        result["ttlMs"] = json!(0);
        result["cacheScope"] = json!("private");
    }
    if index == 0
        && !matches!(
            case,
            "list-empty-zero-items" | "list-exact-descriptor" | "list-descriptor-limit"
        )
    {
        result["nextCursor"] = json!(if case == "list-empty-cursor" {
            ""
        } else {
            "opaque/cursor?☃"
        });
    }
    if case == "list-repeated-cursor" {
        result["nextCursor"] = json!("opaque/cursor?☃");
    }
    if case == "list-null-cursor" {
        result["nextCursor"] = Value::Null;
    }
    if case == "list-invalid-page" {
        result[family] = json!({});
    }
    if case == "list-missing-cache" {
        result.as_object_mut().unwrap().remove("ttlMs");
    }
    if case == "list-unselected-result" {
        result["resultType"] = json!("input_required");
    }
    result
}
async fn serve(
    listener: TcpListener,
    case: String,
    family: String,
    revision: String,
    captured: Arc<Mutex<Vec<Value>>>,
    mut stop: oneshot::Receiver<()>,
) -> Result<(), String> {
    let service = async {
        let mut index = 0;
        loop {
            let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
            let request = read(&mut stream).await?;
            let method = request["body"]["method"]
                .as_str()
                .ok_or("fixture method")?
                .to_owned();
            let count = {
                let mut rows = captured.lock().map_err(|_| "fixture lock")?;
                rows.push(request);
                rows.len()
            };
            if count > 10 {
                return Err("fixture request bound".into());
            }
            let mut status = 200;
            let body = match method.as_str() {
                "server/discover" | "initialize" => {
                    let mut capabilities = json!({"tools":{},"resources":{},"prompts":{}});
                    if case == "list-unsupported-family" {
                        capabilities.as_object_mut().unwrap().remove(&family);
                    }
                    let result = if method == "server/discover" {
                        json!({"resultType":"complete","supportedVersions":["2026-07-28"],"capabilities":capabilities,"ttlMs":0,"cacheScope":"private"})
                    } else {
                        json!({"protocolVersion":"2025-11-25","capabilities":capabilities,"serverInfo":{"name":"fixture","version":"1"}})
                    };
                    serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"result":result})).unwrap()
                }
                "notifications/initialized" => {
                    status = 202;
                    Vec::new()
                }
                method if method == format!("{family}/list") => {
                    if case == "list-deadline" {
                        tokio::time::sleep(Duration::from_millis(650)).await;
                    }
                    let response = if case == "list-late-error" && index == 1 {
                        json!({"jsonrpc":"2.0","id":3,"error":{"code":-32603,"message":"failed page","data":null}})
                    } else {
                        json!({"jsonrpc":"2.0","id":index+2,"result":page(&case,&family,&revision,index)})
                    };
                    index += 1;
                    serde_json::to_vec(&response).unwrap()
                }
                _ => return Err("unexpected fixture method".into()),
            };
            stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.map_err(|e|e.to_string())?;
            stream.write_all(&body).await.map_err(|e| e.to_string())?;
            stream.shutdown().await.map_err(|e| e.to_string())?;
        }
    };
    tokio::select! {r=service=>r,_=&mut stop=>Ok(()),()=tokio::time::sleep(Duration::from_secs(10))=>Err("fixture timeout".into())}
}
pub async fn observe(case: &str, family: &str, revision: &str) -> Result<Value, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let url = format!(
        "http://{}/mcp",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let captured = Arc::new(Mutex::new(Vec::new()));
    let (stop_tx, stop_rx) = oneshot::channel();
    let server = tokio::spawn(serve(
        listener,
        case.into(),
        family.into(),
        revision.into(),
        captured.clone(),
        stop_rx,
    ));
    let action=async{
  let input=serde_json::from_value(json!({"revision":revision,"client_info":{"name":"fixture","version":"1"},"session_id_octets":64,"budget":{"request_octets":4096,"response_octets":4096,"sse_event_octets":4096,"connect_ms":1000,"provider_ms":if case=="list-deadline"{1000}else{3000},"remaining_execution_ms":5000}})).map_err(|e|e.to_string())?;
  let mut connection=b10x_mcp_client::strict_connection::connect(reqwest::Client::builder().no_proxy(),reqwest::Request::new(reqwest::Method::POST,url.parse().map_err(|_|"fixture URL")?),&input,Instant::now()+Duration::from_secs(3)).await.map_err(|_|"fixture setup refused")?;
  let mut limits=json!({"max_pages":3,"max_items":8,"descriptor_octets":1024});
  match case{
   "list-zero-pages"=>limits["max_pages"]=json!(0),
   "list-invalid-limits"=>limits["max_pages"]=json!(-1),
   "list-page-limit"=>limits["max_pages"]=json!(1),
   "list-repeated-cursor"=>limits["max_pages"]=json!(2),
   "list-item-limit"=>limits["max_items"]=json!(1),
   "list-empty-zero-items"=>limits["max_items"]=json!(0),
   "list-exact-descriptor"|"list-descriptor-limit"=>limits["descriptor_octets"]=json!(serde_json::to_vec(&descriptor(family,false)).unwrap().len()-usize::from(case=="list-descriptor-limit")),
   _=>{}
  }
  let limits=serde_json::from_value(limits).map_err(|e|e.to_string())?;
  let family=serde_json::from_value(json!(family)).map_err(|e|e.to_string())?;
  Ok::<Value,String>(match b10x_mcp_client::strict_discovery::discover(&mut connection,family,&limits,Instant::now()+Duration::from_secs(3)).await{
   Ok(catalog)=>json!({"complete":true,"catalog":catalog}),Err(refusal)=>json!({"complete":false,"refusal":refusal})
  })
 }.await;
    let _ = stop_tx.send(());
    server.await.map_err(|e| e.to_string())??;
    let mut actual = action?;
    actual["requests"] = json!(*captured.lock().map_err(|_| "fixture lock")?);
    Ok(actual)
}
