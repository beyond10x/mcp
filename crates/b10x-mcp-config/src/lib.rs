#![forbid(unsafe_code)]
//! Strict named connection registry and local credential custody.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use b10x_mcp_oauth::{OAuthConfig, OAuthCredentialStore, OAuthStateStore};
use b10x_mcp_types::{
    ClientError, ConnectionId, HttpTransportConfig, SecretString, StdioTransportConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Complete local registry. Connection names are validated when loaded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// Locally named MCP endpoints.
    #[serde(default)]
    pub connections: BTreeMap<String, ConnectionConfig>,
}

/// One endpoint and its credential source, but never a credential value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ConnectionConfig {
    /// Streamable HTTP endpoint.
    Http {
        /// Transport facts.
        #[serde(flatten)]
        transport_config: HttpTransportConfig,
        /// HTTP authentication policy.
        #[serde(default)]
        auth: HttpAuth,
    },
    /// Exact stdio process declaration.
    Stdio {
        /// Transport facts.
        #[serde(flatten)]
        transport_config: StdioTransportConfig,
    },
}

impl ConnectionConfig {
    /// HTTP transport facts, when this is an HTTP connection.
    pub fn http(&self) -> Option<(&HttpTransportConfig, &HttpAuth)> {
        match self {
            Self::Http {
                transport_config,
                auth,
            } => Some((transport_config, auth)),
            Self::Stdio { .. } => None,
        }
    }

    /// Stdio transport facts, when this is a stdio connection.
    pub fn stdio(&self) -> Option<&StdioTransportConfig> {
        match self {
            Self::Stdio { transport_config } => Some(transport_config),
            Self::Http { .. } => None,
        }
    }
}

/// Authentication for a Streamable HTTP connection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum HttpAuth {
    /// No HTTP credential.
    #[default]
    None,
    /// Resolve a bearer token from an explicit source at connection time.
    Bearer {
        /// Token source.
        #[serde(flatten)]
        source: BearerSource,
    },
    /// OAuth authorization-code flow with PKCE.
    OAuth {
        /// Non-secret OAuth client configuration.
        #[serde(flatten)]
        config: OAuthConfig,
        /// Optional explicit source for a confidential-client secret.
        client_secret: Option<BearerSource>,
    },
}

/// Explicit source for a bearer-shaped secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BearerSource {
    /// Read one explicitly named process environment variable at use time.
    Environment {
        /// Environment variable name.
        environment: String,
    },
    /// Read one explicitly named JSON file and RFC 6901 pointer at use time.
    JsonFile {
        /// Absolute file path.
        path: PathBuf,
        /// JSON pointer selecting a string.
        pointer: String,
    },
}

impl BearerSource {
    /// Resolve the source without retaining the secret in configuration.
    pub async fn resolve(&self) -> Result<SecretString, ClientError> {
        match self {
            Self::Environment { environment } => {
                validate_environment_name(environment)?;
                let value = std::env::var(environment).map_err(|_| {
                    ClientError::Configuration(format!(
                        "credential environment `{environment}` is absent"
                    ))
                })?;
                if value.is_empty() {
                    return Err(ClientError::Configuration(format!(
                        "credential environment `{environment}` is empty"
                    )));
                }
                Ok(SecretString::new(value))
            }
            Self::JsonFile { path, pointer } => {
                require_absolute(path, "credential JSON file")?;
                let bytes = tokio::fs::read(path).await.map_err(|error| {
                    ClientError::Configuration(format!("reading credential JSON file: {error}"))
                })?;
                let value: Value = serde_json::from_slice(&bytes).map_err(|_| {
                    ClientError::Configuration("credential JSON file is malformed".to_owned())
                })?;
                let secret = value
                    .pointer(pointer)
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ClientError::Configuration(format!(
                            "credential JSON pointer `{pointer}` does not select a string"
                        ))
                    })?;
                if secret.is_empty() {
                    return Err(ClientError::Configuration(
                        "credential JSON value is empty".to_owned(),
                    ));
                }
                Ok(SecretString::new(secret))
            }
        }
    }
}

impl Registry {
    /// Parse and validate strict TOML.
    pub fn parse(document: &str) -> Result<Self, ClientError> {
        let registry: Self = toml::from_str(document)
            .map_err(|error| ClientError::Configuration(format!("invalid registry: {error}")))?;
        registry.validate()?;
        Ok(registry)
    }

    /// Load an explicit registry path.
    pub async fn load(path: &Path) -> Result<Self, ClientError> {
        require_absolute(path, "registry")?;
        let document = tokio::fs::read_to_string(path)
            .await
            .map_err(|error| ClientError::Configuration(format!("reading registry: {error}")))?;
        Self::parse(&document)
    }

    /// Validate names and paths without accessing endpoints or credential sources.
    pub fn validate(&self) -> Result<(), ClientError> {
        for (name, connection) in &self.connections {
            ConnectionId::new(name.clone())?;
            match connection {
                ConnectionConfig::Http { auth, .. } => match auth {
                    HttpAuth::None => {}
                    HttpAuth::Bearer { source } => validate_source(source)?,
                    HttpAuth::OAuth {
                        config,
                        client_secret,
                    } => {
                        if config.resource_url.is_empty() || config.redirect_uri.is_empty() {
                            return Err(ClientError::Configuration(format!(
                                "OAuth connection `{name}` requires resource-url and redirect-uri"
                            )));
                        }
                        if let Some(source) = client_secret {
                            validate_source(source)?;
                        }
                    }
                },
                ConnectionConfig::Stdio { transport_config } => {
                    require_absolute(Path::new(&transport_config.program), "stdio program")?;
                    require_absolute(Path::new(&transport_config.cwd), "stdio cwd")?;
                    for environment in &transport_config.inherit_env {
                        validate_environment_name(environment)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Select a connection by its client-owned name.
    pub fn connection(&self, id: &ConnectionId) -> Result<&ConnectionConfig, ClientError> {
        self.connections
            .get(id.as_str())
            .ok_or_else(|| ClientError::Configuration(format!("unknown MCP connection `{id}`")))
    }

    /// Stable digest used as run and review evidence.
    pub fn sha256(&self) -> Result<String, ClientError> {
        let canonical = serde_json::to_vec(self)
            .map_err(|error| ClientError::Configuration(error.to_string()))?;
        Ok(hex(Sha256::digest(canonical)))
    }
}

/// Conventional local paths shared by the standalone CLI and embedded consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPaths {
    /// Strict registry document.
    pub registry: PathBuf,
    /// Private state root containing credentials and authorization sessions.
    pub state: PathBuf,
}

impl LocalPaths {
    /// Resolve XDG paths. No directory or file is created.
    pub fn discover() -> Result<Self, ClientError> {
        let config_root = xdg_or_home("XDG_CONFIG_HOME", ".config")?;
        let state_root = xdg_or_home("XDG_STATE_HOME", ".local/state")?;
        Ok(Self {
            registry: config_root.join("b10x/mcp.toml"),
            state: state_root.join("b10x/mcp"),
        })
    }

    /// Credential store scoped to one validated connection name.
    pub fn credentials(&self, id: &ConnectionId) -> FileCredentialStore {
        FileCredentialStore {
            path: self.state.join("oauth").join(format!("{id}.json")),
        }
    }

    /// Authorization state store scoped to one validated connection name.
    pub fn authorization_states(&self, id: &ConnectionId) -> FileStateStore {
        FileStateStore {
            directory: self.state.join("authorization").join(id.as_str()),
        }
    }
}

/// Owner-only atomic OAuth credential file.
#[derive(Debug, Clone)]
pub struct FileCredentialStore {
    path: PathBuf,
}

#[async_trait]
impl OAuthCredentialStore for FileCredentialStore {
    async fn load(&self) -> Result<Option<SecretString>, ClientError> {
        match tokio::fs::read_to_string(&self.path).await {
            Ok(value) => Ok(Some(SecretString::new(value))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ClientError::Configuration(format!(
                "reading OAuth credentials: {error}"
            ))),
        }
    }

    async fn save(&self, document: SecretString) -> Result<(), ClientError> {
        atomic_private_write(&self.path, document.expose().as_bytes()).await
    }

    async fn clear(&self) -> Result<(), ClientError> {
        remove_if_present(&self.path).await
    }
}

/// Owner-only PKCE/state files, keyed by a digest of the untrusted state token.
#[derive(Debug, Clone)]
pub struct FileStateStore {
    directory: PathBuf,
}

#[async_trait]
impl OAuthStateStore for FileStateStore {
    async fn save(&self, key: &str, document: SecretString) -> Result<(), ClientError> {
        atomic_private_write(&self.key_path(key), document.expose().as_bytes()).await
    }

    async fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError> {
        match tokio::fs::read_to_string(self.key_path(key)).await {
            Ok(value) => Ok(Some(SecretString::new(value))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ClientError::Configuration(format!(
                "reading OAuth authorization state: {error}"
            ))),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), ClientError> {
        remove_if_present(&self.key_path(key)).await
    }
}

impl FileStateStore {
    fn key_path(&self, key: &str) -> PathBuf {
        self.directory.join(hex(Sha256::digest(key.as_bytes())))
    }
}

async fn atomic_private_write(path: &Path, bytes: &[u8]) -> Result<(), ClientError> {
    let parent = path
        .parent()
        .ok_or_else(|| ClientError::Configuration("private state path has no parent".to_owned()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| ClientError::Configuration(format!("creating private state: {error}")))?;
    set_owner_only(parent, true).await?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|error| ClientError::Configuration(format!("writing private state: {error}")))?;
    set_owner_only(&temporary, false).await?;
    tokio::fs::rename(&temporary, path)
        .await
        .map_err(|error| ClientError::Configuration(format!("committing private state: {error}")))
}

async fn set_owner_only(path: &Path, directory: bool) -> Result<(), ClientError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if directory { 0o700 } else { 0o600 };
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .await
            .map_err(|error| {
                ClientError::Configuration(format!("setting private state permissions: {error}"))
            })?;
    }
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

async fn remove_if_present(path: &Path) -> Result<(), ClientError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ClientError::Configuration(format!(
            "removing private state: {error}"
        ))),
    }
}

fn validate_source(source: &BearerSource) -> Result<(), ClientError> {
    match source {
        BearerSource::Environment { environment } => validate_environment_name(environment),
        BearerSource::JsonFile { path, pointer } => {
            require_absolute(path, "credential JSON file")?;
            if !pointer.is_empty() && !pointer.starts_with('/') {
                return Err(ClientError::Configuration(
                    "credential JSON pointer must be empty or start with `/`".to_owned(),
                ));
            }
            Ok(())
        }
    }
}

fn validate_environment_name(name: &str) -> Result<(), ClientError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(ClientError::Configuration(format!(
            "invalid environment name `{name}`"
        )))
    }
}

fn require_absolute(path: &Path, kind: &str) -> Result<(), ClientError> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(ClientError::Configuration(format!(
            "{kind} path must be absolute"
        )))
    }
}

fn xdg_or_home(variable: &str, fallback: &str) -> Result<PathBuf, ClientError> {
    if let Some(value) = std::env::var_os(variable) {
        let path = PathBuf::from(value);
        require_absolute(&path, variable)?;
        return Ok(path);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| {
        ClientError::Configuration(format!("{variable} and HOME are both absent"))
    })?;
    let home = PathBuf::from(home);
    require_absolute(&home, "HOME")?;
    Ok(home.join(fallback))
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_bearer_values_are_not_in_the_schema() {
        let error = Registry::parse(
            r#"
                [connections.remote]
                transport = "http"
                url = "https://example.test/mcp"

                [connections.remote.auth]
                kind = "bearer"
                token = "SENTINEL-NOT-A-REAL-TOKEN"
            "#,
        )
        .unwrap_err();
        assert!(!error.to_string().contains("SENTINEL"));
    }

    #[tokio::test]
    async fn private_store_round_trips_and_clears() {
        let temporary = tempfile::tempdir().unwrap();
        let store = FileCredentialStore {
            path: temporary.path().join("nested/token.json"),
        };
        store.save(SecretString::new("secret")).await.unwrap();
        assert_eq!(store.load().await.unwrap().unwrap().expose(), "secret");
        store.clear().await.unwrap();
        assert!(store.load().await.unwrap().is_none());
    }
}
