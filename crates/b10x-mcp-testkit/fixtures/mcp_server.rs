//! A controlled MCP server, for driving this workspace's transports against something real.
//!
//! It is compiled by a test with the workspace toolchain rather than shipped as a binary, and it
//! deliberately uses only the standard library: a transport proven against a server that shares its
//! own framing code has proven nothing. Both modes answer the same three methods with the same
//! bytes, so a difference between the two transports cannot come from the server.
//!
//! ```text
//! mcp_server stdio   # newline-delimited JSON-RPC on stdin/stdout
//! mcp_server http    # Streamable HTTP on a loopback port, announced as {"url":"…"} on stdout
//! ```

use std::io::{self, BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("stdio") => serve_stdio(),
        Some("http") => serve_http(),
        other => panic!("expected `stdio` or `http`, got {other:?}"),
    }
}

fn serve_stdio() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return };
        if let Some(response) = mcp_response(&line) {
            writeln!(stdout, "{response}").expect("write MCP response");
            stdout.flush().expect("flush MCP response");
        }
    }
}

fn serve_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind controlled endpoint");
    let address = listener.local_addr().expect("controlled endpoint address");
    println!(r#"{{"url":"http://{address}/mcp"}}"#);
    io::stdout().flush().expect("announce controlled endpoint");

    for stream in listener.incoming() {
        let mut stream = stream.expect("accept controlled request");
        let request = read_request(&mut stream);
        match request.method.as_str() {
            // The transport may open a server-to-client stream. Saying "not here" is the spec's
            // answer for a server that pushes nothing, and it must not look like a protocol error.
            "GET" => respond(&mut stream, "405 Method Not Allowed", ""),
            // Session teardown.
            "DELETE" => respond(&mut stream, "200 OK", ""),
            "POST" => match mcp_response(&request.body) {
                Some(body) => respond(&mut stream, "200 OK", &body),
                None => respond(&mut stream, "202 Accepted", ""),
            },
            other => panic!("unexpected MCP HTTP method {other}"),
        }
    }
}

struct Request {
    method: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Request {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read request line");
    let method = line
        .split_ascii_whitespace()
        .next()
        .expect("request method")
        .to_owned();
    let mut content_length = 0;
    loop {
        line.clear();
        reader.read_line(&mut line).expect("read request header");
        if line == "\r\n" || line.is_empty() {
            break;
        }
        let (name, value) = line.trim_end().split_once(':').expect("valid request header");
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().expect("numeric content length");
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).expect("read request body");
    Request {
        method,
        body: String::from_utf8(body).expect("request body is UTF-8"),
    }
}

/// The three methods a tools-only client asks for, and a named refusal for anything else.
///
/// `None` is a notification — no `id`, so nothing may be written back.
fn mcp_response(request: &str) -> Option<String> {
    let id = json_rpc_id(request)?;
    if request.contains(r#""method":"initialize""#) {
        Some(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"result":{{"protocolVersion":"2026-07-28","capabilities":{{"tools":{{"listChanged":false}}}},"serverInfo":{{"name":"b10x-mcp-controlled","version":"1"}}}}}}"#
        ))
    } else if request.contains(r#""method":"tools/list""#) {
        Some(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"result":{{"tools":[{{"name":"read_issue","description":"Read a synthetic issue","inputSchema":{{"type":"object","properties":{{"id":{{"type":"string"}}}}}},"annotations":{{"readOnlyHint":false}}}},{{"name":"close_issue","description":"Close a synthetic issue","inputSchema":{{"type":"object","properties":{{"id":{{"type":"string"}}}}}},"annotations":{{"readOnlyHint":true}}}}]}}}}"#
        ))
    } else if request.contains(r#""method":"tools/call""#) {
        Some(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"result":{{"content":[{{"type":"text","text":"controlled issue is open"}}],"isError":false}}}}"#
        ))
    } else {
        Some(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":-32601,"message":"method not found"}}}}"#
        ))
    }
}

fn json_rpc_id(request: &str) -> Option<&str> {
    let rest = request.split_once(r#""id":"#)?.1.trim_start();
    if rest.starts_with('"') {
        let end = rest[1..].find('"')? + 2;
        Some(&rest[..end])
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write controlled response");
    stream.flush().expect("flush controlled response");
}
