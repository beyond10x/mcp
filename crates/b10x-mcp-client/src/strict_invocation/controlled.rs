//! Explicit controlled calls over the same private catalogs and parsers.
#[path = "control.rs"]
mod control;
#[path = "outcomes.rs"]
mod outcomes;
use super::{
    Family, InvocationClient, ListLimits, PromptResult, Reason, ResourceResult, ToolResult,
    parameter_headers, parse_prompt, parse_resource, parse_tool,
};
use crate::{strict_cancellation::Cancellation, strict_discovery};
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpLifecycleControlledDiscoveryResult as DiscoveryOutcome,
    McpHttpLifecycleControlledPromptResult as PromptOutcome,
    McpHttpLifecycleControlledResourceResult as ResourceOutcome,
    McpHttpLifecycleControlledToolResult as ToolOutcome, McpHttpLifecycleInvocationPhase as Phase,
    McpHttpLifecycleWorkerCleanup as Cleanup,
};
use control::{Control, Failure};
use serde_json::{Value, json};
use tokio::time::Instant;

impl InvocationClient {
    /// Invalidate the selected catalog before a cancellable refresh. Only complete
    /// discovery replaces it; interrupted or dropped refreshes cannot use stale rows.
    pub async fn discover_cancellable(
        &mut self,
        family: Family,
        limits: &ListLimits,
        deadline: Instant,
        cancellation: &Cancellation,
        teardown_deadline: Instant,
    ) -> DiscoveryOutcome {
        self.invalidate(&family);
        let outcome = strict_discovery::discover_cancellable(
            &mut self.connection,
            family,
            limits,
            deadline,
            cancellation,
            teardown_deadline,
        )
        .await;
        match outcome {
            DiscoveryOutcome::V0(mut complete) => {
                complete.value = Box::new(self.store_catalog(*complete.value));
                DiscoveryOutcome::V0(complete)
            }
            other => other,
        }
    }
    /// Retry bounded cleanup of a retained schema child without dispatching HTTP.
    /// Await this before consuming shutdown when an observed worker reap is needed.
    pub async fn reap_schema_worker(&mut self, deadline: Instant) -> Result<Cleanup, Reason> {
        let end = self
            .connection
            .traversal_deadline(deadline)
            .ok_or(Reason::V2)?;
        self.worker.reap_pending(end).await
    }
    /// Validate, invoke once and validate output with explicit phase observations.
    /// A stopped post-response validation retains the actual business exchange and
    /// never dispatches cancellation for a request that already reached a terminal.
    pub async fn call_tool_cancellable(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Instant,
        cancellation: &Cancellation,
        teardown_deadline: Instant,
    ) -> ToolOutcome {
        let result = async {
            let control = Control::new(self, deadline, cancellation, teardown_deadline)?;
            self.controlled_tool(name, arguments, &control).await
        }
        .await;
        outcomes::tool(result)
    }
    async fn controlled_tool(
        &mut self,
        name: &str,
        arguments: Value,
        control: &Control<'_>,
    ) -> Result<ToolResult, Failure> {
        self.catalog(&Family::V2)?;
        let (tool, parameters) = self
            .tools
            .get(name)
            .cloned()
            .ok_or_else(|| Failure::refused(Reason::V6, Phase::V0))?;
        if !arguments.is_object() {
            return Err(Failure::refused(Reason::V2, Phase::V0));
        }
        let headers = parameter_headers::extract(&parameters, &arguments)
            .map_err(|reason| Failure::refused(reason, Phase::V0))?;
        control
            .schema(
                &mut self.worker,
                tool.input_schema,
                EssPresence::Present(arguments.clone()),
                Reason::V2,
                Phase::V2,
            )
            .await?;
        if let EssPresence::Present(schema) = &tool.output_schema {
            control
                .schema(
                    &mut self.worker,
                    schema.clone(),
                    EssPresence::Absent,
                    Reason::V4,
                    Phase::V3,
                )
                .await?;
        }
        let exchange = control
            .exchange(
                &mut self.connection,
                "tools/call",
                json!({"name":name,"arguments":arguments}),
                headers,
            )
            .await?;
        let result = parse_tool(&exchange, self.modern())
            .map_err(|reason| Failure::refused(reason, Phase::V4).observed(&exchange))?;
        if let EssPresence::Present(schema) = tool.output_schema {
            let EssPresence::Present(instance) = &result.structured_content else {
                return Err(Failure::refused(Reason::V4, Phase::V4).observed(&exchange));
            };
            control
                .schema(
                    &mut self.worker,
                    schema,
                    EssPresence::Present(instance.clone()),
                    Reason::V4,
                    Phase::V4,
                )
                .await
                .map_err(|failure| failure.observed(&exchange))?;
        }
        control.finish(&exchange)?;
        Ok(result)
    }
    /// Read one discovered resource with explicit HTTP interruption observations.
    pub async fn read_resource_cancellable(
        &mut self,
        uri: &str,
        deadline: Instant,
        cancellation: &Cancellation,
        teardown_deadline: Instant,
    ) -> ResourceOutcome {
        let result: Result<ResourceResult, Failure> = async {
            let control = Control::new(self, deadline, cancellation, teardown_deadline)?;
            self.check_resource(uri)?;
            let exchange = control
                .exchange(
                    &mut self.connection,
                    "resources/read",
                    json!({"uri":uri}),
                    reqwest::header::HeaderMap::new(),
                )
                .await?;
            let result = parse_resource(&exchange, self.modern())
                .map_err(|reason| Failure::refused(reason, Phase::V4).observed(&exchange))?;
            control.finish(&exchange)?;
            Ok(result)
        }
        .await;
        outcomes::resource(result)
    }
    /// Retrieve one admitted prompt. Returned instructions and links remain data.
    pub async fn get_prompt_cancellable(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Instant,
        cancellation: &Cancellation,
        teardown_deadline: Instant,
    ) -> PromptOutcome {
        let result: Result<PromptResult, Failure> = async {
            let control = Control::new(self, deadline, cancellation, teardown_deadline)?;
            self.check_prompt(name, &arguments)?;
            let exchange = control
                .exchange(
                    &mut self.connection,
                    "prompts/get",
                    json!({"name":name,"arguments":arguments}),
                    reqwest::header::HeaderMap::new(),
                )
                .await?;
            let result = parse_prompt(&exchange, self.modern())
                .map_err(|reason| Failure::refused(reason, Phase::V4).observed(&exchange))?;
            control.finish(&exchange)?;
            Ok(result)
        }
        .await;
        outcomes::prompt(result)
    }
}
