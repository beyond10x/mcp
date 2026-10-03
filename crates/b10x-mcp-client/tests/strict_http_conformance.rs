//! Actual bounded HTTP conformance, without a typed SDK as the wire oracle.
#![cfg(feature = "strict-http")]
#[path = "support/strict_http.rs"]
mod fixture;

use std::collections::BTreeMap;

use ess_conformance::{
    AdmittedSuite, ConformanceTarget, CountReport, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, ObservedEvent, RedeliveryRequest, Runner, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest, SemanticViewResult,
    TargetError,
};
use ess_primitives::{facts::Number, node::Node};
use serde_json::json;

const SUITE: &str = include_str!("../../../conformance/strict-http/suite.json");

fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the owned HTTP replay fixture binding")
}

struct StrictHttp;
impl ConformanceTarget for StrictHttp {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "b10x-mcp-client actual bounded HTTP exchange",
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
            || request.command.to_string() != "mcp.strict_http_checks.ObserveExchange"
        {
            return Err(unsupported("command or caller authority"));
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
            .map_err(|e| TargetError::unavailable("fixture runtime", e.to_string()))?;
        let (actual, calls) = runtime
            .block_on(fixture::observe(case, revision))
            .map_err(|e| TargetError::unavailable("actual HTTP observation", e))?;
        let calls = i64::try_from(calls).map_err(|_| unsupported("business count range"))?;
        let text = |value: &serde_json::Value| Node::Text(value.as_str().unwrap_or("").to_owned());
        let retained = actual["value"]["observation"]["response"]["counts"]["retained_octets"]
            .as_i64()
            .ok_or_else(|| unsupported("retained count"))?;
        let mut result = SemanticCommandResult::took(
            serde_json::from_value(json!({"command": request.command, "outcome": "observed"}))
                .expect("declared fixture outcome"),
        );
        result.response = Some(BTreeMap::from([
            ("business_calls".into(), Node::Number(Number::from(calls))),
            ("result_kind".into(), text(&actual["kind"])),
            (
                "peer_code".into(),
                Node::Text(
                    actual["value"]["error"]["code"]
                        .as_number()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                ),
            ),
            (
                "http_status".into(),
                Node::Text(
                    actual["value"]["observation"]["http_status"]
                        .as_number()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                ),
            ),
            ("refusal_reason".into(), text(&actual["value"]["reason"])),
            (
                "terminal".into(),
                text(&actual["value"]["observation"]["terminal"]),
            ),
            (
                "peer_data_presence".into(),
                text(&actual["value"]["error"]["data"]["presence"]),
            ),
            (
                "retained_octets".into(),
                Node::Number(Number::from(retained)),
            ),
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
fn strict_http_scenarios_execute_against_owned_wire() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted original HTTP suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &StrictHttp);
    let report = CountReport::from_run(&run, &admitted).expect("counts tied to actual HTTP run");
    println!(
        "HTTP_COUNT_REPORT_BEGIN\n{}HTTP_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "HTTP_RUN_REPORT_BEGIN\n{}HTTP_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 43);
    assert_eq!(
        report.counts().passed,
        43,
        "all authored and generated HTTP cases execute"
    );
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}
