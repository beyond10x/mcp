//! ESS target for actual revision-fixed connection setup.
#![cfg(feature = "strict-http")]
#[path = "support/strict_connection.rs"]
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
const SUITE: &str = include_str!("../../../conformance/strict-connection/suite.json");
fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the owned strict connection fixture")
}
struct Setup;
impl ConformanceTarget for Setup {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "actual revision-fixed HTTP connection",
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
            || request.command.to_string() != "mcp.strict_connection_checks.ObserveSetup"
        {
            return Err(unsupported("command or caller"));
        }
        let case = request
            .input
            .get("case")
            .and_then(Node::as_text)
            .ok_or_else(|| unsupported("case"))?;
        let revision = request
            .input
            .get("revision")
            .and_then(Node::as_text)
            .ok_or_else(|| unsupported("revision"))?;
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| TargetError::unavailable("runtime", e.to_string()))?;
        let actual = runtime
            .block_on(fixture::observe(case, revision))
            .map_err(|e| TargetError::unavailable("actual setup", e))?;
        let requests = actual["requests"]
            .as_array()
            .ok_or_else(|| unsupported("actual requests"))?;
        let first = requests.first().unwrap_or(&Value::Null);
        let last = requests.last().unwrap_or(&Value::Null);
        let text = |value: &Value| Node::Text(value.as_str().unwrap_or("").to_owned());
        let ready = actual["ready"]
            .as_bool()
            .ok_or_else(|| unsupported("actual ready"))?;
        let calls = actual["calls"]
            .as_i64()
            .ok_or_else(|| unsupported("actual calls"))?;
        let metadata = !requests.is_empty() && requests.iter().all(|request| {
            let version =
                &request["body"]["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"];
            version.is_string()
                && request["headers"]["mcp-protocol-version"] == *version
                && request["headers"]["mcp-method"] == request["body"]["method"]
                && request["body"]["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"]
                    == json!({})
        });
        let mut result = SemanticCommandResult::took(
            serde_json::from_value(json!({"command":request.command,"outcome":"observed"}))
                .expect("declared outcome"),
        );
        result.response = Some(BTreeMap::from([
            ("ready".into(), Node::Bool(ready)),
            ("calls".into(), Node::Number(Number::from(calls))),
            ("reason".into(), text(&actual["error"]["reason"])),
            ("first_method".into(), text(&first["body"]["method"])),
            ("last_method".into(), text(&last["body"]["method"])),
            (
                "session_echoed".into(),
                Node::Bool(
                    requests
                        .iter()
                        .any(|r| r["headers"]["mcp-session-id"] == "fixture-secret"),
                ),
            ),
            (
                "configured_revision".into(),
                text(&actual["description"]["configured_revision"]),
            ),
            ("business_kind".into(), text(&actual["business"]["kind"])),
            ("modern_metadata".into(), Node::Bool(metadata)),
        ]));
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
fn setup_scenarios_execute_actual_connection() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted setup suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &Setup);
    let report = CountReport::from_run(&run, &admitted).expect("actual setup counts");
    println!(
        "SETUP_COUNT_REPORT_BEGIN\n{}SETUP_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "SETUP_RUN_REPORT_BEGIN\n{}SETUP_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 32);
    assert_eq!(report.counts().passed, 32);
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}
