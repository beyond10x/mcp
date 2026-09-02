#![forbid(unsafe_code)]
//! Streamable HTTP entry point for the b10x MCP client.

pub use b10x_mcp_client::{Connection, connect_http};
pub use b10x_mcp_types::HttpTransportConfig;
