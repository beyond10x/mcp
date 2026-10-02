#![forbid(unsafe_code)]
//! Repository gate runner.

use clap::{Parser, Subcommand};
use std::process::{Command, ExitCode};

mod specification;

#[derive(Parser)]
#[command(about = "MCP repository checks and explicit pinned tool provisioning")]
struct Cli {
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Run specification checks and the complete Rust repository gate.
    Gate,
    /// Validate pins, ESS suite/refusals and AEP without compiling runtime crates.
    Specification,
    /// Explicitly download verified Linux `x86_64` tools into .cache/tools.
    BootstrapTools,
}

fn main() -> ExitCode {
    let action = Cli::parse().action.unwrap_or(Action::Gate);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Err(error) = std::env::set_current_dir(&root) {
        eprintln!("repository root: {error}");
        return ExitCode::FAILURE;
    }
    let result = match action {
        Action::BootstrapTools => specification::bootstrap(&root),
        Action::Gate | Action::Specification => specification::check(&root),
    };
    if let Err(error) = result {
        eprintln!("specification gate: {error}");
        return ExitCode::FAILURE;
    }
    if !matches!(action, Action::Gate) {
        return ExitCode::SUCCESS;
    }
    for (program, arguments) in [
        ("cargo", &["fmt", "--all", "--check"][..]),
        ("cargo", &["test", "--workspace", "--locked"][..]),
        (
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ][..],
        ),
        (
            "cargo",
            &["doc", "--workspace", "--no-deps", "--locked"][..],
        ),
    ] {
        let status = Command::new(program)
            .args(arguments)
            .env("RUSTDOCFLAGS", "-Dwarnings")
            .status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!("{program} failed with {status}");
                return ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("starting {program}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
