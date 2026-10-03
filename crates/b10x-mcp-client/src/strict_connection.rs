//! Revision-fixed setup and raw exchanges over the bounded HTTP receiver.
//!
//! Peer data is untrusted. Setup descriptions are not authority or family-result
//! acceptance. The caller owns endpoint, credentials and DNS/proxy/TLS admission.
use crate::strict_http;
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpConnectionPeerDescription as PeerDescription,
    McpHttpConnectionSetupInput as SetupInput, McpHttpConnectionSetupRefusal as SetupRefusal,
    McpHttpConnectionSetupRefusalReason as Reason, McpHttpExchangeExchangeInput as ExchangeInput,
    McpHttpExchangeExchangeResult as ExchangeResult,
};
use b10x_mcp_types::{ClientError, SecretString};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::fmt;
use std::time::Duration;
use tokio::time::Instant;

/// Runtime handle bound to one admitted endpoint and one configured revision.
///
/// It owns no grants or family-result validation. Description and raw exchanges
/// are observations only. Formatting this handle never exposes session/headers.
pub struct StrictConnection {
    client: reqwest::Client,
    template: reqwest::Request,
    input: SetupInput,
    description: PeerDescription,
    session: Option<SecretString>,
    next_id: u64,
}
impl fmt::Debug for StrictConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StrictConnection([REDACTED])")
    }
}

fn refusal(reason: Reason) -> SetupRefusal {
    SetupRefusal {
        reason: Box::new(reason),
        exchange: EssPresence::Absent,
        notification: EssPresence::Absent,
    }
}
fn local_error() -> ClientError {
    ClientError::Protocol("invalid strict connection exchange input".into())
}
fn modern(input: &SetupInput) -> bool {
    matches!(
        input.revision.as_ref(),
        b10x_mcp_types::http_exchange::McpHttpObservationsProtocolRevision::V1
    )
}
fn revision(input: &SetupInput) -> &'static str {
    if modern(input) {
        "2026-07-28"
    } else {
        "2025-11-25"
    }
}
fn template(request: &reqwest::Request) -> bool {
    request.method() == reqwest::Method::POST
        && matches!(request.url().scheme(), "http" | "https")
        && request.url().username().is_empty()
        && request.url().password().is_none()
        && request.url().fragment().is_none()
        && request.body().is_none()
        && !request.headers().keys().any(|name| {
            name.as_str().starts_with("mcp-")
                || matches!(
                    name.as_str(),
                    "content-length" | "transfer-encoding" | "content-encoding"
                )
        })
}

/// Establish exactly the configured revision without fallback or session repair.
///
/// Modern performs `server/discover`; legacy performs `initialize` followed by an
/// empty HTTP 202 acknowledgement of `notifications/initialized`. No failed setup
/// returns a handle. All phases share the caller's deadline and input phase caps.
/// The supplied builder must not hide MCP protocol/session/routing headers in its
/// defaults: reqwest does not expose those defaults for inspection or removal.
/// Supply admitted non-protocol headers explicitly on the bodyless POST template.
/// Generated error `Debug` contains untrusted peer data; never log it.
pub async fn connect(
    builder: reqwest::ClientBuilder,
    request: reqwest::Request,
    input: &SetupInput,
    deadline: Instant,
) -> Result<StrictConnection, SetupRefusal> {
    if !template(&request)
        || input
            .session_id_octets
            .0
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .is_none()
    {
        return Err(refusal(Reason::V3));
    }
    let millis = |n: &serde_json::Number| {
        n.as_u64()
            .filter(|n| *n > 0)
            .map(Duration::from_millis)
            .ok_or_else(|| refusal(Reason::V3))
    };
    let start = Instant::now();
    let end = deadline
        .min(
            start
                .checked_add(millis(&input.budget.remaining_execution_ms.0)?)
                .ok_or_else(|| refusal(Reason::V3))?,
        )
        .min(
            start
                .checked_add(millis(&input.budget.provider_ms.0)?)
                .ok_or_else(|| refusal(Reason::V3))?,
        );
    if start >= end {
        return Err(refusal(Reason::V0));
    }
    let client = strict_http::strict_builder(builder)
        .connect_timeout(millis(&input.budget.connect_ms.0)?.min(end - start))
        .build()
        .map_err(|_| refusal(Reason::V3))?;
    let (method, params) = if modern(input) {
        ("server/discover", json!({}))
    } else {
        (
            "initialize",
            json!({"protocolVersion":revision(input),"capabilities":{},"clientInfo":{"name":input.client_info.name,"version":input.client_info.version}}),
        )
    };
    let (wire, mut initial) =
        encode(input, &request, None, 1, method, params).map_err(|_| refusal(Reason::V3))?;
    // No stale caller body or session header may enter the initialization request.
    *initial.body_mut() = None;
    let (exchange, headers) =
        strict_http::pooled_exchange(&client, initial, &wire, end, !modern(input))
            .await
            .map_err(|_| refusal(Reason::V1))?;
    let description = match &exchange {
        ExchangeResult::V2(result) => {
            describe(input, &result.value.result, &result.value.observation)
        }
        ExchangeResult::V0(_) => Err(Reason::V6),
        ExchangeResult::V1(result) => Err(
            if matches!(
                result.value.reason.as_ref(),
                b10x_mcp_types::http_exchange::McpHttpExchangeRefusalReason::V1
            ) {
                Reason::V0
            } else {
                Reason::V1
            },
        ),
    }
    .map_err(|reason| {
        let mut failure = refusal(reason);
        failure.exchange = EssPresence::Present(Box::new(exchange.clone()));
        failure
    })?;
    let session = session(input, &headers).map_err(|reason| {
        let mut failure = refusal(reason);
        failure.exchange = EssPresence::Present(Box::new(exchange));
        failure
    })?;
    if !modern(input) {
        finish_legacy_setup(&client, &request, input, session.as_ref(), end).await?;
    }
    if Instant::now() >= end {
        return Err(refusal(Reason::V0));
    }
    Ok(StrictConnection {
        client,
        template: request,
        input: input.clone(),
        description,
        session,
        next_id: 2,
    })
}

async fn finish_legacy_setup(
    client: &reqwest::Client,
    request: &reqwest::Request,
    input: &SetupInput,
    session: Option<&SecretString>,
    end: Instant,
) -> Result<(), SetupRefusal> {
    let mut notification = request.try_clone().ok_or_else(|| refusal(Reason::V3))?;
    attach_session(&mut notification, session).map_err(|_| refusal(Reason::V4))?;
    let (accepted, observation) =
        strict_http::initialized_notification(client, notification, &input.budget, end)
            .await
            .map_err(|_| refusal(Reason::V5))?;
    if !accepted {
        let mut failure = refusal(if Instant::now() >= end {
            Reason::V0
        } else {
            Reason::V5
        });
        failure.notification = EssPresence::Present(Box::new(observation));
        return Err(failure);
    }
    Ok(())
}

impl StrictConnection {
    pub(crate) fn traversal_deadline(&self, deadline: Instant) -> Option<Instant> {
        let start = Instant::now();
        Some(
            deadline
                .min(start.checked_add(Duration::from_millis(
                    self.input.budget.remaining_execution_ms.0.as_u64()?,
                ))?)
                .min(start.checked_add(Duration::from_millis(
                    self.input.budget.provider_ms.0.as_u64()?,
                ))?),
        )
    }

    /// Peer-reported facts; never a grant or selected endpoint/version.
    pub fn description(&self) -> &PeerDescription {
        &self.description
    }

    /// One raw tool/resource/prompt exchange on this already established boundary.
    ///
    /// The caller still validates the family result and supplies authority. No
    /// discovery cache, list paging, MRTR or retry is driven by this method.
    pub async fn exchange(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Result<ExchangeResult, ClientError> {
        if !matches!(
            method,
            "tools/list"
                | "tools/call"
                | "resources/list"
                | "resources/read"
                | "prompts/list"
                | "prompts/get"
        ) {
            return Err(local_error());
        }
        let family = method.split('/').next().ok_or_else(local_error)?;
        let supported = match family {
            "tools" => !self.description.capabilities.tools.is_absent(),
            "resources" => !self.description.capabilities.resources.is_absent(),
            "prompts" => !self.description.capabilities.prompts.is_absent(),
            _ => false,
        };
        if !supported {
            return Err(ClientError::Protocol(
                "strict connection family not advertised".into(),
            ));
        }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or_else(local_error)?;
        let (wire, request) = encode(
            &self.input,
            &self.template,
            self.session.as_ref(),
            id,
            method,
            params,
        )?;
        strict_http::pooled_exchange(&self.client, request, &wire, deadline, false)
            .await
            .map(|(result, _)| result)
    }
}

fn attach_session(
    request: &mut reqwest::Request,
    session: Option<&SecretString>,
) -> Result<(), ClientError> {
    if let Some(session) = session {
        let mut value =
            reqwest::header::HeaderValue::from_str(session.expose()).map_err(|_| local_error())?;
        value.set_sensitive(true);
        request.headers_mut().insert("mcp-session-id", value);
    }
    Ok(())
}
fn encode(
    input: &SetupInput,
    template: &reqwest::Request,
    session: Option<&SecretString>,
    id: u64,
    method: &str,
    mut params: Value,
) -> Result<(ExchangeInput, reqwest::Request), ClientError> {
    let fields = params.as_object_mut().ok_or_else(local_error)?;
    let mut request = template.try_clone().ok_or_else(local_error)?;
    if modern(input) {
        let meta = fields
            .entry("_meta")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(local_error)?;
        for (key, value) in [
            (
                "io.modelcontextprotocol/protocolVersion",
                json!(revision(input)),
            ),
            (
                "io.modelcontextprotocol/clientInfo",
                json!({"name":input.client_info.name,"version":input.client_info.version}),
            ),
            ("io.modelcontextprotocol/clientCapabilities", json!({})),
        ] {
            if meta.get(key).is_some_and(|present| present != &value) {
                return Err(local_error());
            }
            meta.insert(key.to_owned(), value);
        }
        request.headers_mut().insert(
            "mcp-method",
            reqwest::header::HeaderValue::from_str(method).map_err(|_| local_error())?,
        );
        let name = match method {
            "tools/call" | "prompts/get" => Some("name"),
            "resources/read" => Some("uri"),
            _ => None,
        };
        if let Some(key) = name {
            let name = params
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(local_error)?;
            let value = if name
                .bytes()
                .all(|b| (0x20..=0x7e).contains(&b) || b == b'\t')
                && name.trim() == name
                && !name.starts_with("=?base64?")
            {
                name.to_owned()
            } else {
                format!("=?base64?{}?=", STANDARD.encode(name.as_bytes()))
            };
            request.headers_mut().insert(
                "mcp-name",
                reqwest::header::HeaderValue::from_str(&value).map_err(|_| local_error())?,
            );
        }
    }
    attach_session(&mut request, session)?;
    let body =
        serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .map_err(|_| local_error())?;
    let wire = serde_json::from_value(json!({"revision":revision(input),"request_id":{"kind":"integer","value":id},"encoded_request":STANDARD.encode(body),"budget":input.budget})).map_err(|_| local_error())?;
    Ok((wire, request))
}

fn session(
    input: &SetupInput,
    values: &[reqwest::header::HeaderValue],
) -> Result<Option<SecretString>, Reason> {
    if modern(input) || values.is_empty() {
        return Ok(None);
    }
    if values.len() != 1 {
        return Err(Reason::V4);
    }
    let bytes = values[0].as_bytes();
    if bytes.is_empty() || bytes.iter().any(|b| !(0x21..=0x7e).contains(b)) {
        return Err(Reason::V4);
    }
    if bytes.len() as u64 > input.session_id_octets.0.as_u64().ok_or(Reason::V3)? {
        return Err(Reason::V8);
    }
    Ok(Some(SecretString::new(
        std::str::from_utf8(bytes)
            .map_err(|_| Reason::V4)?
            .to_owned(),
    )))
}

fn describe(
    input: &SetupInput,
    raw: &Value,
    observation: &b10x_mcp_types::http_exchange::McpHttpExchangeExchangeObservation,
) -> Result<PeerDescription, Reason> {
    let mut result = json!({"configured_revision":revision(input),"raw_result":null,"exchange_observation":observation});
    let object = raw.as_object().ok_or(Reason::V2)?;
    let capabilities = object
        .get("capabilities")
        .and_then(Value::as_object)
        .ok_or(Reason::V2)?;
    let mut typed = json!({"reported_keys":capabilities.keys().collect::<Vec<_>>()});
    for family in ["tools", "resources", "prompts"] {
        if let Some(value) = capabilities.get(family) {
            let fields = value.as_object().ok_or(Reason::V2)?;
            let mut capability = json!({});
            for (wire, field) in [("listChanged", "list_changed"), ("subscribe", "subscribe")] {
                if family != "resources" && wire == "subscribe" {
                    continue;
                }
                if let Some(value) = fields.get(wire) {
                    capability[field] = json!(value.as_bool().ok_or(Reason::V2)?);
                }
            }
            typed[family] = capability;
        }
    }
    result["capabilities"] = typed;
    if let Some(instructions) = object.get("instructions") {
        result["instructions"] = json!(instructions.as_str().ok_or(Reason::V2)?);
    }
    if let Some(meta) = object.get("_meta") {
        if !meta.is_object() {
            return Err(Reason::V2);
        }
    }
    let info = if modern(input) {
        if raw["resultType"] != "complete" {
            return Err(Reason::V2);
        }
        let versions = raw["supportedVersions"].as_array().ok_or(Reason::V2)?;
        if !versions.iter().all(Value::is_string) {
            return Err(Reason::V2);
        }
        if !versions.iter().any(|v| v == revision(input)) {
            return Err(Reason::V7);
        }
        result["reported_versions"] = json!(versions);
        let ttl = raw["ttlMs"].as_number().ok_or(Reason::V2)?;
        if !ttl.to_string().bytes().all(|b| b.is_ascii_digit()) {
            return Err(Reason::V2);
        }
        if !matches!(raw["cacheScope"].as_str(), Some("public" | "private")) {
            return Err(Reason::V2);
        }
        result["cache_hints"] = json!({"ttl_ms":ttl,"scope":raw["cacheScope"]});
        raw.get("_meta")
            .and_then(|meta| meta.get("io.modelcontextprotocol/serverInfo"))
    } else {
        let version = raw["protocolVersion"].as_str().ok_or(Reason::V2)?;
        if version != revision(input) {
            return Err(Reason::V7);
        }
        result["reported_versions"] = json!([version]);
        Some(raw.get("serverInfo").ok_or(Reason::V2)?)
    };
    if let Some(info) = info {
        result["server_info"] = json!({"name":info["name"].as_str().ok_or(Reason::V2)?,"version":info["version"].as_str().ok_or(Reason::V2)?});
    }
    let mut description: PeerDescription =
        serde_json::from_value(result).map_err(|_| Reason::V2)?;
    description.raw_result = raw.clone();
    Ok(description)
}
