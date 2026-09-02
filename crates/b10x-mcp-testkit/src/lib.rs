#![forbid(unsafe_code)]
//! Synthetic, credential-free values for consumer conformance tests.

use b10x_mcp_types::{ClientError, ConnectionId, Limits, ToolDescriptor, ToolSnapshot};
use serde_json::json;

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
