pub mod address;
pub mod compute_usage_receipt_v1;
mod config;
pub mod consensus;
mod mining_rpc;
pub mod monetary;
pub mod monetary_state;
pub mod native_activation_v2;
pub mod native_block_body;
pub mod native_block_body_v2;
pub mod native_block_execution_v2;
pub mod native_compute_execution;
pub mod native_compute_fee_v1;
pub mod native_compute_payloads;
pub mod native_contract_execution_v1;
pub mod native_contract_payload_v1;
pub mod native_contract_runtime_registry_v1;
pub mod native_contract_state_v1;
pub mod native_contract_vm_v1;
pub mod native_execution;
pub mod native_execution_commitment_v2;
pub mod native_execution_v2;
pub mod native_mempool;
pub mod native_rpc;
pub mod native_state_v2;
pub mod native_state_v3;
pub mod native_transaction;
pub mod native_transaction_v2;
pub mod nce;
mod p2p;
pub mod p2p_transaction_relay;
pub mod p2p_v3_codec;
pub mod p2p_v3_frame;
pub mod pow;
pub mod service;
pub mod state;
pub mod work;

use address::AddressNetwork;
use config::NodeConfig;
use consensus::{randomx_seed, randomx_seed_height, DEVNET_GENESIS_TARGET};
use mining_rpc::WorkManager;
use native_execution::{execute_block_v1, NativeExecutionContextV1, NativeStateV1};
use native_mempool::NativeMempoolV1;
use state::StateStore;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use work::{Address20, BlockHeaderV1};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "NIAHCIA {VERSION}\n\nUsage:\n  niahcia [OPTIONS]\n\nOptions:\n  -c, --config <PATH>  Load TOML config file\n  -h, --help           Print help\n  -V, --version        Print version\n\nEnvironment overrides:\n  NIAHCIA_NETWORK\n  NIAHCIA_DATA_DIR\n  NIAHCIA_FEE_RECIPIENT\n  NIAHCIA_MINING_RPC_BIND\n  NIAHCIA_P2P_BIND\n  NIAHCIA_P2P_PEERS\n  NIAHCIA_LOG_LEVEL\n\nStatus:\n  Pre-alpha reference node.\n"
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

            let mut native_state = match state.native_state_snapshot(head.block_id()) {
                Ok(Some(state)) => state,
                Ok(None) => {
                    error!("persisted NIAHCIA head is missing its native state snapshot");
                    return ExitCode::FAILURE;
                }
                Err(error) => {
                    error!(%error, "failed to load persisted native state snapshot");
                    return ExitCode::FAILURE;
                }
            };

            let height = match head.header.height.checked_add(1) {
                Some(height) => height,
                None => {
                    error!("NIAHCIA height overflow");
                    return ExitCode::FAILURE;
                }
            };

            let now = unix_timestamp();
            let timestamp = now.max(head.header.timestamp.saturating_add(1));

            let genesis = match state.canonical_block_at_height(0) {
                Ok(Some(genesis)) => genesis,
                Ok(None) => {
                    error!("canonical chain is missing devnet genesis");
                    return ExitCode::FAILURE;
                }
                Err(error) => {
                    error!(%error, "failed to load canonical genesis");
                    return ExitCode::FAILURE;
                }
            };

            let target = match consensus::devnet_next_target(
                genesis.header.timestamp,
                head.header.height,
                head.header.timestamp,
            ) {
                Ok(target) => target,
                Err(error) => {
                    error!(%error, "failed to calculate next mining target");
                    return ExitCode::FAILURE;
                }
            };

            let execution = match execute_block_v1(
                &mut native_state,
                &[],
                AddressNetwork::Devnet,
                NativeExecutionContextV1 {
                    base_fee_per_gas: 0,
                    cpu_producer: fee_recipient,
                },
            ) {
                Ok(execution) => execution,
                Err(error) => {
                    error!(%error, "failed to build native mining execution");
                    return ExitCode::FAILURE;
                }
            };

            let header = BlockHeaderV1 {
                version: 1,
                parent_hash: head.block_id(),
                height,
                timestamp,
                transactions_root: execution.transactions_root,
                execution_root: execution.execution_root,
                target,
                nonce: 0,
                extra_nonce: 0,
            };

            let seed_height = randomx_seed_height(height);
            let seed_block = match state.canonical_block_at_height(seed_height) {
                Ok(Some(block)) => block,
                Ok(None) => {
                    error!(seed_height, "canonical chain missing RandomX seed block");
                    return ExitCode::FAILURE;
                }
                Err(error) => {
                    error!(%error, "failed to load RandomX seed block");
                    return ExitCode::FAILURE;
                }
            };
            let seed = randomx_seed(seed_block.block_id());

            WorkManager::new(header, seed_height, seed, execution, native_state)
        }
        None => {
            info!("no persisted NIAHCIA chain head; starting from native genesis template");

            let mut native_state = NativeStateV1::default();
            let execution = match execute_block_v1(
                &mut native_state,
                &[],
                AddressNetwork::Devnet,
                NativeExecutionContextV1 {
                    base_fee_per_gas: 0,
                    cpu_producer: fee_recipient,
                },
            ) {
                Ok(execution) => execution,
                Err(error) => {
                    error!(%error, "failed to build native genesis execution");
                    return ExitCode::FAILURE;
                }
            };

            let header = BlockHeaderV1 {
                version: 1,
                parent_hash: [0u8; 32],
                height: 0,
                timestamp: unix_timestamp(),
                transactions_root: execution.transactions_root,
                execution_root: execution.execution_root,
                target: DEVNET_GENESIS_TARGET,
                nonce: 0,
                extra_nonce: 0,
            };

            let seed_height = randomx_seed_height(0);
            let seed = randomx_seed([0u8; 32]);

            WorkManager::new(header, seed_height, seed, execution, native_state)
        }
    };

    let native_mempool = Arc::new(RwLock::new(NativeMempoolV1::new(AddressNetwork::Devnet)));

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
        native_mempool.clone(),
        fee_recipient,
        running.clone(),
    ) {
        error!(%error, "failed to start mining RPC");
        return ExitCode::FAILURE;
    }

    if let Err(error) = p2p::spawn_v3(
        config.p2p_bind,
        config.p2p_peers.clone(),
        state.clone(),
        work_manager,
        native_mempool,
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
