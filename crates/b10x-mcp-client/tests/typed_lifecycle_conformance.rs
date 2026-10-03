//! ESS target for actual typed calls, child ownership and discovery refreshes.
#![cfg(all(feature = "test-schema-worker", target_os = "linux"))]
#[path = "support/typed_lifecycle.rs"]
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
const SUITE: &str = include_str!("../../../conformance/typed-lifecycle/suite.json");
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
            "actual controlled typed lifecycle",
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
            || request.command.to_string() != "mcp.typed_lifecycle_checks.ObserveTypedLifecycle"
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
fn typed_lifecycle_scenarios_execute_actual_client() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted lifecycle suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &Lifecycle);
    let report = CountReport::from_run(&run, &admitted).expect("actual counts");
    println!(
        "TYPED_COUNT_REPORT_BEGIN\n{}TYPED_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "TYPED_RUN_REPORT_BEGIN\n{}TYPED_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 31);
    assert_eq!(report.counts().passed, 31);
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}
