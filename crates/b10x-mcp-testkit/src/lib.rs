#![forbid(unsafe_code)]
//! Synthetic, credential-free values and a controlled server for consumer conformance tests.

use std::path::{Path, PathBuf};
use std::process::Command;

use b10x_mcp_types::{ClientError, ConnectionId, Limits, ToolDescriptor, ToolSnapshot};
use serde_json::json;

/// The source of the controlled MCP server, which answers `initialize`, `tools/list` and
/// `tools/call` over stdio or Streamable HTTP with the same bytes on both.
///
/// It is a source file rather than a shipped binary so a caller compiles it with its own
/// toolchain, and it uses only the standard library so a transport is never proven against a
/// server that shares its framing code.
#[must_use]
pub fn controlled_server_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/mcp_server.rs")
}

/// Compile [`controlled_server_source`] into `directory` and answer the executable's path.
///
/// # Errors
///
/// When `rustc` cannot be started, or refuses the fixture.
pub fn build_controlled_server(directory: &Path) -> Result<PathBuf, String> {
    let binary = directory.join("b10x-mcp-controlled-server");
    let output = Command::new("rustc")
        .args(["--edition=2024", "-C", "debuginfo=0", "-o"])
        .arg(&binary)
        .arg(controlled_server_source())
        .output()
        .map_err(|error| format!("starting rustc: {error}"))?;
    if output.status.success() {
        Ok(binary)
    } else {
        Err(format!(
            "the controlled server did not compile: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Build a deterministic two-tool snapshot with deliberately untrusted annotations.
pub fn synthetic_snapshot(connection: &str) -> Result<ToolSnapshot, ClientError> {
    let limits = Limits::default();
    let read = ToolDescriptor::from_raw(
        json!({
            "name": "read_issue",
            "description": "Read a synthetic issue",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}},
            "annotations": {"readOnlyHint": false}
        }),
        limits,
    )?;
    let write = ToolDescriptor::from_raw(
        json!({
            "name": "close_issue",
            "description": "Close a synthetic issue",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}},
            "annotations": {"readOnlyHint": true}
        }),
        limits,
    )?;
    ToolSnapshot::new(
        ConnectionId::new(connection)?,
        b10x_mcp_types::CURRENT_PROTOCOL_VERSION,
        vec![read, write],
        limits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_are_retained_but_not_interpreted() {
        let snapshot = synthetic_snapshot("synthetic").unwrap();
        assert_eq!(snapshot.tools.len(), 2);
        assert_eq!(snapshot.tools[0].raw["annotations"]["readOnlyHint"], false);
    }
}
