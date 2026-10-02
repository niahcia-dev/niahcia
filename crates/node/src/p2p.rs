use crate::address::AddressNetwork;
use crate::consensus::{randomx_seed, randomx_seed_height};
use crate::mining_rpc::{
    install_next_native_work_from_mempool, validate_block_candidate, WorkManager,
};
use crate::native_block_body::NativeBlockBodyV1;
use crate::native_execution::{execute_block_v1, NativeExecutionContextV1, NativeStateV1};
use crate::native_rpc::SharedNativeMempoolV1;
use crate::native_transaction::native_transactions_root_v1;
use crate::p2p_transaction_relay::{
    admit_relay_transactions, inventory_for_mempool, missing_from_inventory,
    transactions_for_request, GetTxV1, TxInvV1, TxV1,
};
use crate::p2p_v3_codec::BlockTransferV3;
use crate::p2p_v3_frame::{read_message_v3, write_message_v3, MessageV3};
use crate::state::StateStore;
use crate::work::{Address20, Hash32};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const MAX_BLOCKS_PER_MESSAGE: u16 = 128;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const STATIC_PEER_RETRY_DELAY: Duration = Duration::from_secs(2);

pub fn spawn_v3(
    bind: SocketAddr,
    peers: Vec<SocketAddr>,
    state: Arc<StateStore>,
    work: WorkManager,
    mempool: SharedNativeMempoolV1,
    fee_recipient: Address20,
    running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, String> {
    let listener = TcpListener::bind(bind).map_err(io_error)?;
    listener.set_nonblocking(true).map_err(io_error)?;

    Ok(thread::spawn(move || {
        for peer in peers {
            let state = Arc::clone(&state);
            let work = work.clone();
            let mempool = mempool.clone();
            let running = Arc::clone(&running);
            thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match TcpStream::connect_timeout(&peer, IO_TIMEOUT) {
                        Ok(stream) => {
                            match sync_peer_v3(stream, &state, &work, &mempool, fee_recipient) {
                                Ok(()) => tracing::info!(%peer, "outbound P2P V3 sync complete"),
                                Err(error) => {
                                    tracing::warn!(%peer, %error, "outbound P2P V3 sync failed")
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%peer, %error, "failed to connect static P2P V3 peer")
                        }
                    }
                    sleep_while_running(&running, STATIC_PEER_RETRY_DELAY);
                }
            });
        }

        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    let state = Arc::clone(&state);
                    let work = work.clone();
                    let mempool = mempool.clone();
                    thread::spawn(move || {
                        if let Err(error) =
                            serve_peer_v3(stream, &state, &work, &mempool, fee_recipient)
                        {
                            tracing::warn!(%peer, %error, "inbound P2P V3 session failed");
                        } else {
                            tracing::info!(%peer, "inbound P2P V3 session complete");
                        }
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    tracing::warn!(%error, "P2P V3 accept failed");
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }))
}

fn sleep_while_running(running: &AtomicBool, duration: Duration) {
    let step = Duration::from_millis(100);
    let mut slept = Duration::ZERO;
    while running.load(Ordering::SeqCst) && slept < duration {
        let remaining = duration.saturating_sub(slept);
        let nap = remaining.min(step);
        thread::sleep(nap);
        slept += nap;
    }
}

fn exchange_hello_v3(mut stream: TcpStream, state: &StateStore) -> Result<HelloV1, String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;

    write_message_v3(&mut stream, &MessageV3::Hello(local_hello(state)?))?;
    match read_message_v3(&mut stream)? {
        MessageV3::Hello(remote) => Ok(remote),
        _ => Err("P2P V3 peer did not send Hello as its first message".into()),
    }
}

fn local_inventory_v3(mempool: &SharedNativeMempoolV1) -> Result<TxInvV1, String> {
    let pool = mempool
        .read()
        .map_err(|_| "native mempool lock poisoned".to_string())?;
    Ok(inventory_for_mempool(&pool))
}

fn requested_transactions_v3(
    mempool: &SharedNativeMempoolV1,
    request: &GetTxV1,
) -> Result<TxV1, String> {
    let pool = mempool
        .read()
        .map_err(|_| "native mempool lock poisoned".to_string())?;
    transactions_for_request(&pool, request)
}

fn admit_transactions_v3(
    mempool: &SharedNativeMempoolV1,
    transactions: &TxV1,
) -> Result<bool, String> {
    let mut pool = mempool
        .write()
        .map_err(|_| "native mempool lock poisoned".to_string())?;

    let mut admitted = false;
    for result in admit_relay_transactions(&mut pool, transactions) {
        match result {
            Ok(_) => admitted = true,
            Err(error) if error.contains("duplicate native transaction") => {}
            Err(error) => return Err(error),
        }
    }
    Ok(admitted)
}

fn sync_mempool_v3(stream: &mut TcpStream, mempool: &SharedNativeMempoolV1) -> Result<(), String> {
    let local_inventory = local_inventory_v3(mempool)?;
    write_message_v3(&mut *stream, &MessageV3::TxInv(local_inventory))?;

    let remote_inventory = match read_message_v3(&mut *stream)? {
        MessageV3::TxInv(inventory) => inventory,
        _ => return Err("P2P V3 peer did not exchange transaction inventory".into()),
    };

    let request = {
        let pool = mempool
            .read()
            .map_err(|_| "native mempool lock poisoned".to_string())?;
        missing_from_inventory(&pool, &remote_inventory)
    };
    write_message_v3(&mut *stream, &MessageV3::GetTx(request))?;

    let remote_request = match read_message_v3(&mut *stream)? {
        MessageV3::GetTx(request) => request,
        _ => return Err("P2P V3 peer did not answer inventory with GetTx".into()),
    };

    let response = requested_transactions_v3(mempool, &remote_request)?;
    write_message_v3(&mut *stream, &MessageV3::Tx(response))?;

    let remote_transactions = match read_message_v3(&mut *stream)? {
        MessageV3::Tx(transactions) => transactions,
        _ => return Err("P2P V3 peer did not answer GetTx with Tx".into()),
    };
    admit_transactions_v3(mempool, &remote_transactions).map(|_| ())
}

fn serve_peer_v3(
    mut stream: TcpStream,
    state: &StateStore,
    work: &WorkManager,
    mempool: &SharedNativeMempoolV1,
    fee_recipient: Address20,
) -> Result<(), String> {
    exchange_hello_v3(stream.try_clone().map_err(io_error)?, state)?;
    sync_mempool_v3(&mut stream, mempool)?;

    loop {
        match read_message_v3(&mut stream) {
            Ok(MessageV3::GetBlocks(request)) => {
                let blocks =
                    canonical_transfer_range_v3(state, request.start_height, request.count)?;
                write_message_v3(&mut stream, &MessageV3::Blocks(blocks))?;
            }
            Ok(MessageV3::Blocks(blocks)) => {
                ingest_blocks_v3(state, work, mempool, fee_recipient, blocks)?
            }
            Ok(MessageV3::TxInv(inventory)) => {
                let request = {
                    let pool = mempool
                        .read()
                        .map_err(|_| "native mempool lock poisoned".to_string())?;
                    missing_from_inventory(&pool, &inventory)
                };
                write_message_v3(&mut stream, &MessageV3::GetTx(request))?;
            }
            Ok(MessageV3::GetTx(request)) => {
                let response = requested_transactions_v3(mempool, &request)?;
                write_message_v3(&mut stream, &MessageV3::Tx(response))?;
            }
            Ok(MessageV3::Tx(transactions)) => {
                if admit_transactions_v3(mempool, &transactions)?
                    && state.best_chain_head()?.is_some()
                {
                    install_next_native_work_from_mempool(work, state, mempool, fee_recipient)?;
                }
            }
            Ok(MessageV3::Hello(_)) => return Err("P2P V3 peer sent duplicate Hello".into()),
            Err(error) if is_disconnect_error(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn sync_peer_v3(
    mut stream: TcpStream,
    state: &StateStore,
    work: &WorkManager,
    mempool: &SharedNativeMempoolV1,
    fee_recipient: Address20,
) -> Result<(), String> {
    let remote = exchange_hello_v3(stream.try_clone().map_err(io_error)?, state)?;
    sync_mempool_v3(&mut stream, mempool)?;

    let Some(remote_height) = remote.best_height else {
        return Ok(());
    };
    let mut start_height = find_sync_start_v3(&mut stream, state, remote_height)?;

    while start_height <= remote_height {
        write_message_v3(
            &mut stream,
            &MessageV3::GetBlocks(GetBlocksV1 {
                start_height,
                count: MAX_BLOCKS_PER_MESSAGE,
            }),
        )?;
        let blocks = match read_message_v3(&mut stream)? {
            MessageV3::Blocks(blocks) => blocks,
            _ => return Err("P2P V3 peer did not answer GetBlocks with Blocks".into()),
        };
        if blocks.is_empty() {
            return Err("P2P V3 peer advertised blocks but returned an empty range".into());
        }
        let received = u64::try_from(blocks.len())
            .map_err(|_| "received V3 block count does not fit u64".to_string())?;
        ingest_blocks_v3(state, work, mempool, fee_recipient, blocks)?;
        start_height = start_height
            .checked_add(received)
            .ok_or_else(|| "P2P V3 sync height overflow".to_string())?;
    }

    Ok(())
}

fn find_sync_start_v3(
    stream: &mut TcpStream,
    state: &StateStore,
    remote_height: u64,
) -> Result<u64, String> {
    let Some(local_head) = state.best_chain_head()? else {
        return Ok(0);
    };

    let mut height = local_head.header.height.min(remote_height);
    loop {
        write_message_v3(
            &mut *stream,
            &MessageV3::GetBlocks(GetBlocksV1 {
                start_height: height,
                count: 1,
            }),
        )?;
        let blocks = match read_message_v3(&mut *stream)? {
            MessageV3::Blocks(blocks) => blocks,
            _ => return Err("P2P V3 peer did not answer common-ancestor probe with Blocks".into()),
        };
        let Some(remote_block) = blocks.first() else {
            return Err("P2P V3 peer returned no block for common-ancestor probe".into());
        };
        let local_block = state
            .canonical_block_at_height(height)?
            .ok_or_else(|| format!("local canonical chain is missing height {height}"))?;

        if local_block.block_id() == remote_block.header.block_id() {
            return height
                .checked_add(1)
                .ok_or_else(|| "P2P V3 sync height overflow".to_string());
        }
        if height == 0 {
            return Err("P2P V3 peer does not share the local canonical genesis".into());
        }
        height -= 1;
    }
}

fn local_hello(state: &StateStore) -> Result<HelloV1, String> {
    match state.best_chain_head()? {
        Some(head) => Ok(HelloV1 {
            best_height: Some(head.header.height),
            best_block_id: Some(head.block_id()),
            cumulative_work: head.chain_work.to_bytes_be(),
        }),
        None => Ok(HelloV1 {
            best_height: None,
            best_block_id: None,
            cumulative_work: Vec::new(),
        }),
    }
}

fn ingest_blocks_v3(
    state: &StateStore,
    work: &WorkManager,
    mempool: &SharedNativeMempoolV1,
    fee_recipient: Address20,
    blocks: Vec<BlockTransferV3>,
) -> Result<(), String> {
    for transfer in blocks {
        transfer.validate_transaction_commitment()?;

        let seed_height = randomx_seed_height(transfer.header.height);
        let seed_block_id = if transfer.header.height == 0 {
            [0_u8; 32]
        } else {
            ancestor_block_id_at_height(state, transfer.header.parent_hash, seed_height)?
        };
        let seed = randomx_seed(seed_block_id);
        validate_block_candidate(&transfer.header, seed, state)?;

        let mut native_state = if transfer.header.height == 0 {
            if transfer.header.parent_hash != [0_u8; 32] {
                return Err("genesis block must have a zero parent hash".into());
            }
            NativeStateV1::default()
        } else {
            state
                .native_state_snapshot(transfer.header.parent_hash)?
                .ok_or_else(|| "candidate parent is missing native state snapshot".to_string())?
        };

        let transactions = transfer.body.decoded_transactions()?;
        let execution = execute_block_v1(
            &mut native_state,
            &transactions,
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 0,
                cpu_producer: transfer.body.producer_fee_recipient,
            },
        )?;
        transfer
            .body
            .validate_fee_recipient_canonicality(execution.producer_priority_fee)?;

        if execution.transactions_root != transfer.header.transactions_root {
            return Err("P2P V3 execution transactions root does not match header".into());
        }
        if execution.execution_root != transfer.header.execution_root {
            return Err("P2P V3 execution root does not match header".into());
        }

        let outcome = state.insert_native_block_with_body_and_execution_outcome(
            transfer.header,
            &transfer.body,
            &execution,
            &native_state,
        )?;

        if outcome.current_best == outcome.block.block_id() {
            let canonical_ids = match outcome.reorg.as_ref() {
                Some(reorg) => reorg.attached.clone(),
                None => vec![outcome.block.block_id()],
            };
            remove_canonical_transactions_v3(state, mempool, &canonical_ids)?;
            install_next_native_work_from_mempool(work, state, mempool, fee_recipient)?;
        }
    }
    Ok(())
}

fn canonical_transfer_range_v3(
    state: &StateStore,
    start_height: u64,
    count: u16,
) -> Result<Vec<BlockTransferV3>, String> {
    if count == 0 || count > MAX_BLOCKS_PER_MESSAGE {
        return Err("GetBlocks count is outside protocol bounds".into());
    }

    let chain = state.canonical_chain()?;
    let start = usize::try_from(start_height)
        .map_err(|_| "GetBlocks start height does not fit this platform".to_string())?;
    if start >= chain.len() {
        return Ok(Vec::new());
    }

    let empty_root = native_transactions_root_v1(&[])?;
    let end = start.saturating_add(count as usize).min(chain.len());
    chain[start..end]
        .iter()
        .map(|block| {
            let body = match state.native_block_body(block.block_id())? {
                Some(body) => body,
                None if block.header.transactions_root == empty_root => NativeBlockBodyV1::empty(),
                None => {
                    return Err(format!(
                        "non-empty canonical block {} is missing NativeBlockBodyV1",
                        hex::encode(block.block_id())
                    ))
                }
            };

            let transfer = BlockTransferV3 {
                header: block.header.clone(),
                body,
            };
            transfer.validate_transaction_commitment()?;
            Ok(transfer)
        })
        .collect()
}

fn remove_canonical_transactions_v3(
    state: &StateStore,
    mempool: &SharedNativeMempoolV1,
    block_ids: &[Hash32],
) -> Result<(), String> {
    let mut tx_ids = Vec::new();

    for block_id in block_ids {
        let Some(body) = state.native_block_body(*block_id)? else {
            continue;
        };
        for transaction in body.decoded_transactions()? {
            tx_ids.push(transaction.tx_id()?);
        }
    }

    let mut pool = mempool
        .write()
        .map_err(|_| "native mempool lock poisoned".to_string())?;
    for tx_id in tx_ids {
        pool.remove(&tx_id);
    }
    Ok(())
}

fn ancestor_block_id_at_height(
    state: &StateStore,
    mut block_id: Hash32,
    target_height: u64,
) -> Result<Hash32, String> {
    loop {
        let block = state
            .load_chain_block(block_id)?
            .ok_or_else(|| "candidate ancestry references missing block".to_string())?;
        if block.header.height == target_height {
            return Ok(block.block_id());
        }
        if block.header.height < target_height || block.header.height == 0 {
            return Err(format!(
                "candidate ancestry does not contain RandomX seed height {target_height}"
            ));
        }
        block_id = block.header.parent_hash;
    }
}

fn io_error(error: std::io::Error) -> String {
    format!("P2P I/O error: {error}")
}

fn is_disconnect_error(error: &str) -> bool {
    error.contains("UnexpectedEof")
        || error.contains("Connection reset")
        || error.contains("connection reset")
        || error.contains("Broken pipe")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloV1 {
    pub best_height: Option<u64>,
    pub best_block_id: Option<Hash32>,
    pub cumulative_work: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBlocksV1 {
    pub start_height: u64,
    pub count: u16,
}
