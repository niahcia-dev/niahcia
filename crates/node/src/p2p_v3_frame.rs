use crate::p2p::{GetBlocksV1, HelloV1};
use crate::p2p_transaction_relay::{GetTxV1, TxInvV1, TxV1};
use crate::p2p_v3_codec::{decode_blocks_v3, encode_blocks_v3, BlockTransferV3};
use crate::work::Hash32;
use std::io::{Read, Write};

pub const DEVNET_MAGIC_V3: [u8; 4] = *b"NIAH";
pub const PROTOCOL_VERSION_V3: u16 = 3;
pub const FRAME_HEADER_LEN_V3: usize = 12;
pub const MAX_FRAME_PAYLOAD_V3: usize = 16 * 1024 * 1024;
pub const MAX_BLOCKS_REQUEST_V3: u16 = 128;

const MSG_HELLO: u16 = 1;
const MSG_GET_BLOCKS: u16 = 2;
const MSG_BLOCKS: u16 = 3;
const MSG_TX_INV: u16 = 4;
const MSG_GET_TX: u16 = 5;
const MSG_TX: u16 = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageV3 {
    Hello(HelloV1),
    GetBlocks(GetBlocksV1),
    Blocks(Vec<BlockTransferV3>),
    TxInv(TxInvV1),
    GetTx(GetTxV1),
    Tx(TxV1),
}

pub fn write_message_v3(mut writer: impl Write, message: &MessageV3) -> Result<(), String> {
    let (message_type, payload) = encode_message_v3(message)?;
    if payload.len() > MAX_FRAME_PAYLOAD_V3 {
        return Err("P2P V3 frame payload exceeds protocol maximum".into());
    }

    writer.write_all(&DEVNET_MAGIC_V3).map_err(io_error)?;
    writer
        .write_all(&PROTOCOL_VERSION_V3.to_be_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&message_type.to_be_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(io_error)?;
    writer.write_all(&payload).map_err(io_error)
}

pub fn read_message_v3(mut reader: impl Read) -> Result<MessageV3, String> {
    let mut header = [0_u8; FRAME_HEADER_LEN_V3];
    reader.read_exact(&mut header).map_err(io_error)?;

    if header[0..4] != DEVNET_MAGIC_V3 {
        return Err("wrong NIAHCIA P2P V3 network magic".into());
    }

    let version = u16::from_be_bytes(header[4..6].try_into().unwrap());
    if version != PROTOCOL_VERSION_V3 {
        return Err(format!(
            "unsupported NIAHCIA P2P V3 protocol version {version}"
        ));
    }

    let message_type = u16::from_be_bytes(header[6..8].try_into().unwrap());
    let payload_len = u32::from_be_bytes(header[8..12].try_into().unwrap()) as usize;
    if payload_len > MAX_FRAME_PAYLOAD_V3 {
        return Err("P2P V3 frame payload exceeds protocol maximum".into());
    }

    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).map_err(io_error)?;
    decode_message_v3(message_type, &payload)
}

fn encode_message_v3(message: &MessageV3) -> Result<(u16, Vec<u8>), String> {
    match message {
        MessageV3::Hello(hello) => Ok((MSG_HELLO, encode_hello(hello)?)),
        MessageV3::GetBlocks(request) => Ok((MSG_GET_BLOCKS, encode_get_blocks(request)?)),
        MessageV3::Blocks(blocks) => Ok((MSG_BLOCKS, encode_blocks_v3(blocks)?)),
        MessageV3::TxInv(message) => Ok((MSG_TX_INV, message.encode()?)),
        MessageV3::GetTx(message) => Ok((MSG_GET_TX, message.encode()?)),
        MessageV3::Tx(message) => Ok((MSG_TX, message.encode()?)),
    }
}

fn decode_message_v3(message_type: u16, payload: &[u8]) -> Result<MessageV3, String> {
    match message_type {
        MSG_HELLO => decode_hello(payload).map(MessageV3::Hello),
        MSG_GET_BLOCKS => decode_get_blocks(payload).map(MessageV3::GetBlocks),
        MSG_BLOCKS => decode_blocks_v3(payload).map(MessageV3::Blocks),
        MSG_TX_INV => TxInvV1::decode(payload).map(MessageV3::TxInv),
        MSG_GET_TX => GetTxV1::decode(payload).map(MessageV3::GetTx),
        MSG_TX => TxV1::decode(payload).map(MessageV3::Tx),
        other => Err(format!("unknown NIAHCIA P2P V3 message type {other}")),
    }
}

fn encode_hello(hello: &HelloV1) -> Result<Vec<u8>, String> {
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
        _ => return Err("Hello best height and block ID must both be present or absent".into()),
    }

    out.extend_from_slice(&(hello.cumulative_work.len() as u16).to_be_bytes());
    out.extend_from_slice(&hello.cumulative_work);
    Ok(out)
}

fn decode_hello(payload: &[u8]) -> Result<HelloV1, String> {
    let mut cursor = Cursor::new(payload);
    let present = cursor.u8()?;
    let (best_height, best_block_id) = match present {
        0 => (None, None),
        1 => (Some(cursor.u64()?), Some(cursor.hash32()?)),
        _ => return Err("invalid Hello best-head presence flag".into()),
    };

    let work_len = cursor.u16()? as usize;
    let cumulative_work = cursor.bytes(work_len)?.to_vec();
    if !cursor.finished() {
        return Err("P2P V3 Hello contains trailing bytes".into());
    }

    Ok(HelloV1 {
        best_height,
        best_block_id,
        cumulative_work,
    })
}

fn encode_get_blocks(request: &GetBlocksV1) -> Result<Vec<u8>, String> {
    if request.count == 0 || request.count > MAX_BLOCKS_REQUEST_V3 {
        return Err("P2P V3 GetBlocks count is outside protocol bounds".into());
    }

    let mut out = Vec::with_capacity(10);
    out.extend_from_slice(&request.start_height.to_be_bytes());
    out.extend_from_slice(&request.count.to_be_bytes());
    Ok(out)
}

fn decode_get_blocks(payload: &[u8]) -> Result<GetBlocksV1, String> {
    let mut cursor = Cursor::new(payload);
    let start_height = cursor.u64()?;
    let count = cursor.u16()?;
    if count == 0 || count > MAX_BLOCKS_REQUEST_V3 {
        return Err("P2P V3 GetBlocks count is outside protocol bounds".into());
    }
    if !cursor.finished() {
        return Err("P2P V3 GetBlocks contains trailing bytes".into());
    }

    Ok(GetBlocksV1 {
        start_height,
        count,
    })
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
            .ok_or_else(|| "P2P V3 payload length overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("truncated P2P V3 message".into());
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
    format!("P2P V3 I/O error: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_block_body::NativeBlockBodyV1;
    use crate::native_transaction::native_transactions_root_v1;
    use crate::work::BlockHeaderV1;

    fn block() -> BlockTransferV3 {
        let body = NativeBlockBodyV1::empty();
        let transactions = body.decoded_transactions().unwrap();
        BlockTransferV3 {
            header: BlockHeaderV1 {
                version: 1,
                parent_hash: [0x11; 32],
                height: 7,
                timestamp: 1_800_000_007,
                transactions_root: native_transactions_root_v1(&transactions).unwrap(),
                execution_root: [0x33; 32],
                target: [0xff; 32],
                nonce: 9,
                extra_nonce: 10,
            },
            body,
        }
    }

    fn round_trip(message: MessageV3) {
        let mut encoded = Vec::new();
        write_message_v3(&mut encoded, &message).unwrap();
        assert_eq!(read_message_v3(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn hello_round_trips_with_v3_frame_prefix() {
        let message = MessageV3::Hello(HelloV1 {
            best_height: Some(7),
            best_block_id: Some([0x44; 32]),
            cumulative_work: vec![1, 2, 3],
        });
        let mut encoded = Vec::new();
        write_message_v3(&mut encoded, &message).unwrap();

        assert_eq!(&encoded[0..4], b"NIAH");
        assert_eq!(&encoded[4..6], &3_u16.to_be_bytes());
        assert_eq!(&encoded[6..8], &MSG_HELLO.to_be_bytes());
        assert_eq!(read_message_v3(encoded.as_slice()).unwrap(), message);
    }

    #[test]
    fn get_blocks_round_trips() {
        round_trip(MessageV3::GetBlocks(GetBlocksV1 {
            start_height: 8,
            count: 32,
        }));
    }

    #[test]
    fn blocks_round_trip_with_canonical_body() {
        round_trip(MessageV3::Blocks(vec![block()]));
    }

    #[test]
    fn transaction_relay_messages_round_trip() {
        round_trip(MessageV3::TxInv(TxInvV1 {
            tx_ids: vec![[0x11; 32]],
        }));
        round_trip(MessageV3::GetTx(GetTxV1 {
            tx_ids: vec![[0x22; 32]],
        }));
        round_trip(MessageV3::Tx(TxV1 {
            canonical_transactions: Vec::new(),
        }));
    }

    #[test]
    fn rejects_wrong_version_and_unknown_message_type() {
        let message = MessageV3::GetBlocks(GetBlocksV1 {
            start_height: 0,
            count: 1,
        });
        let mut encoded = Vec::new();
        write_message_v3(&mut encoded, &message).unwrap();

        let mut wrong_version = encoded.clone();
        wrong_version[4..6].copy_from_slice(&2_u16.to_be_bytes());
        assert!(read_message_v3(wrong_version.as_slice())
            .unwrap_err()
            .contains("version"));

        let mut unknown = encoded;
        unknown[6..8].copy_from_slice(&99_u16.to_be_bytes());
        assert!(read_message_v3(unknown.as_slice())
            .unwrap_err()
            .contains("unknown"));
    }

    #[test]
    fn rejects_oversized_frame_before_payload_read() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&DEVNET_MAGIC_V3);
        frame.extend_from_slice(&PROTOCOL_VERSION_V3.to_be_bytes());
        frame.extend_from_slice(&MSG_TX.to_be_bytes());
        frame.extend_from_slice(&((MAX_FRAME_PAYLOAD_V3 + 1) as u32).to_be_bytes());

        assert!(read_message_v3(frame.as_slice())
            .unwrap_err()
            .contains("maximum"));
    }
}
