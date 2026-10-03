//! Explicit caller-owned cancellation. A signal is not a remote rollback claim.
use b10x_mcp_types::http_exchange::{
    EssPresence, EssShape928035217e9a3240, EssShapeab4932fdb11b0759, EssShapeb303a11909beae35,
    EssShapec593b133f19ee716, McpHttpExchangeExchangeObservation as Observation,
    McpHttpExchangeExchangeResult as Exchange, McpHttpExchangeRefusalReason as Reason,
    McpHttpLifecycleCancellationObservation as Cancelled,
    McpHttpLifecycleControlledExchange as Outcome,
    McpHttpLifecycleInterruptedRequest as Interrupted, McpHttpLifecycleStopCause as Cause,
    McpHttpLifecycleStreamMessage as Message,
};
use std::future::Future;
use tokio::sync::watch;

/// A monotonic local signal that can be cloned while a client is mutably borrowed.
/// Dropping a clone does not cancel; calling `cancel` requests cancellation.
#[derive(Clone)]
pub struct Cancellation {
    sender: watch::Sender<bool>,
}
impl Default for Cancellation {
    fn default() -> Self {
        let (sender, _) = watch::channel(false);
        Self { sender }
    }
}
impl Cancellation {
    /// Create an unsignalled cancellation port.
    pub fn new() -> Self {
        Self::default()
    }
    /// Request cancellation; repeated calls do not dispatch additional controls.
    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }
    pub(crate) fn requested(&self) -> bool {
        *self.sender.borrow()
    }
    async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        let _ = receiver.wait_for(|requested| *requested).await;
    }
}

pub(crate) async fn until<F: Future>(
    cancellation: Option<&Cancellation>,
    action: F,
) -> Result<F::Output, &'static str> {
    let Some(cancellation) = cancellation else {
        return Ok(action.await);
    };
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err("caller_cancelled"),
        result = action => Ok(result),
    }
}

pub(crate) fn finished(exchange: Exchange) -> Outcome {
    Outcome::V1(EssShapeb303a11909beae35 {
        kind: EssShape928035217e9a3240::V0,
        value: Box::new(exchange),
    })
}
pub(crate) fn interrupted(exchange: &Exchange) -> Option<(Cause, &Observation)> {
    let Exchange::V1(refusal) = exchange else {
        return None;
    };
    let cause = match refusal.value.reason.as_ref() {
        Reason::V1 => Cause::V0,
        Reason::V2 => Cause::V1,
        _ => return None,
    };
    Some((cause, &refusal.value.observation))
}
pub(crate) fn cancelled(
    interrupted: Interrupted,
    notification: EssPresence<
        Box<b10x_mcp_types::http_exchange::McpHttpLifecycleControlObservation>,
    >,
) -> Outcome {
    Outcome::V0(EssShapeab4932fdb11b0759 {
        kind: EssShapec593b133f19ee716::V0,
        value: Box::new(Cancelled {
            interrupted: Box::new(interrupted),
            notification,
            late_response: EssPresence::Absent,
        }),
    })
}
pub(crate) fn retained(observation: &Observation) -> u64 {
    let bytes = |wire: &b10x_mcp_types::http_exchange::McpHttpExchangeWireObservation| {
        wire.response
            .counts
            .retained_octets
            .0
            .as_u64()
            .unwrap_or(u64::MAX)
    };
    let mut total = observation
        .response
        .counts
        .retained_octets
        .0
        .as_u64()
        .unwrap_or(u64::MAX);
    if let EssPresence::Present(stream) = &observation.stream {
        for message in &stream.messages {
            total = total.saturating_add(match message.as_ref() {
                Message::V0(message) => bytes(&message.value),
                Message::V1(message) => bytes(&message.value.exchange),
                Message::V2(message) => bytes(&message.value.request)
                    .saturating_add(bytes(&message.value.reply.exchange)),
            });
        }
    }
    total
}
