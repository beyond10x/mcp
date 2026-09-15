//! The Streamable HTTP transport, driven against a controlled server over a real loopback socket.
//!
//! Until this file existed the only exercise of this transport was a consumer's suite in another
//! repository, so this workspace's own gate said nothing about whether Streamable HTTP worked. The
//! server is a standard-library program the testkit compiles here: a transport proven against
//! something that shares its framing code has proven nothing.

use std::io::{BufRead as _, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use b10x_mcp_http::{HttpTransportConfig, connect_http};
use b10x_mcp_types::{ConnectionId, Limits, ToolCall};
use serde_json::{Value, json};

/// Start the controlled server and read the loopback URL it announces on stdout.
fn controlled_endpoint(directory: &std::path::Path) -> (Child, String) {
    let server = b10x_mcp_testkit::build_controlled_server(directory)
        .expect("the controlled server compiles");
    let mut child = Command::new(server)
        .arg("http")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("the controlled server starts");
    let mut line = String::new();
    BufReader::new(child.stdout.as_mut().expect("piped stdout"))
        .read_line(&mut line)
        .expect("the controlled server announces its address");
    let announced: Value = serde_json::from_str(&line).expect("the announcement is JSON");
    let url = announced["url"]
        .as_str()
        .expect("an endpoint URL")
        .to_owned();
    (child, url)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_controlled_http_server_is_negotiated_frozen_and_called() {
    let directory = tempfile::tempdir().expect("a fixture build directory");
    let (mut server, url) = controlled_endpoint(directory.path());
    let config = HttpTransportConfig {
        url,
        headers: std::collections::BTreeMap::new(),
    };

    let connection = connect_http(
        ConnectionId::new("controlled_http").expect("a valid connection id"),
        &config,
        None,
        Limits::default(),
    )
    .await
    .expect("the controlled Streamable HTTP endpoint is reachable");

    let snapshot = connection.snapshot();
    assert_eq!(snapshot.protocol_version, "2026-07-28");
    assert_eq!(
        snapshot
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["read_issue", "close_issue"],
        "the frozen snapshot keeps the server's own order"
    );
    assert_eq!(snapshot.tools[0].raw["annotations"]["readOnlyHint"], false);

    let result = connection
        .call(
            &ToolCall {
                name: "close_issue".to_owned(),
                arguments: json!({"id": "ISSUE-7"}),
            },
            Some(Duration::from_secs(10)),
        )
        .await
        .expect("the controlled tool answers over Streamable HTTP");
    assert!(!result.is_error);
    assert_eq!(result.raw["content"][0]["text"], "controlled issue is open");

    drop(connection);
    server.kill().expect("stop the controlled server");
    server.wait().expect("reap the controlled server");
}

#[tokio::test(flavor = "multi_thread")]
async fn cleartext_off_the_loopback_is_refused_before_a_socket_is_opened() {
    let config = HttpTransportConfig {
        url: "http://mcp.example.com/mcp".to_owned(),
        headers: std::collections::BTreeMap::new(),
    };
    let outcome = connect_http(
        ConnectionId::new("controlled_http").expect("a valid connection id"),
        &config,
        None,
        Limits::default(),
    )
    .await;
    let Err(error) = outcome else {
        panic!("Streamable HTTP requires HTTPS off the loopback");
    };
    assert!(
        error.to_string().contains("HTTPS"),
        "the refusal says why: {error}"
    );
}
