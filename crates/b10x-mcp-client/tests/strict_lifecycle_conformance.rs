//! ESS target for Real revision-specific lifecycle observations.
#![cfg(feature = "strict-http")]
#[path = "support/strict_lifecycle.rs"]
mod fixture;
use ess_conformance::{
    AdmittedSuite, ConformanceTarget, CountReport, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, ObservedEvent, RedeliveryRequest, Runner, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest, SemanticViewResult,
    TargetError,
};
use ess_primitives::{facts::Number, node::Node};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const SUITE: &str = include_str!("../../../conformance/strict-lifecycle/suite.json");
fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the owned strict lifecycle fixture")
}
struct Lifecycle;
fn observations(actual: &Value) -> Result<BTreeMap<String, Node>, TargetError> {
    actual
        .as_object()
        .ok_or_else(|| unsupported("observation object"))?
        .iter()
        .map(|(key, value)| {
            let node = match value {
                Value::Bool(value) => Node::Bool(*value),
                Value::String(value) => Node::Text(value.clone()),
                Value::Number(value) => Node::Number(Number::from(
                    value.as_i64().ok_or_else(|| unsupported("bounded count"))?,
                )),
                _ => return Err(unsupported("observation type")),
            };
            Ok((key.clone(), node))
        })
        .collect()
}
impl ConformanceTarget for Lifecycle {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "actual strict HTTP lifecycle",
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn begin_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        Ok(())
    }
    fn end_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        Ok(())
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        if request.actor.is_some()
            || request.caller.is_some()
            || !matches!(
                request.command.to_string().as_str(),
                "mcp.strict_lifecycle_checks.ObserveLifecycle"
                    | "mcp.strict_stream_checks.ObserveStream"
                    | "mcp.strict_progress_checks.ObserveProgress"
            )
        {
            return Err(unsupported("command/caller"));
        }
        let input = |key| {
            request
                .input
                .get(key)
                .and_then(Node::as_text)
                .ok_or_else(|| unsupported(key))
        };
        let case = input("case")?;
        let revision = input("revision")?;
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| TargetError::unavailable("runtime", e.to_string()))?;
        let actual = runtime
            .block_on(fixture::observations(case, revision))
            .map_err(|e| TargetError::unavailable("actual lifecycle", e))?;
        let mut result = SemanticCommandResult::took(
            serde_json::from_value(json!({"command":request.command,"outcome":"observed"}))
                .expect("declared outcome"),
        );
        result.response = Some(observations(&actual)?);
        Ok(result)
    }
    fn query_view(&self, _: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(unsupported("view"))
    }
    fn observe_events(
        &self,
        _: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Err(unsupported("events"))
    }
    fn configure_external_outcome(&self, _: ExternalOutcomeControl) -> Result<(), TargetError> {
        Err(unsupported("external outcome"))
    }
    fn redeliver_event(&self, _: RedeliveryRequest) -> Result<(), TargetError> {
        Err(unsupported("redelivery"))
    }
}
#[test]
fn lifecycle_scenarios_execute_actual_connection() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted lifecycle suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &Lifecycle);
    let report = CountReport::from_run(&run, &admitted).expect("actual counts");
    println!(
        "LIFECYCLE_COUNT_REPORT_BEGIN\n{}LIFECYCLE_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "LIFECYCLE_RUN_REPORT_BEGIN\n{}LIFECYCLE_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 80);
    assert_eq!(report.counts().passed, 80);
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}
