//! Explicitly selected adversarial schema-worker fixture for lifecycle probes.
use clap::Parser;
use std::io::{Read, Write};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    max_input_bytes: u64,
}
fn main() {
    let args = Args::parse();
    let mut body = Vec::new();
    std::io::stdin()
        .take(args.max_input_bytes.saturating_add(1))
        .read_to_end(&mut body)
        .unwrap();
    let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
    if let Some(path) = request["instance"].as_str() {
        std::fs::write(path, std::process::id().to_string()).unwrap();
    }
    match request["schema"]["mode"].as_str() {
        Some("complete") => {
            std::io::stdout()
                .write_all(b"{\"status\":\"valid\"}")
                .unwrap();
            return;
        }
        Some("invalid-reply") => {
            std::io::stdout().write_all(b"not-json").unwrap();
            return;
        }
        Some("exit-failure") => std::process::exit(7),
        _ => {}
    }
    if request["schema"]["mode"] == "overflow" {
        std::io::stdout().write_all(&[b'x'; 1024]).unwrap();
        std::io::stdout().flush().unwrap();
    }
    std::thread::sleep(std::time::Duration::from_secs(30));
}
