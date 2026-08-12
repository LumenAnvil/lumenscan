//! Download a deployed contract's WASM, for producing test fixtures.
//!
//! Fixtures live in `tests/fixtures/` so that `cargo test` never touches the
//! network. This example is how they are (re)generated:
//!
//! ```console
//! cargo run --example fetch_fixture -- <CONTRACT_ID> tests/fixtures/name.wasm
//! ```
//!
//! Pass `--rpc-url <URL>` (or set `LUMENSCAN_RPC_URL`) to use a network other
//! than testnet.

use std::process::ExitCode;

use lumenscan::rpc::{parse_contract_id, RpcFetcher, WasmFetcher, DEFAULT_RPC_URL};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let contract_id = args
        .next()
        .ok_or("usage: fetch_fixture <CONTRACT_ID> <OUT>")?;
    let out_path = args
        .next()
        .ok_or("usage: fetch_fixture <CONTRACT_ID> <OUT>")?;

    let rpc_url = match (args.next().as_deref(), args.next()) {
        (Some("--rpc-url"), Some(url)) => url,
        _ => std::env::var("LUMENSCAN_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_owned()),
    };

    let contract = parse_contract_id(&contract_id)?;
    let wasm = RpcFetcher::new(&rpc_url).fetch_wasm(&contract)?;
    std::fs::write(&out_path, &wasm)?;

    Ok(format!(
        "wrote {} bytes to {out_path}\n  contract: {contract_id}\n  rpc:      {rpc_url}",
        wasm.len()
    ))
}
