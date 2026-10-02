//! Partial constructor conformance; refused synthesis obligations remain in ess/coverage.md.
use std::collections::BTreeMap;

use b10x_mcp_types::{ClientError, ConnectionId, Limits, ToolDescriptor, ToolResult};
use ess_conformance::{
    AdmittedSuite, ConformanceTarget, CountReport, DeclaredErrorValue, EventObservationRequest,
    ExternalOutcomeControl, ImplementationIdentity, ObservedEvent, RedeliveryRequest, Runner,
    ScenarioContext, SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest,
    SemanticViewResult, TargetError,
};
use ess_primitives::{facts::Number, node::Node};
use serde_json::{Value, json};

const SUITE: &str = include_str!("../../../conformance/constructors.json");
const MAX_BYTES: usize = 1_048_576;
const MAX_NODES: usize = 65_536;
const MAX_DEPTH: usize = 128;

fn unsupported(what: &str) -> TargetError {
    TargetError::unsupported(what, "outside the bounded constructor observation binding")
}

// Native JSON integer extraction never passes through f64. Fractional numbers and integers
// outside i64 are explicitly unsupported in this binding, not rounded into an observation.
fn project(value: &Value) -> Result<Node, TargetError> {
    fn visit(value: &Value, depth: usize, remaining: &mut usize) -> Result<Node, TargetError> {
        if depth > MAX_DEPTH || *remaining == 0 {
            return Err(unsupported("JSON depth or membership bound"));
        }
        *remaining -= 1;
        Ok(match value {
            Value::Null => Node::Null,
            Value::Bool(value) => Node::Bool(*value),
            Value::String(value) => Node::Text(value.clone()),
            Value::Number(value) => Node::Number(Number::from(
                value
                    .as_i64()
                    .ok_or_else(|| unsupported("non-i64 JSON number"))?,
            )),
            Value::Array(values) => Node::Seq(
                values
                    .iter()
                    .map(|v| visit(v, depth + 1, remaining))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(values) => Node::Map(
                values
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), visit(v, depth + 1, remaining)?)))
                    .collect::<Result<_, TargetError>>()?,
            ),
        })
    }
    let mut remaining = MAX_NODES;
    let node = visit(value, 0, &mut remaining)?;
    if serde_json::to_vec(value)
        .map_err(|_| unsupported("JSON serialization"))?
        .len()
        > MAX_BYTES
    {
        return Err(unsupported("JSON byte bound"));
    }
    Ok(node)
}

fn text<'a>(request: &'a SemanticCommandRequest, field: &str) -> Result<&'a str, TargetError> {
    request
        .input
        .get(field)
        .and_then(Node::as_text)
        .ok_or_else(|| unsupported(field))
}

fn boolean(request: &SemanticCommandRequest, field: &str) -> Result<bool, TargetError> {
    match request.input.get(field) {
        Some(Node::Bool(value)) => Ok(*value),
        _ => Err(unsupported(field)),
    }
}

fn returned(
    request: &SemanticCommandRequest,
    field: &str,
    value: &Value,
) -> Result<SemanticCommandResult, TargetError> {
    let mut result = branch(request, "returned");
    result.response = Some(BTreeMap::from([(field.to_owned(), project(value)?)]));
    Ok(result)
}

fn branch(request: &SemanticCommandRequest, name: &str) -> SemanticCommandResult {
    // These names are the binding's declared branches, never runner expectations.
    SemanticCommandResult::took(
        serde_json::from_value(json!({"command": request.command, "outcome": name}))
            .expect("binding outcome reference"),
    )
}

struct Constructors;
impl ConformanceTarget for Constructors {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "b10x-mcp-types constructors",
            env!("CARGO_PKG_VERSION"),
        ))
    }
    // Every invocation constructs fresh values; there is no retained product state.
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
        if request.actor.is_some() || request.caller.is_some() {
            return Err(unsupported("caller authority"));
        }
        match request.command.to_string().as_str() {
            "mcp.constructor_checks.MakeConnectionId" => {
                let value = text(&request, "value")?;
                match ConnectionId::new(value) {
                    Ok(id) => returned(&request, "id", &json!(id)),
                    Err(ClientError::Configuration(message)) => {
                        let outcome = if value.is_empty() {
                            "empty"
                        } else if value.len() > 64 {
                            "too-long"
                        } else {
                            return Err(unsupported("ID alphabet refusal"));
                        };
                        Ok(branch(&request, outcome).with_error(
                            DeclaredErrorValue::new(
                                serde_json::from_value(json!(
                                    "mcp.constructor_checks.Configuration"
                                ))
                                .expect("binding error reference"),
                            )
                            .with("message", Node::Text(message)),
                        ))
                    }
                    Err(error) => Err(TargetError::unavailable(
                        "ConnectionId::new",
                        error.to_string(),
                    )),
                }
            }
            "mcp.constructor_checks.MakeToolDescriptor" => {
                let raw = json!({"name": text(&request, "name")?, "inputSchema": {}, "annotations": {"readOnlyHint": boolean(&request, "read_only_hint")?}, "_meta": {"note": text(&request, "note")?}, "extension": "retained"});
                let value = ToolDescriptor::from_raw(raw, Limits::default()).map_err(|e| {
                    TargetError::unavailable("ToolDescriptor::from_raw", e.to_string())
                })?;
                // The declared null_when_absent serialization maps None and Some(null) to null.
                returned(
                    &request,
                    "descriptor",
                    &serde_json::to_value(value)
                        .map_err(|_| unsupported("descriptor serialization"))?,
                )
            }
            "mcp.constructor_checks.MakeToolResult" => {
                let raw = json!({"content": [{"type": "text", "text": text(&request, "text")?}], "isError": boolean(&request, "is_error")?});
                let value = ToolResult::from_raw(raw, Limits::default())
                    .map_err(|e| TargetError::unavailable("ToolResult::from_raw", e.to_string()))?;
                returned(
                    &request,
                    "result",
                    &serde_json::to_value(value)
                        .map_err(|_| unsupported("result serialization"))?,
                )
            }
            _ => Err(unsupported(&request.command.to_string())),
        }
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
        Err(unsupported("event redelivery"))
    }
}

// Test-owned mutation of the actual return only. It never reads a scenario or expected value.
struct WrongReturn;
impl ConformanceTarget for WrongReturn {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "wrong-return control",
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn begin_scenario(&self, s: &ScenarioContext) -> Result<(), TargetError> {
        Constructors.begin_scenario(s)
    }
    fn end_scenario(&self, s: &ScenarioContext) -> Result<(), TargetError> {
        Constructors.end_scenario(s)
    }
    fn execute_command(
        &self,
        r: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let mut actual = Constructors.execute_command(r)?;
        if let Some(Node::Map(result)) = actual.response.as_mut().and_then(|r| r.get_mut("result"))
            && let Some(Node::Bool(value)) = result.get_mut("is_error")
        {
            *value = !*value;
        }
        Ok(actual)
    }
    fn query_view(&self, r: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Constructors.query_view(r)
    }
    fn observe_events(
        &self,
        r: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Constructors.observe_events(r)
    }
    fn configure_external_outcome(&self, r: ExternalOutcomeControl) -> Result<(), TargetError> {
        Constructors.configure_external_outcome(r)
    }
    fn redeliver_event(&self, r: RedeliveryRequest) -> Result<(), TargetError> {
        Constructors.redeliver_event(r)
    }
}

fn execute(target: &impl ConformanceTarget) -> (CountReport, ess_conformance::ExecutedRun) {
    let admitted = AdmittedSuite::from_json(SUITE).expect("admitted original suite bytes");
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, target);
    let counts = CountReport::from_run(&run, &admitted).expect("report tied to executed suite");
    println!(
        "COUNT_REPORT_BEGIN\n{}COUNT_REPORT_END",
        counts.to_canonical_json().unwrap()
    );
    println!(
        "RUN_REPORT_BEGIN\n{}RUN_REPORT_END",
        run.report().to_canonical_json()
    );
    (counts, run)
}

#[test]
fn supported_constructors_conform() {
    let (report, _) = execute(&Constructors);
    assert_eq!(report.counts().total, 9);
    assert_eq!(
        report.counts().passed,
        9,
        "every emitted scenario must pass"
    );
    assert_eq!(
        report.counts().failed
            + report.counts().error
            + report.counts().unsupported
            + report.counts().skipped,
        0
    );
}

#[test]
fn wrong_actual_return_fails_named_scenarios() {
    let (report, run) = execute(&WrongReturn);
    assert_eq!(report.counts().total, 9);
    assert_eq!(report.counts().passed, 7);
    assert_eq!(report.counts().failed, 2);
    assert_eq!(
        report.counts().error + report.counts().unsupported + report.counts().skipped,
        0
    );
    let failures: Vec<_> = run
        .report()
        .failures()
        .map(|s| s.scenario.to_string())
        .collect();
    assert_eq!(
        failures,
        [
            "mcp.constructor_checks/authored/result-success",
            "mcp.constructor_checks/authored/result-tool-error"
        ]
    );
}

#[test]
fn projection_preserves_exact_integers_and_refuses_other_numbers() {
    for integer in [
        i64::MIN,
        -9_007_199_254_740_993,
        0,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        assert_eq!(
            project(&json!(integer)).unwrap(),
            Node::Number(Number::from(integer))
        );
    }
    for value in [json!(u64::MAX), json!(1.5)] {
        assert!(
            matches!(project(&value), Err(TargetError::Unsupported { observation, .. }) if observation == "non-i64 JSON number")
        );
    }
}

#[test]
fn projection_retains_nulls_and_declared_optional_collapse() {
    let absent =
        ToolDescriptor::from_raw(json!({"name":"read", "inputSchema":{}}), Limits::default())
            .unwrap();
    let explicit_null = ToolDescriptor::from_raw(
        json!({"name":"read", "inputSchema":{}, "outputSchema":null}),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(absent.output_schema, None);
    assert_eq!(explicit_null.output_schema, Some(Value::Null));
    let absent = project(&serde_json::to_value(absent).unwrap()).unwrap();
    let explicit_null = project(&serde_json::to_value(explicit_null).unwrap()).unwrap();
    for node in [&absent, &explicit_null] {
        assert_eq!(
            node.as_map().unwrap().get("output_schema"),
            Some(&Node::Null)
        );
    }
    // The declared optional projection collapses these states; original raw JSON still distinguishes them.
    assert_ne!(
        absent.as_map().unwrap()["raw"],
        explicit_null.as_map().unwrap()["raw"]
    );
    assert_eq!(
        project(&json!([null, true, "x"])).unwrap(),
        Node::Seq(vec![Node::Null, Node::Bool(true), Node::Text("x".into())])
    );
}

#[test]
fn projection_refuses_bounds_without_truncating() {
    assert!(project(&Value::String("x".repeat(MAX_BYTES))).is_err());
    assert!(project(&Value::Array(vec![Value::Null; MAX_NODES])).is_err());
    let mut value = Value::Null;
    for _ in 0..=MAX_DEPTH {
        value = Value::Array(vec![value]);
    }
    assert!(project(&value).is_err());
}

#[test]
fn adversary_projection_exact_limits_preserve_values_and_name_first_refusal() {
    // JSON escaping contributes two bytes per quote, plus the outer quotes.
    let at_byte_limit = "\"".repeat(MAX_BYTES / 2 - 1);
    assert_eq!(
        project(&Value::String(at_byte_limit.clone())).unwrap(),
        Node::Text(at_byte_limit.clone())
    );
    assert!(matches!(
        project(&Value::String(format!("{at_byte_limit}x"))),
        Err(TargetError::Unsupported { observation, .. }) if observation == "JSON byte bound"
    ));

    // The root array is itself one member of the observation budget.
    assert_eq!(
        project(&Value::Array(vec![Value::Null; MAX_NODES - 1])).unwrap(),
        Node::Seq(vec![Node::Null; MAX_NODES - 1])
    );
    assert!(matches!(
        project(&Value::Array(vec![Value::Null; MAX_NODES])),
        Err(TargetError::Unsupported { observation, .. }) if observation == "JSON depth or membership bound"
    ));

    let mut value = Value::Null;
    let mut expected = Node::Null;
    for _ in 0..MAX_DEPTH {
        value = Value::Array(vec![value]);
        expected = Node::Seq(vec![expected]);
    }
    assert_eq!(project(&value).unwrap(), expected);
    assert!(matches!(
        project(&Value::Array(vec![value])),
        Err(TargetError::Unsupported { observation, .. }) if observation == "JSON depth or membership bound"
    ));
}
