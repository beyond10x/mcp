#![forbid(unsafe_code)]
//! MCP OAuth 2.1 mechanics over consumer-supplied HTTP and custody ports.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use b10x_mcp_types::{ClientError, HttpRequest, HttpResponse, SecretString};
use oauth2::http::{Response, StatusCode};
use rmcp::transport::auth::{
    AuthError, AuthorizationManager, AuthorizationRequest, AuthorizationSession, CredentialStore,
    OAuthHttpClient, OAuthHttpClientError, OAuthHttpClientFuture, OAuthHttpRedirectPolicy,
    OAuthHttpRequest, StateStore, StoredAuthorizationState, StoredCredentials,
};
use serde::{Deserialize, Serialize};

/// Non-secret authorization configuration for one MCP resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct OAuthConfig {
    /// Canonical MCP resource URL.
    pub resource_url: String,
    /// Callback URL registered for this client.
    pub redirect_uri: String,
    /// Human-readable DCR client name.
    #[serde(default = "default_client_name")]
    pub client_name: String,
    /// Pre-registered client ID, if available.
    pub client_id: Option<String>,
    /// HTTPS Client ID Metadata Document URL, if hosted by the caller.
    pub client_metadata_url: Option<String>,
    /// Explicit initial scopes. Empty means challenge/metadata selection.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Native for local CLI callbacks, web for hosted Connect Sessions.
    #[serde(default = "default_application_type")]
    pub application_type: String,
}

fn default_client_name() -> String {
    "b10x MCP client".to_owned()
}

fn default_application_type() -> String {
    "native".to_owned()
}

/// Executes one bounded OAuth HTTP operation.
#[async_trait]
pub trait HttpExchange: Send + Sync {
    /// Execute the request without logging credential-bearing headers or bodies.
    async fn execute(
        &self,
        request: HttpRequest,
        follow_redirects: bool,
    ) -> Result<HttpResponse, ClientError>;
}

/// Stores issuer/resource-bound OAuth credentials as one opaque secret document.
#[async_trait]
pub trait OAuthCredentialStore: Send + Sync {
    /// Load the document, if present.
    async fn load(&self) -> Result<Option<SecretString>, ClientError>;
    /// Atomically replace the document.
    async fn save(&self, document: SecretString) -> Result<(), ClientError>;
    /// Delete the document.
    async fn clear(&self) -> Result<(), ClientError>;
    /// Serialize refreshes sharing this store. Implementations may use a no-op guard.
    async fn lock_refresh(&self) -> Result<Box<dyn Send>, ClientError> {
        Ok(Box::new(()))
    }
}

/// Stores short-lived PKCE/state records by CSRF token.
#[async_trait]
pub trait OAuthStateStore: Send + Sync {
    /// Save one opaque state document.
    async fn save(&self, key: &str, document: SecretString) -> Result<(), ClientError>;
    /// Load one opaque state document.
    async fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError>;
    /// Consume or delete one state document.
    async fn delete(&self, key: &str) -> Result<(), ClientError>;
}

/// OAuth engine configured with consumer-owned network and custody adapters.
pub struct OAuthEngine {
    config: OAuthConfig,
    http: Arc<dyn HttpExchange>,
    credentials: Arc<dyn OAuthCredentialStore>,
    states: Arc<dyn OAuthStateStore>,
}

impl OAuthEngine {
    /// Build an engine. No network or store access occurs until another method is called.
    pub fn new(
        config: OAuthConfig,
        http: Arc<dyn HttpExchange>,
        credentials: Arc<dyn OAuthCredentialStore>,
        states: Arc<dyn OAuthStateStore>,
    ) -> Result<Self, ClientError> {
        validate_config(&config)?;
        Ok(Self {
            config,
            http,
            credentials,
            states,
        })
    }

    /// Load and refresh an existing access token without initiating user interaction.
    pub async fn access_token(&self) -> Result<SecretString, ClientError> {
        let mut manager = self.manager().await?;
        let present = manager.initialize_from_store().await.map_err(auth_error)?;
        if !present {
            return Err(ClientError::AuthorizationRequired {
                connection: b10x_mcp_types::ConnectionId::new("oauth")?,
                scope: None,
            });
        }
        manager
            .get_access_token()
            .await
            .map(SecretString::new)
            .map_err(auth_error)
    }

    /// Begin an authorization-code flow using pre-registration, CIMD, then DCR.
    pub async fn begin(
        &self,
        client_secret: Option<&SecretString>,
        challenge: Option<&str>,
    ) -> Result<PendingAuthorization, ClientError> {
        let mut manager = self.manager().await?;
        let resolution = manager
            .resolve_metadata_from_challenge(challenge)
            .await
            .map_err(auth_error)?;
        manager.set_metadata(resolution.metadata);
        let mut request = AuthorizationRequest::new(self.config.redirect_uri.clone())
            .with_client_name(self.config.client_name.clone())
            .with_application_type(self.config.application_type.clone())
            .with_scopes(self.config.scopes.clone());
        if let Some(client_id) = &self.config.client_id {
            request = request.with_preregistered_client(client_id.clone());
        }
        if let Some(secret) = client_secret {
            request = request.with_client_secret(secret.expose().to_owned());
        }
        if let Some(url) = &self.config.client_metadata_url {
            request = request.with_client_metadata_url(url.clone());
        }
        if let Some(challenge) = challenge {
            request = request.with_challenge(challenge.to_owned());
        }
        let session = AuthorizationSession::new(manager, request)
            .await
            .map_err(|(_, error)| auth_error(error))?;
        let authorization_url = session.auth_url.clone();
        Ok(PendingAuthorization {
            authorization_url,
            session,
        })
    }

    /// Remove all stored authorization for this resource.
    pub async fn clear(&self) -> Result<(), ClientError> {
        self.credentials.clear().await
    }

    async fn manager(&self) -> Result<AuthorizationManager, ClientError> {
        let mut manager = AuthorizationManager::new_with_oauth_http_client(
            self.config.resource_url.clone(),
            Arc::new(HttpAdapter(Arc::clone(&self.http))),
        )
        .await
        .map_err(auth_error)?;
        manager.set_credential_store(CredentialAdapter {
            inner: Arc::clone(&self.credentials),
            resource: self.config.resource_url.clone(),
        });
        manager.set_state_store(StateAdapter(Arc::clone(&self.states)));
        Ok(manager)
    }
}

/// An authorization flow waiting for the browser callback.
pub struct PendingAuthorization {
    authorization_url: String,
    session: AuthorizationSession,
}

impl PendingAuthorization {
    /// URL the resource owner must visit.
    pub fn authorization_url(&self) -> &str {
        &self.authorization_url
    }

    /// Validate the callback, exchange the code, persist credentials, and return an access token.
    pub async fn finish(self, callback_url: &str) -> Result<SecretString, ClientError> {
        self.session
            .handle_callback_url(callback_url)
            .await
            .map_err(auth_error)?;
        self.session
            .auth_manager
            .get_access_token()
            .await
            .map(SecretString::new)
            .map_err(auth_error)
    }
}

struct HttpAdapter(Arc<dyn HttpExchange>);

impl OAuthHttpClient for HttpAdapter {
    fn execute(&self, request: OAuthHttpRequest) -> OAuthHttpClientFuture<'_> {
        Box::pin(async move {
            let mut headers = BTreeMap::new();
            for (name, value) in request.request.headers() {
                let value = value
                    .to_str()
                    .map_err(|_| boxed("OAuth response/request header is not text"))?;
                headers.insert(name.as_str().to_owned(), value.to_owned());
            }
            let outbound = HttpRequest {
                method: request.request.method().as_str().to_owned(),
                url: request.request.uri().to_string(),
                headers,
                body: request.request.body().clone(),
            };
            let response = self
                .0
                .execute(
                    outbound,
                    matches!(request.redirect_policy, OAuthHttpRedirectPolicy::Follow),
                )
                .await
                .map_err(|_| boxed("OAuth HTTP exchange failed"))?;
            let mut builder = Response::builder().status(
                StatusCode::from_u16(response.status)
                    .map_err(|_| boxed("invalid OAuth response status"))?,
            );
            for (name, value) in response.headers {
                builder = builder.header(name, value);
            }
            builder
                .body(response.body)
                .map_err(|_| boxed("invalid OAuth response"))
        })
    }
}

struct CredentialAdapter {
    inner: Arc<dyn OAuthCredentialStore>,
    resource: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundCredentials {
    resource: String,
    credentials: StoredCredentials,
}

#[async_trait]
impl CredentialStore for CredentialAdapter {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        let Some(document) = self.inner.load().await.map_err(store_error)? else {
            return Ok(None);
        };
        let bound: BoundCredentials = serde_json::from_str(document.expose()).map_err(|_| {
            AuthError::CredentialStoreError("stored OAuth credentials are malformed".to_owned())
        })?;
        if bound.resource != self.resource {
            self.inner.clear().await.map_err(store_error)?;
            return Ok(None);
        }
        Ok(Some(bound.credentials))
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        let document = serde_json::to_string(&BoundCredentials {
            resource: self.resource.clone(),
            credentials,
        })
        .map_err(|_| {
            AuthError::CredentialStoreError("serializing OAuth credentials failed".to_owned())
        })?;
        self.inner
            .save(SecretString::new(document))
            .await
            .map_err(store_error)
    }

    async fn clear(&self) -> Result<(), AuthError> {
        self.inner.clear().await.map_err(store_error)
    }
}

struct StateAdapter(Arc<dyn OAuthStateStore>);

#[async_trait]
impl StateStore for StateAdapter {
    async fn save(&self, key: &str, state: StoredAuthorizationState) -> Result<(), AuthError> {
        let document = serde_json::to_string(&state).map_err(|_| {
            AuthError::CredentialStoreError("serializing OAuth state failed".to_owned())
        })?;
        self.0
            .save(key, SecretString::new(document))
            .await
            .map_err(store_error)
    }

    async fn load(&self, key: &str) -> Result<Option<StoredAuthorizationState>, AuthError> {
        let Some(document) = self.0.load(key).await.map_err(store_error)? else {
            return Ok(None);
        };
        serde_json::from_str(document.expose())
            .map(Some)
            .map_err(|_| {
                AuthError::CredentialStoreError("stored OAuth state is malformed".to_owned())
            })
    }

    async fn delete(&self, key: &str) -> Result<(), AuthError> {
        self.0.delete(key).await.map_err(store_error)
    }
}

fn validate_config(config: &OAuthConfig) -> Result<(), ClientError> {
    let resource = url::Url::parse(&config.resource_url)
        .map_err(|error| ClientError::Configuration(format!("invalid OAuth resource: {error}")))?;
    if resource.scheme() != "https"
        && !(resource.scheme() == "http"
            && resource.host_str().is_some_and(|host| {
                host == "localhost"
                    || host
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|address| address.is_loopback())
            }))
    {
        return Err(ClientError::Configuration(
            "OAuth resource requires HTTPS except on loopback".to_owned(),
        ));
    }
    let redirect = url::Url::parse(&config.redirect_uri)
        .map_err(|error| ClientError::Configuration(format!("invalid OAuth redirect: {error}")))?;
    if redirect.fragment().is_some() {
        return Err(ClientError::Configuration(
            "OAuth redirect cannot contain a fragment".to_owned(),
        ));
    }
    if config.client_id.is_none()
        && config.client_metadata_url.is_none()
        && config.client_name.is_empty()
    {
        return Err(ClientError::Configuration(
            "OAuth DCR requires a client name".to_owned(),
        ));
    }
    Ok(())
}

fn auth_error(error: AuthError) -> ClientError {
    match error {
        AuthError::AuthorizationRequired | AuthError::TokenRefreshRejected(_) => {
            ClientError::AuthorizationRequired {
                connection: b10x_mcp_types::ConnectionId::new("oauth")
                    .expect("constant connection id"),
                scope: None,
            }
        }
        AuthError::InsufficientScope { required_scope, .. } => ClientError::AuthorizationRequired {
            connection: b10x_mcp_types::ConnectionId::new("oauth").expect("constant connection id"),
            scope: Some(required_scope),
        },
        _ => ClientError::Transport("OAuth operation failed".to_owned()),
    }
}

fn store_error(error: ClientError) -> AuthError {
    let message = error.to_string();
    drop(error);
    AuthError::CredentialStoreError(message)
}

fn boxed(message: impl Into<String>) -> OAuthHttpClientError {
    Box::new(std::io::Error::other(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_cleartext_oauth_is_refused() {
        let config = OAuthConfig {
            resource_url: "http://example.com/mcp".to_owned(),
            redirect_uri: "http://127.0.0.1:3000/callback".to_owned(),
            client_name: default_client_name(),
            client_id: None,
            client_metadata_url: None,
            scopes: Vec::new(),
            application_type: default_application_type(),
        };
        assert!(validate_config(&config).is_err());
    }
}
