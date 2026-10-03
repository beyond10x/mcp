//! Scenario setup and observations for ESS; never reads expected verdicts.
#[path = "strict_invocation.rs"]
mod peer;
use b10x_mcp_types::http_exchange::McpHttpDiscoveryFamily as Family;
use serde_json::{Value, json};

// Keep scenario construction beside the actual observations it exercises.
#[allow(clippy::too_many_lines)]
pub async fn observe(case: &str, family: &str, revision: &str) -> Result<Value, String> {
    let modern = revision == "2026-07-28";
    if !["2025-11-25", "2026-07-28"].contains(&revision) {
        return Err("unknown configured revision".into());
    }
    let (selected, method, mut definition, mut reply) = match family {
        "tools" => (
            Family::V2,
            "tools/call",
            peer::tool(json!({"type":"object"})),
            json!({"content":[{"type":"text","text":"kept","extension":{"$serde_json::private::Number":"7"}}]}),
        ),
        "resources" => (
            Family::V1,
            "resources/read",
            json!({"name":"r","uri":"test:r"}),
            json!({"contents":[{"uri":"test:r","text":"kept","blob":"AA=="}]}),
        ),
        "prompts" => (
            Family::V0,
            "prompts/get",
            json!({"name":"p","arguments":[{"name":"input","required":true}]}),
            json!({"messages":[{"role":"user","content":{"type":"text","text":"kept"}}]}),
        ),
        _ => return Err("unknown family".into()),
    };
    reply = peer::result(reply, modern);
    if modern && family == "resources" {
        reply["ttlMs"] = json!(0);
        reply["cacheScope"] = json!("private");
    }
    let mut arguments = if family == "prompts" {
        json!({"input":"kept"})
    } else {
        json!({})
    };
    let mut dispatch = true;
    let mut refresh = false;
    let mut invalid_tool = None;
    match case {
        "invoke-tool-content"
        | "invoke-resource-content"
        | "invoke-prompt-content"
        | "invoke-no-followup" => {}
        "invoke-tool-structured-presence" => {
            reply["structuredContent"] = if modern {
                Value::Null
            } else {
                json!({"kept":true})
            }
        }
        "invoke-business-error" => reply["isError"] = json!(true),
        "invoke-output-schema" => {
            definition["outputSchema"] = json!({"type":"object","required":["n"],"properties":{"n":{"type":"integer","minimum":2}}});
            reply["structuredContent"] = json!({"n":1});
        }
        "invoke-input-before-dispatch" => {
            definition["inputSchema"] = json!({"type":"object","required":["n"],"properties":{"n":{"type":"integer","minimum":2}}});
            arguments = json!({"n":1});
            dispatch = false;
        }
        "invoke-request-bound" => {
            arguments = json!({"input":"x".repeat(65536)});
            dispatch = false;
        }
        "invoke-prompt-input" => {
            arguments = json!({});
            dispatch = false;
        }
        "invoke-unknown-content" => {
            reply["content"] =
                json!([{"type":"text","text":"prefix"},{"type":"future","kept":true}]);
        }
        "invoke-unselected-result" => reply["resultType"] = json!("input_required"),
        "invoke-peer-error" => {
            reply =
                json!({"fixture_error":{"code":-32603,"message":"opaque","data":{"kept":true}}});
        }
        "invoke-incomplete" => reply = json!({"fixture_incomplete":true}),
        "invoke-deadline" => reply = json!({"fixture_delay":true}),
        "invoke-resource-cache-fields" => {
            reply
                .as_object_mut()
                .ok_or("resource object")?
                .remove("cacheScope");
        }
        "invoke-no-stale-success" => {
            refresh = true;
            dispatch = false;
        }
        "invoke-parameter-headers"
        | "invoke-parameter-header-missing"
        | "invoke-parameter-header-refusal" => {
            definition["inputSchema"] = json!({"type":"object","properties":{"route":{"type":"string","x-mcp-header":"Region"},"count":{"type":"integer","x-mcp-header":"Count"}}});
            if case == "invoke-parameter-headers" {
                arguments = json!({"route":"hello\r\nworld","count":42});
            }
            if case == "invoke-parameter-header-refusal" {
                let mut bad = definition.clone();
                bad["name"] = json!("bad");
                bad["inputSchema"]["properties"]["route"]["x-mcp-header"] = json!("");
                invalid_tool = Some(bad);
            }
        }
        _ => return Err(format!("unknown invocation case {case}")),
    }
    let mut rows = vec![definition];
    if let Some(bad) = invalid_tool {
        rows.push(bad);
    }
    let list = format!("{family}/list");
    let mut script = vec![(list.as_str(), peer::page(family, json!(rows), modern))];
    if refresh {
        script.push((
            list.as_str(),
            json!({"fixture_error":{"code":-32603,"message":"refresh failed"}}),
        ));
    }
    if dispatch {
        script.push((method, reply));
    }
    let (mut client, peer) = peer::start(modern, script).await;
    let catalog = peer::discover(&mut client, selected.clone()).await;
    let usable = if family == "tools" {
        client.tools().count()
    } else {
        catalog.descriptors.len()
    };
    let rejected = client.rejected_tools().len();
    if refresh {
        let limits =
            serde_json::from_value(json!({"max_pages":3,"max_items":16,"descriptor_octets":16384}))
                .map_err(|e| e.to_string())?;
        if client
            .discover(selected, &limits, peer::deadline())
            .await
            .is_ok()
        {
            return Err("failed refresh unexpectedly accepted".into());
        }
    }
    let deadline = if case == "invoke-deadline" {
        tokio::time::Instant::now() + tokio::time::Duration::from_millis(200)
    } else {
        peer::deadline()
    };
    let actual = match family {
        "tools" => client
            .call_tool("run", arguments, deadline)
            .await
            .map(|v| json!(v)),
        "resources" => client
            .read_resource("test:r", deadline)
            .await
            .map(|v| json!(v)),
        _ => client
            .get_prompt("p", arguments, deadline)
            .await
            .map(|v| json!(v)),
    };
    let requests = peer.finish().await;
    let calls: Vec<_> = requests
        .iter()
        .filter(|r| r["body"]["method"] == method)
        .collect();
    let headers = calls.last().map(|r| &r["headers"]);
    let parameter_count = headers.and_then(Value::as_object).map_or(0, |h| {
        h.keys().filter(|k| k.starts_with("mcp-param-")).count()
    });
    let agreement = calls.last().is_some_and(|r| {
        r["headers"]["mcp-param-region"] == "=?base64?aGVsbG8NCndvcmxk?="
            && r["body"]["params"]["arguments"]["route"] == "hello\r\nworld"
            && r["headers"]["mcp-param-count"] == "42"
            && r["body"]["params"]["arguments"]["count"] == 42
    });
    let (complete, reason, result, retained) = match actual {
        Ok(result) => (true, String::new(), result, true),
        Err(refusal) => {
            let retained = !refusal.exchange.is_absent();
            (
                false,
                serde_json::to_value(&refusal.reason)
                    .map_err(|e| e.to_string())?
                    .as_str()
                    .ok_or("reason")?
                    .to_owned(),
                Value::Null,
                retained,
            )
        }
    };
    let blocks = match family {
        "tools" => &result["content"],
        "resources" => &result["contents"],
        _ => &result["messages"],
    }
    .as_array()
    .map_or(0, Vec::len);
    let structured = match result.get("structured_content") {
        None => "absent",
        Some(Value::Null) => "null",
        Some(_) => "value",
    };
    Ok(
        json!({"complete":complete,"reason":reason,"calls":calls.len(),"blocks":blocks,"retained":retained,"structured":structured,"business_error":result["is_error"] == true,"usable":usable,"rejected":rejected,"parameter_headers":parameter_count,"header_agreement":agreement}),
    )
}
