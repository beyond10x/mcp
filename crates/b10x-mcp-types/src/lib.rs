#![forbid(unsafe_code)]
//! I/O-free public values for the b10x MCP client.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// MCP revision preferred by this release.
pub const CURRENT_PROTOCOL_VERSION: &str = "2026-07-28";
/// Legacy revision used when a server does not implement discovery.
pub const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";

/// Resource limits enforced before data crosses a consumer boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Largest newline-delimited or SSE message.
    pub max_frame_bytes: usize,
    /// Largest number of tools accepted from one connection.
    pub max_tools: usize,
    /// Largest serialized tool descriptor.
    pub max_tool_descriptor_bytes: usize,
    /// Largest serialized call argument object.
    pub max_arguments_bytes: usize,
    /// Largest serialized call result.
    pub max_result_bytes: usize,
    /// Maximum number of list pages accepted.
    pub max_pages: usize,
    /// Time allowed for discovery and individual requests absent a tighter consumer deadline.
    pub request_timeout: Duration,
}

/// Streamable HTTP connection facts. Authentication material is supplied separately at call time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HttpTransportConfig {
    /// Absolute MCP endpoint.
    pub url: String,
    /// Additional non-credential headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

/// Exact stdio process declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct StdioTransportConfig {
    /// Absolute executable path. No PATH lookup and no shell expansion occur.
    pub program: String,
    /// Exact argument vector.
    #[serde(default)]
    pub args: Vec<String>,
    /// Explicit working directory.
    pub cwd: String,
    /// Environment variable names copied from the parent when present.
    #[serde(default)]
    pub inherit_env: Vec<String>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 1024 * 1024,
            max_tools: 512,
            max_tool_descriptor_bytes: 64 * 1024,
            max_arguments_bytes: 64 * 1024,
            max_result_bytes: 256 * 1024,
            max_pages: 128,
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// A client-owned stable identifier used to disambiguate aggregated servers.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConnectionId(String);

impl ConnectionId {
    /// Validate and construct an identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid {
            return Err(ClientError::Configuration(
                "connection id must match [a-z0-9_]{1,64}".to_owned(),
            ));
        }
        Ok(Self(value))
    }

    /// Borrow the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ConnectionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ConnectionId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for ConnectionId {
    type Error = ClientError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ConnectionId> for String {
    fn from(value: ConnectionId) -> Self {
        value.0
    }
}

/// Secret text whose formatting never reveals its value.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a secret value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Expose the value only at the transport boundary.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

/// A validated tool descriptor and the complete JSON value received for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDescriptor {
    /// Original MCP tool name.
    pub name: String,
    /// Server-provided prose; never authority.
    pub description: Option<String>,
    /// JSON Schema object for arguments.
    pub input_schema: Value,
    /// Optional JSON Schema object for results.
    pub output_schema: Option<Value>,
    /// Exact descriptor known by the selected codec, including annotations and metadata.
    pub raw: Value,
}

impl ToolDescriptor {
    /// Construct from a raw descriptor while validating fields needed for invocation.
    pub fn from_raw(raw: Value, limits: Limits) -> Result<Self, ClientError> {
        ensure_bound(
            "max_tool_descriptor_bytes",
            serialized_len(&raw)?,
            limits.max_tool_descriptor_bytes,
        )?;
        let object = raw.as_object().ok_or_else(|| {
            ClientError::Protocol("tool descriptor is not a JSON object".to_owned())
        })?;
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| ClientError::Protocol("tool descriptor has no name".to_owned()))?
            .to_owned();
        let input_schema = object
            .get("inputSchema")
            .or_else(|| object.get("input_schema"))
            .cloned()
            .ok_or_else(|| ClientError::Protocol(format!("tool `{name}` has no input schema")))?;
        if !input_schema.is_object() {
            return Err(ClientError::Protocol(format!(
                "tool `{name}` input schema is not an object"
            )));
        }
        Ok(Self {
            name,
            description: object
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned),
            input_schema,
            output_schema: object
                .get("outputSchema")
                .or_else(|| object.get("output_schema"))
                .cloned(),
            raw,
        })
    }
}

/// One immutable tools view for one prepared connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSnapshot {
    /// Client-owned connection identity.
    pub connection: ConnectionId,
    /// Negotiated protocol revision.
    pub protocol_version: String,
    /// Tools in the deterministic order returned by the server.
    pub tools: Vec<ToolDescriptor>,
    /// SHA-256 over the other fields' canonical JSON representation.
    pub sha256: String,
}

impl ToolSnapshot {
    /// Validate a complete discovery result and compute its digest.
    pub fn new(
        connection: ConnectionId,
        protocol_version: impl Into<String>,
        tools: Vec<ToolDescriptor>,
        limits: Limits,
    ) -> Result<Self, ClientError> {
        ensure_bound("max_tools", tools.len(), limits.max_tools)?;
        let mut names = std::collections::BTreeSet::new();
        for tool in &tools {
            if !names.insert(tool.name.clone()) {
                return Err(ClientError::Protocol(format!(
                    "server listed duplicate tool `{}`",
                    tool.name
                )));
            }
        }
        let protocol_version = protocol_version.into();
        let digest_input = serde_json::to_vec(&(&connection, &protocol_version, &tools))
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        let sha256 = hex(Sha256::digest(digest_input));
        Ok(Self {
            connection,
            protocol_version,
            tools,
            sha256,
        })
    }

    /// Find an original MCP tool name in this frozen snapshot.
    pub fn tool(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.iter().find(|tool| tool.name == name)
    }
}

/// One tools/call request after snapshot admission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    /// Original MCP tool name.
    pub name: String,
    /// JSON object passed as arguments.
    pub arguments: Value,
}

impl ToolCall {
    /// Validate arguments and their serialized bound.
    pub fn validate(&self, limits: Limits) -> Result<(), ClientError> {
        if !self.arguments.is_object() {
            return Err(ClientError::Protocol(format!(
                "arguments for `{}` are not a JSON object",
                self.name
            )));
        }
        ensure_bound(
            "max_arguments_bytes",
            serialized_len(&self.arguments)?,
            limits.max_arguments_bytes,
        )
    }
}

/// Complete MCP result retained as JSON for lossless consumer projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolResult {
    /// Full decoded result.
    pub raw: Value,
    /// MCP's tool-level failure marker.
    pub is_error: bool,
}

impl ToolResult {
    /// Validate and construct a result.
    pub fn from_raw(raw: Value, limits: Limits) -> Result<Self, ClientError> {
        ensure_bound(
            "max_result_bytes",
            serialized_len(&raw)?,
            limits.max_result_bytes,
        )?;
        let is_error = raw
            .as_object()
            .and_then(|object| object.get("isError").or_else(|| object.get("is_error")))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok(Self { raw, is_error })
    }
}

/// HTTP request used by MCP and OAuth consumer adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// Uppercase HTTP method.
    pub method: String,
    /// Absolute URL.
    pub url: String,
    /// Header names and values. Credential-bearing values must be redacted by implementations.
    pub headers: BTreeMap<String, String>,
    /// Buffered request body.
    pub body: Vec<u8>,
}

/// Buffered bounded HTTP response used by OAuth and simple adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Response headers.
    pub headers: BTreeMap<String, String>,
    /// Complete bounded body.
    pub body: Vec<u8>,
}

/// Stable failure classes safe to record.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClientError {
    /// A strict configuration could not be applied.
    #[error("configuration: {0}")]
    Configuration(String),
    /// The server or transport violated the selected protocol.
    #[error("protocol: {0}")]
    Protocol(String),
    /// One named resource bound was exceeded.
    #[error("bound `{name}`: {actual} exceeds {limit}")]
    Bound {
        /// Bound identifier.
        name: &'static str,
        /// Observed size or count.
        actual: usize,
        /// Maximum admitted size or count.
        limit: usize,
    },
    /// The transport did not complete.
    #[error("transport: {0}")]
    Transport(String),
    /// Authorization is required or no longer sufficient.
    #[error("authorization required for `{connection}`{scope_suffix}", scope_suffix = scope.as_ref().map(|value| format!(" with scope `{value}`")).unwrap_or_default())]
    AuthorizationRequired {
        /// Connection needing authorization.
        connection: ConnectionId,
        /// Server-challenged scope string, if present.
        scope: Option<String>,
    },
    /// The server requested a feature deliberately outside v1.
    #[error("unsupported MCP feature `{feature}`")]
    UnsupportedFeature {
        /// Feature identifier.
        feature: &'static str,
    },
    /// Consumer or deadline cancellation.
    #[error("cancelled")]
    Cancelled,
}

/// Check one count or size against a named limit.
pub fn ensure_bound(name: &'static str, actual: usize, limit: usize) -> Result<(), ClientError> {
    if actual > limit {
        Err(ClientError::Bound {
            name,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

/// Convert a map into a JSON object without changing entry values.
pub fn object(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
    Value::Object(Map::from_iter(entries))
}

fn serialized_len(value: &Value) -> Result<usize, ClientError> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|error| ClientError::Protocol(error.to_string()))
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .fold(String::new(), |mut output, byte| {
            use fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connection_ids_are_client_owned_and_provider_safe() {
        assert_eq!(ConnectionId::new("git_hub1").unwrap().as_str(), "git_hub1");
        assert!(ConnectionId::new("GitHub").is_err());
        assert!(ConnectionId::new("").is_err());
    }

    #[test]
    fn duplicate_tools_are_refused_by_name() {
        let raw = json!({"name":"same","inputSchema":{"type":"object"}});
        let tool = ToolDescriptor::from_raw(raw, Limits::default()).unwrap();
        let error = ToolSnapshot::new(
            ConnectionId::new("server").unwrap(),
            CURRENT_PROTOCOL_VERSION,
            vec![tool.clone(), tool],
            Limits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("duplicate tool `same`"));
    }

    #[test]
    fn result_bound_refuses_instead_of_truncating() {
        let limits = Limits {
            max_result_bytes: 8,
            ..Limits::default()
        };
        let error = ToolResult::from_raw(json!({"text":"too long"}), limits).unwrap_err();
        assert!(matches!(
            error,
            ClientError::Bound {
                name: "max_result_bytes",
                ..
            }
        ));
    }

    #[test]
    fn secrets_are_redacted_and_zeroized_by_type() {
        let secret = SecretString::new("SENTINEL-NOT-A-REAL-TOKEN");
        assert_eq!(format!("{secret:?}"), "SecretString([REDACTED])");
        assert_eq!(secret.expose(), "SENTINEL-NOT-A-REAL-TOKEN");
    }
}
