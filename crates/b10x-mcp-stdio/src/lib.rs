#![forbid(unsafe_code)]
//! Stdio entry point for the b10x MCP client.

pub use b10x_mcp_client::{Connection, connect_stdio};
pub use b10x_mcp_types::StdioTransportConfig;
