//! Cancellation ports preserve generated observations through invocation phases.
use super::super::{Action, Exchange, InvocationClient, Reason, Refusal, SchemaRequest, Status};
use crate::{
    schema_worker::SchemaWorker, strict_cancellation::Cancellation,
    strict_connection::StrictConnection,
};
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpLifecycleControlledExchange as ControlledExchange,
    McpHttpLifecycleControlledInvocationRefusal as Rejection,
    McpHttpLifecycleControlledSchemaResult as SchemaOutcome,
    McpHttpLifecycleInvocationInterruption as Interruption,
    McpHttpLifecycleInvocationPhase as Phase, McpHttpLifecycleStopCause as Cause,
};
use serde_json::Value;
use tokio::time::Instant;

pub(super) enum Failure {
    Refused(Box<Rejection>),
    Interrupted(Box<Interruption>),
}
impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(Box::new(Rejection {
            refusal: Box::new(refusal),
            phase: Box::new(Phase::V0),
            worker: EssPresence::Absent,
        }))
    }
}
impl Failure {
    pub(super) fn refused(reason: Reason, phase: Phase) -> Self {
        Self::Refused(Box::new(Rejection {
            refusal: Box::new(super::super::refusal(reason)),
            phase: Box::new(phase),
            worker: EssPresence::Absent,
        }))
    }
    fn interrupted(cause: Cause, phase: Phase) -> Self {
        Self::Interrupted(Box::new(Interruption {
            cause: Box::new(cause),
            phase: Box::new(phase),
            worker: EssPresence::Absent,
            cancellation: EssPresence::Absent,
            exchange: EssPresence::Absent,
        }))
    }
    pub(super) fn observed(mut self, exchange: &Exchange) -> Self {
        let slot = match &mut self {
            Self::Refused(value) => &mut value.refusal.exchange,
            Self::Interrupted(value) => &mut value.exchange,
        };
        *slot = EssPresence::Present(Box::new(exchange.clone()));
        self
    }
}
pub(super) struct Control<'a> {
    pub(super) signal: &'a Cancellation,
    pub(super) end: Instant,
    pub(super) teardown: Instant,
}
impl<'a> Control<'a> {
    pub(super) fn new(
        client: &InvocationClient,
        deadline: Instant,
        signal: &'a Cancellation,
        teardown: Instant,
    ) -> Result<Self, Failure> {
        let invalid = || Failure::refused(Reason::V2, Phase::V0);
        let control = Self {
            signal,
            end: client
                .connection
                .traversal_deadline(deadline)
                .ok_or_else(invalid)?,
            teardown: client
                .connection
                .traversal_deadline(teardown)
                .ok_or_else(invalid)?,
        };
        control.check(Phase::V0)?;
        Ok(control)
    }
    fn check(&self, phase: Phase) -> Result<(), Failure> {
        if self.signal.requested() {
            Err(Failure::interrupted(Cause::V0, phase))
        } else if Instant::now() >= self.end {
            Err(Failure::interrupted(Cause::V1, phase))
        } else {
            Ok(())
        }
    }
    pub(super) fn finish(&self, exchange: &Exchange) -> Result<(), Failure> {
        // A terminal already observed wins over a later signal. Parsing still
        // must respect the fixed operation deadline, retaining that terminal.
        if Instant::now() >= self.end {
            Err(Failure::interrupted(Cause::V1, Phase::V4).observed(exchange))
        } else {
            Ok(())
        }
    }
    pub(super) async fn schema(
        &self,
        worker: &mut SchemaWorker,
        schema: Value,
        instance: EssPresence<Value>,
        mismatch: Reason,
        phase: Phase,
    ) -> Result<(), Failure> {
        self.check(phase.clone())?;
        let request = SchemaRequest {
            action: Box::new(if instance.is_absent() {
                Action::V0
            } else {
                Action::V1
            }),
            schema,
            instance,
        };
        let result = worker
            .run_cancellable(&request, self.end, self.signal, self.teardown)
            .await
            .map_err(|r| Failure::refused(r, phase.clone()))?;
        match result {
            SchemaOutcome::V0(reply) => match reply.value.status.as_ref() {
                Status::V4 => Ok(()),
                Status::V0 => Err(Failure::refused(mismatch, phase)),
                Status::V2 | Status::V3 => Err(Failure::refused(Reason::V10, phase)),
                _ => Err(Failure::refused(Reason::V5, phase)),
            },
            SchemaOutcome::V1(stop) => Err(Failure::Interrupted(Box::new(Interruption {
                cause: stop.value.cause,
                phase: Box::new(phase),
                worker: EssPresence::Present(stop.value.cleanup),
                cancellation: EssPresence::Absent,
                exchange: EssPresence::Absent,
            }))),
            SchemaOutcome::V2(rejected) => {
                let reason =
                    if matches!(phase, Phase::V4) && matches!(*rejected.value.reason, Reason::V2) {
                        Reason::V5
                    } else {
                        *rejected.value.reason
                    };
                Err(Failure::Refused(Box::new(Rejection {
                    refusal: Box::new(super::super::refusal(reason)),
                    phase: Box::new(phase),
                    worker: EssPresence::Present(rejected.value.cleanup),
                })))
            }
        }
    }
    pub(super) async fn exchange(
        &self,
        connection: &mut StrictConnection,
        method: &str,
        params: Value,
        parameters: reqwest::header::HeaderMap,
    ) -> Result<Exchange, Failure> {
        self.check(Phase::V1)?;
        let outcome = connection
            .exchange_cancellable_with_parameters(
                method,
                params,
                parameters,
                self.end,
                self.signal,
                self.teardown,
            )
            .await
            .map_err(|_| Failure::refused(Reason::V2, Phase::V1))?;
        match outcome {
            ControlledExchange::V1(result) => Ok(*result.value),
            ControlledExchange::V0(stop) => Err(Failure::Interrupted(Box::new(Interruption {
                cause: stop.value.interrupted.cause.clone(),
                phase: Box::new(Phase::V1),
                cancellation: EssPresence::Present(stop.value),
                worker: EssPresence::Absent,
                exchange: EssPresence::Absent,
            }))),
        }
    }
}
