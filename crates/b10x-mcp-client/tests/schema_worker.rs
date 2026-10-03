//! Actual worker semantics and process deadline/cleanup checks.
#![cfg(feature = "strict-http")]
use b10x_mcp_client::schema_worker::SchemaWorker;
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpInvocationSchemaAction as Action, McpHttpInvocationSchemaRequest as Request,
};
use serde_json::{Value, json};
use tokio::time::{Duration, Instant};
async fn check(schema: Value, instance: Option<Value>) -> Value {
    let request = Request {
        action: Box::new(if instance.is_some() {
            Action::V1
        } else {
            Action::V0
        }),
        schema,
        instance: instance.map_or(EssPresence::Absent, EssPresence::Present),
    };
    let mut worker =
        SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(), 65536).unwrap();
    serde_json::to_value(
        worker
            .run(&request, Instant::now() + Duration::from_secs(3))
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn schema_validation_executes_semantics_and_preserves_present_null() {
    let schema = json!({"type":"object","required":["count"],"properties":{"count":{"type":"integer","minimum":2}},"additionalProperties":false});
    assert_eq!(
        check(schema.clone(), Some(json!({"count":2}))).await["status"],
        "valid"
    );
    assert_eq!(
        check(schema.clone(), Some(json!({"count":1}))).await["status"],
        "invalid_instance"
    );
    assert_eq!(check(schema, None).await["status"], "valid");
    assert_eq!(
        check(json!({"type":"null"}), Some(Value::Null)).await["status"],
        "valid"
    );
}

#[tokio::test]
async fn offline_references_dialect_and_exact_numbers_keep_their_meaning() {
    assert_eq!(
        check(
            json!({"$defs":{"n":{"type":"integer","minimum":2}},"$ref":"#/$defs/n"}),
            Some(json!(2))
        )
        .await["status"],
        "valid"
    );
    assert_eq!(
        check(json!({"$ref":"https://127.0.0.1:9/schema"}), None).await["status"],
        "unsupported_schema"
    );
    assert_eq!(
        check(json!({"$ref":"file:///etc/passwd"}), None).await["status"],
        "unsupported_schema"
    );
    assert_eq!(
        check(
            json!({"$schema":"http://json-schema.org/draft-07/schema#","type":"integer"}),
            None
        )
        .await["status"],
        "unsupported_schema"
    );
    assert_eq!(
        check(json!({"type":"not-a-type"}), None).await["status"],
        "invalid_schema"
    );
    let large: Value = serde_json::from_str("123456789012345678901234567890").unwrap();
    assert_eq!(
        check(json!({"type":"integer","const":large}), Some(large.clone())).await["status"],
        "valid"
    );
    assert_eq!(
        check(json!({"type":"integer","const":large}), Some(json!(1))).await["status"],
        "invalid_instance"
    );
    assert_eq!(
        check(
            json!({"type":"string","format":"email"}),
            Some(json!("not an email"))
        )
        .await["status"],
        "valid"
    );
}
#[tokio::test]
async fn opaque_private_marker_objects_remain_objects_in_worker_ipc() {
    let raw = json!({"$serde_json::private::Number":"7"});
    assert_eq!(
        check(
            json!({"type":"object","required":["$serde_json::private::Number"]}),
            Some(raw)
        )
        .await["status"],
        "valid"
    );
}
#[cfg(all(feature = "test-schema-worker", target_os = "linux"))]
#[tokio::test]
async fn awaited_timeout_and_excess_output_reap_the_actual_child() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/mcp-next-runtime/invocation");
    std::fs::create_dir_all(&root).unwrap();
    for (mode, reason, budget) in [
        ("sleep", "deadline_exhausted", 500),
        ("overflow", "schema_unavailable", 3000),
    ] {
        let dir = tempfile::tempdir_in(&root).unwrap();
        let pid_file = dir.path().join("pid");
        let request = serde_json::from_value(
            json!({"action":"validate","schema":{"mode":mode},"instance":pid_file}),
        )
        .unwrap();
        let mut worker =
            SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-fixture").into(), 65536).unwrap();
        let result = worker
            .run(&request, Instant::now() + Duration::from_millis(budget))
            .await;
        assert_eq!(serde_json::to_value(result.unwrap_err()).unwrap(), reason);
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        assert!(
            !std::path::Path::new("/proc").join(pid).exists(),
            "owned child was not reaped"
        );
    }
}
#[tokio::test]
async fn invalid_ports_bounds_and_expired_deadlines_refuse() {
    assert_eq!(
        serde_json::to_value(SchemaWorker::new("relative".into(), 1024).err().unwrap()).unwrap(),
        "invalid_input"
    );
    let request = serde_json::from_value(json!({"action":"compile","schema":{}})).unwrap();
    let mut worker =
        SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(), 0).unwrap();
    assert_eq!(
        serde_json::to_value(
            worker
                .run(&request, Instant::now() + Duration::from_secs(3))
                .await
                .unwrap_err()
        )
        .unwrap(),
        "invalid_input"
    );
    let mut worker =
        SchemaWorker::new(env!("CARGO_BIN_EXE_b10x-mcp-schema-worker").into(), 65536).unwrap();
    assert_eq!(
        serde_json::to_value(worker.run(&request, Instant::now()).await.unwrap_err()).unwrap(),
        "deadline_exhausted"
    );
}
