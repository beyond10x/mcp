//! ESS target for real bounded selected-family list traversal.
#![cfg(feature = "strict-http")]
#[path = "support/strict_discovery.rs"]
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
const SUITE: &str = include_str!("../../../conformance/strict-discovery/suite.json");
fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the owned strict discovery fixture")
}
struct Discovery;
fn observations(actual: &Value, revision: &str) -> Result<BTreeMap<String, Node>, TargetError> {
    let requests = actual["requests"]
        .as_array()
        .ok_or_else(|| unsupported("actual requests"))?;
    let lists: Vec<_> = requests
        .iter()
        .filter(|r| {
            r["body"]["method"]
                .as_str()
                .is_some_and(|m| m.ends_with("/list"))
        })
        .collect();
    let complete = actual["complete"]
        .as_bool()
        .ok_or_else(|| unsupported("actual completion"))?;
    let count = |value: &Value| value.as_array().map_or(0, Vec::len);
    let integer = |n: usize| {
        Node::Number(Number::from(
            i64::try_from(n).expect("bounded fixture count"),
        ))
    };
    let metadata = !lists.is_empty()
        && lists.iter().all(|r| {
            r["headers"]["mcp-protocol-version"] == revision
                && (revision != "2026-07-28"
                    || (r["headers"]["mcp-method"] == r["body"]["method"]
                        && r["body"]["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"]
                            == revision))
        });
    Ok(BTreeMap::from([
        ("complete".into(), Node::Bool(complete)),
        ("calls".into(), integer(lists.len())),
        (
            "items".into(),
            integer(count(&actual["catalog"]["descriptors"])),
        ),
        ("pages".into(), integer(count(&actual["catalog"]["pages"]))),
        (
            "reason".into(),
            Node::Text(
                actual["refusal"]["reason"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            ),
        ),
        (
            "empty_cursor_sent".into(),
            Node::Bool(lists.iter().any(|r| r["body"]["params"]["cursor"] == "")),
        ),
        ("metadata_match".into(), Node::Bool(metadata)),
        (
            "retained_exchanges".into(),
            integer(if complete {
                count(&actual["catalog"]["pages"])
            } else {
                count(&actual["refusal"]["exchanges"])
            }),
        ),
    ]))
}
impl ConformanceTarget for Discovery {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "actual strict HTTP discovery",
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
            || request.command.to_string() != "mcp.strict_discovery_checks.ObserveList"
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
        let family = input("family")?;
        let revision = input("revision")?;
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| TargetError::unavailable("runtime", e.to_string()))?;
        let actual = runtime
            .block_on(fixture::observe(case, family, revision))
            .map_err(|e| TargetError::unavailable("actual discovery", e))?;
        let mut result = SemanticCommandResult::took(
            serde_json::from_value(json!({"command":request.command,"outcome":"observed"}))
                .expect("declared outcome"),
        );
        result.response = Some(observations(&actual, revision)?);
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
fn discovery_scenarios_execute_actual_connection() {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted discovery suite");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &Discovery);
    let report = CountReport::from_run(&run, &admitted).expect("actual counts");
    println!(
        "DISCOVERY_COUNT_REPORT_BEGIN\n{}DISCOVERY_COUNT_REPORT_END",
        report.to_canonical_json().unwrap()
    );
    println!(
        "DISCOVERY_RUN_REPORT_BEGIN\n{}DISCOVERY_RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    assert_eq!(report.counts().total, 111);
    assert_eq!(report.counts().passed, 111);
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}
