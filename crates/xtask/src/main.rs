#![forbid(unsafe_code)]
//! Repository gate runner.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let action = std::env::args().nth(1).unwrap_or_else(|| "gate".to_owned());
    if action != "gate" {
        eprintln!("unknown xtask `{action}`; expected `gate`");
        return ExitCode::FAILURE;
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
