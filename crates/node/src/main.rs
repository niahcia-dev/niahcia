pub mod address;
mod config;
pub mod consensus;
mod engine;
mod mining_rpc;
pub mod monetary;
pub mod native_rpc;
mod p2p;
pub mod pow;
pub mod service;
pub mod state;
pub mod work;

use config::NodeConfig;
use consensus::{devnet_next_target, randomx_seed, randomx_seed_height, DEVNET_GENESIS_TARGET};
use engine::EngineClient;
use mining_rpc::WorkManager;
use state::StateStore;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use work::BlockHeaderV1;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "NIAHCIA {VERSION}\n\nUsage:\n  niahcia [OPTIONS]\n\nOptions:\n  -c, --config <PATH>  Load TOML config file\n  -h, --help           Print help\n  -V, --version        Print version\n\nEnvironment overrides:\n  NIAHCIA_NETWORK\n  NIAHCIA_DATA_DIR\n  NIAHCIA_RETH_ENGINE_API\n  NIAHCIA_RETH_HTTP_RPC\n  NIAHCIA_FEE_RECIPIENT\n  NIAHCIA_RETH_JWT_PATH\n  NIAHCIA_MINING_RPC_BIND\n  NIAHCIA_P2P_BIND\n  NIAHCIA_P2P_PEERS\n  NIAHCIA_LOG_LEVEL\n\nStatus:\n  Pre-alpha reference node.\n"
    );
}

fn main() -> ExitCode {
    let config_path = match parse_config_path(env::args().skip(1)) {
        Ok(ParseResult::Help) => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Ok(ParseResult::Version) => {
            println!("niahcia {VERSION}");
            return ExitCode::SUCCESS;
        }
        Ok(ParseResult::Run(path)) => path,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("Try 'niahcia --help' for more information.");
            return ExitCode::from(2);
        }
    };

    let config = match NodeConfig::load(config_path.as_deref()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("failed to load configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(error) = init_tracing(&config.log_level) {
        eprintln!("failed to initialize logging: {error}");
        return ExitCode::FAILURE;
    }

    info!(version = VERSION, network = %config.network, "starting NIAHCIA");
    info!(data_dir = %config.data_dir.display(), "data directory");
    info!(reth_engine_api = %config.reth_engine_api, "Reth Engine API");
    info!(reth_http_rpc = %config.reth_http_rpc, "Reth public RPC");
    info!(reth_jwt_path = %config.reth_jwt_path.display(), "Reth JWT path");
    info!(mining_rpc_bind = %config.mining_rpc_bind, "mining RPC bind");
    info!(p2p_bind = %config.p2p_bind, peers = config.p2p_peers.len(), "P2P configuration");

    if let Err(error) = std::fs::create_dir_all(&config.data_dir) {
        error!(%error, "failed to create data directory");
        return ExitCode::FAILURE;
    }

    let jwt_secret = match std::fs::read_to_string(&config.reth_jwt_path) {
        Ok(secret) => secret.trim().to_owned(),
        Err(error) => {
            error!(%error, path = %config.reth_jwt_path.display(), "failed to read Reth JWT secret");
            return ExitCode::FAILURE;
        }
    };

    let engine = EngineClient::new(
        config.reth_engine_api.clone(),
        config.reth_http_rpc.clone(),
        jwt_secret,
        config.fee_recipient,
    );

    let state = match StateStore::open(&config.data_dir) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            error!(%error, "failed to open NIAHCIA state");
            return ExitCode::FAILURE;
        }
    };

    let persisted_head = match state.load_head() {
        Ok(head) => head,
        Err(error) => {
            error!(%error, "failed to load persisted NIAHCIA chain head");
            return ExitCode::FAILURE;
        }
    };

    let (height, parent_hash, target, timestamp, cumulative_work) = match persisted_head {
        Some(head) => {
            info!(height = head.next_height(), hash = %hex::encode(head.hash), "loaded persisted NIAHCIA chain head");
            (
                head.next_height(),
                head.hash,
                devnet_next_target(&state, &head).unwrap_or(head.target),
                head.timestamp.saturating_add(1),
                head.cumulative_work,
            )
        }
        None => {
            info!("no persisted NIAHCIA chain head; starting from genesis template");
            (0, [0u8; 32], DEVNET_GENESIS_TARGET, unix_timestamp(), 0)
        }
    };

    let seed_height = randomx_seed_height(height);
    let seed = randomx_seed(&state, seed_height).unwrap_or_else(|_| [0u8; 32]);

    let execution = match engine.build_payload(parent_hash, timestamp) {
        Ok(execution) => execution,
        Err(error) => {
            error!(%error, "failed to build initial Reth execution payload");
            return ExitCode::FAILURE;
        }
    };

    let header = BlockHeaderV1::new(
        height,
        parent_hash,
        target,
        timestamp,
        execution.block_hash,
        execution.transactions_root,
        execution.state_root,
        seed_height,
        seed,
    );

    let work_manager = Arc::new(WorkManager::new(
        header,
        execution,
        state.clone(),
        engine,
        cumulative_work,
    ));

    let stop = Arc::new(AtomicBool::new(false));
    if let Err(error) = mining_rpc::spawn(config.mining_rpc_bind, work_manager.clone(), stop.clone()) {
        error!(%error, "failed to start mining RPC");
        return ExitCode::FAILURE;
    }

    if let Err(error) = p2p::spawn(
        config.p2p_bind,
        config.p2p_peers.clone(),
        work_manager,
        stop.clone(),
    ) {
        error!(%error, "failed to start P2P service");
        return ExitCode::FAILURE;
    }

    info!("node bootstrap running; press Ctrl-C to stop");
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(250));
    }
    info!("shutdown complete");
    ExitCode::SUCCESS
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn init_tracing(level: &str) -> Result<(), tracing_subscriber::util::TryInitError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(level))
        .try_init()
}

enum ParseResult {
    Help,
    Version,
    Run(Option<PathBuf>),
}

fn parse_config_path<I>(mut args: I) -> Result<ParseResult, String>
where
    I: Iterator<Item = String>,
{
    let mut config_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(ParseResult::Help),
            "-V" | "--version" => return Ok(ParseResult::Version),
            "-c" | "--config" => {
                let path = args
                    .next()
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                config_path = Some(PathBuf::from(path));
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }
    Ok(ParseResult::Run(config_path))
}
