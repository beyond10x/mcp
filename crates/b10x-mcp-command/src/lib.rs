#![forbid(unsafe_code)]
//! Shared command implementation for standalone and consumer-owned MCP UX.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use b10x_mcp_client::{Connection, connect_http, connect_stdio};
use b10x_mcp_config::{ConnectionConfig, HttpAuth, LocalPaths, Registry};
use b10x_mcp_oauth::{HttpExchange, OAuthEngine};
use b10x_mcp_types::{
    ClientError, ConnectionId, HttpRequest, HttpResponse, Limits, SecretString, ToolCall,
};
use clap::{Args, Parser, Subcommand};
use reqwest::redirect::Policy;
use serde_json::Value;

/// Standalone command-line surface.
#[derive(Debug, Parser)]
#[command(
    name = "b10x-mcp",
    version,
    about = "Inspect and call named MCP connections"
)]
pub struct Cli {
    /// Override the shared local registry path.
    #[arg(long, global = true, value_name = "FILE")]
    pub registry: Option<PathBuf>,
    /// Command to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// MCP registry and client operations.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage the local registry.
    Config(ConfigArgs),
    /// Inspect named connections.
    Connections(ConnectionsArgs),
    /// Manage OAuth authorization.
    Auth(AuthArgs),
    /// Discover and call tools.
    Tools(ToolsArgs),
}

/// Registry operations.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Registry operation.
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Registry operations.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Create an empty registry if none exists.
    Init,
    /// Strictly parse and validate the registry.
    Check,
}

/// Connection inspection.
#[derive(Debug, Args)]
pub struct ConnectionsArgs {
    /// Connection operation.
    #[command(subcommand)]
    pub command: ConnectionsCommand,
}

/// Connection operations.
#[derive(Debug, Subcommand)]
pub enum ConnectionsCommand {
    /// List local connection names without contacting servers.
    List,
    /// Show one non-secret connection declaration.
    Show(ConnectionName),
    /// Connect, negotiate, and freeze the tool list.
    Check(ConnectionName),
}

/// OAuth operations.
#[derive(Debug, Args)]
pub struct AuthArgs {
    /// Authorization operation.
    #[command(subcommand)]
    pub command: AuthCommand,
}

/// OAuth operations.
#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Print an authorization URL and wait on the declared loopback callback.
    Login(ConnectionName),
    /// Report whether authorization credentials exist, without printing them.
    Status(ConnectionName),
    /// Delete authorization credentials for one connection.
    Logout(ConnectionName),
}

/// Tool operations.
#[derive(Debug, Args)]
pub struct ToolsArgs {
    /// Tool operation.
    #[command(subcommand)]
    pub command: ToolsCommand,
}

/// Tool operations.
#[derive(Debug, Subcommand)]
pub enum ToolsCommand {
    /// List the frozen discovery result.
    List(ConnectionName),
    /// Print the complete frozen descriptor snapshot as JSON.
    Snapshot(ConnectionName),
    /// Call one tool with a JSON object.
    Call {
        /// Named connection.
        connection: String,
        /// Original MCP tool name.
        tool: String,
        /// JSON object passed to tools/call.
        #[arg(long, default_value = "{}")]
        arguments: String,
    },
}

/// One validated connection argument.
#[derive(Debug, Args)]
pub struct ConnectionName {
    /// Named registry entry.
    pub connection: String,
}

/// Parse process arguments and execute the selected operation.
pub async fn main_entry() -> Result<(), ClientError> {
    run(Cli::parse()).await
}

/// Execute a parsed command. Consumers may embed this while retaining their own top-level CLI.
pub async fn run(cli: Cli) -> Result<(), ClientError> {
    let paths = LocalPaths::discover()?;
    let registry_path = cli.registry.unwrap_or_else(|| paths.registry.clone());
    match cli.command {
        Command::Config(arguments) => match arguments.command {
            ConfigCommand::Init => init_registry(&registry_path).await,
            ConfigCommand::Check => {
                let registry = Registry::load(&registry_path).await?;
                println!("ok {} {}", registry.connections.len(), registry.sha256()?);
                Ok(())
            }
        },
        command => {
            let registry = Registry::load(&registry_path).await?;
            run_loaded(command, &registry, &paths).await
        }
    }
}

async fn run_loaded(
    command: Command,
    registry: &Registry,
    paths: &LocalPaths,
) -> Result<(), ClientError> {
    match command {
        Command::Config(_) => unreachable!("config commands are handled before registry loading"),
        Command::Connections(arguments) => match arguments.command {
            ConnectionsCommand::List => {
                for name in registry.connections.keys() {
                    println!("{name}");
                }
                Ok(())
            }
            ConnectionsCommand::Show(name) => {
                let id = connection_id(name)?;
                let config = registry.connection(&id)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(config)
                        .map_err(|error| ClientError::Protocol(error.to_string()))?
                );
                Ok(())
            }
            ConnectionsCommand::Check(name) => {
                let id = connection_id(name)?;
                let connection = connect_named(registry, paths, id, Limits::default()).await?;
                println!(
                    "ok {} {} {}",
                    connection.snapshot().protocol_version,
                    connection.snapshot().tools.len(),
                    connection.snapshot().sha256
                );
                Ok(())
            }
        },
        Command::Auth(arguments) => run_auth(arguments.command, registry, paths).await,
        Command::Tools(arguments) => run_tools(arguments.command, registry, paths).await,
    }
}

/// Connect one registry entry and freeze its tools for this connection lifetime.
pub async fn connect_named(
    registry: &Registry,
    paths: &LocalPaths,
    id: ConnectionId,
    limits: Limits,
) -> Result<Connection, ClientError> {
    match registry.connection(&id)? {
        ConnectionConfig::Stdio { transport_config } => {
            connect_stdio(id, transport_config, limits).await
        }
        ConnectionConfig::Http {
            transport_config,
            auth,
        } => {
            let bearer = resolve_http_bearer(auth, paths, &id).await?;
            connect_http(id, transport_config, bearer.as_ref(), limits).await
        }
    }
}

async fn resolve_http_bearer(
    auth: &HttpAuth,
    paths: &LocalPaths,
    id: &ConnectionId,
) -> Result<Option<SecretString>, ClientError> {
    match auth {
        HttpAuth::None => Ok(None),
        HttpAuth::Bearer { source } => source.resolve().await.map(Some),
        HttpAuth::OAuth { config, .. } => oauth_engine(config, paths, id)?
            .access_token()
            .await
            .map(Some)
            .map_err(|error| rebind_authorization(error, id)),
    }
}

async fn run_auth(
    command: AuthCommand,
    registry: &Registry,
    paths: &LocalPaths,
) -> Result<(), ClientError> {
    let name = match &command {
        AuthCommand::Login(name) | AuthCommand::Status(name) | AuthCommand::Logout(name) => name,
    };
    let id = connection_id_ref(name)?;
    let config = registry.connection(&id)?;
    let ConnectionConfig::Http { auth, .. } = config else {
        return Err(ClientError::Configuration(format!(
            "connection `{id}` uses stdio and has no HTTP OAuth flow"
        )));
    };
    let HttpAuth::OAuth {
        config,
        client_secret,
    } = auth
    else {
        return Err(ClientError::Configuration(format!(
            "connection `{id}` is not configured for OAuth"
        )));
    };
    let engine = oauth_engine(config, paths, &id)?;
    match command {
        AuthCommand::Status(_) => match engine.access_token().await {
            Ok(_) => println!("authorized {id}"),
            Err(ClientError::AuthorizationRequired { .. }) => {
                println!("authorization-required {id}");
            }
            Err(error) => return Err(rebind_authorization(error, &id)),
        },
        AuthCommand::Logout(_) => {
            engine.clear().await?;
            println!("logged-out {id}");
        }
        AuthCommand::Login(_) => {
            let secret = match client_secret {
                Some(source) => Some(source.resolve().await?),
                None => None,
            };
            let pending = engine.begin(secret.as_ref(), None).await?;
            println!("open {}", pending.authorization_url());
            let callback = wait_loopback_callback(&config.redirect_uri).await?;
            let _token = pending.finish(&callback).await?;
            println!("authorized {id}");
        }
    }
    Ok(())
}

async fn run_tools(
    command: ToolsCommand,
    registry: &Registry,
    paths: &LocalPaths,
) -> Result<(), ClientError> {
    match command {
        ToolsCommand::List(name) => {
            let connection =
                connect_named(registry, paths, connection_id(name)?, Limits::default()).await?;
            for tool in &connection.snapshot().tools {
                println!("{}", tool.name);
            }
        }
        ToolsCommand::Snapshot(name) => {
            let connection =
                connect_named(registry, paths, connection_id(name)?, Limits::default()).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(connection.snapshot())
                    .map_err(|error| ClientError::Protocol(error.to_string()))?
            );
        }
        ToolsCommand::Call {
            connection,
            tool,
            arguments,
        } => {
            let arguments: Value = serde_json::from_str(&arguments).map_err(|error| {
                ClientError::Configuration(format!("invalid arguments: {error}"))
            })?;
            let connection = connect_named(
                registry,
                paths,
                ConnectionId::new(connection)?,
                Limits::default(),
            )
            .await?;
            let result = connection
                .call(
                    &ToolCall {
                        name: tool,
                        arguments,
                    },
                    None,
                )
                .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result.raw)
                    .map_err(|error| ClientError::Protocol(error.to_string()))?
            );
        }
    }
    Ok(())
}

fn oauth_engine(
    config: &b10x_mcp_oauth::OAuthConfig,
    paths: &LocalPaths,
    id: &ConnectionId,
) -> Result<OAuthEngine, ClientError> {
    OAuthEngine::new(
        config.clone(),
        Arc::new(ReqwestExchange::new()?),
        Arc::new(paths.credentials(id)),
        Arc::new(paths.authorization_states(id)),
    )
}

struct ReqwestExchange {
    following: reqwest::Client,
    strict: reqwest::Client,
}

impl ReqwestExchange {
    fn new() -> Result<Self, ClientError> {
        let build = |policy| {
            reqwest::Client::builder()
                .redirect(policy)
                .build()
                .map_err(|error| ClientError::Transport(format!("building HTTP client: {error}")))
        };
        Ok(Self {
            following: build(Policy::limited(5))?,
            strict: build(Policy::none())?,
        })
    }
}

#[async_trait::async_trait]
impl HttpExchange for ReqwestExchange {
    async fn execute(
        &self,
        request: HttpRequest,
        follow_redirects: bool,
    ) -> Result<HttpResponse, ClientError> {
        let client = if follow_redirects {
            &self.following
        } else {
            &self.strict
        };
        let method = request
            .method
            .parse()
            .map_err(|_| ClientError::Protocol("invalid OAuth HTTP method".to_owned()))?;
        let mut builder = client.request(method, &request.url);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        let response = builder
            .body(request.body)
            .send()
            .await
            .map_err(|_| ClientError::Transport("OAuth HTTP request failed".to_owned()))?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_owned(), value.to_owned()))
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|_| ClientError::Transport("OAuth HTTP response failed".to_owned()))?;
        if body.len() > Limits::default().max_frame_bytes {
            return Err(ClientError::Bound {
                name: "max_frame_bytes",
                actual: body.len(),
                limit: Limits::default().max_frame_bytes,
            });
        }
        Ok(HttpResponse {
            status,
            headers,
            body: body.to_vec(),
        })
    }
}

async fn init_registry(path: &Path) -> Result<(), ClientError> {
    if !path.is_absolute() {
        return Err(ClientError::Configuration(
            "registry path must be absolute".to_owned(),
        ));
    }
    if tokio::fs::try_exists(path)
        .await
        .map_err(|error| ClientError::Configuration(format!("checking registry: {error}")))?
    {
        return Err(ClientError::Configuration(
            "registry already exists".to_owned(),
        ));
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            ClientError::Configuration(format!("creating registry directory: {error}"))
        })?;
    }
    tokio::fs::write(
        path,
        "# Named MCP connections. Credential values are not allowed here.\n[connections]\n",
    )
    .await
    .map_err(|error| ClientError::Configuration(format!("writing registry: {error}")))?;
    println!("created {}", path.display());
    Ok(())
}

async fn wait_loopback_callback(redirect_uri: &str) -> Result<String, ClientError> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let url = url::Url::parse(redirect_uri)
        .map_err(|error| ClientError::Configuration(format!("invalid OAuth redirect: {error}")))?;
    if url.scheme() != "http" {
        return Err(ClientError::Configuration(
            "standalone login requires an HTTP loopback redirect".to_owned(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| ClientError::Configuration("OAuth redirect has no host".to_owned()))?;
    let address: std::net::IpAddr = host.parse().map_err(|_| {
        ClientError::Configuration("OAuth redirect host must be a loopback IP".to_owned())
    })?;
    if !address.is_loopback() {
        return Err(ClientError::Configuration(
            "OAuth redirect host must be loopback".to_owned(),
        ));
    }
    let port = url.port().ok_or_else(|| {
        ClientError::Configuration("OAuth redirect requires an explicit port".to_owned())
    })?;
    let listener = tokio::net::TcpListener::bind((address, port))
        .await
        .map_err(|error| ClientError::Transport(format!("binding OAuth callback: {error}")))?;
    let (mut stream, _) =
        tokio::time::timeout(Limits::default().request_timeout * 10, listener.accept())
            .await
            .map_err(|_| ClientError::Transport("OAuth callback timed out".to_owned()))?
            .map_err(|error| {
                ClientError::Transport(format!("accepting OAuth callback: {error}"))
            })?;
    let mut bytes = vec![0_u8; 16 * 1024];
    let count = stream
        .read(&mut bytes)
        .await
        .map_err(|error| ClientError::Transport(format!("reading OAuth callback: {error}")))?;
    let request = std::str::from_utf8(&bytes[..count])
        .map_err(|_| ClientError::Protocol("OAuth callback is not UTF-8".to_owned()))?;
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| ClientError::Protocol("OAuth callback request is malformed".to_owned()))?;
    let callback = format!("{}://{}:{}{target}", url.scheme(), host, port);
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 32\r\nConnection: close\r\n\r\nAuthorization complete. Close tab.")
        .await
        .map_err(|error| ClientError::Transport(format!("writing OAuth callback: {error}")))?;
    Ok(callback)
}

fn connection_id(name: ConnectionName) -> Result<ConnectionId, ClientError> {
    ConnectionId::new(name.connection)
}

fn connection_id_ref(name: &ConnectionName) -> Result<ConnectionId, ClientError> {
    ConnectionId::new(name.connection.clone())
}

fn rebind_authorization(error: ClientError, id: &ConnectionId) -> ClientError {
    match error {
        ClientError::AuthorizationRequired { scope, .. } => ClientError::AuthorizationRequired {
            connection: id.clone(),
            scope,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    #[test]
    fn command_surface_is_well_formed() {
        Cli::command().debug_assert();
    }
}
