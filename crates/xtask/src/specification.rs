//! Pinned specification gate and explicit tool provisioning.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    version: String,
    revision: String,
    url: String,
    sha256: String,
    member: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pins {
    ess: Tool,
    aep: Tool,
    scenarios: Vec<String>,
    http_replay_scenarios: Vec<String>,
    strict_http_scenarios: Vec<String>,
    strict_connection_scenarios: Vec<String>,
    strict_discovery_scenarios: Vec<String>,
    strict_invocation_scenarios: Vec<String>,
    partial_refusals: Vec<String>,
    authored_refusals: Vec<String>,
}

fn pins(root: &Path) -> Result<Pins, String> {
    serde_json::from_slice(&fs::read(root.join("toolchain.json")).map_err(|e| e.to_string())?)
        .map_err(|e| format!("toolchain.json: {e}"))
}

fn check_version(name: &str, version: &str, output: &str) -> Result<(), String> {
    if output.trim() == format!("{name} {version}") {
        Ok(())
    } else {
        Err(format!("expected {name} {version}, received {output:?}"))
    }
}

fn check_inventory(actual: &[String], expected: &[String]) -> Result<(), String> {
    let a: BTreeSet<_> = actual.iter().collect();
    let e: BTreeSet<_> = expected.iter().collect();
    if !e.is_empty() && a == e && a.len() == actual.len() && e.len() == expected.len() {
        Ok(())
    } else {
        Err(format!(
            "inventory drift: expected {expected:?}, observed {actual:?}"
        ))
    }
}

fn check_suite(actual: &[u8], committed: &[u8], ids: &[String]) -> Result<(), String> {
    if actual != committed {
        return Err("generated conformance suite differs from committed bytes".into());
    }
    let suite: serde_json::Value = serde_json::from_slice(actual).map_err(|e| e.to_string())?;
    let scenarios = suite
        .get("scenarios")
        .and_then(serde_json::Value::as_object)
        .ok_or("suite has no scenario map")?;
    check_inventory(&scenarios.keys().cloned().collect::<Vec<_>>(), ids)
}

fn check_archive(bytes: &[u8], digest: &str) -> Result<(), String> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual == digest {
        Ok(())
    } else {
        Err(format!(
            "archive SHA256 mismatch: expected {digest}, observed {actual}"
        ))
    }
}

fn output(command: &mut Command) -> Result<Output, String> {
    command
        .output()
        .map_err(|e| format!("start {}: {e}", command.get_program().to_string_lossy()))
}

fn diagnostics(output: &Output) -> Result<String, String> {
    let stdout = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
    let stderr = std::str::from_utf8(&output.stderr).map_err(|e| e.to_string())?;
    print!("{stdout}");
    eprint!("{stderr}");
    Ok(format!("{stdout}\n{stderr}"))
}

fn run(command: &mut Command, expected: i32) -> Result<String, String> {
    let result = output(command)?;
    let text = diagnostics(&result)?;
    if result.status.code() != Some(expected) {
        return Err(format!(
            "{}: expected exit {expected}, observed {}",
            command.get_program().to_string_lossy(),
            result.status
        ));
    }
    Ok(text)
}

fn tool(root: &Path, name: &str, pin: &Tool) -> Result<PathBuf, String> {
    let supplied = std::env::var_os(format!("{}_BIN", name.to_uppercase()));
    let local = root.join(".cache/tools").join(name);
    let executable = if let Some(supplied) = supplied {
        PathBuf::from(supplied)
    } else {
        match fs::symlink_metadata(&local) {
            Ok(_) => local,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => PathBuf::from(name),
            Err(error) => return Err(format!("inspect local {name} selection: {error}")),
        }
    };
    let version = output(Command::new(&executable).arg("--version"))?;
    if !version.status.success() {
        return Err(format!(
            "{} --version failed: {}",
            executable.display(),
            version.status
        ));
    }
    check_version(
        name,
        &pin.version,
        std::str::from_utf8(&version.stdout).map_err(|e| e.to_string())?,
    )?;
    Ok(executable)
}

fn refusals(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .map(|line| line.strip_prefix("refused: ").unwrap_or(line))
        .filter(|line| line.starts_with("refusal["))
        .map(str::to_owned)
        .collect()
}

fn source_pins(root: &Path, pins: &Pins) -> Result<(), String> {
    for package in ["b10x-mcp-types", "b10x-mcp-client"] {
        let manifest: toml::Value = toml::from_str(
            &fs::read_to_string(root.join(format!("crates/{package}/Cargo.toml")))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        for name in ["ess-conformance", "ess-primitives"] {
            if manifest
                .get("dev-dependencies")
                .and_then(|v| v.get(name))
                .and_then(|v| v.get("rev"))
                .and_then(toml::Value::as_str)
                != Some(pins.ess.revision.as_str())
            {
                return Err(format!(
                    "{package}: {name} source revision differs from ESS tool pin"
                ));
            }
        }
    }
    let project: serde_json::Value = serde_yaml::from_slice(
        &fs::read(root.join(".engineering/project.yaml")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if project.get("protocols").and_then(serde_json::Value::as_str)
        != Some(format!("git+https://github.com/beyond10x/aep#{}", pins.aep.revision).as_str())
    {
        return Err("AEP protocol source differs from tool pin".into());
    }
    Ok(())
}

pub(super) fn check(root: &Path) -> Result<(), String> {
    let pins = pins(root)?;
    source_pins(root, &pins)?;
    let ess = tool(root, "ess", &pins.ess)?;
    let aep = tool(root, "aep", &pins.aep)?;
    fs::create_dir_all(root.join(".cache")).map_err(|e| e.to_string())?;
    let temporary = tempfile::tempdir_in(root.join(".cache")).map_err(|e| e.to_string())?;
    run(
        Command::new(&ess)
            .args(["specify", "validate", "--path", "."])
            .current_dir(root),
        0,
    )?;
    let suite_path = temporary.path().join("constructors.json");
    let partial = run(
        Command::new(&ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                ".",
                "--scenarios",
                ".",
                "--out",
            ])
            .arg(&suite_path)
            .current_dir(root),
        0,
    )?;
    check_inventory(&refusals(&partial), &pins.partial_refusals)?;
    check_suite(
        &fs::read(&suite_path).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/constructors.json")).map_err(|e| e.to_string())?,
        &pins.scenarios,
    )?;
    let full = run(
        Command::new(&ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                ".",
                "--scenarios",
                "conformance/scenarios",
                "--out",
            ])
            .arg(temporary.path().join("full.json"))
            .current_dir(root),
        1,
    )?;
    check_inventory(&refusals(&full), &pins.authored_refusals)?;
    run(
        Command::new(&ess)
            .args([
                "specify",
                "validate",
                "--path",
                "conformance/http-replay/spec",
            ])
            .current_dir(root),
        0,
    )?;
    let http_suite = temporary.path().join("http-replay.json");
    let http = run(
        Command::new(&ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "conformance/http-replay/spec",
                "--scenarios",
                "conformance/http-replay/scenarios",
                "--out",
            ])
            .arg(&http_suite)
            .current_dir(root),
        0,
    )?;
    if !refusals(&http).is_empty() {
        return Err("HTTP replay selection has synthesis refusals".into());
    }
    check_suite(
        &fs::read(&http_suite).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/http-replay/suite.json")).map_err(|e| e.to_string())?,
        &pins.http_replay_scenarios,
    )?;
    check_strict_profiles(root, &ess, temporary.path(), &pins)?;
    run(
        Command::new(aep)
            .args(["plan", "artifact", "validate"])
            .current_dir(root),
        0,
    )?;
    println!(
        "specification: constructor, HTTP replay, strict HTTP, connection, discovery and invocation suites plus generated types verified; explicit partial refusal inventory retained; execution follows in workspace tests; full conformance remains inconclusive"
    );
    Ok(())
}

fn check_strict_profiles(
    root: &Path,
    ess: &Path,
    temporary: &Path,
    pins: &Pins,
) -> Result<(), String> {
    check_strict_http(root, ess, temporary, &pins.strict_http_scenarios)?;
    check_strict_connection(root, ess, temporary, &pins.strict_connection_scenarios)?;
    check_strict_discovery(root, ess, temporary, &pins.strict_discovery_scenarios)?;
    check_strict_invocation(root, ess, temporary, &pins.strict_invocation_scenarios)
}

fn check_strict_invocation(
    root: &Path,
    ess: &Path,
    temporary: &Path,
    ids: &[String],
) -> Result<(), String> {
    run(
        Command::new(ess)
            .args([
                "specify",
                "validate",
                "--path",
                "conformance/strict-invocation/spec",
            ])
            .current_dir(root),
        0,
    )?;
    let suite = temporary.join("strict-invocation.json");
    let text = run(
        Command::new(ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "conformance/strict-invocation/spec",
                "--scenarios",
                "conformance/strict-invocation/scenarios",
                "--out",
            ])
            .arg(&suite)
            .current_dir(root),
        0,
    )?;
    if !refusals(&text).is_empty() {
        return Err("strict invocation selection has synthesis refusals".into());
    }
    check_suite(
        &fs::read(suite).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/strict-invocation/suite.json"))
            .map_err(|e| e.to_string())?,
        ids,
    )
}

fn check_strict_discovery(
    root: &Path,
    ess: &Path,
    temporary: &Path,
    ids: &[String],
) -> Result<(), String> {
    run(
        Command::new(ess)
            .args([
                "specify",
                "validate",
                "--path",
                "conformance/strict-discovery/spec",
            ])
            .current_dir(root),
        0,
    )?;
    let suite_path = temporary.join("strict-discovery.json");
    let output = run(
        Command::new(ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "conformance/strict-discovery/spec",
                "--scenarios",
                "conformance/strict-discovery/scenarios",
                "--out",
            ])
            .arg(&suite_path)
            .current_dir(root),
        0,
    )?;
    if !refusals(&output).is_empty() {
        return Err("strict discovery selection has synthesis refusals".into());
    }
    check_suite(
        &fs::read(&suite_path).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/strict-discovery/suite.json"))
            .map_err(|e| e.to_string())?,
        ids,
    )
}

fn check_strict_http(
    root: &Path,
    ess: &Path,
    temporary: &Path,
    ids: &[String],
) -> Result<(), String> {
    run(
        Command::new(ess)
            .args([
                "specify",
                "validate",
                "--path",
                "conformance/strict-http/spec",
            ])
            .current_dir(root),
        0,
    )?;
    let strict_suite = temporary.join("strict-http.json");
    let strict = run(
        Command::new(ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "conformance/strict-http/spec",
                "--scenarios",
                "conformance/strict-http/scenarios",
                "--out",
            ])
            .arg(&strict_suite)
            .current_dir(root),
        0,
    )?;
    if !refusals(&strict).is_empty() {
        return Err("strict HTTP selection has synthesis refusals".into());
    }
    check_suite(
        &fs::read(&strict_suite).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/strict-http/suite.json")).map_err(|e| e.to_string())?,
        ids,
    )?;
    check_generated_types(root, ess, temporary)
}

fn check_generated_types(root: &Path, ess: &Path, temporary: &Path) -> Result<(), String> {
    let generated = temporary.join("http-values");
    run(
        Command::new(ess)
            .args([
                "generate",
                "types",
                "--path",
                "ess",
                "--target",
                "rust",
                "--package",
                "mcp-http-exchange-values",
                "--root",
                "mcp.http_exchange.ExchangeInput",
                "--root",
                "mcp.http_exchange.ExchangeResult",
                "--root",
                "mcp.http_connection.SetupInput",
                "--root",
                "mcp.http_connection.PeerDescription",
                "--root",
                "mcp.http_connection.SetupRefusal",
                "--root",
                "mcp.http_discovery.ListLimits",
                "--root",
                "mcp.http_discovery.Catalog",
                "--root",
                "mcp.http_discovery.Refusal",
                "--root",
                "mcp.http_invocation.SchemaRequest",
                "--root",
                "mcp.http_invocation.SchemaReply",
                "--root",
                "mcp.http_invocation.ToolResult",
                "--root",
                "mcp.http_invocation.ResourceResult",
                "--root",
                "mcp.http_invocation.PromptResult",
                "--root",
                "mcp.http_invocation.Refusal",
                "--root",
                "mcp.http_invocation.ParameterHeader",
                "--root",
                "mcp.http_invocation.RejectedTool",
                "--out",
            ])
            .arg(&generated)
            .current_dir(root),
        0,
    )?;
    for file in [
        "Cargo.toml",
        "types.rs",
        "types-report.json",
        "source.schema.json",
    ] {
        let committed = root
            .join("crates/b10x-mcp-types/src/http_exchange_generated")
            .join(file);
        if fs::read(generated.join(file)).map_err(|e| e.to_string())?
            != fs::read(committed).map_err(|e| e.to_string())?
        {
            return Err(format!("strict HTTP generated type drift: {file}"));
        }
    }
    Ok(())
}

fn check_strict_connection(
    root: &Path,
    ess: &Path,
    temporary: &Path,
    ids: &[String],
) -> Result<(), String> {
    let suite = temporary.join("strict-connection.json");
    let output = run(
        Command::new(ess)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "conformance/strict-connection/spec",
                "--scenarios",
                "conformance/strict-connection/scenarios",
                "--out",
            ])
            .arg(&suite)
            .current_dir(root),
        0,
    )?;
    if !refusals(&output).is_empty() {
        return Err("strict connection selection has synthesis refusals".into());
    }
    check_suite(
        &fs::read(suite).map_err(|e| e.to_string())?,
        &fs::read(root.join("conformance/strict-connection/suite.json"))
            .map_err(|e| e.to_string())?,
        ids,
    )
}

pub(super) fn bootstrap(root: &Path) -> Result<(), String> {
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return Err("bootstrap-tools supports Linux x86_64 only; supply pinned ESS_BIN and AEP_BIN on other hosts".into());
    }
    let pins = pins(root)?;
    let destination = root.join(".cache/tools");
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    for (name, pin) in [("ess", &pins.ess), ("aep", &pins.aep)] {
        let temporary = tempfile::tempdir_in(&destination).map_err(|e| e.to_string())?;
        let archive = temporary.path().join("archive.tar.gz");
        run(
            Command::new("curl")
                .args([
                    "--fail",
                    "--location",
                    "--silent",
                    "--show-error",
                    "--proto",
                    "=https",
                    "--proto-redir",
                    "=https",
                    "--max-time",
                    "120",
                    "--max-filesize",
                    "67108864",
                    "--output",
                ])
                .arg(&archive)
                .arg(&pin.url),
            0,
        )?;
        check_archive(&fs::read(&archive).map_err(|e| e.to_string())?, &pin.sha256)?;
        let extracted = output(
            Command::new("tar")
                .arg("-xOzf")
                .arg(&archive)
                .arg("--")
                .arg(&pin.member),
        )?;
        if !extracted.status.success() || extracted.stdout.is_empty() {
            return Err(format!(
                "extract pinned {name} member: {}",
                extracted.status
            ));
        }
        let binary = temporary.path().join(name);
        fs::write(&binary, extracted.stdout).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        let version = output(Command::new(&binary).arg("--version"))?;
        if !version.status.success() {
            return Err(format!("downloaded {name} --version: {}", version.status));
        }
        check_version(
            name,
            &pin.version,
            std::str::from_utf8(&version.stdout).map_err(|e| e.to_string())?,
        )?;
        fs::rename(&binary, destination.join(name)).map_err(|e| e.to_string())?;
        println!("installed pinned {name} {} in .cache/tools", pin.version);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn dangling_local_tool_refuses_instead_of_falling_back_to_path() {
        const CHILD: &str = "MCP_GATE_DANGLING_TOOL_CHILD";
        const CASE: &str =
            "specification::tests::dangling_local_tool_refuses_instead_of_falling_back_to_path";
        if std::env::var_os(CHILD).is_none() {
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CASE, "--nocapture"])
                .env(CHILD, "1")
                .env_remove("RUSTC_BIN")
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "isolated tool-resolution case failed: {}\n{}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr)
            );
            return;
        }

        let version = Command::new("rustc").arg("--version").output().unwrap();
        assert!(version.status.success());
        let version = String::from_utf8(version.stdout).unwrap();
        let pin = Tool {
            version: version.trim().strip_prefix("rustc ").unwrap().to_owned(),
            revision: String::new(),
            url: String::new(),
            sha256: String::new(),
            member: String::new(),
        };
        let scratch = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/gate-review");
        fs::create_dir_all(&scratch).unwrap();
        let fixture = tempfile::tempdir_in(scratch).unwrap();
        // A truly absent local selection permits the explicitly documented PATH fallback.
        assert_eq!(
            tool(fixture.path(), "rustc", &pin).unwrap(),
            PathBuf::from("rustc")
        );
        let local = fixture.path().join(".cache/tools/rustc");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(fixture.path().join("missing-rustc"), &local).unwrap();
        assert!(
            fs::symlink_metadata(&local)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            tool(fixture.path(), "rustc", &pin).is_err(),
            "dangling selected local executable silently fell back to PATH"
        );
    }

    #[test]
    fn wrong_or_ambiguous_tool_version_is_refused() {
        assert!(check_version("ess", "0.50.0", "ess 0.50.0\n").is_ok());
        for output in ["ess 0.51.0", "ess 0.50.0\nextra", "aep 0.50.0", ""] {
            assert!(
                check_version("ess", "0.50.0", output).is_err(),
                "accepted {output:?}"
            );
        }
    }

    #[test]
    fn missing_new_duplicate_and_empty_refusal_inventory_are_refused() {
        let expected = vec![
            "refusal[A]: first".to_owned(),
            "refusal[B]: second".to_owned(),
        ];
        assert!(check_inventory(&expected, &expected).is_ok());
        for actual in [
            vec![],
            vec![expected[0].clone()],
            vec![expected[0].clone(); 2],
            vec!["refusal[C]: new".to_owned(), expected[1].clone()],
        ] {
            assert!(check_inventory(&actual, &expected).is_err());
        }
    }

    #[test]
    fn suite_drift_and_zero_or_missing_cases_are_refused() {
        let suite = br#"{"scenarios":{"one":{"steps":[1]}}}"#;
        let ids = vec!["one".to_owned()];
        assert!(check_suite(suite, suite, &ids).is_ok());
        assert!(check_suite(br#"{"scenarios":{"one":{"steps":[2]}}}"#, suite, &ids).is_err());
        let empty = br#"{"scenarios":{}}"#;
        assert!(check_suite(empty, empty, &ids).is_err());
        assert!(check_suite(empty, empty, &[]).is_err());
        assert!(check_suite(suite, suite, &["two".to_owned()]).is_err());
    }

    #[test]
    fn archive_digest_is_checked_before_installation() {
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(check_archive(b"abc", digest).is_ok());
        assert!(check_archive(b"abd", digest).is_err());
    }
}
