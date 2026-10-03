//! Modern tool-schema projection and exact argument-to-header encoding.
use b10x_mcp_types::http_exchange::{
    McpHttpInvocationHeaderPrimitive as Primitive,
    McpHttpInvocationHeaderRefusalReason as HeaderReason,
    McpHttpInvocationParameterHeader as Parameter, McpHttpInvocationRefusalReason as Reason,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn project(schema: &Value) -> Result<Vec<Parameter>, HeaderReason> {
    let mut pending = vec![(schema, Some(Vec::<String>::new()))];
    let mut names = BTreeSet::new();
    let mut parameters = Vec::new();
    while let Some((schema, path)) = pending.pop() {
        let Some(fields) = schema.as_object() else {
            continue;
        };
        if let Some(annotation) = fields.get("x-mcp-header") {
            let path = path
                .as_ref()
                .filter(|p| !p.is_empty())
                .ok_or(HeaderReason::V1)?;
            let suffix = annotation.as_str().ok_or(HeaderReason::V2)?;
            if suffix.is_empty() || HeaderName::from_bytes(suffix.as_bytes()).is_err() {
                return Err(HeaderReason::V2);
            }
            if !names.insert(suffix.to_ascii_lowercase()) {
                return Err(HeaderReason::V0);
            }
            let primitive = match fields.get("type").and_then(Value::as_str) {
                Some("boolean") => Primitive::V0,
                Some("integer") => Primitive::V1,
                Some("string") => Primitive::V2,
                _ => return Err(HeaderReason::V3),
            };
            parameters.push(Parameter {
                parameter_path: path.clone(),
                suffix: suffix.to_owned(),
                primitive: Box::new(primitive),
            });
        }
        for (keyword, value) in fields {
            match keyword.as_str() {
                "properties" => {
                    if let Some(properties) = value.as_object() {
                        for (name, child) in properties {
                            let child_path = path.as_ref().map(|p| {
                                let mut p = p.clone();
                                p.push(name.clone());
                                p
                            });
                            pending.push((child, child_path));
                        }
                    }
                }
                "$defs" | "definitions" | "patternProperties" | "dependentSchemas"
                | "dependencies" => {
                    if let Some(children) = value.as_object() {
                        pending.extend(children.values().map(|child| (child, None)));
                    }
                }
                "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
                    if let Some(children) = value.as_array() {
                        pending.extend(children.iter().map(|child| (child, None)));
                    }
                }
                "items" if value.is_array() => {
                    pending.extend(
                        value
                            .as_array()
                            .expect("array checked")
                            .iter()
                            .map(|child| (child, None)),
                    );
                }
                "items"
                | "additionalItems"
                | "contains"
                | "unevaluatedItems"
                | "additionalProperties"
                | "unevaluatedProperties"
                | "propertyNames"
                | "not"
                | "if"
                | "then"
                | "else"
                | "contentSchema" => pending.push((value, None)),
                // const/enum/examples and unknown extension values are data, not schemas.
                _ => {}
            }
        }
    }
    parameters.sort_by(|a, b| a.parameter_path.cmp(&b.parameter_path));
    Ok(parameters)
}

pub(crate) fn encode(value: &str) -> String {
    if value
        .bytes()
        .all(|b| (0x20..=0x7e).contains(&b) || b == b'\t')
        && value.trim() == value
        && !(value.starts_with("=?base64?") && value.ends_with("?="))
    {
        value.to_owned()
    } else {
        format!("=?base64?{}?=", STANDARD.encode(value.as_bytes()))
    }
}

// JSON integer semantics include decimal/exponent forms. Do not round through f64:
// 9007199254740991.1 must never turn into an admitted safe integer.
fn integer(value: &Value) -> Option<String> {
    let text = value.as_number()?.to_string();
    let (negative, unsigned) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |s| (true, s));
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let fraction = mantissa.split_once('.').map_or(0, |(_, f)| f.len());
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some("0".into());
    }
    let exponent = exponent.parse::<i64>().ok()?;
    let shift = exponent.checked_sub(i64::try_from(fraction).ok()?)?;
    let mut whole = if shift < 0 {
        let cut = usize::try_from(shift.checked_neg()?).ok()?;
        if cut >= digits.len() || !digits[digits.len() - cut..].bytes().all(|b| b == b'0') {
            return None;
        }
        digits[..digits.len() - cut].to_owned()
    } else {
        let zeros = usize::try_from(shift).ok()?;
        if digits.len().checked_add(zeros)? > 16 {
            return None;
        }
        let mut whole = digits.to_owned();
        whole.extend(std::iter::repeat_n('0', zeros));
        whole
    };
    if whole.len() > 16 || whole.parse::<u64>().ok()? > 9_007_199_254_740_991 {
        return None;
    }
    if negative {
        whole.insert(0, '-');
    }
    Some(whole)
}

pub(crate) fn extract(parameters: &[Parameter], arguments: &Value) -> Result<HeaderMap, Reason> {
    let mut headers = HeaderMap::new();
    for parameter in parameters {
        let mut value = Some(arguments);
        for segment in &parameter.parameter_path {
            value = value.and_then(|v| v.as_object()?.get(segment));
        }
        let Some(value) = value else { continue };
        let value = match parameter.primitive.as_ref() {
            Primitive::V0 => value.as_bool().map(|b| b.to_string()),
            Primitive::V1 => integer(value),
            Primitive::V2 => value.as_str().map(encode),
        }
        .ok_or(Reason::V2)?;
        let name = HeaderName::from_bytes(format!("mcp-param-{}", parameter.suffix).as_bytes())
            .map_err(|_| Reason::V2)?;
        let mut value = HeaderValue::from_str(&value).map_err(|_| Reason::V2)?;
        value.set_sensitive(true);
        headers.try_insert(name, value).map_err(|_| Reason::V2)?;
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn schema_locations_are_distinct_from_literal_data() {
        let literal = json!({"x-mcp-header":"", "type":"number"});
        let valid = json!({"type":"object","const":literal,"examples":[literal],"enum":[literal],"properties":{"nested":{"properties":{"x":{"type":"string","x-mcp-header":"X"}}}}});
        let params = project(&valid).unwrap();
        assert_eq!(params[0].parameter_path, ["nested", "x"]);
        for keyword in [
            "items",
            "not",
            "if",
            "then",
            "else",
            "additionalProperties",
            "contentSchema",
        ] {
            let bad = json!({keyword:{"properties":{"x":{"type":"string","x-mcp-header":"X"}}}});
            assert!(matches!(project(&bad), Err(HeaderReason::V1)), "{keyword}");
        }
        for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
            let bad = json!({keyword:[{"properties":{"x":{"type":"string","x-mcp-header":"X"}}}]});
            assert!(matches!(project(&bad), Err(HeaderReason::V1)), "{keyword}");
        }
        let bad = json!({"$defs":{"x":{"type":"string","x-mcp-header":"X"}},"properties":{"x":{"$ref":"#/$defs/x"}}});
        assert!(matches!(project(&bad), Err(HeaderReason::V1)));
    }
    #[test]
    fn schema_names_types_and_uniqueness_are_enforced() {
        for name in ["", "a b", "x\r\ny", "ümlaut", ":"] {
            assert!(matches!(
                project(&json!({"properties":{"x":{"type":"string","x-mcp-header":name}}})),
                Err(HeaderReason::V2)
            ));
        }
        for kind in [
            json!("number"),
            json!("object"),
            json!(["string", "null"]),
            Value::Null,
        ] {
            assert!(matches!(
                project(&json!({"properties":{"x":{"type":kind,"x-mcp-header":"X"}}})),
                Err(HeaderReason::V3)
            ));
        }
        assert!(matches!(
            project(
                &json!({"properties":{"x":{"type":"string","x-mcp-header":"X"},"y":{"type":"boolean","x-mcp-header":"x"}}})
            ),
            Err(HeaderReason::V0)
        ));
    }
    #[test]
    fn exact_numbers_encoding_and_missing_paths() {
        for (raw, expected) in [
            ("1.0", Some("1")),
            ("1e3", Some("1000")),
            ("-0", Some("0")),
            ("-9007199254740991", Some("-9007199254740991")),
            ("9007199254740992", None),
            ("9007199254740991.1", None),
            ("1e100000", None),
            ("0.1", None),
        ] {
            assert_eq!(
                integer(&serde_json::from_str(raw).unwrap()).as_deref(),
                expected,
                "{raw}"
            );
        }
        let params = project(&json!({"properties":{"nested":{"properties":{"x":{"type":"string","x-mcp-header":"Region"}}},"n":{"type":"integer","x-mcp-header":"Count"},"b":{"type":"boolean","x-mcp-header":"Flag"}}})).unwrap();
        assert!(extract(&params, &json!({})).unwrap().is_empty());
        let headers = extract(
            &params,
            &json!({"nested":{"x":"hello\r\nworld"},"n":42,"b":false}),
        )
        .unwrap();
        assert_eq!(headers["mcp-param-region"], "=?base64?aGVsbG8NCndvcmxk?=");
        assert_eq!(headers["mcp-param-count"], "42");
        assert_eq!(headers["mcp-param-flag"], "false");
        assert_eq!(encode("=?base64?x?="), "=?base64?PT9iYXNlNjQ/eD89?=");
        assert_eq!(encode("=?base64?plain"), "=?base64?plain");
        assert_eq!(encode(" leading"), "=?base64?IGxlYWRpbmc=?=");
        assert!(extract(&params, &json!({"n":9_007_199_254_740_992_u64})).is_err());
    }
}
