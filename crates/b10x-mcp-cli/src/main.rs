#![forbid(unsafe_code)]
//! Standalone MCP client executable.

fn main() {
    let runtime = tokio_runtime();
    if let Err(error) = runtime.block_on(b10x_mcp_command::main_entry()) {
        eprintln!("b10x-mcp: {error}");
        std::process::exit(1);
    }
}

fn tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("b10x-mcp: creating runtime: {error}");
            std::process::exit(1);
        })
}
