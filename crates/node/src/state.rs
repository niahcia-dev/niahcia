use crate::consensus::block_work;
use crate::work::{BlockHeaderV1, Hash32, BLOCK_HEADER_V1_LEN};
use num_bigint::BigUint;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SERVICE_EVIDENCE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_evidence_v1");
const SERVICE_EPOCHS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_epochs_v1");
const CHAIN_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_blocks_v1");
const CHAIN_EXECUTION: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_execution_v1");
const EXECUTION_PAYLOADS: TableDefinition<&[u8], &[u8]> =\n    TableDefinition::new("execution_payloads_v1");
const CHAIN_META: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_meta_v1");

const BEST_HEAD_KEY: &[u8] = b"best_head";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReorg {
    pub old_head: Hash32,
    pub new_head: Hash32,
    pub common_ancestor: Hash32,
    pub detached: Vec<Hash32>,
    pub attached: Vec<Hash32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainInsertOutcome {
    pub block: PersistedChainBlock,
    pub previous_best: Option<Hash32>,
    pub current_best: Hash32,
    pub reorg: Option<ChainReorg>,
}

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
                .open_table(CHAIN_EXECUTION)
                .map_err(|e| format!("failed to initialize chain execution table: {e}"))?;
            write
                .open_table(EXECUTION_PAYLOADS)
                .map_err(|e| format!("failed to initialize execution payload table: {e}"))?;
            write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to initialize chain metadata table: {e}"))?;
        }
        write
            .commit()
            .map_err(|e| format!("failed to commit state initialization: {e}"))?;

        Ok(Self { db })
    }

    pub fn load_chain_block(
        &self,
        block_id: Hash32,
    ) -> Result<Option<PersistedChainBlock>, String> {
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

    pub fn store_execution_payload(&self, hash: Hash32, payload: &[u8]) -> Result<(), String> {
        let write = self\n            .db\n            .begin_write()\n            .map_err(|e| format!("failed to begin payload write: {e}"))?;
        {
            let mut table = write.open_table(EXECUTION_PAYLOADS)
                .map_err(|e| format!("failed to open payload table: {e}"))?;
            if let Some(existing) = table.get(hash.as_slice())
                .map_err(|e| format!("failed to inspect payload: {e}"))? {
                if existing.value() != payload {
                    return Err("execution hash already has a different replay payload".into());
                }
                return Ok(());
            }
            table.insert(hash.as_slice(), payload)
                .map_err(|e| format!("failed to persist execution payload: {e}"))?;
        }
        write\n            .commit()\n            .map_err(|e| format!("failed to commit execution payload: {e}"))
    }

    pub fn execution_payload(&self, hash: Hash32) -> Result<Option<Vec<u8>>, String> {
        let read = self.db.begin_read().map_err(|e| format!("failed to begin payload read: {e}"))?;
        let table = read.open_table(EXECUTION_PAYLOADS)
            .map_err(|e| format!("failed to open payload table: {e}"))?;
        let result = table.get(hash.as_slice())
            .map_err(|e| format!("failed to read execution payload: {e}"))?
            .map(|value| value.value().to_vec());
        Ok(result)
    }

    pub fn store_execution_hash(
        &self,
        block_id: Hash32,
        execution_hash: Hash32,
    ) -> Result<(), String> {
        if self.load_chain_block(block_id)?.is_none() {
            return Err("cannot map execution hash for an unpersisted NIAHCIA block".into());
        }
        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin execution mapping write: {e}"))?;
        {
            let mut table = write
                .open_table(CHAIN_EXECUTION)
                .map_err(|e| format!("failed to open chain execution table: {e}"))?;
            let existing: Option<Hash32> = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to read execution mapping: {e}"))?
                .map(|value| {
                    value
                        .value()
                        .try_into()
                        .map_err(|_| "invalid persisted execution hash length".to_string())
                })
                .transpose()?;

            if let Some(existing) = existing {
                if existing != execution_hash {
                    return Err("NIAHCIA block already maps to a different execution hash".into());
                }
            } else {
                table
                    .insert(block_id.as_slice(), execution_hash.as_slice())
                    .map_err(|e| format!("failed to persist execution mapping: {e}"))?;
            }
        }
        write
            .commit()
            .map_err(|e| format!("failed to commit execution mapping: {e}"))
    }

    pub fn execution_hash(&self, block_id: Hash32) -> Result<Option<Hash32>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin execution mapping read: {e}"))?;
        let table = read
            .open_table(CHAIN_EXECUTION)
            .map_err(|e| format!("failed to open chain execution table: {e}"))?;
        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read execution mapping: {e}"))?
        {
            Some(value) => value
                .value()
                .try_into()
                .map(Some)
                .map_err(|_| "invalid persisted execution hash length".to_string()),
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

    pub fn canonical_reorg(
        &self,
        old_head: Hash32,
        new_head: Hash32,
    ) -> Result<Option<ChainReorg>, String> {
        if old_head == new_head {
            return Ok(None);
        }

        let mut old_cursor = self
            .load_chain_block(old_head)?
            .ok_or_else(|| "old canonical head is not persisted".to_string())?;
        let mut new_cursor = self
            .load_chain_block(new_head)?
            .ok_or_else(|| "new canonical head is not persisted".to_string())?;

        let mut detached = Vec::new();
        let mut attached_reverse = Vec::new();

        while old_cursor.header.height > new_cursor.header.height {
            detached.push(old_cursor.block_id());
            old_cursor = self
                .load_chain_block(old_cursor.header.parent_hash)?
                .ok_or_else(|| "old canonical ancestry references missing parent".to_string())?;
        }

        while new_cursor.header.height > old_cursor.header.height {
            attached_reverse.push(new_cursor.block_id());
            new_cursor = self
                .load_chain_block(new_cursor.header.parent_hash)?
                .ok_or_else(|| "new canonical ancestry references missing parent".to_string())?;
        }

        while old_cursor.block_id() != new_cursor.block_id() {
            if old_cursor.header.height == 0 || new_cursor.header.height == 0 {
                return Err("canonical heads do not share a persisted genesis".into());
            }

            detached.push(old_cursor.block_id());
            attached_reverse.push(new_cursor.block_id());

            old_cursor = self
                .load_chain_block(old_cursor.header.parent_hash)?
                .ok_or_else(|| "old canonical ancestry references missing parent".to_string())?;
            new_cursor = self
                .load_chain_block(new_cursor.header.parent_hash)?
                .ok_or_else(|| "new canonical ancestry references missing parent".to_string())?;
        }

        attached_reverse.reverse();

        Ok(Some(ChainReorg {
            old_head,
            new_head,
            common_ancestor: old_cursor.block_id(),
            detached,
            attached: attached_reverse,
        }))
    }

    pub fn canonical_block_at_height(
        &self,
        height: u64,
    ) -> Result<Option<PersistedChainBlock>, String> {
        let Some(mut cursor) = self.best_chain_head()? else {
            return Ok(None);
        };

        if height > cursor.header.height {
            return Ok(None);
        }

        while cursor.header.height > height {
            cursor = self
                .load_chain_block(cursor.header.parent_hash)?
                .ok_or_else(|| "best-chain ancestry references missing parent".to_string())?;
        }

        Ok(Some(cursor))
    }

    pub fn is_on_best_chain(&self, block_id: Hash32) -> Result<bool, String> {
        let Some(mut cursor) = self.best_chain_head()? else {
            return Ok(false);
        };

        loop {
            if cursor.block_id() == block_id {
                return Ok(true);
            }
            if cursor.header.height == 0 {
                return Ok(false);
            }
            cursor = self
                .load_chain_block(cursor.header.parent_hash)?
                .ok_or_else(|| "best-chain ancestry references missing parent".to_string())?;
        }
    }

    pub fn insert_chain_block_with_outcome(
        &self,
        header: BlockHeaderV1,
    ) -> Result<ChainInsertOutcome, String> {
        let previous_best = self.best_chain_head()?.map(|block| block.block_id());
        let block = self.insert_chain_block(header)?;
        let current_best = self
            .best_chain_head()?
            .ok_or_else(|| "chain insert committed without a best head".to_string())?
            .block_id();

        let reorg = match previous_best {
            Some(old_head) if old_head != current_best => {
                self.canonical_reorg(old_head, current_best)?
            }
            _ => None,
        };

        Ok(ChainInsertOutcome {
            block,
            previous_best,
            current_best,
            reorg,
        })
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

        let current_best_id: Option<Hash32> = {
            let meta = write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to open chain metadata table: {e}"))?;
            let current = meta
                .get(BEST_HEAD_KEY)
                .map_err(|e| format!("failed to read current best head: {e}"))?;
            current
                .map(|best_id| {
                    best_id
                        .value()
                        .try_into()
                        .map_err(|_| "invalid persisted best-head ID length".to_string())
                })
                .transpose()?
        };

        let should_promote = match current_best_id {
            Some(best_id) => {
                let blocks = write
                    .open_table(CHAIN_BLOCKS)
                    .map_err(|e| format!("failed to open chain block table: {e}"))?;
                let best = blocks
                    .get(best_id.as_slice())
                    .map_err(|e| format!("failed to read current best block: {e}"))?
                    .ok_or_else(|| "best-head metadata references missing block".to_string())?;
                let best = PersistedChainBlock::decode(best.value())?;
                record.chain_work > best.chain_work
                    || (record.chain_work == best.chain_work && block_id < best_id)
            }
            None => true,
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
    use super::{ChainInsertOutcome, ChainReorg, PersistedServiceSuccess, StateStore};
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
    fn insert_outcome_surfaces_canonical_head_change() {
        let path = temp_state_path("insert-outcome");
        let store = StateStore::open(&path).unwrap();

        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();
        let genesis_outcome = store.insert_chain_block_with_outcome(genesis).unwrap();
        assert_eq!(
            genesis_outcome,
            ChainInsertOutcome {
                block: store.load_chain_block(genesis_id).unwrap().unwrap(),
                previous_best: None,
                current_best: genesis_id,
                reorg: None,
            }
        );

        let easy = header(genesis_id, 1, [0xff; 32], 2);
        let easy_id = easy.block_id();
        store.insert_chain_block(easy).unwrap();

        let hard = header(genesis_id, 1, [0x7f; 32], 3);
        let hard_id = hard.block_id();
        let outcome = store.insert_chain_block_with_outcome(hard).unwrap();

        assert_eq!(outcome.previous_best, Some(easy_id));
        assert_eq!(outcome.current_best, hard_id);
        let reorg = outcome.reorg.unwrap();
        assert_eq!(reorg.old_head, easy_id);
        assert_eq!(reorg.new_head, hard_id);
        assert_eq!(reorg.common_ancestor, genesis_id);
        assert_eq!(reorg.detached, vec![easy_id]);
        assert_eq!(reorg.attached, vec![hard_id]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn equal_work_tie_break_is_block_id_order() {
        let path = temp_state_path("equal-work-tie-break");
        let store = StateStore::open(&path).unwrap();

        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();
        store.insert_chain_block(genesis).unwrap();

        let left = header(genesis_id, 1, [0xff; 32], 2);
        let right = header(genesis_id, 1, [0xff; 32], 3);
        let left_id = left.block_id();
        let right_id = right.block_id();

        let (first, first_id, second, second_id) = if left_id > right_id {
            (left, left_id, right, right_id)
        } else {
            (right, right_id, left, left_id)
        };

        store.insert_chain_block(first).unwrap();
        assert_eq!(
            store.best_chain_head().unwrap().unwrap().block_id(),
            first_id
        );

        store.insert_chain_block(second).unwrap();
        assert_eq!(
            store.best_chain_head().unwrap().unwrap().block_id(),
            second_id
        );
        assert!(second_id < first_id);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn canonical_reorg_reports_detached_and_attached_paths() {
        let path = temp_state_path("chain-reorg");
        let store = StateStore::open(&path).unwrap();

        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();
        store.insert_chain_block(genesis).unwrap();

        let a1 = header(genesis_id, 1, [0xff; 32], 2);
        let a1_id = a1.block_id();
        store.insert_chain_block(a1).unwrap();
        let a2 = header(a1_id, 2, [0xff; 32], 3);
        let a2_id = a2.block_id();
        store.insert_chain_block(a2).unwrap();

        let b1 = header(genesis_id, 1, [0x7f; 32], 4);
        let b1_id = b1.block_id();
        store.insert_chain_block(b1).unwrap();
        let b2 = header(b1_id, 2, [0x7f; 32], 5);
        let b2_id = b2.block_id();
        store.insert_chain_block(b2).unwrap();

        assert_eq!(store.best_chain_head().unwrap().unwrap().block_id(), b2_id);

        let reorg = store.canonical_reorg(a2_id, b2_id).unwrap().unwrap();
        assert_eq!(
            reorg,
            ChainReorg {
                old_head: a2_id,
                new_head: b2_id,
                common_ancestor: genesis_id,
                detached: vec![a2_id, a1_id],
                attached: vec![b1_id, b2_id],
            }
        );
        assert!(store.canonical_reorg(b2_id, b2_id).unwrap().is_none());

        let _ = std::fs::remove_file(path);
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
            assert_eq!(
                store.best_chain_head().unwrap().unwrap().block_id(),
                hard_child_id
            );
            assert!(store.is_on_best_chain(genesis_id).unwrap());
            assert!(store.is_on_best_chain(hard_child_id).unwrap());
            assert!(!store.is_on_best_chain(easy.block_id()).unwrap());
            assert!(store.load_chain_block(easy.block_id()).unwrap().is_some());
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            let best = reopened.best_chain_head().unwrap().unwrap();
            assert_eq!(best.block_id(), hard_child_id);
            assert_eq!(best.header.height, 1);
            assert!(reopened.is_on_best_chain(hard_child_id).unwrap());
            assert_eq!(
                reopened
                    .canonical_block_at_height(0)
                    .unwrap()
                    .unwrap()
                    .block_id(),
                genesis_id
            );
            assert_eq!(
                reopened
                    .canonical_block_at_height(1)
                    .unwrap()
                    .unwrap()
                    .block_id(),
                hard_child_id
            );
            assert!(reopened.canonical_block_at_height(2).unwrap().is_none());
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
