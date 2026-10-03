//! One-shot offline JSON Schema2020-12 validator; stdout contains only a status.
use b10x_mcp_types::http_exchange::{
    EssPresence, McpHttpInvocationSchemaAction as Action, McpHttpInvocationSchemaReply as Reply,
    McpHttpInvocationSchemaRequest as Request, McpHttpInvocationSchemaStatus as Status,
};
use clap::Parser;
use jsonschema::{Draft, PatternOptions, error::ValidationErrorKind};
use std::io::Read;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    max_input_bytes: u64,
}
fn validate(request: Request) -> Status {
    if !matches!(
        (request.action.as_ref(), &request.instance),
        (Action::V0, EssPresence::Absent) | (Action::V1, EssPresence::Present(_))
    ) {
        return Status::V1;
    }
    if let Some(dialect) = request.schema.get("$schema") {
        let Some(dialect) = dialect.as_str() else {
            return Status::V2;
        };
        if dialect.strip_suffix('#').unwrap_or(dialect)
            != "https://json-schema.org/draft/2020-12/schema"
        {
            return Status::V3;
        }
    }
    let compiled = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .offline()
        // The parent process owns the interruptible execution budget. An internal
        // backtracking cap must not masquerade as an instance-validation mismatch.
        .with_pattern_options(PatternOptions::fancy_regex().backtrack_limit(usize::MAX))
        .build(&request.schema);
    let validator = match compiled {
        Ok(validator) => validator,
        Err(error) => {
            return if matches!(error.kind(), ValidationErrorKind::Referencing(_)) {
                Status::V3
            } else {
                Status::V2
            };
        }
    };
    let EssPresence::Present(instance) = request.instance else {
        return Status::V4;
    };
    match validator.validate(&instance) {
        Ok(()) => Status::V4,
        Err(error) => {
            if unavailable(error.kind()) {
                Status::V5
            } else {
                Status::V0
            }
        }
    }
}
fn unavailable(kind: &ValidationErrorKind) -> bool {
    match kind {
        ValidationErrorKind::BacktrackLimitExceeded { .. }
        | ValidationErrorKind::RegexEngineFailure { .. }
        | ValidationErrorKind::Referencing(_) => true,
        ValidationErrorKind::AnyOf { context }
        | ValidationErrorKind::OneOfNotValid { context }
        | ValidationErrorKind::OneOfMultipleValid { context } => {
            context.iter().flatten().any(|e| unavailable(e.kind()))
        }
        ValidationErrorKind::PropertyNames { error } => unavailable(error.kind()),
        _ => false,
    }
}
fn read(limit: u64) -> Status {
    let mut bytes = Vec::new();
    if std::io::stdin()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > limit
    {
        return Status::V1;
    }
    match b10x_mcp_client::schema_worker::decode_request(&bytes) {
        Some(request) => validate(request),
        None => Status::V1,
    }
}
fn main() {
    let args = Args::parse();
    let status = read(args.max_input_bytes);
    let _ = serde_json::to_writer(
        std::io::stdout().lock(),
        &Reply {
            status: Box::new(status),
        },
    );
}
