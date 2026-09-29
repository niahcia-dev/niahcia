use crate::consensus::block_work;
use crate::work::{BlockHeaderV1, Hash32, BLOCK_HEADER_V1_LEN};
use num_bigint::BigUint;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SERVICE_EVIDENCE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_evidence_v1");
const SERVICE_EPOCHS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_epochs_v1");
const CHAIN_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_blocks_v1");
const CHAIN_META: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_meta_v1");

const BEST_HEAD_KEY: &[u8] = b"best_head";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedChainBlock {
    pub header: BlockHeaderV1,
    pub chain_work: BigUint,
}

impl PersistedChainBlock {
    fn encode_value(&self) -> Result<Vec<u8>, String> {
        let work = self.chain_work.to_bytes_be();
        let work_len = u32::try_from(work.len())
            .map_err(|_| "chain work encoding is too large".to_string())?;

        let mut out = Vec::with_capacity(BLOCK_HEADER_V1_LEN + 4 + work.len());
        out.extend_from_slice(&self.header.canonical_bytes());
        out.extend_from_slice(&work_len.to_be_bytes());
        out.extend_from_slice(&work);
        Ok(out)
    }

    fn decode(value: &[u8]) -> Result<Self, String> {
        if value.len() < BLOCK_HEADER_V1_LEN + 4 {
            return Err("persisted chain block is truncated".into());
        }

        let header = BlockHeaderV1::from_canonical_bytes(&value[..BLOCK_HEADER_V1_LEN])?;
        let work_len = u32::from_be_bytes(
            value[BLOCK_HEADER_V1_LEN..BLOCK_HEADER_V1_LEN + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let expected_len = BLOCK_HEADER_V1_LEN + 4 + work_len;
        if value.len() != expected_len {
            return Err(format!(
                "persisted chain block length mismatch: expected {expected_len}, found {}",
                value.len()
            ));
        }

        Ok(Self {
            header,
            chain_work: BigUint::from_bytes_be(&value[BLOCK_HEADER_V1_LEN + 4..]),
        })
    }

    pub fn block_id(&self) -> Hash32 {
        self.header.block_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedServiceSuccess {
    pub evidence_key: Hash32,
    pub epoch_start_height: u64,
    pub epoch_end_height: u64,
    pub requester_id: Hash32,
    pub challenge_block_id: Hash32,
    pub verified_bytes: u64,
}

impl PersistedServiceSuccess {
    const VALUE_LEN: usize = 88;

    fn encode_value(&self) -> [u8; Self::VALUE_LEN] {
        let mut out = [0_u8; Self::VALUE_LEN];
        out[0..8].copy_from_slice(&self.epoch_start_height.to_be_bytes());
        out[8..16].copy_from_slice(&self.epoch_end_height.to_be_bytes());
        out[16..48].copy_from_slice(&self.requester_id);
        out[48..80].copy_from_slice(&self.challenge_block_id);
        out[80..88].copy_from_slice(&self.verified_bytes.to_be_bytes());
        out
    }

    fn decode(evidence_key: Hash32, value: &[u8]) -> Result<Self, String> {
        if value.len() != Self::VALUE_LEN {
            return Err(format!(
                "invalid persisted service success length: {}",
                value.len()
            ));
        }

        Ok(Self {
            evidence_key,
            epoch_start_height: u64::from_be_bytes(value[0..8].try_into().unwrap()),
            epoch_end_height: u64::from_be_bytes(value[8..16].try_into().unwrap()),
            requester_id: value[16..48].try_into().unwrap(),
            challenge_block_id: value[48..80].try_into().unwrap(),
            verified_bytes: u64::from_be_bytes(value[80..88].try_into().unwrap()),
        })
    }
}

pub struct StateStore {
    db: Database,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create state directory: {e}"))?;
        }

        let db =
            Database::create(path).map_err(|e| format!("failed to open state database: {e}"))?;

        // Open each table once so the on-disk namespace is created deliberately.
        let write = db
            .begin_write()
            .map_err(|e| format!("failed to begin state initialization: {e}"))?;
        {
            write
                .open_table(SERVICE_EVIDENCE)
                .map_err(|e| format!("failed to initialize service evidence table: {e}"))?;
            write
                .open_table(SERVICE_EPOCHS)
                .map_err(|e| format!("failed to initialize service epoch table: {e}"))?;
            write
                .open_table(CHAIN_BLOCKS)
                .map_err(|e| format!("failed to initialize chain block table: {e}"))?;
            write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to initialize chain metadata table: {e}"))?;
        }
        write
            .commit()
            .map_err(|e| format!("failed to commit state initialization: {e}"))?;

        Ok(Self { db })
    }

    pub fn load_chain_block(&self, block_id: Hash32) -> Result<Option<PersistedChainBlock>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin chain block read: {e}"))?;
        let table = read
            .open_table(CHAIN_BLOCKS)
            .map_err(|e| format!("failed to open chain block table: {e}"))?;

        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read chain block: {e}"))?
        {
            Some(value) => PersistedChainBlock::decode(value.value()).map(Some),
            None => Ok(None),
        }
    }

    pub fn best_chain_head(&self) -> Result<Option<PersistedChainBlock>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin best-head read: {e}"))?;
        let meta = read
            .open_table(CHAIN_META)
            .map_err(|e| format!("failed to open chain metadata table: {e}"))?;

        let Some(best_id) = meta
            .get(BEST_HEAD_KEY)
            .map_err(|e| format!("failed to read best-head ID: {e}"))?
        else {
            return Ok(None);
        };

        let block_id: Hash32 = best_id
            .value()
            .try_into()
            .map_err(|_| "invalid persisted best-head ID length".to_string())?;
        drop(meta);
        drop(read);
        self.load_chain_block(block_id)
    }

    pub fn insert_chain_block(&self, header: BlockHeaderV1) -> Result<PersistedChainBlock, String> {
        let parent_work = if header.height == 0 {
            if header.parent_hash != [0_u8; 32] {
                return Err("genesis block must have a zero parent hash".into());
            }
            BigUint::default()
        } else {
            let parent = self
                .load_chain_block(header.parent_hash)?
                .ok_or_else(|| "chain block parent is not persisted".to_string())?;
            if parent.header.height.checked_add(1) != Some(header.height) {
                return Err("chain block height does not follow persisted parent".into());
            }
            parent.chain_work
        };

        let record = PersistedChainBlock {
            chain_work: parent_work + block_work(header.target),
            header,
        };
        let block_id = record.block_id();
        let encoded = record.encode_value()?;

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin chain block transaction: {e}"))?;

        {
            let mut blocks = write
                .open_table(CHAIN_BLOCKS)
                .map_err(|e| format!("failed to open chain block table: {e}"))?;
            if let Some(existing) = blocks
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to check existing chain block: {e}"))?
            {
                let existing = PersistedChainBlock::decode(existing.value())?;
                if existing != record {
                    return Err("block ID collision with different persisted record".into());
                }
                return Ok(existing);
            }

            blocks
                .insert(block_id.as_slice(), encoded.as_slice())
                .map_err(|e| format!("failed to persist chain block: {e}"))?;
        }

        let should_promote = {
            let meta = write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to open chain metadata table: {e}"))?;
            match meta
                .get(BEST_HEAD_KEY)
                .map_err(|e| format!("failed to read current best head: {e}"))?
            {
                Some(best_id) => {
                    let best_id: Hash32 = best_id
                        .value()
                        .try_into()
                        .map_err(|_| "invalid persisted best-head ID length".to_string())?;
                    let blocks = write
                        .open_table(CHAIN_BLOCKS)
                        .map_err(|e| format!("failed to open chain block table: {e}"))?;
                    let best = blocks
                        .get(best_id.as_slice())
                        .map_err(|e| format!("failed to read current best block: {e}"))?
                        .ok_or_else(|| "best-head metadata references missing block".to_string())?;
                    record.chain_work > PersistedChainBlock::decode(best.value())?.chain_work
                }
                None => true,
            }
        };

        if should_promote {
            let mut meta = write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to open chain metadata table: {e}"))?;
            meta.insert(BEST_HEAD_KEY, block_id.as_slice())
                .map_err(|e| format!("failed to persist best-head ID: {e}"))?;
        }

        write
            .commit()
            .map_err(|e| format!("failed to commit chain block: {e}"))?;
        Ok(record)
    }

    pub fn insert_service_success(
        &self,
        success: &PersistedServiceSuccess,
    ) -> Result<bool, String> {
        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin service evidence transaction: {e}"))?;

        let inserted = {
            let mut table = write
                .open_table(SERVICE_EVIDENCE)
                .map_err(|e| format!("failed to open service evidence table: {e}"))?;

            if table
                .get(success.evidence_key.as_slice())
                .map_err(|e| format!("failed to read service evidence key: {e}"))?
                .is_some()
            {
                false
            } else {
                let value = success.encode_value();
                table
                    .insert(success.evidence_key.as_slice(), value.as_slice())
                    .map_err(|e| format!("failed to persist service evidence: {e}"))?;
                true
            }
        };

        if inserted {
            write
                .commit()
                .map_err(|e| format!("failed to commit service evidence: {e}"))?;
        } else {
            write
                .abort()
                .map_err(|e| format!("failed to abort duplicate evidence transaction: {e}"))?;
        }

        Ok(inserted)
    }

    pub fn load_service_successes(
        &self,
        epoch_start_height: u64,
        epoch_end_height: u64,
    ) -> Result<Vec<PersistedServiceSuccess>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin service evidence scan: {e}"))?;
        let table = read
            .open_table(SERVICE_EVIDENCE)
            .map_err(|e| format!("failed to open service evidence table: {e}"))?;

        let mut out = Vec::new();
        let iter = table
            .iter()
            .map_err(|e| format!("failed to iterate service evidence table: {e}"))?;
        for entry in iter {
            let (key, value) =
                entry.map_err(|e| format!("failed to read service evidence entry: {e}"))?;
            let evidence_key: Hash32 = key
                .value()
                .try_into()
                .map_err(|_| "invalid persisted service evidence key length".to_string())?;
            let success = PersistedServiceSuccess::decode(evidence_key, value.value())?;
            if success.epoch_start_height == epoch_start_height
                && success.epoch_end_height == epoch_end_height
            {
                out.push(success);
            }
        }

        out.sort_by_key(|entry| entry.evidence_key);
        Ok(out)
    }

    pub fn has_service_evidence(&self, evidence_key: Hash32) -> Result<bool, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin service evidence read: {e}"))?;
        let table = read
            .open_table(SERVICE_EVIDENCE)
            .map_err(|e| format!("failed to open service evidence table: {e}"))?;

        table
            .get(evidence_key.as_slice())
            .map(|entry| entry.is_some())
            .map_err(|e| format!("failed to read service evidence key: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{PersistedServiceSuccess, StateStore};
    use crate::work::BlockHeaderV1;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "niahcia-{name}-{}-{nonce}.redb",
            std::process::id()
        ))
    }

    fn header(parent_hash: [u8; 32], height: u64, target: [u8; 32], marker: u8) -> BlockHeaderV1 {
        BlockHeaderV1 {
            version: 1,
            parent_hash,
            height,
            timestamp: 1_800_000_000 + height,
            transactions_root: [marker; 32],
            execution_root: [marker.wrapping_add(1); 32],
            target,
            nonce: marker as u64,
            extra_nonce: 0,
        }
    }

    #[test]
    fn chain_blocks_survive_restart_and_best_head_uses_cumulative_work() {
        let path = temp_state_path("chain-work");
        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();

        let easy_child = header(genesis_id, 1, [0xff; 32], 2);
        let hard_child = header(genesis_id, 1, [0x7f; 32], 3);
        let hard_child_id = hard_child.block_id();

        {
            let store = StateStore::open(&path).unwrap();
            let genesis_record = store.insert_chain_block(genesis).unwrap();
            assert_eq!(genesis_record.chain_work.to_str_radix(10), "1");

            let easy = store.insert_chain_block(easy_child).unwrap();
            assert_eq!(easy.chain_work.to_str_radix(10), "2");

            let hard = store.insert_chain_block(hard_child).unwrap();
            assert!(hard.chain_work > easy.chain_work);
            assert_eq!(store.best_chain_head().unwrap().unwrap().block_id(), hard_child_id);
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            let best = reopened.best_chain_head().unwrap().unwrap();
            assert_eq!(best.block_id(), hard_child_id);
            assert_eq!(best.header.height, 1);
            assert_eq!(
                reopened
                    .load_chain_block(genesis_id)
                    .unwrap()
                    .unwrap()
                    .header
                    .height,
                0
            );
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn chain_block_rejects_missing_parent_and_wrong_height() {
        let path = temp_state_path("chain-parent");
        let store = StateStore::open(&path).unwrap();

        let missing_parent = header([0x44; 32], 1, [0xff; 32], 4);
        assert!(store.insert_chain_block(missing_parent).is_err());

        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();
        store.insert_chain_block(genesis).unwrap();

        let wrong_height = header(genesis_id, 2, [0xff; 32], 5);
        assert!(store.insert_chain_block(wrong_height).is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn service_evidence_survives_restart_and_rejects_replay() {
        let path = temp_state_path("service-replay");
        let evidence_key = [0x42; 32];

        {
            let store = StateStore::open(&path).unwrap();
            let success = PersistedServiceSuccess {
                evidence_key,
                epoch_start_height: 0,
                epoch_end_height: 720,
                requester_id: [0x11; 32],
                challenge_block_id: [0x22; 32],
                verified_bytes: 4096,
            };
            assert!(store.insert_service_success(&success).unwrap());
            assert!(store.has_service_evidence(evidence_key).unwrap());
            assert!(!store.insert_service_success(&success).unwrap());

            let loaded = store.load_service_successes(0, 720).unwrap();
            assert_eq!(loaded, vec![success]);
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert!(reopened.has_service_evidence(evidence_key).unwrap());
            let loaded = reopened.load_service_successes(0, 720).unwrap();
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].verified_bytes, 4096);
        }

        let _ = std::fs::remove_file(path);
    }
}
