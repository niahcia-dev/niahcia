mod config;
pub mod consensus;
mod engine;
mod mining_rpc;
pub mod service;
pub mod state;
pub mod work;

use config::NodeConfig;
use consensus::{randomx_seed, randomx_seed_height};
use engine::EngineClient;
use mining_rpc::WorkManager;
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
        "NIAHCIA {VERSION}

Usage:
  niahcia [OPTIONS]

Options:
  -c, --config <PATH>  Load TOML config file
  -h, --help           Print help
  -V, --version        Print version

Environment overrides:
  NIAHCIA_NETWORK
  NIAHCIA_DATA_DIR
  NIAHCIA_RETH_ENGINE_API
  NIAHCIA_RETH_HTTP_RPC
  NIAHCIA_FEE_RECIPIENT
  NIAHCIA_RETH_JWT_PATH
  NIAHCIA_MINING_RPC_BIND
  NIAHCIA_LOG_LEVEL

Status:
  Pre-alpha reference node.
"
    );
}

fn parse_config_path() -> Result<Option<PathBuf>, String> {
    let mut args = env::args().skip(1);
    let mut config_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" | "-V" | "--version" => {}
            "-c" | "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--config requires a path".to_string())?;
                config_path = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(config_path)
}

fn parse_fee_recipient(value: &str) -> Result<[u8; 20], String> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(raw).map_err(|e| format!("invalid fee recipient hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("fee recipient must be 20 bytes; found {}", v.len()))
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("niahcia {VERSION}");
        return ExitCode::SUCCESS;
    }

    let config_path = match parse_config_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            eprintln!("try 'niahcia --help'");
            return ExitCode::from(2);
        }
    };

    let config = match NodeConfig::load(config_path.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("configuration error: {e}");
            return ExitCode::from(2);
        }
    };

    init_logging(&config.log_level);

    info!(version = VERSION, network = %config.network, "starting NIAHCIA");
    info!(data_dir = %config.data_dir.display(), "data directory");
    info!(reth_engine_api = %config.reth_engine_api, "Reth Engine API");
    info!(reth_http_rpc = %config.reth_http_rpc, "Reth public RPC");
    info!(reth_jwt_path = %config.reth_jwt_path.display(), "Reth JWT path");
    info!(mining_rpc_bind = %config.mining_rpc_bind, "mining RPC bind");

    if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
        error!(error = %e, path = %config.data_dir.display(), "failed to create data directory");
        return ExitCode::from(1);
    }

    let engine = match EngineClient::new(
        config.reth_engine_api.clone(),
        config.reth_http_rpc.clone(),
        &config.reth_jwt_path,
    ) {
        Ok(client) => client,
        Err(e) => {
            error!(error = %e, "failed to initialize Reth Engine API client");
            return ExitCode::from(1);
        }
    };

    match engine.exchange_capabilities() {
        Ok(capabilities) => {
            info!(
                count = capabilities.len(),
                capabilities = ?capabilities,
                "connected to Reth Engine API"
            );
        }
        Err(e) => {
            error!(error = %e, "Reth Engine API check failed");
            return ExitCode::from(1);
        }
    }

    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(e) => {
            error!(error = %e, "system clock error");
            return ExitCode::from(1);
        }
    };

    let parent = match engine.latest_block() {
        Ok(block) => block,
        Err(e) => {
            error!(error = %e, "failed to read latest Reth block");
            return ExitCode::from(1);
        }
    };

    let fee_recipient = match parse_fee_recipient(&config.fee_recipient) {
        Ok(address) => address,
        Err(e) => {
            error!(error = %e, "invalid fee recipient");
            return ExitCode::from(1);
        }
    };

    let candidate_timestamp = now.max(parent.timestamp.saturating_add(1));
    let built = match engine.build_payload_v3(&parent, candidate_timestamp, fee_recipient) {
        Ok(payload) => payload,
        Err(e) => {
            error!(error = %e, "failed to build Reth execution candidate");
            return ExitCode::from(1);
        }
    };

    let execution = &built.commitments;
    let niahcia_parent_hash = [0_u8; 32];

    let header = BlockHeaderV1 {
        version: 1,
        parent_hash: niahcia_parent_hash,
        height: execution.block_number,
        timestamp: execution.timestamp,
        transactions_root: execution.transactions_root,
        execution_root: execution.commitment_hash(),
        target: [0xff; 32],
        nonce: 0,
        extra_nonce: 0,
    };

    let seed_height = randomx_seed_height(header.height);
    let seed_block_id = niahcia_parent_hash;
    let seed = randomx_seed(seed_block_id);
    let work_manager = WorkManager::new(header, seed_height, seed);

    info!(
        height = execution.block_number,
        niahcia_parent = %hex::encode(niahcia_parent_hash),
        execution_parent = %hex::encode(execution.execution_parent_hash),
        execution_payload_hash = %hex::encode(built.execution_payload_hash),
        transactions_root = %hex::encode(execution.transactions_root),
        execution_root = %hex::encode(execution.commitment_hash()),
        randomx_seed_height = seed_height,
        randomx_seed = %hex::encode(seed),
        "installed Reth-backed NIAHCIA mining template"
    );

    let running = Arc::new(AtomicBool::new(true));
    let signal_running = Arc::clone(&running);

    if let Err(e) = ctrlc::set_handler(move || {
        signal_running.store(false, Ordering::SeqCst);
    }) {
        error!(error = %e, "failed to install shutdown handler");
        return ExitCode::from(1);
    }

    let rpc_handle =
        match mining_rpc::spawn(config.mining_rpc_bind, work_manager, Arc::clone(&running)) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "failed to start mining RPC");
                return ExitCode::from(1);
            }
        };

    info!("node bootstrap running; press Ctrl-C to stop");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(200));
    }

    info!("shutdown requested");

    if rpc_handle.join().is_err() {
        error!("mining RPC thread terminated unexpectedly");
        return ExitCode::from(1);
    }

    info!("NIAHCIA stopped cleanly");
    ExitCode::SUCCESS
}
