//! Controlled traversal ports and direct generated outcome construction.
use super::{Catalog, ExchangeResult, Fault, Reason, Refusal, StrictConnection};
use crate::strict_cancellation::Cancellation;
use b10x_mcp_types::http_exchange::{
    EssPresence, EssShape17c8d42bb96dc547, EssShape64ada33bc56c63b9, EssShape3667691150a7def4,
    EssShapee86ac8b3bf5e5f1d, EssShapeefdc0f4111f43bf2, EssShapef7f37ee1e93186d3,
    McpHttpLifecycleControlledDiscoveryResult as Outcome,
    McpHttpLifecycleControlledExchange as ControlledExchange,
    McpHttpLifecycleDiscoveryInterruption as Interruption, McpHttpLifecycleStopCause as Cause,
};
use serde_json::Value;
use tokio::time::Instant;

pub(super) enum Failure {
    Refused(Box<Refusal>),
    Interrupted(Box<Interruption>),
}
impl From<Refusal> for Failure {
    fn from(value: Refusal) -> Self {
        Self::Refused(Box::new(value))
    }
}
pub(super) type Port<'a> = Option<(&'a Cancellation, Instant)>;

pub(super) fn outcome(result: Result<Catalog, Failure>) -> Outcome {
    match result {
        Ok(catalog) => Outcome::V0(EssShape64ada33bc56c63b9 {
            kind: EssShape3667691150a7def4::V0,
            value: Box::new(catalog),
        }),
        Err(Failure::Refused(value)) => Outcome::V2(EssShapeefdc0f4111f43bf2 {
            kind: EssShapef7f37ee1e93186d3::V0,
            value,
        }),
        Err(Failure::Interrupted(value)) => Outcome::V1(EssShape17c8d42bb96dc547 {
            kind: EssShapee86ac8b3bf5e5f1d::V0,
            value,
        }),
    }
}
// Raw callers have no interruption port. Preserve their pre-existing deadline
// refusal representation without changing the controlled public carrier.
pub(super) fn raw(failure: Failure) -> Refusal {
    match failure {
        Failure::Refused(value) => *value,
        Failure::Interrupted(value) => Refusal {
            reason: Box::new(Reason::V0),
            exchanges: value.exchanges,
            actual: EssPresence::Absent,
            limit: EssPresence::Absent,
        },
    }
}
pub(super) fn refusal(fault: Fault, exchanges: Vec<ExchangeResult>) -> Failure {
    super::refusal(fault, exchanges).into()
}
pub(super) fn boundary(
    port: Port<'_>,
    end: Instant,
    observed: &[ExchangeResult],
) -> Result<(), Failure> {
    let cause = if port.is_some_and(|(signal, _)| signal.requested()) {
        Some(Cause::V0)
    } else if Instant::now() >= end {
        Some(Cause::V1)
    } else {
        None
    };
    let Some(cause) = cause else {
        return Ok(());
    };
    if port.is_none() {
        return Err(refusal(super::fault(Reason::V0), observed.to_vec()));
    }
    Err(Failure::Interrupted(Box::new(Interruption {
        cause: Box::new(cause),
        exchanges: observed.iter().cloned().map(Box::new).collect(),
        cancellation: EssPresence::Absent,
    })))
}
pub(super) async fn exchange(
    connection: &mut StrictConnection,
    method: &str,
    params: Value,
    end: Instant,
    port: Port<'_>,
    observed: &[ExchangeResult],
) -> Result<ExchangeResult, Failure> {
    let Some((signal, teardown)) = port else {
        return connection
            .exchange(method, params, end)
            .await
            .map_err(|_| refusal(super::fault(Reason::V3), observed.to_vec()));
    };
    let result = connection
        .exchange_cancellable(method, params, end, signal, teardown)
        .await
        .map_err(|_| refusal(super::fault(Reason::V3), observed.to_vec()))?;
    match result {
        ControlledExchange::V1(finished) => Ok(*finished.value),
        ControlledExchange::V0(cancelled) => Err(Failure::Interrupted(Box::new(Interruption {
            cause: cancelled.value.interrupted.cause.clone(),
            exchanges: observed.iter().cloned().map(Box::new).collect(),
            cancellation: EssPresence::Present(cancelled.value),
        }))),
    }
}
