use crate::consensus::{randomx_seed, randomx_seed_height};
use crate::engine::EngineClient;
use crate::mining_rpc::validate_block_candidate;
use crate::state::StateStore;
use crate::work::{BlockHeaderV1, Hash32, BLOCK_HEADER_V1_LEN};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const DEVNET_MAGIC: [u8; 4] = *b"NIAH";
pub const PROTOCOL_VERSION: u16 = 1;
pub const FRAME_HEADER_LEN: usize = 12;
pub const MAX_FRAME_PAYLOAD: usize = 4 * 1024 * 1024;
pub const MAX_BLOCKS_PER_MESSAGE: u16 = 128;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

pub fn spawn(
    bind: SocketAddr,
    peers: Vec<SocketAddr>,
    state: Arc<StateStore>,
    engine: Arc<EngineClient>,
    running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, String> {
    let listener = TcpListener::bind(bind).map_err(io_error)?;
    listener.set_nonblocking(true).map_err(io_error)?;

    Ok(thread::spawn(move || {
        for peer in peers {
            let state = Arc::clone(&state);
            let engine = Arc::clone(&engine);
            thread::spawn(
                move || match TcpStream::connect_timeout(&peer, IO_TIMEOUT) {
                    Ok(stream) => {
                        if let Err(error) = sync_peer(stream, &state, &engine) {
                            tracing::warn!(%peer, %error, "outbound P2P handshake failed");
                        } else {
                            tracing::info!(%peer, "outbound P2P handshake complete");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%peer, %error, "failed to connect static P2P peer")
                    }
                },
            );
        }

        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    let state = Arc::clone(&state);
                    let engine = Arc::clone(&engine);
                    thread::spawn(move || {
                        if let Err(error) = serve_peer(stream, &state, &engine) {
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

fn exchange_hello(mut stream: TcpStream, state: &StateStore) -> Result<HelloV1, String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(io_error)?;
    let local = local_hello(state)?;
    write_message(&mut stream, &MessageV1::Hello(local))?;
    match read_message(&mut stream)? {
        MessageV1::Hello(remote) => Ok(remote),
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
    engine: &EngineClient,
) -> Result<(), String> {
    exchange_hello(stream.try_clone().map_err(io_error)?, state)?;

    loop {
        match read_message(&mut stream) {
            Ok(MessageV1::GetBlocks(request)) => {
                let blocks = canonical_transfer_range(state, request.start_height, request.count)?;
                write_message(&mut stream, &MessageV1::Blocks(blocks))?;
            }
            Ok(MessageV1::Hello(_)) => return Err("peer sent duplicate Hello".into()),
            Ok(MessageV1::Blocks(blocks)) => ingest_blocks(state, engine, blocks)?,
            Err(error) if is_disconnect_error(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn sync_peer(
    mut stream: TcpStream,
    state: &StateStore,
    engine: &EngineClient,
) -> Result<(), String> {
    let remote = exchange_hello(stream.try_clone().map_err(io_error)?, state)?;
    let local_height = state.best_chain_head()?.map(|head| head.header.height);
    let start_height = local_height.map_or(0, |height| height.saturating_add(1));
    if remote
        .best_height
        .is_some_and(|height| height >= start_height)
    {
        write_message(
            &mut stream,
            &MessageV1::GetBlocks(GetBlocksV1 {
                start_height,
                count: MAX_BLOCKS_PER_MESSAGE,
            }),
        )?;
        match read_message(&mut stream)? {
            MessageV1::Blocks(blocks) => ingest_blocks(state, engine, blocks)?,
            _ => return Err("peer did not answer GetBlocks with Blocks".into()),
        }
    }
    Ok(())
}

fn ingest_blocks(
    state: &StateStore,
    engine: &EngineClient,
    blocks: Vec<BlockTransferV1>,
) -> Result<(), String> {
    for transfer in blocks {
        let seed_height = randomx_seed_height(transfer.header.height);
        let seed_block_id = if transfer.header.height == 0 {
            [0_u8; 32]
        } else {
            state
                .canonical_block_at_height(seed_height)?
                .ok_or_else(|| format!("missing RandomX seed block {seed_height}"))?
                .block_id()
        };
        let seed = randomx_seed(seed_block_id);
        validate_block_candidate(&transfer.header, seed, state)?;
        engine.replay_execution_payload(&transfer.replay_payload, transfer.execution_hash)?;
        state.store_execution_payload(transfer.execution_hash, &transfer.replay_payload)?;
        let outcome = state
            .insert_mined_block_with_execution_outcome(transfer.header, transfer.execution_hash)?;
        if outcome.current_best == outcome.block.block_id() {
            engine.set_canonical_head_v3(transfer.execution_hash)?;
        }
    }
    Ok(())
}

fn canonical_transfer_range(
    state: &StateStore,
    start_height: u64,
    count: u16,
) -> Result<Vec<BlockTransferV1>, String> {
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
    let mut transfers = Vec::with_capacity(end - start);
    for block in &chain[start..end] {
        let block_id = block.block_id();
        let execution_hash = state.execution_hash(block_id)?.ok_or_else(|| {
            format!(
                "canonical block {} has no execution mapping",
                block.header.height
            )
        })?;
        let replay_payload = state.execution_payload(execution_hash)?.ok_or_else(|| {
            format!(
                "canonical block {} has no execution replay payload",
                block.header.height
            )
        })?;
        transfers.push(BlockTransferV1 {
            header: block.header.clone(),
            execution_hash,
            replay_payload,
        });
    }
    Ok(transfers)
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
pub struct BlockTransferV1 {
    pub header: BlockHeaderV1,
    pub execution_hash: Hash32,
    pub replay_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageV1 {
    Hello(HelloV1),
    GetBlocks(GetBlocksV1),
    Blocks(Vec<BlockTransferV1>),
}

pub fn write_message(mut writer: impl Write, message: &MessageV1) -> Result<(), String> {
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

pub fn read_message(mut reader: impl Read) -> Result<MessageV1, String> {
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

fn encode_message(message: &MessageV1) -> Result<(u16, Vec<u8>), String> {
    match message {
        MessageV1::Hello(hello) => {
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
        MessageV1::GetBlocks(request) => {
            if request.count == 0 || request.count > MAX_BLOCKS_PER_MESSAGE {
                return Err("GetBlocks count is outside protocol bounds".into());
            }
            let mut out = Vec::with_capacity(10);
            out.extend_from_slice(&request.start_height.to_be_bytes());
            out.extend_from_slice(&request.count.to_be_bytes());
            Ok((MSG_GET_BLOCKS, out))
        }
        MessageV1::Blocks(blocks) => {
            if blocks.len() > MAX_BLOCKS_PER_MESSAGE as usize {
                return Err("Blocks message exceeds protocol batch limit".into());
            }
            let mut out = Vec::new();
            out.extend_from_slice(&(blocks.len() as u16).to_be_bytes());
            for block in blocks {
                if block.replay_payload.len() > u32::MAX as usize {
                    return Err("execution replay payload is too large".into());
                }
                out.extend_from_slice(&block.header.canonical_bytes());
                out.extend_from_slice(&block.execution_hash);
                out.extend_from_slice(&(block.replay_payload.len() as u32).to_be_bytes());
                out.extend_from_slice(&block.replay_payload);
            }
            Ok((MSG_BLOCKS, out))
        }
    }
}

fn decode_message(message_type: u16, payload: &[u8]) -> Result<MessageV1, String> {
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
            MessageV1::Hello(HelloV1 {
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
            MessageV1::GetBlocks(GetBlocksV1 {
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
                let execution_hash = cursor.hash32()?;
                let replay_len = cursor.u32()? as usize;
                let replay_payload = cursor.bytes(replay_len)?.to_vec();
                blocks.push(BlockTransferV1 {
                    header,
                    execution_hash,
                    replay_payload,
                });
            }
            MessageV1::Blocks(blocks)
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

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
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

    fn round_trip(message: MessageV1) {
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();
        assert_eq!(read_message(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn hello_v1_round_trips_and_locks_frame_prefix() {
        let message = MessageV1::Hello(HelloV1 {
            best_height: Some(7),
            best_block_id: [0x44; 32].into(),
            cumulative_work: vec![0x01, 0x02, 0x03],
        });
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();
        assert_eq!(&encoded[0..4], b"NIAH");
        assert_eq!(&encoded[4..6], &1_u16.to_be_bytes());
        assert_eq!(&encoded[6..8], &MSG_HELLO.to_be_bytes());
        assert_eq!(read_message(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn get_blocks_v1_round_trips() {
        round_trip(MessageV1::GetBlocks(GetBlocksV1 {
            start_height: 8,
            count: 32,
        }));
    }

    #[test]
    fn blocks_v1_round_trips() {
        round_trip(MessageV1::Blocks(vec![BlockTransferV1 {
            header: header(),
            execution_hash: [0x55; 32],
            replay_payload: b"replay".to_vec(),
        }]));
    }

    #[test]
    fn rejects_wrong_magic_version_and_oversized_frame_before_payload_read() {
        let message = MessageV1::GetBlocks(GetBlocksV1 {
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
        wrong_version[4..6].copy_from_slice(&2_u16.to_be_bytes());
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
            &MessageV1::Blocks(vec![BlockTransferV1 {
                header: header(),
                execution_hash: [0x55; 32],
                replay_payload: b"replay".to_vec(),
            }]),
        )
        .unwrap();
        truncated.pop();
        assert!(read_message(truncated.as_slice()).is_err());

        let mut trailing = Vec::new();
        write_message(
            &mut trailing,
            &MessageV1::GetBlocks(GetBlocksV1 {
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
            &MessageV1::GetBlocks(GetBlocksV1 {
                start_height: 0,
                count: MAX_BLOCKS_PER_MESSAGE + 1,
            }),
        )
        .is_err());
    }
}
