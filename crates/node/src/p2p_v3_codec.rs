use crate::native_block_body::NativeBlockBodyV1;
use crate::native_transaction::native_transactions_root_v1;
use crate::work::{BlockHeaderV1, BLOCK_HEADER_V1_LEN};

pub const MAX_BLOCKS_PER_V3_MESSAGE: usize = 128;
pub const MAX_BLOCK_BODY_BYTES_V3: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockTransferV3 {
    pub header: BlockHeaderV1,
    pub body: NativeBlockBodyV1,
}

impl BlockTransferV3 {
    pub fn validate_transaction_commitment(&self) -> Result<(), String> {
        let transactions = self.body.decoded_transactions()?;
        let root = native_transactions_root_v1(&transactions)?;
        if root != self.header.transactions_root {
            return Err("P2P V3 block body transactions root does not match header".into());
        }
        Ok(())
    }
}

pub fn encode_blocks_v3(blocks: &[BlockTransferV3]) -> Result<Vec<u8>, String> {
    if blocks.len() > MAX_BLOCKS_PER_V3_MESSAGE {
        return Err("P2P V3 block batch exceeds protocol limit".into());
    }

    let mut out = Vec::new();
    out.extend_from_slice(&(blocks.len() as u16).to_be_bytes());

    for block in blocks {
        block.validate_transaction_commitment()?;
        let body = block.body.canonical_bytes()?;
        if body.len() > MAX_BLOCK_BODY_BYTES_V3 {
            return Err("P2P V3 native block body exceeds byte limit".into());
        }

        out.extend_from_slice(&block.header.canonical_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
    }

    Ok(out)
}

pub fn decode_blocks_v3(bytes: &[u8]) -> Result<Vec<BlockTransferV3>, String> {
    let mut cursor = Cursor::new(bytes);
    let count = cursor.u16()? as usize;
    if count > MAX_BLOCKS_PER_V3_MESSAGE {
        return Err("P2P V3 block batch exceeds protocol limit".into());
    }

    let mut blocks = Vec::with_capacity(count);
    for _ in 0..count {
        let header = BlockHeaderV1::from_canonical_bytes(cursor.bytes(BLOCK_HEADER_V1_LEN)?)?;
        let body_len = cursor.u32()? as usize;
        if body_len > MAX_BLOCK_BODY_BYTES_V3 {
            return Err("P2P V3 native block body exceeds byte limit".into());
        }

        let body = NativeBlockBodyV1::from_canonical_bytes(cursor.bytes(body_len)?)?;
        let transfer = BlockTransferV3 { header, body };
        transfer.validate_transaction_commitment()?;
        blocks.push(transfer);
    }

    if !cursor.finished() {
        return Err("P2P V3 block batch contains trailing bytes".into());
    }

    Ok(blocks)
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
            return Err("truncated P2P V3 block payload".into());
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_block_body::NativeBlockBodyV1;
    use crate::work::BlockHeaderV1;

    fn empty_transfer(marker: u8) -> BlockTransferV3 {
        let body = NativeBlockBodyV1::empty();
        let transactions = body.decoded_transactions().unwrap();
        let transactions_root = native_transactions_root_v1(&transactions).unwrap();

        BlockTransferV3 {
            header: BlockHeaderV1 {
                version: 1,
                parent_hash: [marker; 32],
                height: marker as u64,
                timestamp: 1_800_000_000 + marker as u64,
                transactions_root,
                execution_root: [marker.wrapping_add(1); 32],
                target: [0xff; 32],
                nonce: marker as u64,
                extra_nonce: marker as u64 + 1,
            },
            body,
        }
    }

    #[test]
    fn v3_block_batch_round_trips_exact_body() {
        let blocks = vec![empty_transfer(1), empty_transfer(2)];
        let encoded = encode_blocks_v3(&blocks).unwrap();
        let decoded = decode_blocks_v3(&encoded).unwrap();
        assert_eq!(decoded, blocks);
    }

    #[test]
    fn v3_decoder_rejects_trailing_bytes() {
        let blocks = vec![empty_transfer(1)];
        let mut encoded = encode_blocks_v3(&blocks).unwrap();
        encoded.push(0);
        assert!(decode_blocks_v3(&encoded).is_err());
    }

    #[test]
    fn v3_rejects_body_header_transaction_root_mismatch() {
        let mut transfer = empty_transfer(1);
        transfer.header.transactions_root[0] ^= 1;
        assert!(encode_blocks_v3(&[transfer]).is_err());
    }

    #[test]
    fn v3_rejects_truncated_body() {
        let blocks = vec![empty_transfer(1)];
        let mut encoded = encode_blocks_v3(&blocks).unwrap();
        encoded.pop();
        assert!(decode_blocks_v3(&encoded).is_err());
    }

    #[test]
    fn v3_rejects_batch_above_limit() {
        let block = empty_transfer(1);
        let blocks = vec![block; MAX_BLOCKS_PER_V3_MESSAGE + 1];
        assert!(encode_blocks_v3(&blocks).is_err());
    }
}
