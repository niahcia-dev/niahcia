use crate::address::AddressNetwork;
use crate::consensus::{randomx_seed, randomx_seed_height};
use crate::mining_rpc::{install_next_native_work, validate_block_candidate, WorkManager};
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
use crate::work::{Address20, BlockHeaderV1, Hash32, BLOCK_HEADER_V1_LEN};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const DEVNET_MAGIC: [u8; 4] = *b"NIAH";
pub const PROTOCOL_VERSION: u16 = 2;
pub const FRAME_HEADER_LEN: usize = 12;
pub const MAX_FRAME_PAYLOAD: usize = 4 * 1024 * 1024;
pub const MAX_BLOCKS_PER_MESSAGE: u16 = 128;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const STATIC_PEER_RETRY_DELAY: Duration = Duration::from_secs(2);

pub fn spawn(
    bind: SocketAddr,
    peers: Vec<SocketAddr>,
    state: Arc<StateStore>,
    work: WorkManager,
    fee_recipient: Address20,
    running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, String> {
    let listener = TcpListener::bind(bind).map_err(io_error)?;
    listener.set_nonblocking(true).map_err(io_error)?;

    Ok(thread::spawn(move || {
        for peer in peers {
            let state = Arc::clone(&state);
            let work = work.clone();
            let running = Arc::clone(&running);
            thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match TcpStream::connect_timeout(&peer, IO_TIMEOUT) {
                        Ok(stream) => match sync_peer(stream, &state, &work, fee_recipient) {
                            Ok(()) => tracing::info!(%peer, "outbound P2P sync complete"),
                            Err(error) => {
                                tracing::warn!(%peer, %error, "outbound P2P sync failed")
                            }
                        },
                        Err(error) => {
                            tracing::warn!(%peer, %error, "failed to connect static P2P peer")
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
                    thread::spawn(move || {
                        if let Err(error) = serve_peer(stream, &state, &work, fee_recipient) {
                            tracing::warn!(%peer, %error, "inbound P2P handshake failed");
                        } else {
                            tracing::info!(%peer, "inbound P2P handshake complete");
                        }
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    tracing::warn!(%error, "P2P accept failed");
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }))
}

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
) -> Result<(), String> {
    let mut pool = mempool
        .write()
        .map_err(|_| "native mempool lock poisoned".to_string())?;

    for result in admit_relay_transactions(&mut pool, transactions) {
        if let Err(error) = result {
            if !error.contains("duplicate native transaction") {
                return Err(error);
            }
        }
    }
    Ok(())
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
    admit_transactions_v3(mempool, &remote_transactions)
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
                admit_transactions_v3(mempool, &transactions)?;
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

fn exchange_hello(mut stream: TcpStream, state: &StateStore) -> Result<HelloV1, String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;
    let local = local_hello(state)?;
    write_message(&mut stream, &MessageV2::Hello(local))?;
    match read_message(&mut stream)? {
        MessageV2::Hello(remote) => Ok(remote),
        _ => Err("peer did not send Hello as its first message".into()),
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

fn serve_peer(
    mut stream: TcpStream,
    state: &StateStore,
    work: &WorkManager,
    fee_recipient: Address20,
) -> Result<(), String> {
    exchange_hello(stream.try_clone().map_err(io_error)?, state)?;

    loop {
        match read_message(&mut stream) {
            Ok(MessageV2::GetBlocks(request)) => {
                let blocks = canonical_transfer_range(state, request.start_height, request.count)?;
                write_message(&mut stream, &MessageV2::Blocks(blocks))?;
            }
            Ok(MessageV2::Hello(_)) => return Err("peer sent duplicate Hello".into()),
            Ok(MessageV2::Blocks(blocks)) => ingest_blocks(state, work, fee_recipient, blocks)?,
            Err(error) if is_disconnect_error(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn sync_peer(
    mut stream: TcpStream,
    state: &StateStore,
    work: &WorkManager,
    fee_recipient: Address20,
) -> Result<(), String> {
    let remote = exchange_hello(stream.try_clone().map_err(io_error)?, state)?;
    let Some(remote_height) = remote.best_height else {
        return Ok(());
    };
    let mut start_height = find_sync_start(&mut stream, state, remote_height)?;

    while start_height <= remote_height {
        write_message(
            &mut stream,
            &MessageV2::GetBlocks(GetBlocksV1 {
                start_height,
                count: MAX_BLOCKS_PER_MESSAGE,
            }),
        )?;
        let blocks = match read_message(&mut stream)? {
            MessageV2::Blocks(blocks) => blocks,
            _ => return Err("peer did not answer GetBlocks with Blocks".into()),
        };
        if blocks.is_empty() {
            return Err("peer advertised blocks but returned an empty range".into());
        }
        let received = u64::try_from(blocks.len())
            .map_err(|_| "received block count does not fit u64".to_string())?;
        ingest_blocks(state, work, fee_recipient, blocks)?;
        start_height = start_height
            .checked_add(received)
            .ok_or_else(|| "P2P sync height overflow".to_string())?;
    }
    Ok(())
}

fn find_sync_start(
    stream: &mut TcpStream,
    state: &StateStore,
    remote_height: u64,
) -> Result<u64, String> {
    let Some(local_head) = state.best_chain_head()? else {
        return Ok(0);
    };

    let mut height = local_head.header.height.min(remote_height);
    loop {
        write_message(
            &mut *stream,
            &MessageV2::GetBlocks(GetBlocksV1 {
                start_height: height,
                count: 1,
            }),
        )?;
        let blocks = match read_message(&mut *stream)? {
            MessageV2::Blocks(blocks) => blocks,
            _ => return Err("peer did not answer common-ancestor probe with Blocks".into()),
        };
        let Some(remote_block) = blocks.first() else {
            return Err("peer returned no block for common-ancestor probe".into());
        };
        let local_block = state
            .canonical_block_at_height(height)?
            .ok_or_else(|| format!("local canonical chain is missing height {height}"))?;

        if local_block.block_id() == remote_block.header.block_id() {
            return height
                .checked_add(1)
                .ok_or_else(|| "P2P sync height overflow".to_string());
        }
        if height == 0 {
            return Err("peer does not share the local canonical genesis".into());
        }
        height -= 1;
    }
}

fn ingest_blocks(
    state: &StateStore,
    work: &WorkManager,
    fee_recipient: Address20,
    blocks: Vec<BlockTransferV2>,
) -> Result<(), String> {
    for transfer in blocks {
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

        let execution = execute_block_v1(
            &mut native_state,
            &[],
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 0,
                cpu_producer: [0_u8; 20],
            },
        )?;

        let outcome = state.insert_native_block_with_execution_outcome(
            transfer.header,
            &execution,
            &native_state,
        )?;

        if outcome.current_best == outcome.block.block_id() {
            install_next_native_work(work, state, fee_recipient)?;
        }
    }
    Ok(())
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
            install_next_native_work(work, state, fee_recipient)?;
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

fn canonical_transfer_range(
    state: &StateStore,
    start_height: u64,
    count: u16,
) -> Result<Vec<BlockTransferV2>, String> {
    if count == 0 || count > MAX_BLOCKS_PER_MESSAGE {
        return Err("GetBlocks count is outside protocol bounds".into());
    }

    let chain = state.canonical_chain()?;
    let start = usize::try_from(start_height)
        .map_err(|_| "GetBlocks start height does not fit this platform".to_string())?;
    if start >= chain.len() {
        return Ok(Vec::new());
    }

    let end = start.saturating_add(count as usize).min(chain.len());
    Ok(chain[start..end]
        .iter()
        .map(|block| BlockTransferV2 {
            header: block.header.clone(),
        })
        .collect())
}

fn is_disconnect_error(error: &str) -> bool {
    error.contains("UnexpectedEof")
        || error.contains("Connection reset")
        || error.contains("connection reset")
        || error.contains("Broken pipe")
}

const MSG_HELLO: u16 = 1;
const MSG_GET_BLOCKS: u16 = 2;
const MSG_BLOCKS: u16 = 3;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTransferV2 {
    pub header: BlockHeaderV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageV2 {
    Hello(HelloV1),
    GetBlocks(GetBlocksV1),
    Blocks(Vec<BlockTransferV2>),
}

pub fn write_message(mut writer: impl Write, message: &MessageV2) -> Result<(), String> {
    let (message_type, payload) = encode_message(message)?;
    if payload.len() > MAX_FRAME_PAYLOAD {
        return Err("P2P frame payload exceeds protocol maximum".into());
    }
    writer.write_all(&DEVNET_MAGIC).map_err(io_error)?;
    writer
        .write_all(&PROTOCOL_VERSION.to_be_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&message_type.to_be_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(io_error)?;
    writer.write_all(&payload).map_err(io_error)
}

pub fn read_message(mut reader: impl Read) -> Result<MessageV2, String> {
    let mut header = [0_u8; FRAME_HEADER_LEN];
    reader.read_exact(&mut header).map_err(io_error)?;
    if header[0..4] != DEVNET_MAGIC {
        return Err("wrong NIAHCIA P2P network magic".into());
    }
    let version = u16::from_be_bytes(header[4..6].try_into().unwrap());
    if version != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported NIAHCIA P2P protocol version {version}"
        ));
    }
    let message_type = u16::from_be_bytes(header[6..8].try_into().unwrap());
    let payload_len = u32::from_be_bytes(header[8..12].try_into().unwrap()) as usize;
    if payload_len > MAX_FRAME_PAYLOAD {
        return Err("P2P frame payload exceeds protocol maximum".into());
    }
    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).map_err(io_error)?;
    decode_message(message_type, &payload)
}

fn encode_message(message: &MessageV2) -> Result<(u16, Vec<u8>), String> {
    match message {
        MessageV2::Hello(hello) => {
            if hello.cumulative_work.len() > u16::MAX as usize {
                return Err("cumulative work encoding is too large".into());
            }
            let mut out = Vec::new();
            match (hello.best_height, hello.best_block_id) {
                (None, None) => out.push(0),
                (Some(height), Some(block_id)) => {
                    out.push(1);
                    out.extend_from_slice(&height.to_be_bytes());
                    out.extend_from_slice(&block_id);
                }
                _ => {
                    return Err(
                        "Hello best height and block ID must both be present or absent".into(),
                    )
                }
            }
            out.extend_from_slice(&(hello.cumulative_work.len() as u16).to_be_bytes());
            out.extend_from_slice(&hello.cumulative_work);
            Ok((MSG_HELLO, out))
        }
        MessageV2::GetBlocks(request) => {
            if request.count == 0 || request.count > MAX_BLOCKS_PER_MESSAGE {
                return Err("GetBlocks count is outside protocol bounds".into());
            }
            let mut out = Vec::with_capacity(10);
            out.extend_from_slice(&request.start_height.to_be_bytes());
            out.extend_from_slice(&request.count.to_be_bytes());
            Ok((MSG_GET_BLOCKS, out))
        }
        MessageV2::Blocks(blocks) => {
            if blocks.len() > MAX_BLOCKS_PER_MESSAGE as usize {
                return Err("Blocks message exceeds protocol batch limit".into());
            }
            let mut out = Vec::with_capacity(
                2usize.saturating_add(blocks.len().saturating_mul(BLOCK_HEADER_V1_LEN)),
            );
            out.extend_from_slice(&(blocks.len() as u16).to_be_bytes());
            for block in blocks {
                out.extend_from_slice(&block.header.canonical_bytes());
            }
            Ok((MSG_BLOCKS, out))
        }
    }
}

fn decode_message(message_type: u16, payload: &[u8]) -> Result<MessageV2, String> {
    let mut cursor = Cursor::new(payload);
    let message = match message_type {
        MSG_HELLO => {
            let present = cursor.u8()?;
            let (best_height, best_block_id) = match present {
                0 => (None, None),
                1 => (Some(cursor.u64()?), Some(cursor.hash32()?)),
                _ => return Err("invalid Hello best-head presence flag".into()),
            };
            let work_len = cursor.u16()? as usize;
            let cumulative_work = cursor.bytes(work_len)?.to_vec();
            MessageV2::Hello(HelloV1 {
                best_height,
                best_block_id,
                cumulative_work,
            })
        }
        MSG_GET_BLOCKS => {
            let start_height = cursor.u64()?;
            let count = cursor.u16()?;
            if count == 0 || count > MAX_BLOCKS_PER_MESSAGE {
                return Err("GetBlocks count is outside protocol bounds".into());
            }
            MessageV2::GetBlocks(GetBlocksV1 {
                start_height,
                count,
            })
        }
        MSG_BLOCKS => {
            let count = cursor.u16()? as usize;
            if count > MAX_BLOCKS_PER_MESSAGE as usize {
                return Err("Blocks message exceeds protocol batch limit".into());
            }
            let mut blocks = Vec::with_capacity(count);
            for _ in 0..count {
                let header =
                    BlockHeaderV1::from_canonical_bytes(cursor.bytes(BLOCK_HEADER_V1_LEN)?)?;
                blocks.push(BlockTransferV2 { header });
            }
            MessageV2::Blocks(blocks)
        }
        other => return Err(format!("unknown NIAHCIA P2P message type {other}")),
    };
    if !cursor.finished() {
        return Err("P2P message contains trailing bytes".into());
    }
    Ok(message)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "P2P payload length overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("truncated P2P message".into());
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_be_bytes(self.bytes(8)?.try_into().unwrap()))
    }

    fn hash32(&mut self) -> Result<Hash32, String> {
        Ok(self.bytes(32)?.try_into().unwrap())
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn io_error(error: std::io::Error) -> String {
    format!("P2P I/O error: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> BlockHeaderV1 {
        BlockHeaderV1 {
            version: 1,
            parent_hash: [0x11; 32],
            height: 7,
            timestamp: 1_800_000_007,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 9,
            extra_nonce: 10,
        }
    }

    fn round_trip(message: MessageV2) {
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();
        assert_eq!(read_message(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn hello_v1_round_trips_and_locks_frame_prefix() {
        let message = MessageV2::Hello(HelloV1 {
            best_height: Some(7),
            best_block_id: [0x44; 32].into(),
            cumulative_work: vec![0x01, 0x02, 0x03],
        });
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();
        assert_eq!(&encoded[0..4], b"NIAH");
        assert_eq!(&encoded[4..6], &2_u16.to_be_bytes());
        assert_eq!(&encoded[6..8], &MSG_HELLO.to_be_bytes());
        assert_eq!(read_message(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn get_blocks_v1_round_trips() {
        round_trip(MessageV2::GetBlocks(GetBlocksV1 {
            start_height: 8,
            count: 32,
        }));
    }

    #[test]
    fn blocks_v2_round_trips() {
        round_trip(MessageV2::Blocks(vec![BlockTransferV2 {
            header: header(),
        }]));
    }

    #[test]
    fn rejects_wrong_magic_version_and_oversized_frame_before_payload_read() {
        let message = MessageV2::GetBlocks(GetBlocksV1 {
            start_height: 0,
            count: 1,
        });
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();

        let mut wrong_magic = encoded.clone();
        wrong_magic[0] ^= 0xff;
        assert!(read_message(wrong_magic.as_slice())
            .unwrap_err()
            .contains("magic"));

        let mut wrong_version = encoded.clone();
        wrong_version[4..6].copy_from_slice(&1_u16.to_be_bytes());
        assert!(read_message(wrong_version.as_slice())
            .unwrap_err()
            .contains("version"));

        let mut oversized = encoded[..FRAME_HEADER_LEN].to_vec();
        oversized[8..12].copy_from_slice(&((MAX_FRAME_PAYLOAD + 1) as u32).to_be_bytes());
        assert!(read_message(oversized.as_slice())
            .unwrap_err()
            .contains("maximum"));
    }

    #[test]
    fn rejects_truncated_trailing_and_out_of_bounds_batches() {
        let mut truncated = Vec::new();
        write_message(
            &mut truncated,
            &MessageV2::Blocks(vec![BlockTransferV2 { header: header() }]),
        )
        .unwrap();
        truncated.pop();
        assert!(read_message(truncated.as_slice()).is_err());

        let mut trailing = Vec::new();
        write_message(
            &mut trailing,
            &MessageV2::GetBlocks(GetBlocksV1 {
                start_height: 0,
                count: 1,
            }),
        )
        .unwrap();
        let payload_len = u32::from_be_bytes(trailing[8..12].try_into().unwrap());
        trailing[8..12].copy_from_slice(&(payload_len + 1).to_be_bytes());
        trailing.push(0);
        assert!(read_message(trailing.as_slice())
            .unwrap_err()
            .contains("trailing"));

        assert!(write_message(
            Vec::new(),
            &MessageV2::GetBlocks(GetBlocksV1 {
                start_height: 0,
                count: MAX_BLOCKS_PER_MESSAGE + 1,
            }),
        )
        .is_err());
    }
}
