//! Typed invocation over privately owned, same-connection discovery observations.
//! The consumer still owns grants. Peer content and generated Debug are untrusted.
use crate::{
    parameter_headers, schema_worker::SchemaWorker, strict_connection::StrictConnection,
    strict_discovery,
};
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpDiscoveryCatalog as Catalog, McpHttpDiscoveryDescriptor as Descriptor,
    McpHttpDiscoveryFamily as Family, McpHttpDiscoveryListLimits as ListLimits,
    McpHttpDiscoveryRefusal as DiscoveryRefusal, McpHttpDiscoveryTool as Tool,
    McpHttpExchangeCompleteResult as Complete, McpHttpExchangeExchangeResult as Exchange,
    McpHttpInvocationContent as Content, McpHttpInvocationParameterHeader as Parameter,
    McpHttpInvocationPromptMessage as Message, McpHttpInvocationPromptResult as PromptResult,
    McpHttpInvocationRefusal as Refusal, McpHttpInvocationRefusalReason as Reason,
    McpHttpInvocationRejectedTool as RejectedTool,
    McpHttpInvocationResourceContent as ResourceContent,
    McpHttpInvocationResourceResult as ResourceResult, McpHttpInvocationSchemaAction as Action,
    McpHttpInvocationSchemaRequest as SchemaRequest, McpHttpInvocationSchemaStatus as Status,
    McpHttpInvocationToolResult as ToolResult, McpHttpObservationsProtocolRevision as Revision,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio::time::Instant;

/// Owns a connection and the observations obtained through it. Public decoded
/// catalogs cannot be imported as invocation admission. No grant is created here.
pub struct InvocationClient {
    connection: StrictConnection,
    worker: SchemaWorker,
    catalogs: Vec<Catalog>,
    tools: BTreeMap<String, (Tool, Vec<Parameter>)>,
    rejected: Vec<RejectedTool>,
}
fn refusal(reason: Reason) -> Refusal {
    Refusal {
        reason: Box::new(reason),
        exchange: EssPresence::Absent,
    }
}
fn observed(reason: Reason, exchange: &Exchange) -> Refusal {
    Refusal {
        reason: Box::new(reason),
        exchange: EssPresence::Present(Box::new(exchange.clone())),
    }
}
fn in_time(end: Instant) -> Result<(), Reason> {
    if Instant::now() >= end {
        Err(Reason::V0)
    } else {
        Ok(())
    }
}
impl InvocationClient {
    /// Take ownership; each desired family must then be explicitly discovered.
    pub fn new(connection: StrictConnection, worker: SchemaWorker) -> Self {
        Self {
            connection,
            worker,
            catalogs: Vec::new(),
            tools: BTreeMap::new(),
            rejected: Vec::new(),
        }
    }
    fn modern(&self) -> bool {
        matches!(
            self.connection.description().configured_revision.as_ref(),
            Revision::V1
        )
    }
    /// Discover one selected family, invalidating any older observation first.
    /// A failed refresh never enables stale invocation. Raw pages remain in the
    /// catalog; its tool descriptors exclude invalid modern header annotations.
    pub async fn discover(
        &mut self,
        family: Family,
        limits: &ListLimits,
        deadline: Instant,
    ) -> Result<Catalog, DiscoveryRefusal> {
        self.catalogs.retain(|c| c.family.as_ref() != &family);
        if matches!(family, Family::V2) {
            self.tools.clear();
            self.rejected.clear();
        }
        let mut catalog =
            strict_discovery::discover(&mut self.connection, family.clone(), limits, deadline)
                .await?;
        if matches!(family, Family::V2) {
            for descriptor in &catalog.descriptors {
                if let Descriptor::V2(tool) = descriptor.as_ref() {
                    let projection = if self.modern() {
                        parameter_headers::project(&tool.value.input_schema)
                    } else {
                        Ok(Vec::new())
                    };
                    match projection {
                        Ok(parameters) => {
                            self.tools.insert(
                                tool.value.name.clone(),
                                ((*tool.value).clone(), parameters),
                            );
                        }
                        Err(reason) => self.rejected.push(RejectedTool {
                            name: tool.value.name.clone(),
                            reason: Box::new(reason),
                        }),
                    }
                }
            }
            catalog.descriptors.retain(|d| matches!(d.as_ref(), Descriptor::V2(t) if self.tools.contains_key(&t.value.name)));
        }
        self.catalogs.push(catalog.clone());
        Ok(catalog)
    }
    /// Usable tool descriptors; raw observations and excluded tools confer no authority.
    pub fn tools(&self) -> impl Iterator<Item = &Tool> {
        self.tools.values().map(|(tool, _)| tool)
    }
    /// Invalid modern header annotations, retained without logging peer names.
    pub fn rejected_tools(&self) -> &[RejectedTool] {
        &self.rejected
    }
    fn catalog(&self, family: &Family) -> Result<&Catalog, Refusal> {
        self.catalogs
            .iter()
            .find(|c| c.family.as_ref() == family)
            .ok_or_else(|| refusal(Reason::V8))
    }
    fn end(&self, deadline: Instant) -> Result<Instant, Refusal> {
        let end = self
            .connection
            .traversal_deadline(deadline)
            .ok_or_else(|| refusal(Reason::V2))?;
        in_time(end).map_err(refusal)?;
        Ok(end)
    }
    async fn schema(
        &mut self,
        schema: Value,
        instance: EssPresence<Value>,
        end: Instant,
        mismatch: Reason,
    ) -> Result<(), Reason> {
        let action = if instance.is_absent() {
            Action::V0
        } else {
            Action::V1
        };
        let result = self
            .worker
            .run(
                &SchemaRequest {
                    action: Box::new(action),
                    schema,
                    instance,
                },
                end,
            )
            .await?;
        match result.status.as_ref() {
            Status::V4 => Ok(()),
            Status::V0 => Err(mismatch),
            Status::V2 | Status::V3 => Err(Reason::V10),
            _ => Err(Reason::V5),
        }
    }
    /// Validate arguments, dispatch once, then validate and preserve the full result.
    /// A business isError result is data; output schema failure is never rollback.
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Instant,
    ) -> Result<ToolResult, Refusal> {
        let end = self.end(deadline)?;
        self.catalog(&Family::V2)?;
        let (tool, parameters) = self
            .tools
            .get(name)
            .cloned()
            .ok_or_else(|| refusal(Reason::V6))?;
        if !arguments.is_object() {
            return Err(refusal(Reason::V2));
        }
        let headers = parameter_headers::extract(&parameters, &arguments).map_err(refusal)?;
        self.schema(
            tool.input_schema,
            EssPresence::Present(arguments.clone()),
            end,
            Reason::V2,
        )
        .await
        .map_err(refusal)?;
        if let EssPresence::Present(schema) = &tool.output_schema {
            self.schema(schema.clone(), EssPresence::Absent, end, Reason::V4)
                .await
                .map_err(refusal)?;
        }
        let exchange = self
            .connection
            .exchange_with_parameters(
                "tools/call",
                json!({"name":name,"arguments":arguments}),
                headers,
                end,
            )
            .await
            .map_err(|_| refusal(Reason::V2))?;
        let result = parse_tool(&exchange, self.modern()).map_err(|r| observed(r, &exchange))?;
        if let EssPresence::Present(schema) = tool.output_schema {
            let EssPresence::Present(instance) = &result.structured_content else {
                return Err(observed(Reason::V4, &exchange));
            };
            self.schema(
                schema,
                EssPresence::Present(instance.clone()),
                end,
                Reason::V4,
            )
            .await
            .map_err(|r| {
                // The business response already arrived. Worker IPC capacity is
                // unavailable validation, never a new caller-input defect.
                observed(
                    if matches!(r, Reason::V2) {
                        Reason::V5
                    } else {
                        r
                    },
                    &exchange,
                )
            })?;
        }
        in_time(end).map_err(|r| observed(r, &exchange))?;
        Ok(result)
    }
    /// Read a discovered URI once, with no cache or stale-success fallback.
    pub async fn read_resource(
        &mut self,
        uri: &str,
        deadline: Instant,
    ) -> Result<ResourceResult, Refusal> {
        let end = self.end(deadline)?;
        if !self
            .catalog(&Family::V1)?
            .descriptors
            .iter()
            .any(|d| matches!(d.as_ref(),Descriptor::V1(r) if r.value.uri == uri))
        {
            return Err(refusal(Reason::V6));
        }
        let exchange = self
            .connection
            .exchange("resources/read", json!({"uri":uri}), end)
            .await
            .map_err(|_| refusal(Reason::V2))?;
        let result =
            parse_resource(&exchange, self.modern()).map_err(|r| observed(r, &exchange))?;
        in_time(end).map_err(|r| observed(r, &exchange))?;
        Ok(result)
    }
    /// Fetch a discovered prompt with only declared string arguments. Returned
    /// instructions and links stay data; this operation executes neither.
    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Instant,
    ) -> Result<PromptResult, Refusal> {
        let end = self.end(deadline)?;
        let prompt = self
            .catalog(&Family::V0)?
            .descriptors
            .iter()
            .find_map(|d| match d.as_ref() {
                Descriptor::V0(p) if p.value.name == name => Some(p.value.as_ref()),
                _ => None,
            })
            .ok_or_else(|| refusal(Reason::V6))?;
        let args = arguments.as_object().ok_or_else(|| refusal(Reason::V2))?;
        let declared = match &prompt.arguments {
            EssPresence::Present(a) => a.as_slice(),
            EssPresence::Absent => &[],
        };
        if args
            .iter()
            .any(|(name, value)| !value.is_string() || !declared.iter().any(|a| &a.name == name))
            || declared.iter().any(|a| {
                matches!(a.required, EssPresence::Present(true)) && !args.contains_key(&a.name)
            })
        {
            return Err(refusal(Reason::V2));
        }
        let exchange = self
            .connection
            .exchange(
                "prompts/get",
                json!({"name":name,"arguments":arguments}),
                end,
            )
            .await
            .map_err(|_| refusal(Reason::V2))?;
        let result = parse_prompt(&exchange, self.modern()).map_err(|r| observed(r, &exchange))?;
        in_time(end).map_err(|r| observed(r, &exchange))?;
        Ok(result)
    }
}

fn complete(exchange: &Exchange, modern: bool) -> Result<&Complete, Reason> {
    let result = match exchange {
        Exchange::V2(v) => v.value.as_ref(),
        Exchange::V1(v) => {
            use b10x_mcp_types::http_exchange::McpHttpExchangeRefusalReason as ExchangeReason;
            return Err(match v.value.reason.as_ref() {
                ExchangeReason::V1 => Reason::V0,
                ExchangeReason::V3 | ExchangeReason::V5 => Reason::V2,
                _ => Reason::V1,
            });
        }
        Exchange::V0(_) => return Err(Reason::V1),
    };
    if !result.result.is_object() || result.result.get("_meta").is_some_and(|v| !v.is_object()) {
        return Err(Reason::V3);
    }
    if modern {
        match result.result.get("resultType").and_then(Value::as_str) {
            Some("complete") => {}
            Some(_) => return Err(Reason::V9),
            None => return Err(Reason::V3),
        }
    }
    Ok(result)
}
fn string(raw: &Value, key: &str) -> Result<String, Reason> {
    raw.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(Reason::V3)
}
fn optional_string(raw: &Value, key: &str) -> Result<EssPresence<String>, Reason> {
    raw.get(key).map_or(Ok(EssPresence::Absent), |_| {
        string(raw, key).map(EssPresence::Present)
    })
}
fn resource_content(raw: &Value) -> Result<ResourceContent, Reason> {
    if raw.get("_meta").is_some_and(|v| !v.is_object()) {
        return Err(Reason::V3);
    }
    let uri = string(raw, "uri")?;
    reqwest::Url::parse(&uri).map_err(|_| Reason::V3)?;
    let text = optional_string(raw, "text")?;
    let blob = optional_string(raw, "blob")?;
    if text.is_absent() && blob.is_absent() {
        return Err(Reason::V3);
    }
    if let EssPresence::Present(blob) = &blob {
        STANDARD.decode(blob).map_err(|_| Reason::V3)?;
    }
    Ok(ResourceContent {
        uri,
        text,
        blob,
        mime_type: optional_string(raw, "mimeType")?,
        raw: raw.clone(),
    })
}
fn content(raw: &Value, modern: bool) -> Result<Content, Reason> {
    if raw.get("_meta").is_some_and(|v| !v.is_object()) {
        return Err(Reason::V3);
    }
    strict_discovery::annotations(raw, &Family::V1).map_err(|_| Reason::V3)?;
    let kind = raw.get("type").and_then(Value::as_str).ok_or(Reason::V3)?;
    let value = match kind {
        "text" => json!({"text":string(raw,"text")?,"raw":null}),
        "audio" | "image" => {
            let data = string(raw, "data")?;
            STANDARD.decode(&data).map_err(|_| Reason::V3)?;
            json!({"data":data,"mime_type":string(raw,"mimeType")?,"raw":null})
        }
        "resource" => {
            json!({"resource":{"uri":"placeholder:resource","text":"","raw":null},"raw":null})
        }
        "resource_link" => {
            json!({"resource":{"name":"","uri":"placeholder:resource","raw":null},"raw":null})
        }
        _ => return Err(Reason::V7),
    };
    // Decode structural fields only; opaque values are assigned without a second
    // serde Value deserialization (which interprets private number-marker keys).
    let mut content: Content =
        serde_json::from_value(json!({"kind":kind,"value":value})).map_err(|_| Reason::V3)?;
    match &mut content {
        Content::V0(v) => v.value.raw = raw.clone(),
        Content::V1(v) => v.value.raw = raw.clone(),
        Content::V2(v) => {
            v.value.resource = Box::new(resource_content(&raw["resource"])?);
            v.value.raw = raw.clone();
        }
        Content::V3(v) => {
            let Descriptor::V1(resource) =
                strict_discovery::descriptor(raw, &Family::V1, modern).map_err(|_| Reason::V3)?
            else {
                return Err(Reason::V3);
            };
            v.value.resource = resource.value;
            v.value.raw = raw.clone();
        }
        Content::V4(v) => v.value.raw = raw.clone(),
    }
    Ok(content)
}
fn parse_tool(exchange: &Exchange, modern: bool) -> Result<ToolResult, Reason> {
    let result = complete(exchange, modern)?;
    let raw = &result.result;
    let blocks = raw["content"].as_array().ok_or(Reason::V3)?;
    let content = blocks
        .iter()
        .map(|b| content(b, modern).map(Box::new))
        .collect::<Result<_, _>>()?;
    let structured_content = match raw.get("structuredContent") {
        Some(v) if modern || v.is_object() => EssPresence::Present(v.clone()),
        Some(_) => return Err(Reason::V3),
        None => EssPresence::Absent,
    };
    let is_error = match raw.get("isError") {
        Some(v) => EssPresence::Present(v.as_bool().ok_or(Reason::V3)?),
        None => EssPresence::Absent,
    };
    Ok(ToolResult {
        content,
        structured_content,
        is_error,
        exchange: Box::new(result.clone()),
    })
}
fn parse_resource(exchange: &Exchange, modern: bool) -> Result<ResourceResult, Reason> {
    let result = complete(exchange, modern)?;
    let cache_hints = if modern {
        EssPresence::Present(Box::new(
            strict_discovery::cache(&result.result).map_err(|_| Reason::V3)?,
        ))
    } else {
        EssPresence::Absent
    };
    let contents = result.result["contents"]
        .as_array()
        .ok_or(Reason::V3)?
        .iter()
        .map(|r| resource_content(r).map(Box::new))
        .collect::<Result<_, _>>()?;
    Ok(ResourceResult {
        contents,
        cache_hints,
        exchange: Box::new(result.clone()),
    })
}
fn parse_prompt(exchange: &Exchange, modern: bool) -> Result<PromptResult, Reason> {
    let result = complete(exchange, modern)?;
    let mut messages = Vec::new();
    for raw in result.result["messages"].as_array().ok_or(Reason::V3)? {
        let role = serde_json::from_value(raw["role"].clone()).map_err(|_| Reason::V3)?;
        messages.push(Box::new(Message {
            role,
            content: Box::new(content(&raw["content"], modern)?),
            raw: raw.clone(),
        }));
    }
    Ok(PromptResult {
        description: optional_string(&result.result, "description")?,
        messages,
        exchange: Box::new(result.clone()),
    })
}
