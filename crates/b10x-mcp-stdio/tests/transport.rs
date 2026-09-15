//! The stdio transport, driven against a controlled server over real pipes.
//!
//! Until this file existed the only exercise of this transport was a consumer's suite in another
//! repository, so this workspace's own gate said nothing about whether stdio worked. The server is
//! a standard-library program the testkit compiles here: a transport proven against something that
//! shares its framing code has proven nothing.

use std::time::Duration;

use b10x_mcp_stdio::{StdioTransportConfig, connect_stdio};
use b10x_mcp_types::{ConnectionId, Limits, ToolCall};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn a_controlled_stdio_server_is_negotiated_frozen_and_called() {
    let directory = tempfile::tempdir().expect("a fixture build directory");
    let server = b10x_mcp_testkit::build_controlled_server(directory.path())
        .expect("the controlled server compiles");
    let config = StdioTransportConfig {
        program: server.to_str().expect("a UTF-8 fixture path").to_owned(),
        args: vec!["stdio".to_owned()],
        cwd: directory
            .path()
            .to_str()
            .expect("a UTF-8 fixture directory")
            .to_owned(),
        inherit_env: Vec::new(),
    };
    let limits = Limits::default();

    let mut connection = connect_stdio(
        ConnectionId::new("controlled_stdio").expect("a valid connection id"),
        &config,
        limits,
    )
    .await
    .expect("the controlled stdio server is reachable");

    // Negotiation reached the version the server offered, not the one the client would rather have.
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
    // An annotation is retained exactly and interpreted by nobody here.
    assert_eq!(snapshot.tools[1].raw["annotations"]["readOnlyHint"], true);

    let result = connection
        .call(
            &ToolCall {
                name: "read_issue".to_owned(),
                arguments: json!({"id": "ISSUE-7"}),
            },
            Some(Duration::from_secs(10)),
        )
        .await
        .expect("the controlled tool answers over stdio");
    assert!(!result.is_error);
    assert_eq!(result.raw["content"][0]["text"], "controlled issue is open");

    // A name outside the frozen snapshot is refused here and never reaches the pipe.
    let refusal = connection
        .call(
            &ToolCall {
                name: "delete_everything".to_owned(),
                arguments: json!({}),
            },
            Some(Duration::from_secs(10)),
        )
        .await
        .expect_err("a tool absent from the frozen snapshot is refused");
    assert!(
        refusal.to_string().contains("delete_everything"),
        "the refusal names the tool: {refusal}"
    );

    connection.close().await.expect("the child is cleaned up");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relative_program_is_refused_before_anything_is_spawned() {
    let config = StdioTransportConfig {
        program: "mcp-server".to_owned(),
        args: Vec::new(),
        cwd: "/".to_owned(),
        inherit_env: Vec::new(),
    };
    let outcome = connect_stdio(
        ConnectionId::new("controlled_stdio").expect("a valid connection id"),
        &config,
        Limits::default(),
    )
    .await;
    let Err(error) = outcome else {
        panic!("a PATH lookup is not a transport this crate performs");
    };
    assert!(
        error.to_string().contains("absolute"),
        "the refusal says why: {error}"
    );
}
