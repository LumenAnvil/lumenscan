use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use lumenscan::output::{Human, Json, Renderer};
use lumenscan::rpc::{parse_contract_id, RpcFetcher, WasmFetcher, DEFAULT_RPC_URL};
use lumenscan::{Inspection, Result, Source};

/// Inspect a deployed Soroban contract's interface and metadata.
#[derive(Debug, Parser)]
#[command(
    name = "lumenscan",
    version,
    about,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(flatten)]
    target: Target,

    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    options: Options,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the contract's metadata only.
    Meta {
        #[command(flatten)]
        target: Target,

        #[command(flatten)]
        options: Options,
    },
}

/// What to inspect: a deployed contract, or a local WASM file.
#[derive(Debug, Args)]
#[group(required = false, multiple = false)]
struct Target {
    /// Contract ID to fetch from the network, e.g. CDLZFC3S…
    contract_id: Option<String>,

    /// Inspect a local .wasm file instead of fetching from the network.
    #[arg(long, value_name = "PATH")]
    wasm: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct Options {
    /// Stellar RPC endpoint used to fetch deployed contracts.
    #[arg(long, value_name = "URL", env = "LUMENSCAN_RPC_URL", default_value = DEFAULT_RPC_URL)]
    rpc_url: String,

    /// Emit machine-readable JSON instead of a human-readable listing.
    #[arg(long)]
    json: bool,
}

impl Options {
    fn renderer(&self) -> Box<dyn Renderer> {
        if self.json {
            Box::new(Json)
        } else {
            Box::new(Human)
        }
    }
}

impl Target {
    /// Resolve the target to a source and its WASM bytes.
    fn load(&self, rpc_url: &str) -> Result<(Source, Vec<u8>)> {
        match (&self.contract_id, &self.wasm) {
            (Some(id), None) => {
                let contract = parse_contract_id(id)?;
                let bytes = RpcFetcher::new(rpc_url).fetch_wasm(&contract)?;
                Ok((
                    Source::Contract {
                        id: contract.to_string(),
                    },
                    bytes,
                ))
            }
            (None, Some(path)) => Source::read_wasm_file(path),
            // clap's arg group guarantees the two are never both set.
            _ => unreachable!("clap requires exactly one of CONTRACT_ID or --wasm"),
        }
    }

    /// Whether the user supplied anything to inspect.
    fn is_empty(&self) -> bool {
        self.contract_id.is_none() && self.wasm.is_none()
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            // Print the chain so the underlying cause is never swallowed.
            let mut source = std::error::Error::source(&error);
            while let Some(cause) = source {
                eprintln!("  caused by: {cause}");
                source = cause.source();
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    match cli.command {
        Some(Command::Meta { target, options }) => {
            require_target(&target);
            let (source, bytes) = target.load(&options.rpc_url)?;
            let (source, meta) = Inspection::meta_from_wasm(source, &bytes)?;
            options.renderer().meta(&meta, &source)
        }
        None => {
            require_target(&cli.target);
            let (source, bytes) = cli.target.load(&cli.options.rpc_url)?;
            let inspection = Inspection::from_wasm(source, &bytes)?;
            cli.options.renderer().interface(
                &inspection.interface,
                &inspection.meta,
                &inspection.source,
            )
        }
    }
}

/// Exit with clap's own usage error when nothing was given to inspect.
///
/// The arg group cannot express "exactly one of" for a subcommand-bearing CLI
/// without also rejecting a bare `lumenscan`, so the check lives here.
fn require_target(target: &Target) -> &Target {
    use clap::CommandFactory;

    if target.is_empty() {
        Cli::command()
            .error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "provide a contract ID or --wasm <PATH>",
            )
            .exit();
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_a_bare_contract_id() {
        let cli = Cli::try_parse_from(["lumenscan", "CDLZ"]).unwrap();
        assert_eq!(cli.target.contract_id.as_deref(), Some("CDLZ"));
        assert!(cli.command.is_none());
        assert!(!cli.options.json);
        assert_eq!(cli.options.rpc_url, DEFAULT_RPC_URL);
    }

    #[test]
    fn parses_a_local_wasm_path_with_json() {
        let cli = Cli::try_parse_from(["lumenscan", "--wasm", "a.wasm", "--json"]).unwrap();
        assert_eq!(
            cli.target.wasm.as_deref(),
            Some(std::path::Path::new("a.wasm"))
        );
        assert!(cli.options.json);
    }

    #[test]
    fn rejects_a_contract_id_together_with_wasm() {
        assert!(Cli::try_parse_from(["lumenscan", "CDLZ", "--wasm", "a.wasm"]).is_err());
    }

    #[test]
    fn parses_the_meta_subcommand_for_both_target_kinds() {
        let cli = Cli::try_parse_from(["lumenscan", "meta", "CDLZ"]).unwrap();
        let Some(Command::Meta { target, .. }) = cli.command else {
            panic!("expected the meta subcommand");
        };
        assert_eq!(target.contract_id.as_deref(), Some("CDLZ"));

        let cli = Cli::try_parse_from(["lumenscan", "meta", "--wasm", "a.wasm", "--json"]).unwrap();
        let Some(Command::Meta { target, options }) = cli.command else {
            panic!("expected the meta subcommand");
        };
        assert!(target.wasm.is_some());
        assert!(options.json);
    }

    #[test]
    fn accepts_a_custom_rpc_url() {
        let cli =
            Cli::try_parse_from(["lumenscan", "CDLZ", "--rpc-url", "https://example.org"]).unwrap();
        assert_eq!(cli.options.rpc_url, "https://example.org");
    }
}
