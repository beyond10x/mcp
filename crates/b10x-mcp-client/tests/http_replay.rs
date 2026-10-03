//! Actual HTTP conformance for automatic expired-session redispatch.
#[path = "support/http_replay.rs"]
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

const SUITE: &str = include_str!("../../../conformance/http-replay/suite.json");

fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the owned HTTP replay fixture binding")
}

struct HttpReplay;
impl ConformanceTarget for HttpReplay {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "b10x-mcp-client real HTTP session recovery",
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
            || request.command.to_string() != "mcp.http_replay_checks.ObserveSessionCall"
        {
            return Err(unsupported("command or caller authority"));
        }
        let injected = match request.input.get("constructor").and_then(Node::as_text) {
            Some("default_http") => false,
            Some("injected_http") => true,
            _ => return Err(unsupported("constructor")),
        };
        let expire = match request.input.get("expire_session") {
            Some(Node::Bool(expire)) => *expire,
            _ => return Err(unsupported("expire_session")),
        };
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| TargetError::unavailable("fixture runtime", e.to_string()))?;
        let (calls, initializations, succeeded, text) = runtime
            .block_on(fixture::observe(injected, expire))
            .map_err(|e| TargetError::unavailable("actual HTTP observation", e))?;
        let calls = i64::try_from(calls).map_err(|_| unsupported("business count range"))?;
        let initializations = i64::try_from(initializations)
            .map_err(|_| unsupported("initialization count range"))?;
        let mut result = SemanticCommandResult::took(
            serde_json::from_value(json!({"command": request.command, "outcome": "observed"}))
                .expect("declared fixture outcome"),
        );
        result.response = Some(BTreeMap::from([
            ("business_calls".into(), Node::Number(Number::from(calls))),
            (
                "initialization_calls".into(),
                Node::Number(Number::from(initializations)),
            ),
            ("call_succeeded".into(), Node::Bool(succeeded)),
            ("returned_text".into(), Node::Text(text)),
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
fn public_http_constructors_do_not_redispatch_after_session_expiry() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted original HTTP suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &HttpReplay);
    let report = CountReport::from_run(&run, &admitted).expect("counts tied to actual HTTP run");
    println!(
        "HTTP_COUNT_REPORT_BEGIN\n{}HTTP_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "HTTP_RUN_REPORT_BEGIN\n{}HTTP_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 5);
    assert_eq!(
        report.counts().passed,
        5,
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
