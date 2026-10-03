//! One exchange's opted-in token and exact, bounded decimal progress ordering.
use b10x_mcp_types::http_exchange::McpHttpLifecycleStreamMessage as StreamMessage;
use num_bigint::BigInt;
use serde_json::{Number, Value, json};
use std::{cmp::Ordering, str::FromStr};

#[derive(Default)]
pub(crate) struct Progress {
    token: Option<Value>,
    previous: Option<Decimal>,
}
impl Progress {
    pub(crate) fn configure(&mut self, body: &[u8]) -> bool {
        let Some(request) = crate::strict_http::parse(body) else {
            return false;
        };
        let Some(meta) = request.get("params").and_then(|p| p.get("_meta")) else {
            return true;
        };
        let Some(meta) = meta.as_object() else {
            return false;
        };
        self.token = meta.get("progressToken").cloned();
        self.token.as_ref().is_none_or(valid_token)
    }

    pub(crate) fn observe(
        &mut self,
        params: &Value,
        wire: &Value,
    ) -> Option<(StreamMessage, bool)> {
        let params = params.as_object()?;
        let token = params.get("progressToken").filter(|v| valid_token(v))?;
        let progress = params.get("progress")?.as_number()?;
        let current = Decimal::new(progress)?;
        let mut observation = json!({
            "token":{"kind":if token.is_string() {"string"} else {"integer"},"value":token},
            "progress":progress,"exchange":wire,
        });
        if let Some(total) = params.get("total") {
            observation["total"] = json!(total.as_number()?);
        }
        if let Some(message) = params.get("message") {
            observation["message"] = json!(message.as_str()?);
        }
        let matched = self
            .token
            .as_ref()
            .is_some_and(|active| same_token(active, token));
        let non_increasing = matched
            && self
                .previous
                .as_ref()
                .is_some_and(|last| current.compare(last) != Ordering::Greater);
        observation["disposition"] = json!(if !matched {
            "unmatched_token"
        } else if non_increasing {
            "non_increasing"
        } else {
            "accepted"
        });
        if matched && !non_increasing {
            self.previous = Some(current);
        }
        let message =
            serde_json::from_value(json!({"kind":"progress","value":observation})).ok()?;
        Some((message, non_increasing))
    }
}

fn valid_token(value: &Value) -> bool {
    value.is_string()
        || value.as_number().is_some_and(|n| {
            let text = n.to_string();
            let digits = text.strip_prefix('-').unwrap_or(&text);
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        })
}
fn same_token(left: &Value, right: &Value) -> bool {
    match (left.as_number(), right.as_number()) {
        (Some(left), Some(right)) => Decimal::new(left)
            .zip(Decimal::new(right))
            .is_some_and(|(left, right)| left.compare(&right) == Ordering::Equal),
        _ => left == right,
    }
}

// Compare normalized scientific magnitudes without expanding exponents or using
// floating point. Both digit and exponent storage are bounded by input spelling.
struct Decimal {
    sign: i8,
    magnitude: BigInt,
    digits: Vec<u8>,
}
impl Decimal {
    fn new(number: &Number) -> Option<Self> {
        let text = number.to_string();
        let (sign, unsigned) = text
            .strip_prefix('-')
            .map_or((1, text.as_str()), |s| (-1, s));
        let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
        let fraction = mantissa.split_once('.').map_or(0, |(_, f)| f.len());
        let digits: Vec<_> = mantissa
            .bytes()
            .filter(|b| *b != b'.')
            .skip_while(|b| *b == b'0')
            .collect();
        let magnitude =
            BigInt::from_str(exponent).ok()? + BigInt::from(digits.len()) - BigInt::from(fraction);
        Some(Self {
            sign: if digits.is_empty() { 0 } else { sign },
            magnitude,
            digits,
        })
    }
    fn compare(&self, other: &Self) -> Ordering {
        let signs = self.sign.cmp(&other.sign);
        if signs != Ordering::Equal || self.sign == 0 {
            return signs;
        }
        let length = self.digits.len().max(other.digits.len());
        let absolute = self.magnitude.cmp(&other.magnitude).then_with(|| {
            self.digits
                .iter()
                .copied()
                .chain(std::iter::repeat(0x30))
                .take(length)
                .cmp(
                    other
                        .digits
                        .iter()
                        .copied()
                        .chain(std::iter::repeat(0x30))
                        .take(length),
                )
        });
        if self.sign < 0 {
            absolute.reverse()
        } else {
            absolute
        }
    }
}
