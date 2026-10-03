//! One bounded, revision-selected HTTP exchange before typed SDK decoding.
//!
//! The caller owns endpoint admission, request headers and the HTTP builder's
//! DNS, proxy and TLS policy. This layer supplies no credentials or fallback
//! endpoint, performs no negotiation, and does not accept a family result as
//! business success. Observations contain untrusted, possibly sensitive data;
//! never log their generated `Debug` representation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::Future;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use b10x_mcp_types::ClientError;
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpExchangeExchangeInput as ExchangeInput,
    McpHttpExchangeExchangeResult as ExchangeResult,
    McpHttpLifecycleControlObservation as ControlObservation,
    McpHttpLifecycleStreamMessage as StreamMessage,
    McpHttpLifecycleStreamObservation as StreamObservation, McpHttpObservationsPeerData,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use tokio::time::{Instant, timeout_at};

/// Execute one caller-admitted POST, preserving bounded response observations.
///
/// `deadline` is the caller's existing monotonic absolute deadline. Input budgets
/// can tighten it, never extend it. The supplied builder retains its DNS, proxy,
/// TLS and other admission settings; redirects, automatic retries and transparent
/// decompression are disabled. No already-built client can be inspected for those
/// policies, so the strict entry point deliberately requires its builder.
///
/// The request supplies endpoint and headers; its body must be absent or exactly
/// match `input.encoded_request`. This operation neither negotiates nor repairs a
/// session. Both JSON and SSE return the protocol envelope only; a family API
/// must still validate its configured revision's result shape. Legacy callers
/// must have completed initialization; modern requests have no handshake.
///
/// This invocation builds its own client from the supplied builder. It does not
/// establish a reusable connection or alter the older compatibility constructors.
pub async fn exchange(
    builder: reqwest::ClientBuilder,
    request: reqwest::Request,
    input: &ExchangeInput,
    deadline: Instant,
) -> Result<ExchangeResult, ClientError> {
    exchange_inner(
        Driver::Builder(Box::new(builder)),
        request,
        input,
        deadline,
        &mut Vec::new(),
    )
    .await
}

enum Driver {
    Builder(Box<reqwest::ClientBuilder>),
    Pooled(reqwest::Client, Option<usize>),
}

pub(crate) fn strict_builder(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    builder
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
}

pub(crate) async fn pooled_exchange(
    client: &reqwest::Client,
    request: reqwest::Request,
    input: &ExchangeInput,
    deadline: Instant,
    initialize_session_limit: Option<usize>,
) -> Result<(ExchangeResult, Vec<reqwest::header::HeaderValue>), ClientError> {
    let mut session = Vec::new();
    let result = exchange_inner(
        Driver::Pooled(client.clone(), initialize_session_limit),
        request,
        input,
        deadline,
        &mut session,
    )
    .await?;
    Ok((result, session))
}

async fn exchange_inner(
    driver: Driver,
    mut request: reqwest::Request,
    input: &ExchangeInput,
    deadline: Instant,
    session: &mut Vec<reqwest::header::HeaderValue>,
) -> Result<ExchangeResult, ClientError> {
    let controlled = matches!(&driver, Driver::Pooled(..));
    let session_limit = match &driver {
        Driver::Pooled(_, limit) => *limit,
        Driver::Builder(_) => None,
    };
    let initialize = session_limit.is_some();
    let started = Instant::now();
    let mut received = Reception::new(
        input
            .budget
            .response_octets
            .0
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(0),
    );
    if controlled {
        received.shared = Some(Arc::new(AtomicUsize::new(received.limit)));
    }
    let (body, id, revision, provider, connect, execution, event_limit) =
        match validate(input, initialize) {
            Ok(valid) => valid,
            Err(reason) => return received.refuse(reason),
        };
    let Some(provider_end) = started.checked_add(provider) else {
        return received.refuse("invalid_input");
    };
    let Some(execution_end) = started.checked_add(execution) else {
        return received.refuse("invalid_input");
    };
    let end = deadline.min(provider_end).min(execution_end);
    if Instant::now() >= end {
        return received.refuse("deadline_exhausted");
    }
    let semantics = if revision == "2026-07-28" {
        HttpSemantics::Modern
    } else if request.headers().contains_key("mcp-session-id") {
        HttpSemantics::LegacySession
    } else {
        HttpSemantics::LegacySessionless
    };
    if let Err(reason) = prepare_request(&mut request, body, &revision) {
        return received.refuse(reason);
    }
    let remaining = end.saturating_duration_since(Instant::now());
    *request.timeout_mut() = Some(
        request
            .timeout()
            .copied()
            .map_or(remaining, |t| t.min(remaining)),
    );
    let client = match driver {
        Driver::Pooled(client, _) => client,
        Driver::Builder(builder) => match strict_builder(*builder)
            .connect_timeout(connect.min(remaining))
            .build()
        {
            Ok(client) => client,
            Err(_) => return received.refuse("invalid_input"),
        },
    };
    if Instant::now() >= end {
        return received.refuse("deadline_exhausted");
    }
    let mut control = if controlled && !matches!(semantics, HttpSemantics::Modern) {
        Some(ControlPort::new(&client, &request, input)?)
    } else {
        None
    };
    let response = match send_request(&client, request, &mut received, end).await {
        Ok(response) => response,
        Err(reason) => return received.refuse(reason),
    };
    if initialize {
        session.extend(response.headers().get_all("mcp-session-id").iter().cloned());
        if let Some(control) = &mut control {
            control.install_session(session, session_limit.unwrap_or(0));
        }
    }
    receive(
        response,
        received,
        &ResponseContext {
            id,
            event_limit,
            end,
            semantics,
            controlled,
            control,
        },
    )
    .await
}

async fn send_request(
    client: &reqwest::Client,
    request: reqwest::Request,
    received: &mut Reception,
    end: Instant,
) -> Result<reqwest::Response, &'static str> {
    // Entering execute is not evidence that request bytes left.
    received.send = "unknown";
    let response = match timeout_at(end, client.execute(request)).await {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            return Err(if error.is_timeout() {
                "deadline_exhausted"
            } else {
                "transport_failure"
            });
        }
        Err(_) => return Err("deadline_exhausted"),
    };
    received.send = "send_observed";
    if !(100..=599).contains(&response.status().as_u16()) {
        return Err("invalid_response");
    }
    received.status = Some(response.status().as_u16());
    Ok(response)
}

struct ResponseContext {
    id: Value,
    event_limit: usize,
    end: Instant,
    semantics: HttpSemantics,
    controlled: bool,
    control: Option<ControlPort>,
}

struct ControlPort {
    client: reqwest::Client,
    template: reqwest::Request,
    request_limit: usize,
    session_valid: bool,
}
impl ControlPort {
    fn fork(&self) -> Result<Self, ClientError> {
        Ok(Self {
            client: self.client.clone(),
            template: self.template.try_clone().ok_or_else(internal_error)?,
            request_limit: self.request_limit,
            session_valid: self.session_valid,
        })
    }
    fn new(
        client: &reqwest::Client,
        request: &reqwest::Request,
        input: &ExchangeInput,
    ) -> Result<Self, ClientError> {
        let mut template = request.try_clone().ok_or_else(internal_error)?;
        *template.body_mut() = None;
        for name in ["content-length", "mcp-method", "mcp-name"] {
            template.headers_mut().remove(name);
        }
        let parameters: Vec<_> = template
            .headers()
            .keys()
            .filter(|k| k.as_str().starts_with("mcp-param-"))
            .cloned()
            .collect();
        for name in parameters {
            template.headers_mut().remove(name);
        }
        Ok(Self {
            client: client.clone(),
            template,
            request_limit: input
                .budget
                .request_octets
                .0
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(internal_error)?,
            session_valid: true,
        })
    }

    fn install_session(&mut self, values: &[reqwest::header::HeaderValue], limit: usize) {
        if values.is_empty() {
            return;
        }
        self.session_valid = values.len() == 1
            && !values[0].is_empty()
            && values[0].as_bytes().len() <= limit
            && values[0]
                .as_bytes()
                .iter()
                .all(|b| (0x21..=0x7e).contains(b));
        if self.session_valid {
            let mut value = values[0].clone();
            value.set_sensitive(true);
            self.template.headers_mut().insert("mcp-session-id", value);
        }
    }

    async fn reply(
        &self,
        message: &Value,
        received: &Mutex<Reception>,
        end: Instant,
    ) -> Result<ControlObservation, ClientError> {
        let ping = message["method"] == "ping";
        let body = if ping {
            json!({"jsonrpc":"2.0","id":message["id"],"result":{}})
        } else {
            json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32601,"message":"Client method not supported"}})
        };
        let body = serde_json::to_vec(&body).map_err(|_| internal_error())?;
        let mut request = self.template.try_clone().ok_or_else(internal_error)?;
        let outcome = async {
            if !self.session_valid {
                return Err("invalid_response");
            }
            if body.len() > self.request_limit {
                return Err("request_bound");
            }
            if Instant::now() >= end {
                return Err("deadline_exhausted");
            }
            prepare_request(&mut request, body, "2025-11-25")?;
            let remaining = end.saturating_duration_since(Instant::now());
            *request.timeout_mut() = Some(
                request
                    .timeout()
                    .copied()
                    .map_or(remaining, |t| t.min(remaining)),
            );
            received.lock().map_err(|_| "transport_failure")?.send = "unknown";
            let response = match timeout_at(end, self.client.execute(request)).await {
                Ok(Ok(response)) => response,
                Ok(Err(error)) => {
                    return Err(if error.is_timeout() {
                        "deadline_exhausted"
                    } else {
                        "transport_failure"
                    });
                }
                Err(_) => return Err("deadline_exhausted"),
            };
            receive_reply(response, received, end).await?;
            let state = received.lock().map_err(|_| "transport_failure")?;
            match state.status {
                Some(202) if state.seen == 0 => Ok(()),
                Some(200..=299) => Err("invalid_response"),
                Some(401 | 403) => Err("authorization_required"),
                Some(404) if self.template.headers().contains_key("mcp-session-id") => {
                    Err("session_expired")
                }
                _ => Err("http_status"),
            }
        }
        .await;
        control_observation(
            received,
            if ping {
                "ping_reply"
            } else {
                "unsupported_request_reply"
            },
            outcome,
        )
    }
}

fn control_observation(
    state: &Mutex<Reception>,
    kind: &str,
    outcome: Result<(), &str>,
) -> Result<ControlObservation, ClientError> {
    let received = state.lock().map_err(|_| internal_error())?;
    let mut value = json!({"control":kind,"exchange":received.wire_observation(),"disposition":if outcome.is_ok() {"accepted"} else {"refused"}});
    if let Err(reason) = outcome {
        value["refusal"] = json!(reason);
    }
    serde_json::from_value(value).map_err(|_| internal_error())
}

async fn receive_reply(
    mut response: reqwest::Response,
    state: &Mutex<Reception>,
    end: Instant,
) -> Result<(), &'static str> {
    {
        let mut received = state.lock().map_err(|_| "transport_failure")?;
        received.send = "send_observed";
        let status = response.status().as_u16();
        if !(100..=599).contains(&status) {
            return Err("invalid_response");
        }
        received.status = Some(status);
    }
    let encoded = response
        .headers()
        .get_all("content-encoding")
        .iter()
        .any(|v| v != "identity");
    loop {
        let chunk = timeout_at(end, response.chunk()).await;
        let mut received = state.lock().map_err(|_| "transport_failure")?;
        match chunk {
            Ok(Ok(Some(chunk))) if received.append(&chunk) => {}
            Ok(Ok(Some(_))) => return Err("response_bound"),
            Ok(Ok(None)) => {
                received.whole = true;
                break;
            }
            Ok(Err(error)) => {
                return Err(if error.is_timeout() {
                    "deadline_exhausted"
                } else {
                    "transport_failure"
                });
            }
            Err(_) => return Err("deadline_exhausted"),
        }
        if Instant::now() >= end {
            return Err("deadline_exhausted");
        }
    }
    if Instant::now() >= end {
        Err("deadline_exhausted")
    } else if encoded {
        Err("unsupported_response")
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum HttpSemantics {
    Modern,
    LegacySession,
    LegacySessionless,
}

pub(crate) async fn initialized_notification(
    client: &reqwest::Client,
    mut request: reqwest::Request,
    budget: &b10x_mcp_types::http_exchange::McpHttpExchangeExchangeBudget,
    end: Instant,
) -> Result<
    (
        bool,
        b10x_mcp_types::http_exchange::McpHttpExchangeExchangeObservation,
    ),
    ClientError,
> {
    let mut received = Reception::new(
        budget
            .response_octets
            .0
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(internal_error)?,
    );
    let body = br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_vec();
    let action = async {
        if body.len() as u64
            > budget
                .request_octets
                .0
                .as_u64()
                .ok_or_else(internal_error)?
            || Instant::now() >= end
        {
            return Ok(false);
        }
        prepare_request(&mut request, body, "2025-11-25").map_err(|_| internal_error())?;
        let remaining = end.saturating_duration_since(Instant::now());
        *request.timeout_mut() = Some(
            request
                .timeout()
                .copied()
                .map_or(remaining, |t| t.min(remaining)),
        );
        received.send = "unknown";
        let Ok(Ok(mut response)) = timeout_at(end, client.execute(request)).await else {
            return Ok(false);
        };
        received.send = "send_observed";
        let status = response.status().as_u16();
        if !(100..=599).contains(&status) {
            return Ok(false);
        }
        received.status = Some(status);
        let encoded = response
            .headers()
            .get_all("content-encoding")
            .iter()
            .any(|v| v != "identity");
        loop {
            match timeout_at(end, response.chunk()).await {
                Ok(Ok(Some(chunk))) if received.append(&chunk) => {}
                Ok(Ok(None)) => {
                    received.whole = true;
                    break;
                }
                _ => return Ok(false),
            }
        }
        Ok::<bool, ClientError>(
            status == 202 && !encoded && received.seen == 0 && Instant::now() < end,
        )
    }
    .await?;
    let observation =
        serde_json::from_value(received.observation()).map_err(|_| internal_error())?;
    Ok((action, observation))
}

type Validated = (Vec<u8>, Value, String, Duration, Duration, Duration, usize);

// No JSON-RPC terminal is inferred from this HTTP control exchange. The same
// admitted client supplies retry/redirect/decompression policy as business I/O.
pub(crate) async fn session_delete(
    client: &reqwest::Client,
    mut request: reqwest::Request,
    budget: &b10x_mcp_types::http_exchange::McpHttpExchangeExchangeBudget,
    end: Instant,
) -> Result<b10x_mcp_types::http_exchange::McpHttpLifecycleControlObservation, ClientError> {
    let mut received = Reception::new(
        budget
            .response_octets
            .0
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(internal_error)?,
    );
    let outcome = async {
        if Instant::now() >= end {
            return Err("deadline_exhausted");
        }
        *request.method_mut() = reqwest::Method::DELETE;
        *request.body_mut() = None;
        request.headers_mut().remove("content-type");
        request.headers_mut().insert(
            "mcp-protocol-version",
            "2025-11-25".parse().expect("static header"),
        );
        request.headers_mut().insert(
            "accept",
            "application/json, text/event-stream"
                .parse()
                .expect("static header"),
        );
        let remaining = end.saturating_duration_since(Instant::now());
        *request.timeout_mut() = Some(
            request
                .timeout()
                .copied()
                .map_or(remaining, |t| t.min(remaining)),
        );
        received.send = "unknown";
        let response = match timeout_at(end, client.execute(request)).await {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                return Err(if error.is_timeout() {
                    "deadline_exhausted"
                } else {
                    "transport_failure"
                });
            }
            Err(_) => return Err("deadline_exhausted"),
        };
        receive_control(response, &mut received, end).await?;
        match received.status {
            Some(200..=299) => Ok("accepted"),
            Some(405) => Ok("delete_not_allowed"),
            Some(401 | 403) => Err("authorization_required"),
            Some(404) => Err("session_expired"),
            _ => Err("http_status"),
        }
    }
    .await;
    let mut observation = json!({"control":"session_delete", "exchange":received.observation(),
        "disposition":outcome.as_ref().copied().unwrap_or("refused")});
    if let Err(reason) = outcome {
        observation["refusal"] = json!(reason);
    }
    serde_json::from_value(observation).map_err(|_| internal_error())
}

async fn receive_control(
    mut response: reqwest::Response,
    received: &mut Reception,
    end: Instant,
) -> Result<(), &'static str> {
    received.send = "send_observed";
    let status = response.status().as_u16();
    if !(100..=599).contains(&status) {
        return Err("invalid_response");
    }
    received.status = Some(status);
    let encoded = response
        .headers()
        .get_all("content-encoding")
        .iter()
        .any(|v| v != "identity");
    loop {
        match timeout_at(end, response.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                if !received.append(&chunk) {
                    return Err("response_bound");
                }
            }
            Ok(Ok(None)) => {
                received.whole = true;
                break;
            }
            Ok(Err(error)) => {
                return Err(if error.is_timeout() {
                    "deadline_exhausted"
                } else {
                    "transport_failure"
                });
            }
            Err(_) => return Err("deadline_exhausted"),
        }
        if Instant::now() >= end {
            return Err("deadline_exhausted");
        }
    }
    if Instant::now() >= end {
        Err("deadline_exhausted")
    } else if encoded {
        Err("unsupported_response")
    } else {
        Ok(())
    }
}

fn validate(input: &ExchangeInput, initialize: bool) -> Result<Validated, &'static str> {
    let budget = &input.budget;
    let request_limit = budget
        .request_octets
        .0
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("invalid_input")?;
    budget
        .response_octets
        .0
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("invalid_input")?;
    let event_limit = budget
        .sse_event_octets
        .0
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("invalid_input")?;
    let millis = |number: &serde_json::Number| {
        number
            .as_u64()
            .filter(|n| *n > 0)
            .map(Duration::from_millis)
            .ok_or("invalid_input")
    };
    let provider = millis(&budget.provider_ms.0)?;
    let connect = millis(&budget.connect_ms.0)?;
    let execution = millis(&budget.remaining_execution_ms.0)?;
    // Refuse an oversized encoded input before allocating its decoded bytes.
    if input
        .encoded_request
        .len()
        .div_ceil(4)
        .saturating_mul(3)
        .saturating_sub(2)
        > request_limit
    {
        return Err("request_bound");
    }
    let body = STANDARD
        .decode(&input.encoded_request)
        .map_err(|_| "invalid_input")?;
    if body.len() > request_limit {
        return Err("request_bound");
    }
    if STANDARD.encode(&body) != input.encoded_request {
        return Err("invalid_input");
    }
    let request = parse(&body).ok_or("invalid_input")?;
    let object = request.as_object().ok_or("invalid_input")?;
    let carrier_id = serde_json::to_value(&input.request_id).map_err(|_| "invalid_input")?;
    let id = carrier_id
        .get("value")
        .filter(|id| valid_id(id))
        .ok_or("invalid_input")?
        .clone();
    if object.get("jsonrpc") != Some(&json!("2.0"))
        || object.get("id") != Some(&id)
        || !object
            .get("method")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty() && (s != "initialize" || initialize))
        || object.contains_key("result")
        || object.contains_key("error")
        || object
            .get("params")
            .is_some_and(|p| !p.is_object() && !p.is_array())
    {
        return Err("invalid_input");
    }
    let revision = serde_json::to_value(&input.revision)
        .map_err(|_| "invalid_input")?
        .as_str()
        .ok_or("invalid_input")?
        .to_owned();
    Ok((
        body,
        id,
        revision,
        provider,
        connect,
        execution,
        event_limit,
    ))
}

fn integral(value: &Value) -> bool {
    let Some(number) = value.as_number() else {
        return false;
    };
    let text = number.to_string();
    let digits = text.strip_prefix('-').unwrap_or(&text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
fn valid_id(value: &Value) -> bool {
    value.is_string() || integral(value)
}

// Parsing into Value alone accepts duplicate object members by overwriting them.
// First visit every object (including nested peer data), rejecting duplicates.
struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate object members")
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_unit<E: de::Error>(self) -> Result<UniqueJson, E> {
                Ok(UniqueJson)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<UniqueJson, A::Error> {
                while seq.next_element::<UniqueJson>()?.is_some() {}
                Ok(UniqueJson)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<UniqueJson, A::Error> {
                let mut seen = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key) {
                        return Err(de::Error::custom("duplicate JSON member"));
                    }
                    map.next_value::<UniqueJson>()?;
                }
                Ok(UniqueJson)
            }
        }
        d.deserialize_any(UniqueVisitor)
    }
}
pub(crate) fn parse(bytes: &[u8]) -> Option<Value> {
    serde_json::from_slice::<UniqueJson>(bytes).ok()?;
    raw_json(serde_json::from_slice::<&serde_json::value::RawValue>(bytes).ok()?)
}

// Arbitrary-precision serde uses private object-key markers when decoding Value.
// On wire those keys are legal opaque peer members, not number instructions.
// Preserve raw object/array structure before decoding primitive JSON values.
fn raw_json(raw: &serde_json::value::RawValue) -> Option<Value> {
    let text = raw.get();
    match text.as_bytes().first()? {
        b'{' => {
            let fields: BTreeMap<String, &serde_json::value::RawValue> =
                serde_json::from_str(text).ok()?;
            let fields: Option<serde_json::Map<String, Value>> = fields
                .into_iter()
                .map(|(k, v)| Some((k, raw_json(v)?)))
                .collect();
            Some(Value::Object(fields?))
        }
        b'[' => {
            let items: Vec<&serde_json::value::RawValue> = serde_json::from_str(text).ok()?;
            Some(Value::Array(
                items.into_iter().map(raw_json).collect::<Option<_>>()?,
            ))
        }
        _ => serde_json::from_str(text).ok(),
    }
}

// Mutable receiver state, not a second serialized model. Generated carriers are
// constructed only from actual observations and never deserialize peer metadata.
struct Reception {
    bytes: Vec<u8>,
    seen: usize,
    limit: usize,
    whole: bool,
    terminal: bool,
    send: &'static str,
    status: Option<u16>,
    stream: Vec<StreamMessage>,
    shared: Option<Arc<AtomicUsize>>,
}
impl Reception {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            seen: 0,
            limit,
            whole: false,
            terminal: false,
            send: "not_sent",
            status: None,
            stream: Vec::new(),
            shared: None,
        }
    }
    fn append(&mut self, bytes: &[u8]) -> bool {
        self.seen = self.seen.saturating_add(bytes.len());
        let mut keep = self.limit.saturating_sub(self.bytes.len()).min(bytes.len());
        if let Some(shared) = &self.shared {
            let available = shared
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    Some(n.saturating_sub(keep))
                })
                .expect("infallible update");
            keep = keep.min(available);
        }
        self.bytes.extend_from_slice(&bytes[..keep]);
        self.seen <= self.limit && keep == bytes.len()
    }
    fn observation(&self) -> Value {
        let mut value = self.wire_observation();
        if !self.stream.is_empty() {
            value["stream"] = json!({"messages":self.stream});
        }
        value
    }
    fn wire_observation(&self) -> Value {
        let mut value = json!({
            "send":self.send, "terminal":if self.terminal {"correlated_terminal"} else {"incomplete"},
            "response":{"bytes":STANDARD.encode(&self.bytes),
                "counts":{"retained_octets":self.bytes.len(),"limit_octets":self.limit},
                "message_length":{"kind":if self.whole {"exact"} else {"at_least"},"value":self.seen},
                "retention":if self.whole {"whole_message"} else {"prefix"}}
        });
        if let Some(status) = self.status {
            value["http_status"] = json!(status);
        }
        value
    }
    fn consume_message(&mut self) {
        if self.shared.is_none() {
            self.limit = self.limit.saturating_sub(self.bytes.len());
        }
        self.bytes.clear();
        self.seen = 0;
        self.whole = false;
        self.terminal = false;
    }
    fn refuse(&self, reason: &str) -> Result<ExchangeResult, ClientError> {
        carrier(
            json!({"kind":"refused","value":{"observation":self.observation(),"reason":reason}}),
        )
    }
    fn envelope(
        &mut self,
        id: &Value,
        allow_notification: bool,
    ) -> Result<Option<ExchangeResult>, ClientError> {
        let Some(value) = parse(&self.bytes) else {
            return self.refuse("invalid_response").map(Some);
        };
        let Some(object) = value.as_object() else {
            return self.refuse("invalid_response").map(Some);
        };
        if object.get("jsonrpc") != Some(&json!("2.0")) {
            return self.refuse("invalid_response").map(Some);
        }
        if allow_notification
            && !object.contains_key("id")
            && object.get("method").and_then(Value::as_str).is_some()
            && !object.contains_key("result")
            && !object.contains_key("error")
        {
            return Ok(None);
        }
        if object.get("id") != Some(id)
            || object.contains_key("method")
            || object.contains_key("result") == object.contains_key("error")
        {
            return self.refuse("invalid_response").map(Some);
        }
        let (kind, field, payload) = if let Some(result) = object.get("result") {
            if !result.is_object() {
                return self.refuse("invalid_response").map(Some);
            }
            ("result", "result", result.clone())
        } else {
            let Some(error) = object.get("error").and_then(Value::as_object) else {
                return self.refuse("invalid_response").map(Some);
            };
            if !error.get("code").is_some_and(integral)
                || !error.get("message").is_some_and(Value::is_string)
            {
                return self.refuse("invalid_response").map(Some);
            }
            let data = error.get("data").map_or_else(
                || json!({"presence":"absent","value":"absent"}),
                |v| json!({"presence":"present","value":v}),
            );
            (
                "peer_error",
                "error",
                json!({"code":error["code"],"message":error["message"],"data":data}),
            )
        };
        self.terminal = true;
        let mut result = json!({"kind":kind,"value":{"observation":self.observation()}});
        result["value"][field] = payload;
        carrier(result).map(Some)
    }
}
fn internal_error() -> ClientError {
    ClientError::Protocol("strict exchange carrier construction failed".into())
}
fn carrier(mut value: Value) -> Result<ExchangeResult, ClientError> {
    // Only decode our structural envelope. Opaque JSON has already been decoded
    // from exact bytes above; decoding it again could reinterpret private keys.
    let result = value.pointer_mut("/value/result").map(Value::take);
    let peer_data = if value
        .pointer("/value/error/data/presence")
        .and_then(Value::as_str)
        == Some("present")
    {
        value
            .pointer_mut("/value/error/data/value")
            .map(Value::take)
    } else {
        None
    };
    let mut decoded: ExchangeResult =
        serde_json::from_value(value).map_err(|_| internal_error())?;
    if let Some(result) = result {
        let ExchangeResult::V2(branch) = &mut decoded else {
            return Err(internal_error());
        };
        branch.value.result = result;
    }
    if let Some(data) = peer_data {
        let ExchangeResult::V0(branch) = &mut decoded else {
            return Err(internal_error());
        };
        let McpHttpObservationsPeerData::V1(present) = branch.value.error.data.as_mut() else {
            return Err(internal_error());
        };
        present.value = data;
    }
    Ok(decoded)
}

async fn receive(
    response: reqwest::Response,
    mut received: Reception,
    context: &ResponseContext,
) -> Result<ExchangeResult, ClientError> {
    let mut controls = Controls::default();
    let result = receive_inner(response, &mut received, context, &mut controls).await;
    let overflow = controls.finish(&mut received, context.end).await?;
    let mut result = if overflow {
        received.refuse("response_bound")?
    } else {
        result?
    };
    let observation = match &mut result {
        ExchangeResult::V0(branch) => &mut branch.value.observation,
        ExchangeResult::V1(branch) => &mut branch.value.observation,
        ExchangeResult::V2(branch) => &mut branch.value.observation,
    };
    if !received.stream.is_empty() {
        observation.stream = EssPresence::Present(Box::new(StreamObservation {
            messages: received.stream.into_iter().map(Box::new).collect(),
        }));
    }
    Ok(result)
}

async fn receive_inner(
    mut response: reqwest::Response,
    received: &mut Reception,
    context: &ResponseContext,
    controls: &mut Controls,
) -> Result<ExchangeResult, ClientError> {
    let ResponseContext {
        id,
        event_limit,
        end,
        semantics,
        ..
    } = context;
    let end = *end;
    let (media, encoded) = response_format(&response);
    let status = received.status.unwrap_or(0);
    let status_reason = match status {
        200..=299 => None,
        401 | 403 => Some("authorization_required"),
        404 if matches!(semantics, HttpSemantics::LegacySession) => Some("session_expired"),
        _ => Some("http_status"),
    };
    let is_sse = media == "text/event-stream" && status_reason.is_none() && !encoded;
    let mode = if !context.controlled {
        EventMode::Raw
    } else if matches!(semantics, HttpSemantics::Modern) {
        EventMode::Modern
    } else {
        EventMode::Legacy
    };
    let mut sse = Events::new(*event_limit, mode);
    loop {
        let chunk = match controls
            .drive(received, timeout_at(end, response.chunk()))
            .await?
        {
            Ok(Ok(chunk)) => chunk,
            Ok(Err(error)) => {
                return received.refuse(if error.is_timeout() {
                    "deadline_exhausted"
                } else {
                    "transport_failure"
                });
            }
            Err(_) => return received.refuse("deadline_exhausted"),
        };
        let Some(chunk) = chunk else {
            break;
        };
        if is_sse {
            for byte in chunk {
                if let Some(frame) = sse.byte(byte, received, id)? {
                    if let Some(result) = dispatch_frame(frame, received, context, controls)? {
                        return Ok(result);
                    }
                }
            }
        } else if !received.append(&chunk) {
            return received.refuse("response_bound");
        }
        if Instant::now() >= end {
            return received.refuse("deadline_exhausted");
        }
    }
    if is_sse {
        if let Some(frame) = sse.end(received, id)? {
            if let Some(result) = dispatch_frame(frame, received, context, controls)? {
                return Ok(result);
            }
        }
        // An EOF never dispatches a pending SSE event. Only line delimiters do.
        received.whole = false;
        return received.refuse("invalid_response");
    }
    received.whole = true;
    if let Some(reason) = status_reason {
        // Modern protocol errors use 400 and method-not-found uses 404. HTTP
        // failure is not enough to erase a complete correlated peer error, nor
        // can a success-shaped body promote an error status into success.
        if matches!(semantics, HttpSemantics::Modern)
            && matches!(status, 400 | 404)
            && !encoded
            && media == "application/json"
        {
            let result = received.envelope(id, false)?.ok_or_else(internal_error)?;
            if Instant::now() >= end {
                return received.refuse("deadline_exhausted");
            }
            if matches!(result, ExchangeResult::V0(_)) {
                return Ok(result);
            }
            received.terminal = false;
        }
        return received.refuse(reason);
    }
    if encoded || media != "application/json" {
        return received.refuse("unsupported_response");
    }
    let result = received.envelope(id, false)?.ok_or_else(internal_error)?;
    if Instant::now() >= end {
        received.refuse("deadline_exhausted")
    } else {
        Ok(result)
    }
}

fn response_format(response: &reqwest::Response) -> (String, bool) {
    let media = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let encoded = response
        .headers()
        .get_all("content-encoding")
        .iter()
        .any(|v| v != "identity");
    (media, encoded)
}

enum Frame {
    Terminal(ExchangeResult),
    Message(Value),
}
fn terminal_frame(result: ExchangeResult) -> Frame {
    Frame::Terminal(result)
}

fn dispatch_frame(
    frame: Frame,
    received: &mut Reception,
    context: &ResponseContext,
    controls: &mut Controls,
) -> Result<Option<ExchangeResult>, ClientError> {
    if Instant::now() >= context.end {
        return received.refuse("deadline_exhausted").map(Some);
    }
    let Frame::Message(message) = frame else {
        let Frame::Terminal(result) = frame else {
            unreachable!()
        };
        return Ok(Some(result));
    };
    let raw = received.wire_observation();
    if message.get("id").is_none() {
        received.stream.push(
            serde_json::from_value(json!({"kind":"notification","value":raw}))
                .map_err(|_| internal_error())?,
        );
        received.consume_message();
        return Ok(None);
    }
    let Some(control) = &context.control else {
        return received.refuse("invalid_response").map(Some);
    };
    controls.start(control, message, received, context.end)?;
    received.consume_message();
    Ok(None)
}

struct PendingReply {
    raw: Value,
    kind: &'static str,
    state: Arc<Mutex<Reception>>,
}
#[derive(Default)]
struct Controls {
    jobs: FuturesUnordered<BoxFuture<'static, (usize, Result<ControlObservation, ClientError>)>>,
    pending: BTreeMap<usize, PendingReply>,
    overflow: bool,
}
impl Controls {
    fn start(
        &mut self,
        port: &ControlPort,
        message: Value,
        received: &mut Reception,
        end: Instant,
    ) -> Result<(), ClientError> {
        let port = port.fork()?;
        let shared = received.shared.clone().ok_or_else(internal_error)?;
        let mut reception = Reception::new(shared.load(Ordering::Relaxed));
        reception.shared = Some(shared);
        let state = Arc::new(Mutex::new(reception));
        let kind = if message["method"] == "ping" {
            "ping_reply"
        } else {
            "unsupported_request_reply"
        };
        let raw = received.wire_observation();
        let index = received.stream.len();
        // Internal placeholder only; finish replaces every slot before a report
        // is returned. It reserves original message order independently of I/O.
        received.stream.push(
            serde_json::from_value(json!({"kind":"notification","value":raw}))
                .map_err(|_| internal_error())?,
        );
        self.pending.insert(
            index,
            PendingReply {
                raw,
                kind,
                state: state.clone(),
            },
        );
        self.jobs.push(Box::pin(async move {
            (index, port.reply(&message, &state, end).await)
        }));
        Ok(())
    }
    fn complete(
        &mut self,
        index: usize,
        reply: ControlObservation,
        received: &mut Reception,
    ) -> Result<(), ClientError> {
        let pending = self.pending.remove(&index).ok_or_else(internal_error)?;
        let reply = serde_json::to_value(reply).map_err(|_| internal_error())?;
        self.overflow |= reply["refusal"] == "response_bound";
        received.stream[index] = serde_json::from_value(
            json!({"kind":"server_request","value":{"request":pending.raw,"reply":reply}}),
        )
        .map_err(|_| internal_error())?;
        Ok(())
    }
    async fn drive<F: Future>(
        &mut self,
        received: &mut Reception,
        read: F,
    ) -> Result<F::Output, ClientError> {
        tokio::pin!(read);
        loop {
            tokio::select! {
                result = &mut read => return Ok(result),
                Some((index, reply)) = self.jobs.next(), if !self.jobs.is_empty() => {
                    self.complete(index, reply?, received)?;
                }
            }
        }
    }
    async fn finish(
        &mut self,
        received: &mut Reception,
        end: Instant,
    ) -> Result<bool, ClientError> {
        while !self.jobs.is_empty() {
            match timeout_at(end, self.jobs.next()).await {
                Ok(Some((index, reply))) => self.complete(index, reply?, received)?,
                Ok(None) | Err(_) => break,
            }
        }
        // The exchange owns these futures directly. Dropping it or reaching its
        // deadline drops every pending reply; no library control task survives.
        drop(std::mem::take(&mut self.jobs));
        let unresolved: Vec<_> = self.pending.keys().copied().collect();
        for index in unresolved {
            let pending = &self.pending[&index];
            let reply =
                control_observation(&pending.state, pending.kind, Err("deadline_exhausted"))?;
            self.complete(index, reply, received)?;
        }
        Ok(self.overflow)
    }
}

#[derive(Clone, Copy)]
enum StreamStart {
    Initial,
    Passed,
}
enum LineEnding {
    None,
    PendingCr,
    SkipLf(Option<usize>),
}
#[derive(Clone, Copy)]
enum EventMode {
    Raw,
    Modern,
    Legacy,
}

struct Events {
    limit: usize,
    octets: usize,
    line: Vec<u8>,
    event_type: Vec<u8>,
    has_data: bool,
    line_data_started: bool,
    ending: LineEnding,
    stream_start: StreamStart,
    mode: EventMode,
}
impl Events {
    fn new(limit: usize, mode: EventMode) -> Self {
        Self {
            limit,
            octets: 0,
            line: Vec::new(),
            event_type: Vec::new(),
            has_data: false,
            line_data_started: false,
            ending: LineEnding::None,
            stream_start: StreamStart::Initial,
            mode,
        }
    }
    fn byte(
        &mut self,
        byte: u8,
        received: &mut Reception,
        id: &Value,
    ) -> Result<Option<Frame>, ClientError> {
        let ending = std::mem::replace(&mut self.ending, LineEnding::None);
        if let LineEnding::SkipLf(previous) = ending {
            if byte == b'\n' {
                let count = previous.unwrap_or(self.octets).saturating_add(1);
                if count > self.limit {
                    return received
                        .refuse("sse_event_bound")
                        .map(terminal_frame)
                        .map(Some);
                }
                if previous.is_none() {
                    self.octets = count;
                }
                return Ok(None);
            }
        }
        if matches!(ending, LineEnding::PendingCr) {
            if byte == b'\n' {
                self.octets = self.octets.saturating_add(1);
                if self.octets > self.limit {
                    return received
                        .refuse("sse_event_bound")
                        .map(terminal_frame)
                        .map(Some);
                }
                return self.line(received, id);
            }
            if let Some(result) = self.line(received, id)? {
                return Ok(Some(result));
            }
        }
        self.octets = self.octets.saturating_add(1);
        if self.octets > self.limit {
            return received
                .refuse("sse_event_bound")
                .map(terminal_frame)
                .map(Some);
        }
        match byte {
            // A bare CR completes a line. Waiting for the next byte can
            // deadlock a legacy peer waiting for its ping reply. A following
            // LF is suppressed as a second delimiter but still charged, even
            // when it arrives in a later network chunk after a control reply.
            b'\r' if !matches!(self.mode, EventMode::Raw) => {
                self.ending = LineEnding::SkipLf(self.line.is_empty().then_some(self.octets));
                return self.line(received, id);
            }
            b'\r' => self.ending = LineEnding::PendingCr,
            b'\n' => return self.line(received, id),
            _ => {
                self.line.push(byte);
                let field_line = without_bom(&self.line, self.stream_start);
                if field_line.starts_with(b"data:") {
                    if !self.line_data_started {
                        self.line_data_started = true;
                        if self.has_data && !received.append(b"\n") {
                            return received
                                .refuse("response_bound")
                                .map(terminal_frame)
                                .map(Some);
                        }
                        self.has_data = true;
                    }
                    if field_line.len() > 5
                        && !(field_line.len() == 6 && byte == b' ')
                        && !received.append(&[byte])
                    {
                        return received
                            .refuse("response_bound")
                            .map(terminal_frame)
                            .map(Some);
                    }
                }
            }
        }
        Ok(None)
    }
    fn line(&mut self, received: &mut Reception, id: &Value) -> Result<Option<Frame>, ClientError> {
        let line = std::mem::take(&mut self.line);
        let line = without_bom(
            &line,
            std::mem::replace(&mut self.stream_start, StreamStart::Passed),
        );
        let data_started = std::mem::replace(&mut self.line_data_started, false);
        if line.is_empty() {
            self.octets = 0;
            if self.has_data {
                received.whole = true;
                if !self.event_type.is_empty() && self.event_type != b"message" {
                    return received
                        .refuse("unsupported_response")
                        .map(terminal_frame)
                        .map(Some);
                }
                // Legacy Streamable HTTP explicitly permits an empty data
                // priming event. It carries no JSON-RPC message or authority.
                if matches!(self.mode, EventMode::Legacy) && received.bytes.is_empty() {
                    self.event_type.clear();
                    self.has_data = false;
                    received.whole = false;
                    return Ok(None);
                }
                if !matches!(self.mode, EventMode::Raw) {
                    if let Some(message) =
                        parse(&received.bytes).filter(|v| v.get("method").is_some())
                    {
                        if !valid_message(&message) {
                            return received
                                .refuse("invalid_response")
                                .map(terminal_frame)
                                .map(Some);
                        }
                        self.event_type.clear();
                        self.has_data = false;
                        return Ok(Some(Frame::Message(message)));
                    }
                }
                if let Some(result) = received.envelope(id, true)? {
                    return Ok(Some(terminal_frame(result)));
                }
                // Completed notifications are not the pending terminal message.
                received.bytes.clear();
                received.seen = 0;
                received.whole = false;
            }
            self.event_type.clear();
            self.has_data = false;
        } else {
            let (field, value) = line
                .iter()
                .position(|b| *b == b':')
                .map_or((line, &[][..]), |i| (&line[..i], &line[i + 1..]));
            let value = value.strip_prefix(b" ").unwrap_or(value);
            if field == b"data" && !data_started {
                if self.has_data && !received.append(b"\n") {
                    return received
                        .refuse("response_bound")
                        .map(terminal_frame)
                        .map(Some);
                }
                self.has_data = true;
                if !received.append(value) {
                    return received
                        .refuse("response_bound")
                        .map(terminal_frame)
                        .map(Some);
                }
            } else if field == b"event" {
                self.event_type = value.to_vec();
            }
        }
        Ok(None)
    }
    fn end(&mut self, received: &mut Reception, id: &Value) -> Result<Option<Frame>, ClientError> {
        if matches!(self.ending, LineEnding::PendingCr) {
            self.ending = LineEnding::None;
            return self.line(received, id);
        }
        Ok(None)
    }
}

fn valid_message(message: &Value) -> bool {
    message["jsonrpc"] == "2.0"
        && message["method"].as_str().is_some_and(|m| !m.is_empty())
        && message.get("id").is_none_or(valid_id)
        && message.get("result").is_none()
        && message.get("error").is_none()
        && message
            .get("params")
            .is_none_or(|p| p.is_object() || p.is_array())
}

fn without_bom(line: &[u8], stream_start: StreamStart) -> &[u8] {
    if matches!(stream_start, StreamStart::Initial) {
        line.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(line)
    } else {
        line
    }
}

fn prepare_request(
    request: &mut reqwest::Request,
    body: Vec<u8>,
    revision: &str,
) -> Result<(), &'static str> {
    if request.method() != reqwest::Method::POST
        || !matches!(request.url().scheme(), "http" | "https")
        || request.url().fragment().is_some()
        || !request.url().username().is_empty()
        || request.url().password().is_some()
        || request
            .body()
            .is_some_and(|b| b.as_bytes() != Some(body.as_slice()))
        || request.headers().contains_key("transfer-encoding")
        || request.headers().get("content-length").is_some_and(|v| {
            v.to_str().ok().and_then(|v| v.parse::<usize>().ok()) != Some(body.len())
        })
        || request
            .headers()
            .get("mcp-protocol-version")
            .is_some_and(|v| v != revision)
        || request.headers().contains_key("content-encoding")
    {
        return Err("invalid_input");
    }
    *request.body_mut() = Some(body.into());
    request.headers_mut().insert(
        "content-type",
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    request.headers_mut().insert(
        "accept",
        reqwest::header::HeaderValue::from_static("application/json, text/event-stream"),
    );
    request.headers_mut().insert(
        "accept-encoding",
        reqwest::header::HeaderValue::from_static("identity"),
    );
    request.headers_mut().insert(
        "mcp-protocol-version",
        reqwest::header::HeaderValue::from_str(revision).map_err(|_| "invalid_input")?,
    );
    Ok(())
}
