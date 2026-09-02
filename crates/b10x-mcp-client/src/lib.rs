#![forbid(unsafe_code)]
//! Tools-only MCP client over the standard transports.

use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use b10x_mcp_types::{
    ClientError, ConnectionId, HttpTransportConfig, LEGACY_PROTOCOL_VERSION, Limits, SecretString,
    StdioTransportConfig, ToolCall, ToolDescriptor, ToolResult, ToolSnapshot,
};
use http::{HeaderName, HeaderValue};
use rmcp::RoleClient;
use rmcp::model::{CallToolRequestParams, CallToolResponse, ClientInfo, ProtocolVersion};
use rmcp::service::{ClientLifecycleMode, ClientServiceExt, RunningService};
use rmcp::transport::{
    StreamableHttpClientTransport, TokioChildProcess,
    streamable_http_client::StreamableHttpClientTransportConfig,
};

/// A connected MCP client with one frozen tool snapshot.
pub struct Connection {
    id: ConnectionId,
    limits: Limits,
    service: RunningService<RoleClient, ClientInfo>,
    snapshot: ToolSnapshot,
}

impl Connection {
    /// Client-owned identity for this connection.
    pub fn id(&self) -> &ConnectionId {
        &self.id
    }

    /// Immutable tool list discovered during connection preparation.
    pub fn snapshot(&self) -> &ToolSnapshot {
        &self.snapshot
    }

    /// Call one tool admitted by the frozen snapshot.
    pub async fn call(
        &self,
        call: &ToolCall,
        deadline: Option<Duration>,
    ) -> Result<ToolResult, ClientError> {
        call.validate(self.limits)?;
        if self.snapshot.tool(&call.name).is_none() {
            return Err(ClientError::Protocol(format!(
                "tool `{}` is absent from connection `{}`'s frozen snapshot",
                call.name, self.id
            )));
        }
        let arguments =
            call.arguments.as_object().cloned().ok_or_else(|| {
                ClientError::Protocol("tool arguments are not an object".to_owned())
            })?;
        let request = CallToolRequestParams::new(call.name.clone()).with_arguments(arguments);
        let duration = deadline.map_or(self.limits.request_timeout, |remaining| {
            remaining.min(self.limits.request_timeout)
        });
        let response = tokio::time::timeout(duration, self.service.call_tool_once(request))
            .await
            .map_err(|_| ClientError::Transport(format!("tool `{}` timed out", call.name)))?
            .map_err(|error| map_service_error(&error, &self.id))?;
        match response {
            CallToolResponse::Complete(result) => {
                let raw = serde_json::to_value(result)
                    .map_err(|error| ClientError::Protocol(error.to_string()))?;
                ToolResult::from_raw(raw, self.limits)
            }
            CallToolResponse::InputRequired(result) => {
                let _preserved = serde_json::to_value(result)
                    .map_err(|error| ClientError::Protocol(error.to_string()))?;
                Err(ClientError::UnsupportedFeature {
                    feature: "input-required",
                })
            }
            CallToolResponse::Task(result) => {
                let _preserved = serde_json::to_value(result)
                    .map_err(|error| ClientError::Protocol(error.to_string()))?;
                Err(ClientError::UnsupportedFeature { feature: "tasks" })
            }
            _ => Err(ClientError::UnsupportedFeature {
                feature: "unknown-tools-call-response",
            }),
        }
    }

    /// Close the transport and bound cleanup.
    pub async fn close(&mut self) -> Result<(), ClientError> {
        self.service
            .close_with_timeout(Duration::from_secs(3))
            .await
            .map_err(|_| ClientError::Transport("client cleanup task failed".to_owned()))?
            .ok_or_else(|| ClientError::Transport("client cleanup timed out".to_owned()))?;
        Ok(())
    }
}

/// Connect to a Streamable HTTP endpoint and freeze its tool list.
pub async fn connect_http(
    id: ConnectionId,
    config: &HttpTransportConfig,
    bearer: Option<&SecretString>,
    limits: Limits,
) -> Result<Connection, ClientError> {
    validate_http_url(&config.url)?;
    let headers = parse_headers(&config.headers)?;
    let transport_config = StreamableHttpClientTransportConfig::with_uri(config.url.clone())
        .custom_headers(headers)
        .max_sse_event_size(limits.max_frame_bytes)
        .max_concurrent_requests(16);
    let transport_config = if let Some(bearer) = bearer {
        transport_config.auth_header(bearer.expose().to_owned())
    } else {
        transport_config
    };
    let transport = StreamableHttpClientTransport::from_config(transport_config);
    connect(id, transport, limits).await
}

/// Connect through a caller-supplied Streamable HTTP client and freeze its tool list.
///
/// This is the embedding boundary for a host that owns network admission, DNS policy,
/// credential custody, or observability. The supplied client receives every HTTP exchange; this
/// crate still owns MCP lifecycle negotiation, the frozen tool snapshot, named bounds, and tool
/// calls. `config.headers` remains value-only metadata and cannot contain `authorization`.
pub async fn connect_http_with_client<C>(
    id: ConnectionId,
    config: &HttpTransportConfig,
    bearer: Option<&SecretString>,
    limits: Limits,
    client: C,
) -> Result<Connection, ClientError>
where
    C: rmcp::transport::streamable_http_client::StreamableHttpClient,
{
    validate_http_url(&config.url)?;
    let headers = parse_headers(&config.headers)?;
    let transport_config = StreamableHttpClientTransportConfig::with_uri(config.url.clone())
        .custom_headers(headers)
        .max_sse_event_size(limits.max_frame_bytes)
        .max_concurrent_requests(16);
    let transport_config = if let Some(bearer) = bearer {
        transport_config.auth_header(bearer.expose().to_owned())
    } else {
        transport_config
    };
    let transport = StreamableHttpClientTransport::with_client(client, transport_config);
    connect(id, transport, limits).await
}

/// Start an exact stdio child and freeze its tool list.
pub async fn connect_stdio(
    id: ConnectionId,
    config: &StdioTransportConfig,
    limits: Limits,
) -> Result<Connection, ClientError> {
    let program = std::path::Path::new(&config.program);
    if !program.is_absolute() {
        return Err(ClientError::Configuration(
            "stdio program must be an absolute path".to_owned(),
        ));
    }
    let cwd = std::path::Path::new(&config.cwd);
    if !cwd.is_absolute() {
        return Err(ClientError::Configuration(
            "stdio cwd must be an absolute path".to_owned(),
        ));
    }
    let mut command = tokio::process::Command::new(program);
    command.args(&config.args).current_dir(cwd).env_clear();
    for name in &config.inherit_env {
        validate_env_name(name)?;
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command.kill_on_drop(true);
    let (transport, _stderr) = TokioChildProcess::builder(command)
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| ClientError::Transport(format!("starting stdio server: {error}")))?;
    connect(id, transport, limits).await
}

async fn connect<T, E, A>(
    id: ConnectionId,
    transport: T,
    limits: Limits,
) -> Result<Connection, ClientError>
where
    T: rmcp::transport::IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let lifecycle = ClientLifecycleMode::Auto {
        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
        legacy_version: Some(ProtocolVersion::V_2025_11_25),
    };
    let service = tokio::time::timeout(
        limits.request_timeout,
        ClientInfo::default().serve_with_lifecycle(transport, lifecycle),
    )
    .await
    .map_err(|_| ClientError::Transport("MCP lifecycle negotiation timed out".to_owned()))?
    .map_err(|error| {
        if error.is_authorization_required() {
            ClientError::AuthorizationRequired {
                connection: id.clone(),
                scope: challenge_scope(error.auth_challenge()),
            }
        } else {
            ClientError::Transport("MCP lifecycle negotiation failed".to_owned())
        }
    })?;
    prepare(id, service, limits).await
}

async fn prepare(
    id: ConnectionId,
    service: RunningService<RoleClient, ClientInfo>,
    limits: Limits,
) -> Result<Connection, ClientError> {
    let protocol_version = service.peer_info().map_or_else(
        || LEGACY_PROTOCOL_VERSION.to_owned(),
        |info| info.protocol_version.to_string(),
    );
    let listed = tokio::time::timeout(limits.request_timeout, service.list_all_tools())
        .await
        .map_err(|_| ClientError::Transport("tools/list timed out".to_owned()))?
        .map_err(|error| map_service_error(&error, &id))?;
    if listed.len() > limits.max_tools {
        return Err(ClientError::Bound {
            name: "max_tools",
            actual: listed.len(),
            limit: limits.max_tools,
        });
    }
    let tools = listed
        .into_iter()
        .map(|tool| {
            serde_json::to_value(tool)
                .map_err(|error| ClientError::Protocol(error.to_string()))
                .and_then(|raw| ToolDescriptor::from_raw(raw, limits))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = ToolSnapshot::new(id.clone(), protocol_version, tools, limits)?;
    Ok(Connection {
        id,
        limits,
        service,
        snapshot,
    })
}

fn parse_headers(
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<HashMap<HeaderName, HeaderValue>, ClientError> {
    headers
        .iter()
        .map(|(name, value)| {
            if name.eq_ignore_ascii_case("authorization") {
                return Err(ClientError::Configuration(
                    "authorization is a credential and cannot appear in connection headers"
                        .to_owned(),
                ));
            }
            let name = HeaderName::try_from(name.as_str())
                .map_err(|_| ClientError::Configuration(format!("invalid HTTP header `{name}`")))?;
            let value = HeaderValue::try_from(value.as_str()).map_err(|_| {
                ClientError::Configuration(format!("invalid value for HTTP header `{name}`"))
            })?;
            Ok((name, value))
        })
        .collect()
}

fn validate_http_url(value: &str) -> Result<(), ClientError> {
    let url = reqwest::Url::parse(value)
        .map_err(|error| ClientError::Configuration(format!("invalid MCP URL: {error}")))?;
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err(ClientError::Configuration(
            "Streamable HTTP requires HTTPS except on loopback".to_owned(),
        ));
    }
    if url.username().is_empty() && url.password().is_none() && url.fragment().is_none() {
        Ok(())
    } else {
        Err(ClientError::Configuration(
            "MCP URL cannot contain userinfo or a fragment".to_owned(),
        ))
    }
}

fn validate_env_name(name: &str) -> Result<(), ClientError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(ClientError::Configuration(format!(
            "invalid inherited environment name `{name}`"
        )))
    }
}

fn map_service_error(error: &rmcp::service::ServiceError, id: &ConnectionId) -> ClientError {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if let Some(scope) = current
            .downcast_ref::<rmcp::transport::streamable_http_client::InsufficientScopeError>()
            .and_then(|failure| failure.required_scope.clone())
        {
            return ClientError::AuthorizationRequired {
                connection: id.clone(),
                scope: Some(scope),
            };
        }
        if current
            .downcast_ref::<rmcp::transport::streamable_http_client::AuthRequiredError>()
            .is_some()
        {
            return ClientError::AuthorizationRequired {
                connection: id.clone(),
                scope: None,
            };
        }
        source = current.source();
    }
    // SDK errors may carry server prose. Keep the class and never debug-format a request or token.
    let message = match error {
        rmcp::service::ServiceError::Cancelled { .. } => "request cancelled",
        rmcp::service::ServiceError::Timeout { .. } => "request timed out",
        rmcp::service::ServiceError::TransportClosed => "transport closed",
        _ => "MCP request failed",
    };
    ClientError::Transport(message.to_owned())
}

fn challenge_scope(challenge: Option<&str>) -> Option<String> {
    challenge.and_then(|value| {
        value.split(',').find_map(|part| {
            let (name, raw) = part.trim().split_once('=')?;
            (name.trim().eq_ignore_ascii_case("scope"))
                .then(|| raw.trim().trim_matches('"').to_owned())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_cleartext_is_refused_but_loopback_is_allowed() {
        assert!(validate_http_url("http://127.0.0.1:8080/mcp").is_ok());
        assert!(validate_http_url("http://example.com/mcp").is_err());
        assert!(validate_http_url("https://example.com/mcp").is_ok());
    }

    #[test]
    fn authorization_cannot_hide_in_static_headers() {
        let headers = std::collections::BTreeMap::from([(
            "Authorization".to_owned(),
            "SENTINEL-NOT-A-REAL-TOKEN".to_owned(),
        )]);
        assert!(parse_headers(&headers).is_err());
    }
}
