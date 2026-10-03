//! Direct generated carriers retain opaque JSON without decoding a second time.
use super::control::Failure;
use b10x_mcp_types::http_exchange::{
    EssShape1e90a263e411c9f4, EssShape4bd06133730c7a77, EssShape4cb83b7898aa9c0b,
    EssShape6c3b454369ef7404, EssShape59db0b3507c2fc39, EssShape355ee2fdef36609c,
    EssShape503c00fdabb1915d, EssShape2542ef374030badc, EssShape5859e42b1633f929,
    EssShape6933343151af4d0e, EssShapea43958865af4916d, EssShapeb9c6ab31d49f4b45,
    EssShapebc3c360d78fadfc2, EssShapec0e969e3791718e9, EssShaped5cac64e11c8f207,
    EssShaped7aefb90e562a1da, EssShaped285ed1d387b27ec, EssShapee735b2223d3a792a,
    McpHttpInvocationPromptResult, McpHttpInvocationResourceResult, McpHttpInvocationToolResult,
    McpHttpLifecycleControlledPromptResult, McpHttpLifecycleControlledResourceResult,
    McpHttpLifecycleControlledToolResult,
};
pub(super) fn tool(
    result: Result<McpHttpInvocationToolResult, Failure>,
) -> McpHttpLifecycleControlledToolResult {
    match result {
        Ok(value) => McpHttpLifecycleControlledToolResult::V0(EssShape503c00fdabb1915d {
            kind: EssShaped285ed1d387b27ec::V0,
            value: Box::new(value),
        }),
        Err(Failure::Interrupted(value)) => {
            McpHttpLifecycleControlledToolResult::V1(EssShape4cb83b7898aa9c0b {
                kind: EssShapeb9c6ab31d49f4b45::V0,
                value,
            })
        }
        Err(Failure::Refused(value)) => {
            McpHttpLifecycleControlledToolResult::V2(EssShape4bd06133730c7a77 {
                kind: EssShape6c3b454369ef7404::V0,
                value,
            })
        }
    }
}

pub(super) fn resource(
    result: Result<McpHttpInvocationResourceResult, Failure>,
) -> McpHttpLifecycleControlledResourceResult {
    match result {
        Ok(value) => McpHttpLifecycleControlledResourceResult::V0(EssShape2542ef374030badc {
            kind: EssShape1e90a263e411c9f4::V0,
            value: Box::new(value),
        }),
        Err(Failure::Interrupted(value)) => {
            McpHttpLifecycleControlledResourceResult::V1(EssShape59db0b3507c2fc39 {
                kind: EssShapea43958865af4916d::V0,
                value,
            })
        }
        Err(Failure::Refused(value)) => {
            McpHttpLifecycleControlledResourceResult::V2(EssShapee735b2223d3a792a {
                kind: EssShapec0e969e3791718e9::V0,
                value,
            })
        }
    }
}

pub(super) fn prompt(
    result: Result<McpHttpInvocationPromptResult, Failure>,
) -> McpHttpLifecycleControlledPromptResult {
    match result {
        Ok(value) => McpHttpLifecycleControlledPromptResult::V0(EssShape355ee2fdef36609c {
            kind: EssShapebc3c360d78fadfc2::V0,
            value: Box::new(value),
        }),
        Err(Failure::Interrupted(value)) => {
            McpHttpLifecycleControlledPromptResult::V1(EssShaped5cac64e11c8f207 {
                kind: EssShape6933343151af4d0e::V0,
                value,
            })
        }
        Err(Failure::Refused(value)) => {
            McpHttpLifecycleControlledPromptResult::V2(EssShaped7aefb90e562a1da {
                kind: EssShape5859e42b1633f929::V0,
                value,
            })
        }
    }
}
