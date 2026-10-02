pub mod address;
mod config;
pub mod consensus;
mod engine;
mod mining_rpc;
pub mod monetary;
pub mod monetary_state;
pub mod native_execution;
pub mod native_rpc;
pub mod native_transaction;
pub mod nce;
mod p2p;
pub mod pow;
pub mod service;
pub mod state;
pub mod work;

use config::NodeConfig;
use consensus::{randomx_seed, randomx_seed_height, DEVNET_GENESIS_TARGET};
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
use work::{Address20, BlockHeaderV1};

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

    let fee_recipient: Address20 = match hex::decode(
        config
            .fee_recipient
            .strip_prefix("0x")
            .unwrap_or(&config.fee_recipient),
    ) {
        Ok(bytes) => match bytes.try_into() {
            Ok(address) => address,
            Err(_) => {
                error!("normalized fee recipient is not 20 bytes");
                return ExitCode::FAILURE;
            }
        },
        Err(error) => {
            error!(%error, "failed to decode normalized fee recipient");
            return ExitCode::FAILURE;
        }
    };

    let engine = match EngineClient::new(
        config.reth_engine_api.clone(),
        config.reth_http_rpc.clone(),
        &config.reth_jwt_path,
    ) {
        Ok(engine) => Arc::new(engine),
        Err(error) => {
            error!(%error, "failed to initialize Reth Engine API client");
            return ExitCode::FAILURE;
        }
    };

    match engine.exchange_capabilities() {
        Ok(capabilities) => {
            info!(
                capabilities = ?capabilities,
                "Reth Engine API capability exchange complete"
            );
        }
        Err(error) => {
            error!(%error, "Reth Engine API capability exchange failed");
            return ExitCode::FAILURE;
        }
    }

    let state_path = config.data_dir.join("state.redb");
    let state = match StateStore::open(&state_path) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            error!(%error, "failed to open NIAHCIA state");
            return ExitCode::FAILURE;
        }
    };

    let persisted_head = match state.best_chain_head() {
        Ok(head) => head,
        Err(error) => {
            error!(%error, "failed to load persisted NIAHCIA chain head");
            return ExitCode::FAILURE;
        }
    };

    let work_manager = match persisted_head {
        Some(head) => {
            info!(
                height = head.header.height,
                hash = %hex::encode(head.block_id()),
                "loaded persisted NIAHCIA chain head"
            );

            let execution_hash = match state.execution_hash(head.block_id()) {
                Ok(Some(hash)) => hash,
                Ok(None) => {
                    error!("persisted NIAHCIA head is missing its execution mapping");
                    return ExitCode::FAILURE;
                }
                Err(error) => {
                    error!(%error, "failed to load persisted execution mapping");
                    return ExitCode::FAILURE;
                }
            };

            let placeholder = BlockHeaderV1 {
                version: 1,
                parent_hash: head.block_id(),
                height: head.header.height.saturating_add(1),
                timestamp: head.header.timestamp.saturating_add(1),
                transactions_root: [0u8; 32],
                execution_root: [0u8; 32],
                target: head.header.target,
                nonce: 0,
                extra_nonce: 0,
            };
            let manager = WorkManager::new(placeholder, 0, [0u8; 32], execution_hash);

            if let Err(error) = mining_rpc::install_next_work(
                &manager,
                &state,
                &engine,
                fee_recipient,
                execution_hash,
            ) {
                error!(%error, "failed to restore Reth-backed mining work");
                return ExitCode::FAILURE;
            }

            manager
        }
        None => {
            info!("no persisted NIAHCIA chain head; starting from genesis template");

            let parent = match engine.latest_block() {
                Ok(parent) => parent,
                Err(error) => {
                    error!(%error, "failed to load latest Reth block for genesis");
                    return ExitCode::FAILURE;
                }
            };

            let timestamp = unix_timestamp().max(parent.timestamp.saturating_add(1));
            let built = match engine.build_payload_v3(&parent, timestamp, fee_recipient) {
                Ok(built) => built,
                Err(error) => {
                    error!(%error, "failed to build initial Reth execution payload");
                    return ExitCode::FAILURE;
                }
            };

            if let Err(error) = engine.validate_payload_v3(&built) {
                error!(%error, "failed to validate initial Reth execution payload");
                return ExitCode::FAILURE;
            }

            let execution = &built.commitments;
            let header = BlockHeaderV1 {
                version: 1,
                parent_hash: [0u8; 32],
                height: 0,
                timestamp: execution.timestamp,
                transactions_root: execution.transactions_root,
                execution_root: execution.commitment_hash(),
                target: DEVNET_GENESIS_TARGET,
                nonce: 0,
                extra_nonce: 0,
            };

            let seed_height = randomx_seed_height(0);
            let seed = randomx_seed([0u8; 32]);
            let execution_hash = built.execution_payload_hash;

            let replay_bytes = match engine.encode_replay_payload(&built) {
                Ok(bytes) => bytes,
                Err(error) => {
                    error!(%error, "failed to encode initial execution replay payload");
                    return ExitCode::FAILURE;
                }
            };

            if let Err(error) = state.store_execution_payload(execution_hash, &replay_bytes) {
                error!(%error, "failed to persist initial execution replay payload");
                return ExitCode::FAILURE;
            }

            WorkManager::new(header, seed_height, seed, execution_hash)
        }
    };

    let running = Arc::new(AtomicBool::new(true));

    {
        let running = running.clone();
        if let Err(error) = ctrlc::set_handler(move || {
            running.store(false, Ordering::Relaxed);
        }) {
            error!(%error, "failed to install shutdown signal handler");
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = mining_rpc::spawn(
        config.mining_rpc_bind,
        work_manager.clone(),
        state.clone(),
        engine.clone(),
        fee_recipient,
        running.clone(),
    ) {
        error!(%error, "failed to start mining RPC");
        return ExitCode::FAILURE;
    }

    if let Err(error) = p2p::spawn(
        config.p2p_bind,
        config.p2p_peers.clone(),
        state.clone(),
        engine.clone(),
        work_manager,
        fee_recipient,
        running.clone(),
    ) {
        error!(%error, "failed to start P2P service");
        return ExitCode::FAILURE;
    }

    info!("node bootstrap running; press Ctrl-C to stop");
    while running.load(Ordering::Relaxed) {
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

fn init_tracing(level: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
