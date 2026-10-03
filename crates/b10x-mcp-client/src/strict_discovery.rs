//! Complete, bounded descriptor observations on an existing strict connection.
//!
//! Catalogs are untrusted observations, not invocation admission or JSON Schema
//! semantic validation. Generated Debug includes peer data and is not safe logging.
use crate::strict_connection::StrictConnection;
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpConnectionCacheHints as CacheHints, McpHttpDiscoveryCatalog as Catalog,
    McpHttpDiscoveryDescriptor as Descriptor, McpHttpDiscoveryFamily as Family,
    McpHttpDiscoveryListLimits as ListLimits, McpHttpDiscoveryPage as Page,
    McpHttpDiscoveryPromptArgument as PromptArgument, McpHttpDiscoveryRefusal as Refusal,
    McpHttpDiscoveryRefusalReason as Reason, McpHttpExchangeCompleteResult as CompleteResult,
    McpHttpExchangeExchangeResult as ExchangeResult,
    McpHttpObservationsProtocolRevision as Revision,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use tokio::time::Instant;

type Fault = (Reason, Option<usize>, Option<usize>);
fn fault(reason: Reason) -> Fault {
    (reason, None, None)
}
fn refusal((reason, actual, limit): Fault, exchanges: Vec<ExchangeResult>) -> Refusal {
    Refusal {
        reason: Box::new(reason),
        exchanges: exchanges.into_iter().map(Box::new).collect(),
        actual: actual.map_or(EssPresence::Absent, |v| EssPresence::Present(v.into())),
        limit: limit.map_or(EssPresence::Absent, |v| EssPresence::Present(v.into())),
    }
}
fn number(n: &serde_json::Number) -> Option<usize> {
    n.as_u64().and_then(|n| usize::try_from(n).ok())
}
fn label(family: &Family) -> &'static str {
    match family {
        Family::V0 => "prompts",
        Family::V1 => "resources",
        Family::V2 => "tools",
    }
}
fn supported(connection: &StrictConnection, family: &Family) -> bool {
    let caps = &connection.description().capabilities;
    match family {
        Family::V0 => !caps.prompts.is_absent(),
        Family::V1 => !caps.resources.is_absent(),
        Family::V2 => !caps.tools.is_absent(),
    }
}
/// Traverse one selected family with a single deadline and cumulative limits.
///
/// Zero pages sends no request. Empty cursors continue unchanged. A refusal retains
/// actual bounded exchanges but never returns an accumulated prefix as a catalog.
/// Descriptor bytes are compact JSON encoding; response wire bytes are separately
/// bounded by the existing connection budget. No cache or authority is established.
pub async fn discover(
    connection: &mut StrictConnection,
    family: Family,
    limits: &ListLimits,
    deadline: Instant,
) -> Result<Catalog, Refusal> {
    let invalid = || refusal(fault(Reason::V5), vec![]);
    let pages = number(&limits.max_pages).ok_or_else(invalid)?;
    let items = number(&limits.max_items).ok_or_else(invalid)?;
    let descriptor_limit = number(&limits.descriptor_octets.0).ok_or_else(invalid)?;
    let end = connection
        .traversal_deadline(deadline)
        .ok_or_else(invalid)?;
    if !supported(connection, &family) {
        return Err(refusal(fault(Reason::V9), vec![]));
    }
    let modern = matches!(
        connection.description().configured_revision.as_ref(),
        Revision::V1
    );
    let mut catalog = Catalog {
        family: Box::new(family.clone()),
        descriptors: vec![],
        pages: vec![],
    };
    let mut observed = Vec::new();
    let mut cursor = None;
    let mut identities = BTreeSet::new();
    loop {
        if Instant::now() >= end {
            return Err(refusal(fault(Reason::V0), observed));
        }
        if catalog.pages.len() >= pages {
            return Err(refusal(
                (
                    Reason::V8,
                    Some(catalog.pages.len().saturating_add(1)),
                    Some(pages),
                ),
                observed,
            ));
        }
        let params = cursor
            .take()
            .map_or_else(|| json!({}), |value| json!({"cursor":value}));
        let exchange = connection
            .exchange(&format!("{}/list", label(&family)), params, end)
            .await
            .map_err(|_| refusal(fault(Reason::V3), observed.clone()))?;
        observed.push(exchange.clone());
        let complete =
            complete(exchange).map_err(|reason| refusal(fault(reason), observed.clone()))?;
        let (page, rows) = parse_page(complete, &family, modern)
            .map_err(|e| refusal(fault(e), observed.clone()))?;
        for raw in rows {
            let actual = catalog.descriptors.len().saturating_add(1);
            if actual > items {
                return Err(refusal((Reason::V7, Some(actual), Some(items)), observed));
            }
            let bytes = serde_json::to_vec(&raw)
                .map_err(|_| refusal(fault(Reason::V4), observed.clone()))?
                .len();
            if bytes > descriptor_limit {
                return Err(refusal(
                    (Reason::V1, Some(bytes), Some(descriptor_limit)),
                    observed,
                ));
            }
            let descriptor = descriptor(&raw, &family, modern)
                .map_err(|e| refusal(fault(e), observed.clone()))?;
            let identity = match &descriptor {
                Descriptor::V0(v) => &v.value.name,
                Descriptor::V1(v) => &v.value.uri,
                Descriptor::V2(v) => &v.value.name,
            };
            if !identities.insert(identity.clone()) {
                return Err(refusal(fault(Reason::V2), observed));
            }
            catalog.descriptors.push(Box::new(descriptor));
        }
        cursor = match &page.next_cursor {
            EssPresence::Present(value) => Some(value.clone()),
            EssPresence::Absent => None,
        };
        catalog.pages.push(Box::new(page));
        if Instant::now() >= end {
            return Err(refusal(fault(Reason::V0), observed));
        }
        if cursor.is_none() {
            return Ok(catalog);
        }
    }
}
fn complete(exchange: ExchangeResult) -> Result<CompleteResult, Reason> {
    match exchange {
        ExchangeResult::V2(result) => Ok(*result.value),
        ExchangeResult::V1(result)
            if matches!(
                result.value.reason.as_ref(),
                b10x_mcp_types::http_exchange::McpHttpExchangeRefusalReason::V1
            ) =>
        {
            Err(Reason::V0)
        }
        _ => Err(Reason::V3),
    }
}
fn cache(raw: &Value) -> Result<CacheHints, Reason> {
    let ttl = raw["ttlMs"].as_number().ok_or(Reason::V6)?;
    if !ttl.to_string().bytes().all(|b| b.is_ascii_digit()) {
        return Err(Reason::V6);
    }
    serde_json::from_value(json!({"ttl_ms":ttl,"scope":raw["cacheScope"]})).map_err(|_| Reason::V6)
}
fn parse_page(
    exchange: CompleteResult,
    family: &Family,
    modern: bool,
) -> Result<(Page, Vec<Value>), Reason> {
    let raw = &exchange.result;
    if !raw.is_object() || raw.get("_meta").is_some_and(|v| !v.is_object()) {
        return Err(Reason::V6);
    }
    let cache_hints = if modern {
        match raw.get("resultType").and_then(Value::as_str) {
            Some("complete") => {}
            Some(_) => return Err(Reason::V10),
            None => return Err(Reason::V6),
        }
        EssPresence::Present(Box::new(cache(raw)?))
    } else {
        EssPresence::Absent
    };
    let next_cursor = optional_string(raw, "nextCursor").map_err(|_| Reason::V6)?;
    let rows = raw[label(family)].as_array().ok_or(Reason::V6)?.clone();
    Ok((
        Page {
            exchange: Box::new(exchange),
            next_cursor,
            cache_hints,
        },
        rows,
    ))
}
fn optional_string(raw: &Value, key: &str) -> Result<EssPresence<String>, Reason> {
    raw.get(key).map_or(Ok(EssPresence::Absent), |v| {
        v.as_str()
            .map(|s| EssPresence::Present(s.to_owned()))
            .ok_or(Reason::V4)
    })
}
fn name(raw: &Value) -> Result<&str, Reason> {
    raw["name"].as_str().ok_or(Reason::V4)
}
fn base(raw: &Value) -> Result<Value, Reason> {
    let mut value = json!({"name":name(raw)?,"raw":null});
    for key in ["title", "description"] {
        if let EssPresence::Present(v) = optional_string(raw, key)? {
            value[key] = json!(v);
        }
    }
    if raw.get("_meta").is_some_and(|v| !v.is_object()) {
        return Err(Reason::V4);
    }
    Ok(value)
}
fn schema(raw: &Value, object_root: bool, legacy: bool) -> Result<(), Reason> {
    if !raw.is_object()
        || (object_root && raw["type"] != "object")
        || raw.get("$schema").is_some_and(|v| !v.is_string())
    {
        return Err(Reason::V4);
    }
    if legacy
        && (raw.get("properties").is_some_and(|v| {
            v.as_object()
                .is_none_or(|p| !p.values().all(Value::is_object))
        }) || raw
            .get("required")
            .is_some_and(|v| v.as_array().is_none_or(|a| !a.iter().all(Value::is_string))))
    {
        return Err(Reason::V4);
    }
    Ok(())
}
fn descriptor(raw: &Value, family: &Family, modern: bool) -> Result<Descriptor, Reason> {
    let mut value = base(raw)?;
    icons(raw)?;
    annotations(raw, family)?;
    match family {
        Family::V2 => {
            schema(&raw["inputSchema"], true, !modern)?;
            if let Some(output) = raw.get("outputSchema") {
                schema(output, !modern, !modern)?;
            }
            if !modern {
                legacy_execution(raw)?;
            }
            value["input_schema"] = Value::Null;
            let mut descriptor: Descriptor =
                serde_json::from_value(json!({"kind":"tool","value":value}))
                    .map_err(|_| Reason::V4)?;
            if let Descriptor::V2(v) = &mut descriptor {
                v.value.input_schema = raw["inputSchema"].clone();
                v.value.output_schema = raw
                    .get("outputSchema")
                    .cloned()
                    .map_or(EssPresence::Absent, EssPresence::Present);
                v.value.raw = raw.clone();
            }
            Ok(descriptor)
        }
        Family::V1 => {
            let uri = raw["uri"].as_str().ok_or(Reason::V4)?;
            reqwest::Url::parse(uri).map_err(|_| Reason::V4)?;
            value["uri"] = json!(uri);
            if let EssPresence::Present(mime) = optional_string(raw, "mimeType")? {
                value["mime_type"] = json!(mime);
            }
            if raw.get("size").is_some_and(|v| !v.is_number()) {
                return Err(Reason::V4);
            }
            let mut descriptor: Descriptor =
                serde_json::from_value(json!({"kind":"resource","value":value}))
                    .map_err(|_| Reason::V4)?;
            if let Descriptor::V1(v) = &mut descriptor {
                v.value.size = raw
                    .get("size")
                    .cloned()
                    .map_or(EssPresence::Absent, EssPresence::Present);
                v.value.raw = raw.clone();
            }
            Ok(descriptor)
        }
        Family::V0 => {
            let arguments = arguments(raw)?;
            let mut descriptor: Descriptor =
                serde_json::from_value(json!({"kind":"prompt","value":value}))
                    .map_err(|_| Reason::V4)?;
            if let Descriptor::V0(v) = &mut descriptor {
                v.value.arguments = match arguments {
                    EssPresence::Absent => EssPresence::Absent,
                    EssPresence::Present(args) => {
                        EssPresence::Present(args.into_iter().map(Box::new).collect())
                    }
                };
                v.value.raw = raw.clone();
            }
            Ok(descriptor)
        }
    }
}
fn arguments(raw: &Value) -> Result<EssPresence<Vec<PromptArgument>>, Reason> {
    let Some(raw) = raw.get("arguments") else {
        return Ok(EssPresence::Absent);
    };
    let rows = raw.as_array().ok_or(Reason::V4)?;
    let mut names = BTreeSet::new();
    let mut args = Vec::new();
    for row in rows {
        let mut value = base(row)?;
        if !names.insert(name(row)?.to_owned()) {
            return Err(Reason::V2);
        }
        if let Some(required) = row.get("required") {
            value["required"] = json!(required.as_bool().ok_or(Reason::V4)?);
        }
        let mut arg: PromptArgument = serde_json::from_value(value).map_err(|_| Reason::V4)?;
        arg.raw = row.clone();
        args.push(arg);
    }
    Ok(EssPresence::Present(args))
}
fn icons(raw: &Value) -> Result<(), Reason> {
    if let Some(icons) = raw.get("icons") {
        for icon in icons.as_array().ok_or(Reason::V4)? {
            icon["src"].as_str().ok_or(Reason::V4)?;
            optional_string(icon, "mimeType")?;
            if icon
                .get("sizes")
                .is_some_and(|v| v.as_array().is_none_or(|a| !a.iter().all(Value::is_string)))
                || icon
                    .get("theme")
                    .is_some_and(|v| !matches!(v.as_str(), Some("light" | "dark")))
            {
                return Err(Reason::V4);
            }
        }
    }
    Ok(())
}
fn annotations(raw: &Value, family: &Family) -> Result<(), Reason> {
    // Prompt has no standard annotations field in either selected schema.
    if matches!(family, Family::V0) {
        return Ok(());
    }
    let Some(hints) = raw.get("annotations") else {
        return Ok(());
    };
    if !hints.is_object() {
        return Err(Reason::V4);
    }
    if matches!(family, Family::V2) {
        optional_string(hints, "title")?;
        for key in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            if hints.get(key).is_some_and(|v| !v.is_boolean()) {
                return Err(Reason::V4);
            }
        }
    } else if matches!(family, Family::V1) {
        optional_string(hints, "lastModified")?;
        if hints.get("audience").is_some_and(|v| {
            v.as_array().is_none_or(|a| {
                !a.iter()
                    .all(|r| matches!(r.as_str(), Some("user" | "assistant")))
            })
        }) {
            return Err(Reason::V4);
        }
        if hints
            .get("priority")
            .is_some_and(|v| v.as_f64().is_none_or(|n| !(0.0..=1.0).contains(&n)))
        {
            return Err(Reason::V4);
        }
    }
    Ok(())
}
fn legacy_execution(raw: &Value) -> Result<(), Reason> {
    if let Some(execution) = raw.get("execution") {
        if !execution.is_object()
            || execution
                .get("taskSupport")
                .is_some_and(|v| !matches!(v.as_str(), Some("forbidden" | "optional" | "required")))
        {
            return Err(Reason::V4);
        }
    }
    Ok(())
}
