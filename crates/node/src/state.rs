use crate::consensus::block_work;
use crate::native_block_body::NativeBlockBodyV1;
use crate::native_block_body_v2::NativeBlockBodyV2;
use crate::native_execution::{NativeBlockExecutionResultV1, NativeStateV1};
use crate::native_execution_commitment_v2::NativeBlockExecutionResultV2;
use crate::native_state_v2::NativeStateV2;
use crate::native_transaction::native_transactions_root_v1;
use crate::work::{BlockHeaderV1, Hash32, BLOCK_HEADER_V1_LEN};
use num_bigint::BigUint;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SERVICE_EVIDENCE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_evidence_v1");
const SERVICE_EPOCHS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_epochs_v1");
const CHAIN_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_blocks_v1");
const NATIVE_STATE_SNAPSHOTS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("native_state_snapshots_v1");
const NATIVE_BLOCK_EXECUTION: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("native_block_execution_v1");
const NATIVE_BLOCK_BODIES: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("native_block_bodies_v1");
const INACTIVE_NATIVE_BLOCK_BODIES_V2: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("inactive_native_block_bodies_v2");
const INACTIVE_NATIVE_BLOCK_EXECUTION_V2: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("inactive_native_block_execution_v2");
const INACTIVE_NATIVE_STATE_SNAPSHOTS_V2: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("inactive_native_state_snapshots_v2");
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NativeStateSnapshotVersion {
    V1 = 1,
    V2 = 2,
}

impl TryFrom<u8> for NativeStateSnapshotVersion {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            other => Err(format!(
                "unsupported native state snapshot version: {other}"
            )),
        }
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
                .open_table(NATIVE_STATE_SNAPSHOTS)
                .map_err(|e| format!("failed to initialize native state snapshot table: {e}"))?;
            write
                .open_table(NATIVE_BLOCK_EXECUTION)
                .map_err(|e| format!("failed to initialize native block execution table: {e}"))?;
            write
                .open_table(NATIVE_BLOCK_BODIES)
                .map_err(|e| format!("failed to initialize native block body table: {e}"))?;
            write
                .open_table(INACTIVE_NATIVE_BLOCK_BODIES_V2)
                .map_err(|e| format!("failed to initialize inactive V2 block body table: {e}"))?;
            write
                .open_table(INACTIVE_NATIVE_BLOCK_EXECUTION_V2)
                .map_err(|e| format!("failed to initialize inactive V2 execution table: {e}"))?;
            write
                .open_table(INACTIVE_NATIVE_STATE_SNAPSHOTS_V2)
                .map_err(|e| format!("failed to initialize inactive V2 state table: {e}"))?;
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

    pub fn native_block_execution(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeBlockExecutionResultV1>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin native execution read: {e}"))?;
        let table = read
            .open_table(NATIVE_BLOCK_EXECUTION)
            .map_err(|e| format!("failed to open native block execution table: {e}"))?;

        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read native block execution: {e}"))?
        {
            Some(value) => NativeBlockExecutionResultV1::from_canonical_bytes(value.value())
                .map(Some)
                .map_err(|e| format!("invalid persisted native execution: {e}")),
            None => Ok(None),
        }
    }

    pub fn native_block_body(&self, block_id: Hash32) -> Result<Option<NativeBlockBodyV1>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin native block body read: {e}"))?;
        let table = read
            .open_table(NATIVE_BLOCK_BODIES)
            .map_err(|e| format!("failed to open native block body table: {e}"))?;

        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read native block body: {e}"))?
        {
            Some(value) => NativeBlockBodyV1::from_canonical_bytes(value.value())
                .map(Some)
                .map_err(|e| format!("invalid persisted native block body: {e}")),
            None => Ok(None),
        }
    }

    pub fn store_native_block_body(
        &self,
        block_id: Hash32,
        body: &NativeBlockBodyV1,
    ) -> Result<(), String> {
        if self.load_chain_block(block_id)?.is_none() {
            return Err("cannot persist native block body for an unpersisted NIAHCIA block".into());
        }

        let encoded = body.canonical_bytes()?;

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin native block body write: {e}"))?;
        {
            let mut table = write
                .open_table(NATIVE_BLOCK_BODIES)
                .map_err(|e| format!("failed to open native block body table: {e}"))?;

            if let Some(existing) = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native block body: {e}"))?
            {
                if existing.value() != encoded.as_slice() {
                    return Err("NIAHCIA block already has a different native block body".into());
                }
                return Ok(());
            }

            table
                .insert(block_id.as_slice(), encoded.as_slice())
                .map_err(|e| format!("failed to persist native block body: {e}"))?;
        }

        write
            .commit()
            .map_err(|e| format!("failed to commit native block body: {e}"))
    }

    pub fn inactive_native_block_body_v2(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeBlockBodyV2>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin inactive V2 body read: {e}"))?;
        let table = read
            .open_table(INACTIVE_NATIVE_BLOCK_BODIES_V2)
            .map_err(|e| format!("failed to open inactive V2 body table: {e}"))?;

        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read inactive V2 body: {e}"))?
        {
            Some(value) => NativeBlockBodyV2::from_canonical_bytes(value.value())
                .map(Some)
                .map_err(|e| format!("invalid persisted inactive V2 body: {e}")),
            None => Ok(None),
        }
    }

    pub fn inactive_native_block_execution_v2(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeBlockExecutionResultV2>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin inactive V2 execution read: {e}"))?;
        let table = read
            .open_table(INACTIVE_NATIVE_BLOCK_EXECUTION_V2)
            .map_err(|e| format!("failed to open inactive V2 execution table: {e}"))?;

        match table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read inactive V2 execution: {e}"))?
        {
            Some(value) => NativeBlockExecutionResultV2::from_canonical_bytes(value.value())
                .map(Some)
                .map_err(|e| format!("invalid persisted inactive V2 execution: {e}")),
            None => Ok(None),
        }
    }

    pub fn inactive_native_state_v2_snapshot(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeStateV2>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin inactive V2 state read: {e}"))?;
        let table = read
            .open_table(INACTIVE_NATIVE_STATE_SNAPSHOTS_V2)
            .map_err(|e| format!("failed to open inactive V2 state table: {e}"))?;

        let Some(value) = table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read inactive V2 state: {e}"))?
        else {
            return Ok(None);
        };
        let value = value.value();
        if value.len() < 32 {
            return Err("persisted inactive V2 state is truncated".into());
        }
        let expected_root: Hash32 = value[..32].try_into().unwrap();
        let state = NativeStateV2::from_canonical_bytes(&value[32..])?;
        if state.state_root()? != expected_root {
            return Err("persisted inactive V2 state root mismatch".into());
        }
        Ok(Some(state))
    }

    pub fn store_inactive_native_v2_bundle(
        &self,
        block_id: Hash32,
        body: &NativeBlockBodyV2,
        execution: &NativeBlockExecutionResultV2,
        state: &NativeStateV2,
    ) -> Result<(), String> {
        let block = self.load_chain_block(block_id)?.ok_or_else(|| {
            "cannot persist inactive V2 bundle for an unpersisted block".to_string()
        })?;

        let transactions_root = body.transactions_root();
        if transactions_root != block.header.transactions_root
            || execution.transactions_root != block.header.transactions_root
        {
            return Err("inactive V2 bundle transactions root does not match header".into());
        }
        if execution.execution_root != block.header.execution_root {
            return Err("inactive V2 bundle execution root does not match header".into());
        }
        if state.state_root()? != execution.state_root {
            return Err("inactive V2 state root does not match execution result".into());
        }
        body.validate_fee_recipient_canonicality(execution.producer_priority_fee)?;

        let encoded_body = body.canonical_bytes()?;
        let encoded_execution = execution.canonical_bytes()?;
        let snapshot = state.canonical_bytes()?;
        let mut encoded_state = Vec::with_capacity(32 + snapshot.len());
        encoded_state.extend_from_slice(&execution.state_root);
        encoded_state.extend_from_slice(&snapshot);

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin inactive V2 bundle write: {e}"))?;

        {
            let mut table = write
                .open_table(INACTIVE_NATIVE_BLOCK_BODIES_V2)
                .map_err(|e| format!("failed to open inactive V2 body table: {e}"))?;
            let existing = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 body: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_body.as_slice() {
                    return Err("block already has a different inactive V2 body".into());
                }
            } else {
                table
                    .insert(block_id.as_slice(), encoded_body.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 body: {e}"))?;
            }
        }

        {
            let mut table = write
                .open_table(INACTIVE_NATIVE_BLOCK_EXECUTION_V2)
                .map_err(|e| format!("failed to open inactive V2 execution table: {e}"))?;
            let existing = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 execution: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_execution.as_slice() {
                    return Err("block already has a different inactive V2 execution".into());
                }
            } else {
                table
                    .insert(block_id.as_slice(), encoded_execution.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 execution: {e}"))?;
            }
        }

        {
            let mut table = write
                .open_table(INACTIVE_NATIVE_STATE_SNAPSHOTS_V2)
                .map_err(|e| format!("failed to open inactive V2 state table: {e}"))?;
            let existing = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 state: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_state.as_slice() {
                    return Err("block already has a different inactive V2 state".into());
                }
            } else {
                table
                    .insert(block_id.as_slice(), encoded_state.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 state: {e}"))?;
            }
        }

        write
            .commit()
            .map_err(|e| format!("failed to commit inactive V2 bundle: {e}"))
    }

    pub fn native_state_snapshot_version(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeStateSnapshotVersion>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin native state snapshot version read: {e}"))?;
        let table = read
            .open_table(NATIVE_STATE_SNAPSHOTS)
            .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

        let Some(value) = table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read native state snapshot version: {e}"))?
        else {
            return Ok(None);
        };

        let value = value.value();
        if value.len() < 33 {
            return Err("persisted native state snapshot is truncated".into());
        }

        NativeStateSnapshotVersion::try_from(value[32]).map(Some)
    }

    pub fn store_native_state_snapshot(
        &self,
        block_id: Hash32,
        state: &NativeStateV1,
    ) -> Result<(), String> {
        if self.load_chain_block(block_id)?.is_none() {
            return Err("cannot persist native state for an unpersisted NIAHCIA block".into());
        }

        let state_root = state.state_root();
        let snapshot = state.canonical_bytes()?;
        let mut value = Vec::with_capacity(32 + snapshot.len());
        value.extend_from_slice(&state_root);
        value.extend_from_slice(&snapshot);

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin native state snapshot write: {e}"))?;
        {
            let mut table = write
                .open_table(NATIVE_STATE_SNAPSHOTS)
                .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

            if let Some(existing) = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native state snapshot: {e}"))?
            {
                if existing.value() != value.as_slice() {
                    return Err(
                        "NIAHCIA block already has a different native state snapshot".into(),
                    );
                }
                return Ok(());
            }

            table
                .insert(block_id.as_slice(), value.as_slice())
                .map_err(|e| format!("failed to persist native state snapshot: {e}"))?;
        }

        write
            .commit()
            .map_err(|e| format!("failed to commit native state snapshot: {e}"))
    }

    pub fn store_native_state_v2_snapshot(
        &self,
        block_id: Hash32,
        state: &NativeStateV2,
    ) -> Result<(), String> {
        if self.load_chain_block(block_id)?.is_none() {
            return Err("cannot persist native state V2 for an unpersisted NIAHCIA block".into());
        }

        let state_root = state.state_root()?;
        let snapshot = state.canonical_bytes()?;
        let mut value = Vec::with_capacity(32 + snapshot.len());
        value.extend_from_slice(&state_root);
        value.extend_from_slice(&snapshot);

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin native state V2 snapshot write: {e}"))?;
        {
            let mut table = write
                .open_table(NATIVE_STATE_SNAPSHOTS)
                .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

            if let Some(existing) = table
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native state V2 snapshot: {e}"))?
            {
                if existing.value() != value.as_slice() {
                    return Err(
                        "NIAHCIA block already has a different native state snapshot".into(),
                    );
                }
                return Ok(());
            }

            table
                .insert(block_id.as_slice(), value.as_slice())
                .map_err(|e| format!("failed to persist native state V2 snapshot: {e}"))?;
        }

        write
            .commit()
            .map_err(|e| format!("failed to commit native state V2 snapshot: {e}"))
    }

    pub fn native_state_v2_snapshot(
        &self,
        block_id: Hash32,
    ) -> Result<Option<NativeStateV2>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin native state V2 snapshot read: {e}"))?;
        let table = read
            .open_table(NATIVE_STATE_SNAPSHOTS)
            .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

        let Some(value) = table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read native state V2 snapshot: {e}"))?
        else {
            return Ok(None);
        };

        let value = value.value();
        if value.len() < 32 {
            return Err("persisted native state V2 snapshot is truncated".into());
        }

        let expected_root: Hash32 = value[..32]
            .try_into()
            .map_err(|_| "invalid persisted native state V2 root length".to_string())?;
        let state = NativeStateV2::from_canonical_bytes(&value[32..])?;
        let actual_root = state.state_root()?;

        if actual_root != expected_root {
            return Err("persisted native state V2 snapshot root mismatch".into());
        }

        Ok(Some(state))
    }

    pub fn native_state_snapshot(&self, block_id: Hash32) -> Result<Option<NativeStateV1>, String> {
        let read = self
            .db
            .begin_read()
            .map_err(|e| format!("failed to begin native state snapshot read: {e}"))?;
        let table = read
            .open_table(NATIVE_STATE_SNAPSHOTS)
            .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

        let Some(value) = table
            .get(block_id.as_slice())
            .map_err(|e| format!("failed to read native state snapshot: {e}"))?
        else {
            return Ok(None);
        };

        let value = value.value();
        if value.len() < 32 {
            return Err("persisted native state snapshot is truncated".into());
        }

        let expected_root: Hash32 = value[..32]
            .try_into()
            .map_err(|_| "invalid persisted native state root length".to_string())?;
        let state = NativeStateV1::from_canonical_bytes(&value[32..])?;
        let actual_root = state.state_root();

        if actual_root != expected_root {
            return Err("persisted native state snapshot root mismatch".into());
        }

        Ok(Some(state))
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

    pub fn canonical_chain(&self) -> Result<Vec<PersistedChainBlock>, String> {
        let Some(mut cursor) = self.best_chain_head()? else {
            return Ok(Vec::new());
        };
        let mut reverse = Vec::with_capacity(cursor.header.height.saturating_add(1) as usize);
        loop {
            reverse.push(cursor.clone());
            if cursor.header.height == 0 {
                break;
            }
            cursor = self
                .load_chain_block(cursor.header.parent_hash)?
                .ok_or_else(|| "best-chain ancestry references missing parent".to_string())?;
        }
        reverse.reverse();
        Ok(reverse)
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

    pub fn insert_native_block_with_execution_outcome(
        &self,
        header: BlockHeaderV1,
        execution: &NativeBlockExecutionResultV1,
        state: &NativeStateV1,
    ) -> Result<ChainInsertOutcome, String> {
        self.insert_native_block_internal(header, None, execution, state)
    }

    pub fn insert_native_block_with_body_and_execution_outcome(
        &self,
        header: BlockHeaderV1,
        body: &NativeBlockBodyV1,
        execution: &NativeBlockExecutionResultV1,
        state: &NativeStateV1,
    ) -> Result<ChainInsertOutcome, String> {
        let transactions = body.decoded_transactions()?;
        let body_transactions_root = native_transactions_root_v1(&transactions)?;
        if body_transactions_root != header.transactions_root {
            return Err("native block body transactions root does not match header".into());
        }
        body.validate_fee_recipient_canonicality(execution.producer_priority_fee)?;

        self.insert_native_block_internal(header, Some(body), execution, state)
    }

    fn insert_native_block_internal(
        &self,
        header: BlockHeaderV1,
        body: Option<&NativeBlockBodyV1>,
        execution: &NativeBlockExecutionResultV1,
        state: &NativeStateV1,
    ) -> Result<ChainInsertOutcome, String> {
        if header.transactions_root != execution.transactions_root {
            return Err("native block transactions root does not match execution result".into());
        }
        if header.execution_root != execution.execution_root {
            return Err("native block execution root does not match execution result".into());
        }
        if state.state_root() != execution.state_root {
            return Err("native state root does not match execution result".into());
        }

        let previous_best = self.best_chain_head()?.map(|block| block.block_id());

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
        let encoded_block = record.encode_value()?;

        let encoded_execution = execution.canonical_bytes()?;
        let encoded_body = body.map(NativeBlockBodyV1::canonical_bytes).transpose()?;

        let snapshot = state.canonical_bytes()?;
        let mut encoded_state = Vec::with_capacity(32 + snapshot.len());
        encoded_state.extend_from_slice(&execution.state_root);
        encoded_state.extend_from_slice(&snapshot);

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin native block transaction: {e}"))?;

        {
            let mut blocks = write
                .open_table(CHAIN_BLOCKS)
                .map_err(|e| format!("failed to open chain block table: {e}"))?;

            let existing = blocks
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to check existing chain block: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                let existing = PersistedChainBlock::decode(&existing)?;
                if existing != record {
                    return Err("block ID collision with different persisted record".into());
                }
            } else {
                blocks
                    .insert(block_id.as_slice(), encoded_block.as_slice())
                    .map_err(|e| format!("failed to persist chain block: {e}"))?;
            }
        }

        {
            let mut executions = write
                .open_table(NATIVE_BLOCK_EXECUTION)
                .map_err(|e| format!("failed to open native block execution table: {e}"))?;

            let existing = executions
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native block execution: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_execution.as_slice() {
                    return Err(
                        "NIAHCIA block already has a different native execution result".into(),
                    );
                }
            } else {
                executions
                    .insert(block_id.as_slice(), encoded_execution.as_slice())
                    .map_err(|e| format!("failed to persist native block execution: {e}"))?;
            }
        }

        if let Some(encoded_body) = encoded_body.as_ref() {
            let mut bodies = write
                .open_table(NATIVE_BLOCK_BODIES)
                .map_err(|e| format!("failed to open native block body table: {e}"))?;

            let existing = bodies
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native block body: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_body.as_slice() {
                    return Err("NIAHCIA block already has a different native block body".into());
                }
            } else {
                bodies
                    .insert(block_id.as_slice(), encoded_body.as_slice())
                    .map_err(|e| format!("failed to persist native block body: {e}"))?;
            }
        }

        {
            let mut snapshots = write
                .open_table(NATIVE_STATE_SNAPSHOTS)
                .map_err(|e| format!("failed to open native state snapshot table: {e}"))?;

            let existing = snapshots
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect native state snapshot: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_state.as_slice() {
                    return Err(
                        "NIAHCIA block already has a different native state snapshot".into(),
                    );
                }
            } else {
                snapshots
                    .insert(block_id.as_slice(), encoded_state.as_slice())
                    .map_err(|e| format!("failed to persist native state snapshot: {e}"))?;
            }
        }

        let current_best_id: Option<Hash32> = {
            let meta = write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to open chain metadata table: {e}"))?;
            let current = meta
                .get(BEST_HEAD_KEY)
                .map_err(|e| format!("failed to read current best head: {e}"))?
                .map(|best_id| {
                    best_id
                        .value()
                        .try_into()
                        .map_err(|_| "invalid persisted best-head ID length".to_string())
                })
                .transpose()?;
            current
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
            .map_err(|e| format!("failed to commit native block transaction: {e}"))?;

        let current_best = self
            .best_chain_head()?
            .ok_or_else(|| "native block transaction committed without a best head".to_string())?
            .block_id();

        let reorg = match previous_best {
            Some(old_head) if old_head != current_best => {
                self.canonical_reorg(old_head, current_best)?
            }
            _ => None,
        };

        Ok(ChainInsertOutcome {
            block: record,
            previous_best,
            current_best,
            reorg,
        })
    }

    pub fn insert_inactive_native_v2_block_with_body_and_execution_outcome(
        &self,
        header: BlockHeaderV1,
        body: &NativeBlockBodyV2,
        execution: &NativeBlockExecutionResultV2,
        state: &NativeStateV2,
    ) -> Result<ChainInsertOutcome, String> {
        let transactions_root = body.transactions_root();
        if transactions_root != header.transactions_root
            || execution.transactions_root != header.transactions_root
        {
            return Err("inactive V2 block transactions root does not match header".into());
        }
        if execution.execution_root != header.execution_root {
            return Err("inactive V2 block execution root does not match header".into());
        }
        if state.state_root()? != execution.state_root {
            return Err("inactive V2 state root does not match execution result".into());
        }
        body.validate_fee_recipient_canonicality(execution.producer_priority_fee)?;

        let previous_best = self.best_chain_head()?.map(|block| block.block_id());

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
        let encoded_block = record.encode_value()?;
        let encoded_body = body.canonical_bytes()?;
        let encoded_execution = execution.canonical_bytes()?;
        let snapshot = state.canonical_bytes()?;
        let mut encoded_state = Vec::with_capacity(32 + snapshot.len());
        encoded_state.extend_from_slice(&execution.state_root);
        encoded_state.extend_from_slice(&snapshot);

        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin inactive V2 block transaction: {e}"))?;

        {
            let mut blocks = write
                .open_table(CHAIN_BLOCKS)
                .map_err(|e| format!("failed to open chain block table: {e}"))?;
            let existing = blocks
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to check existing chain block: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                let existing = PersistedChainBlock::decode(&existing)?;
                if existing != record {
                    return Err("block ID collision with different persisted record".into());
                }
            } else {
                blocks
                    .insert(block_id.as_slice(), encoded_block.as_slice())
                    .map_err(|e| format!("failed to persist chain block: {e}"))?;
            }
        }

        {
            let mut bodies = write
                .open_table(INACTIVE_NATIVE_BLOCK_BODIES_V2)
                .map_err(|e| format!("failed to open inactive V2 block body table: {e}"))?;
            let existing = bodies
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 block body: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_body.as_slice() {
                    return Err("block already has a different inactive V2 block body".into());
                }
            } else {
                bodies
                    .insert(block_id.as_slice(), encoded_body.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 block body: {e}"))?;
            }
        }

        {
            let mut executions = write
                .open_table(INACTIVE_NATIVE_BLOCK_EXECUTION_V2)
                .map_err(|e| format!("failed to open inactive V2 execution table: {e}"))?;
            let existing = executions
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 execution: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_execution.as_slice() {
                    return Err("block already has a different inactive V2 execution".into());
                }
            } else {
                executions
                    .insert(block_id.as_slice(), encoded_execution.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 execution: {e}"))?;
            }
        }

        {
            let mut snapshots = write
                .open_table(INACTIVE_NATIVE_STATE_SNAPSHOTS_V2)
                .map_err(|e| format!("failed to open inactive V2 state table: {e}"))?;
            let existing = snapshots
                .get(block_id.as_slice())
                .map_err(|e| format!("failed to inspect inactive V2 state: {e}"))?
                .map(|value| value.value().to_vec());

            if let Some(existing) = existing {
                if existing.as_slice() != encoded_state.as_slice() {
                    return Err("block already has a different inactive V2 state".into());
                }
            } else {
                snapshots
                    .insert(block_id.as_slice(), encoded_state.as_slice())
                    .map_err(|e| format!("failed to persist inactive V2 state: {e}"))?;
            }
        }

        let current_best_id: Option<Hash32> = {
            let meta = write
                .open_table(CHAIN_META)
                .map_err(|e| format!("failed to open chain metadata table: {e}"))?;
            let current = meta
                .get(BEST_HEAD_KEY)
                .map_err(|e| format!("failed to read current best head: {e}"))?
                .map(|best_id| {
                    best_id
                        .value()
                        .try_into()
                        .map_err(|_| "invalid persisted best-head ID length".to_string())
                })
                .transpose()?;
            current
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
            .map_err(|e| format!("failed to commit inactive V2 block transaction: {e}"))?;

        let current_best = self
            .best_chain_head()?
            .ok_or_else(|| {
                "inactive V2 block transaction committed without a best head".to_string()
            })?
            .block_id();

        let reorg = match previous_best {
            Some(old_head) if old_head != current_best => {
                self.canonical_reorg(old_head, current_best)?
            }
            _ => None,
        };

        Ok(ChainInsertOutcome {
            block: record,
            previous_best,
            current_best,
            reorg,
        })
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

    fn sign_v2_for_state_test(
        key: &k256::ecdsa::SigningKey,
        body: crate::native_transaction_v2::NativeTransactionBodyV2,
    ) -> crate::native_transaction_v2::SignedNativeTransactionV2 {
        use k256::ecdsa::{signature::hazmat::PrehashSigner, Signature};

        let mut transaction = crate::native_transaction_v2::SignedNativeTransactionV2 {
            body,
            public_key: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            signature: vec![0; 64],
        };
        let digest = transaction.signing_digest().unwrap();
        let signature: Signature = key.sign_prehash(&digest).unwrap();
        transaction.signature = signature.to_bytes().to_vec();
        transaction
    }

    fn funded_native_state_v2(
        key: &k256::ecdsa::SigningKey,
        balance: u128,
    ) -> (crate::native_state_v2::NativeStateV2, [u8; 20]) {
        use crate::address::AddressNetwork;
        use crate::native_execution::{AccountStateV1, NativeStateV1};
        use crate::native_transaction::{
            DEVNET_CHAIN_ID, DEVNET_NETWORK_ID, NATIVE_TRANSFER_GAS_V1,
        };
        use crate::native_transaction_v2::{NativeActionV2, NativeTransactionBodyV2};

        let probe = sign_v2_for_state_test(
            key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0x01; 20],
                value: 1,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );
        let funding = probe
            .authenticated_sender(AddressNetwork::Devnet)
            .unwrap()
            .payload;

        let mut accounts = NativeStateV1::default();
        accounts.set_account(funding, AccountStateV1 { balance, nonce: 0 });

        (
            crate::native_state_v2::NativeStateV2::from_v1(accounts),
            funding,
        )
    }

    #[test]
    fn inactive_v2_atomic_insert_persists_header_body_execution_and_state_together() {
        use crate::address::AddressNetwork;
        use crate::native_block_body_v2::NativeBlockBodyV2;
        use crate::native_block_execution_v2::execute_inactive_versioned_block_v2;
        use crate::native_execution_commitment_v2::build_inactive_execution_result_v2;
        use crate::native_state_v2::NativeStateV2;

        let path = temp_state_path("inactive-v2-atomic-insert");
        let store = StateStore::open(&path).unwrap();

        let body = NativeBlockBodyV2::empty();
        let mut state = NativeStateV2::default();
        let transition =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 0, 0)
                .unwrap();
        let execution = build_inactive_execution_result_v2(&body, &transition).unwrap();

        let mut header = header([0_u8; 32], 0, [0xff; 32], 0x51);
        header.transactions_root = execution.transactions_root;
        header.execution_root = execution.execution_root;
        let block_id = header.block_id();

        let outcome = store
            .insert_inactive_native_v2_block_with_body_and_execution_outcome(
                header, &body, &execution, &state,
            )
            .unwrap();

        assert_eq!(outcome.block.block_id(), block_id);
        assert_eq!(outcome.current_best, block_id);
        assert_eq!(
            store.inactive_native_block_body_v2(block_id).unwrap(),
            Some(body)
        );
        assert_eq!(
            store.inactive_native_block_execution_v2(block_id).unwrap(),
            Some(execution)
        );
        assert_eq!(
            store.inactive_native_state_v2_snapshot(block_id).unwrap(),
            Some(state)
        );
        assert!(store.native_block_body(block_id).unwrap().is_none());
        assert!(store.native_block_execution(block_id).unwrap().is_none());
        assert!(store.native_state_snapshot(block_id).unwrap().is_none());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn inactive_v2_atomic_insert_rejects_mismatch_without_persisting_header() {
        use crate::address::AddressNetwork;
        use crate::native_block_body_v2::NativeBlockBodyV2;
        use crate::native_block_execution_v2::execute_inactive_versioned_block_v2;
        use crate::native_execution_commitment_v2::build_inactive_execution_result_v2;
        use crate::native_state_v2::NativeStateV2;

        let path = temp_state_path("inactive-v2-atomic-reject");
        let store = StateStore::open(&path).unwrap();

        let body = NativeBlockBodyV2::empty();
        let mut state = NativeStateV2::default();
        let transition =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 0, 0)
                .unwrap();
        let execution = build_inactive_execution_result_v2(&body, &transition).unwrap();

        let mut header = header([0_u8; 32], 0, [0xff; 32], 0x52);
        header.transactions_root = execution.transactions_root;
        header.execution_root = execution.execution_root;
        header.execution_root[0] ^= 0x01;
        let block_id = header.block_id();

        let error = store
            .insert_inactive_native_v2_block_with_body_and_execution_outcome(
                header, &body, &execution, &state,
            )
            .unwrap_err();

        assert!(error.contains("execution root"));
        assert!(store.load_chain_block(block_id).unwrap().is_none());
        assert!(store
            .inactive_native_block_body_v2(block_id)
            .unwrap()
            .is_none());
        assert!(store
            .inactive_native_block_execution_v2(block_id)
            .unwrap()
            .is_none());
        assert!(store
            .inactive_native_state_v2_snapshot(block_id)
            .unwrap()
            .is_none());
        assert!(store.best_chain_head().unwrap().is_none());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn inactive_v2_mixed_block_restart_and_reorg_restore_winning_branch_state() {
        use crate::address::AddressNetwork;
        use crate::native_block_body_v2::{NativeBlockBodyV2, VersionedSignedNativeTransaction};
        use crate::native_block_execution_v2::execute_inactive_versioned_block_v2;
        use crate::native_compute_fee_v1::NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1;
        use crate::native_compute_payloads::ComputeChannelOpenPayloadV1;
        use crate::native_execution_commitment_v2::build_inactive_execution_result_v2;
        use crate::native_state_v2::ComputeChannelSettlementPolicyV1;
        use crate::native_transaction::{
            DEVNET_CHAIN_ID, DEVNET_NETWORK_ID, NATIVE_TRANSFER_GAS_V1,
        };
        use crate::native_transaction_v2::{NativeActionV2, NativeTransactionBodyV2};

        let path = temp_state_path("inactive-v2-mixed-reorg");
        let funding_key = k256::ecdsa::SigningKey::from_slice(&[0x61; 32]).unwrap();
        let channel_key = k256::ecdsa::SigningKey::from_slice(&[0x62; 32]).unwrap();
        let (genesis_state, funding) = funded_native_state_v2(&funding_key, 250_000);

        let store = StateStore::open(&path).unwrap();

        let genesis_body = NativeBlockBodyV2::empty();
        let mut committed_genesis_state = genesis_state.clone();
        let genesis_transition = execute_inactive_versioned_block_v2(
            &mut committed_genesis_state,
            &genesis_body,
            AddressNetwork::Devnet,
            0,
            0,
        )
        .unwrap();
        let genesis_execution =
            build_inactive_execution_result_v2(&genesis_body, &genesis_transition).unwrap();

        let mut genesis_header = header([0_u8; 32], 0, [0xff; 32], 0x60);
        genesis_header.transactions_root = genesis_execution.transactions_root;
        genesis_header.execution_root = genesis_execution.execution_root;
        let genesis_record = store.insert_chain_block(genesis_header).unwrap();
        let genesis_id = genesis_record.block_id();
        store
            .store_inactive_native_v2_bundle(
                genesis_id,
                &genesis_body,
                &genesis_execution,
                &committed_genesis_state,
            )
            .unwrap();

        let transfer_a = sign_v2_for_state_test(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0xa1; 20],
                value: 100,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 2,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );

        let channel_public_key: [u8; 65] = channel_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .try_into()
            .unwrap();
        let open_payload = ComputeChannelOpenPayloadV1 {
            worker_id: [0xb1; 32],
            operator_id: [0xb2; 32],
            channel_public_key,
            worker_payment_account: [0xb3; 20],
            authorized_amount: 1_000,
            expiry_height: 20,
            claim_deadline_height: 25,
            refund_available_height: 26,
            service_scope_commitment: [0xb4; 32],
            model_scope_commitment: [0xb5; 32],
            execution_profile_scope_commitment: [0xb6; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        };
        let open_a = sign_v2_for_state_test(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 1,
                action: NativeActionV2::ComputeChannelOpen,
                target_payload: Vec::new(),
                value: 1_000,
                gas_limit: NATIVE_COMPUTE_CHANNEL_OPEN_GAS_V1,
                max_fee_per_gas: 1,
                max_priority_fee_per_gas: 0,
                data: open_payload.canonical_bytes().unwrap(),
            },
        );

        let body_a = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[
                VersionedSignedNativeTransaction::V2(transfer_a),
                VersionedSignedNativeTransaction::V2(open_a),
            ],
        )
        .unwrap();
        let mut state_a = genesis_state.clone();
        let transition_a = execute_inactive_versioned_block_v2(
            &mut state_a,
            &body_a,
            AddressNetwork::Devnet,
            1,
            1,
        )
        .unwrap();
        let execution_a = build_inactive_execution_result_v2(&body_a, &transition_a).unwrap();

        let mut header_a = header(genesis_id, 1, [0xff; 32], 0x61);
        header_a.transactions_root = execution_a.transactions_root;
        header_a.execution_root = execution_a.execution_root;
        let a_record = store.insert_chain_block(header_a).unwrap();
        let a_id = a_record.block_id();
        store
            .store_inactive_native_v2_bundle(a_id, &body_a, &execution_a, &state_a)
            .unwrap();

        assert_eq!(store.best_chain_head().unwrap().unwrap().block_id(), a_id);
        assert_eq!(state_a.accounts().account(funding).nonce, 2);
        assert_eq!(state_a.channel_count(), 1);

        let transfer_b = sign_v2_for_state_test(
            &funding_key,
            NativeTransactionBodyV2 {
                network_id: DEVNET_NETWORK_ID,
                chain_id: DEVNET_CHAIN_ID,
                nonce: 0,
                action: NativeActionV2::Transfer,
                target_payload: vec![0xc1; 20],
                value: 250,
                gas_limit: NATIVE_TRANSFER_GAS_V1,
                max_fee_per_gas: 2,
                max_priority_fee_per_gas: 0,
                data: Vec::new(),
            },
        );
        let body_b = NativeBlockBodyV2::from_versioned_transactions(
            [0_u8; 20],
            &[VersionedSignedNativeTransaction::V2(transfer_b)],
        )
        .unwrap();
        let mut state_b = genesis_state.clone();
        let transition_b = execute_inactive_versioned_block_v2(
            &mut state_b,
            &body_b,
            AddressNetwork::Devnet,
            1,
            1,
        )
        .unwrap();
        let execution_b = build_inactive_execution_result_v2(&body_b, &transition_b).unwrap();

        let mut header_b = header(genesis_id, 1, [0x7f; 32], 0x62);
        header_b.transactions_root = execution_b.transactions_root;
        header_b.execution_root = execution_b.execution_root;
        let outcome_b = store.insert_chain_block_with_outcome(header_b).unwrap();
        let b_id = outcome_b.block.block_id();
        store
            .store_inactive_native_v2_bundle(b_id, &body_b, &execution_b, &state_b)
            .unwrap();

        let reorg = outcome_b
            .reorg
            .expect("harder V2 branch must become canonical");
        assert_eq!(reorg.old_head, a_id);
        assert_eq!(reorg.new_head, b_id);
        assert_eq!(reorg.common_ancestor, genesis_id);
        assert_eq!(reorg.detached, vec![a_id]);
        assert_eq!(reorg.attached, vec![b_id]);

        assert_eq!(state_b.accounts().account(funding).nonce, 1);
        assert_eq!(state_b.channel_count(), 0);
        assert_ne!(state_a.state_root().unwrap(), state_b.state_root().unwrap());

        let canonical_height_one = store
            .canonical_block_at_height(1)
            .unwrap()
            .unwrap()
            .block_id();
        assert_eq!(canonical_height_one, b_id);
        assert_eq!(
            store
                .inactive_native_state_v2_snapshot(canonical_height_one)
                .unwrap(),
            Some(state_b.clone())
        );
        assert_eq!(
            store.inactive_native_state_v2_snapshot(a_id).unwrap(),
            Some(state_a.clone())
        );

        drop(store);
        let reopened = StateStore::open(&path).unwrap();
        assert_eq!(
            reopened.best_chain_head().unwrap().unwrap().block_id(),
            b_id
        );
        let reopened_canonical = reopened
            .canonical_block_at_height(1)
            .unwrap()
            .unwrap()
            .block_id();
        assert_eq!(reopened_canonical, b_id);
        assert_eq!(
            reopened.inactive_native_block_body_v2(b_id).unwrap(),
            Some(body_b)
        );
        assert_eq!(
            reopened.inactive_native_block_execution_v2(b_id).unwrap(),
            Some(execution_b)
        );
        assert_eq!(
            reopened
                .inactive_native_state_v2_snapshot(reopened_canonical)
                .unwrap(),
            Some(state_b)
        );
        assert_eq!(
            reopened.inactive_native_state_v2_snapshot(a_id).unwrap(),
            Some(state_a)
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn inactive_v2_bundle_round_trips_without_touching_active_v1_tables() {
        use crate::address::AddressNetwork;
        use crate::native_block_body_v2::NativeBlockBodyV2;
        use crate::native_block_execution_v2::execute_inactive_versioned_block_v2;
        use crate::native_execution_commitment_v2::build_inactive_execution_result_v2;
        use crate::native_state_v2::NativeStateV2;

        let path = temp_state_path("inactive-v2-bundle");
        let store = StateStore::open(&path).unwrap();

        let body = NativeBlockBodyV2::empty();
        let mut state = NativeStateV2::default();
        let transition =
            execute_inactive_versioned_block_v2(&mut state, &body, AddressNetwork::Devnet, 0, 0)
                .unwrap();
        let execution = build_inactive_execution_result_v2(&body, &transition).unwrap();

        let mut block = header([0_u8; 32], 0, [0xff; 32], 0x31);
        block.transactions_root = execution.transactions_root;
        block.execution_root = execution.execution_root;
        let persisted = store.insert_chain_block(block).unwrap();
        let block_id = persisted.block_id();

        store
            .store_inactive_native_v2_bundle(block_id, &body, &execution, &state)
            .unwrap();

        assert_eq!(
            store.inactive_native_block_body_v2(block_id).unwrap(),
            Some(body.clone())
        );
        assert_eq!(
            store.inactive_native_block_execution_v2(block_id).unwrap(),
            Some(execution.clone())
        );
        assert_eq!(
            store.inactive_native_state_v2_snapshot(block_id).unwrap(),
            Some(state.clone())
        );
        assert_eq!(store.native_block_body(block_id).unwrap(), None);
        assert_eq!(store.native_block_execution(block_id).unwrap(), None);
        assert_eq!(store.native_state_snapshot(block_id).unwrap(), None);

        drop(store);
        let reopened = StateStore::open(&path).unwrap();
        assert_eq!(
            reopened.inactive_native_block_body_v2(block_id).unwrap(),
            Some(body)
        );
        assert_eq!(
            reopened
                .inactive_native_block_execution_v2(block_id)
                .unwrap(),
            Some(execution)
        );
        assert_eq!(
            reopened
                .inactive_native_state_v2_snapshot(block_id)
                .unwrap(),
            Some(state)
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn native_v3_block_commit_atomically_persists_body_execution_and_state() {
        use crate::address::AddressNetwork;
        use crate::native_block_body::NativeBlockBodyV1;
        use crate::native_execution::{execute_block_v1, NativeExecutionContextV1, NativeStateV1};

        let path = temp_state_path("native-v3-block-atomic");
        let mut state = NativeStateV1::default();
        let execution = execute_block_v1(
            &mut state,
            &[],
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 0,
                cpu_producer: [0_u8; 20],
            },
        )
        .unwrap();
        let body = NativeBlockBodyV1::empty();

        let mut block = header([0_u8; 32], 0, [0xff; 32], 1);
        block.transactions_root = execution.transactions_root;
        block.execution_root = execution.execution_root;
        let block_id = block.block_id();

        {
            let store = StateStore::open(&path).unwrap();
            let outcome = store
                .insert_native_block_with_body_and_execution_outcome(
                    block.clone(),
                    &body,
                    &execution,
                    &state,
                )
                .unwrap();

            assert_eq!(outcome.block.block_id(), block_id);
            assert_eq!(
                store.native_block_body(block_id).unwrap(),
                Some(body.clone())
            );
            assert_eq!(
                store.native_block_execution(block_id).unwrap(),
                Some(execution.clone())
            );
            assert_eq!(
                store.native_state_snapshot(block_id).unwrap(),
                Some(state.clone())
            );
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert_eq!(reopened.native_block_body(block_id).unwrap(), Some(body));
            assert_eq!(
                reopened.native_block_execution(block_id).unwrap(),
                Some(execution)
            );
            assert_eq!(
                reopened.native_state_snapshot(block_id).unwrap(),
                Some(state)
            );
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_v3_block_commit_rejects_body_root_mismatch_without_persisting() {
        use crate::address::AddressNetwork;
        use crate::native_block_body::NativeBlockBodyV1;
        use crate::native_execution::{execute_block_v1, NativeExecutionContextV1, NativeStateV1};

        let path = temp_state_path("native-v3-body-root-mismatch");
        let store = StateStore::open(&path).unwrap();

        let mut state = NativeStateV1::default();
        let execution = execute_block_v1(
            &mut state,
            &[],
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 0,
                cpu_producer: [0_u8; 20],
            },
        )
        .unwrap();

        let body = NativeBlockBodyV1::empty();
        let mut block = header([0_u8; 32], 0, [0xff; 32], 2);
        block.transactions_root = [0x55; 32];
        block.execution_root = execution.execution_root;
        let block_id = block.block_id();

        let err = store
            .insert_native_block_with_body_and_execution_outcome(block, &body, &execution, &state)
            .unwrap_err();
        assert!(err.contains("block body transactions root"));

        assert!(store.load_chain_block(block_id).unwrap().is_none());
        assert!(store.native_block_body(block_id).unwrap().is_none());
        assert!(store.native_block_execution(block_id).unwrap().is_none());
        assert!(store.native_state_snapshot(block_id).unwrap().is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_block_commit_atomically_persists_execution_and_state() {
        use crate::address::AddressNetwork;
        use crate::native_execution::{execute_block_v1, NativeExecutionContextV1, NativeStateV1};

        let path = temp_state_path("native-block-atomic");
        let mut state = NativeStateV1::default();
        let execution = execute_block_v1(
            &mut state,
            &[],
            AddressNetwork::Devnet,
            NativeExecutionContextV1 {
                base_fee_per_gas: 3,
                cpu_producer: [0x09; 20],
            },
        )
        .unwrap();

        let mut block = header([0_u8; 32], 0, [0xff; 32], 1);
        block.transactions_root = execution.transactions_root;
        block.execution_root = execution.execution_root;
        let block_id = block.block_id();

        {
            let store = StateStore::open(&path).unwrap();
            let outcome = store
                .insert_native_block_with_execution_outcome(block.clone(), &execution, &state)
                .unwrap();

            assert_eq!(outcome.block.block_id(), block_id);
            assert_eq!(outcome.current_best, block_id);
            assert_eq!(
                store.native_block_execution(block_id).unwrap(),
                Some(execution.clone())
            );
            assert_eq!(
                store.native_state_snapshot(block_id).unwrap(),
                Some(state.clone())
            );

            let repeated = store
                .insert_native_block_with_execution_outcome(block, &execution, &state)
                .unwrap();
            assert_eq!(repeated.block.block_id(), block_id);
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert_eq!(
                reopened.native_block_execution(block_id).unwrap(),
                Some(execution.clone())
            );
            assert_eq!(
                reopened.native_state_snapshot(block_id).unwrap(),
                Some(state)
            );
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_block_commit_rejects_commitment_mismatches_without_persisting() {
        use crate::address::AddressNetwork;
        use crate::native_execution::{
            execute_block_v1, AccountStateV1, NativeExecutionContextV1, NativeStateV1,
        };

        let context = NativeExecutionContextV1 {
            base_fee_per_gas: 3,
            cpu_producer: [0x09; 20],
        };

        for case in 0..3 {
            let path = temp_state_path(&format!("native-block-mismatch-{case}"));
            let store = StateStore::open(&path).unwrap();

            let mut state = NativeStateV1::default();
            let execution =
                execute_block_v1(&mut state, &[], AddressNetwork::Devnet, context).unwrap();

            let mut block = header([0_u8; 32], 0, [0xff; 32], case + 10);
            block.transactions_root = execution.transactions_root;
            block.execution_root = execution.execution_root;

            let block_id = block.block_id();

            let result = match case {
                0 => {
                    block.transactions_root[0] ^= 0x01;
                    store.insert_native_block_with_execution_outcome(block, &execution, &state)
                }
                1 => {
                    block.execution_root[0] ^= 0x01;
                    store.insert_native_block_with_execution_outcome(block, &execution, &state)
                }
                _ => {
                    let mut wrong_state = state.clone();
                    wrong_state.set_account(
                        [0x44; 20],
                        AccountStateV1 {
                            balance: 1,
                            nonce: 0,
                        },
                    );
                    store.insert_native_block_with_execution_outcome(
                        block,
                        &execution,
                        &wrong_state,
                    )
                }
            };

            assert!(result.is_err());
            assert!(store.best_chain_head().unwrap().is_none());

            assert!(store.load_chain_block(block_id).unwrap().is_none());
            assert!(store.native_block_execution(block_id).unwrap().is_none());
            assert!(store.native_state_snapshot(block_id).unwrap().is_none());

            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn native_block_body_survives_restart_and_is_idempotent() {
        use crate::native_block_body::NativeBlockBodyV1;

        let path = temp_state_path("native-block-body-restart");
        let block = header([0_u8; 32], 0, [0xff; 32], 1);
        let block_id = block.block_id();
        let body = NativeBlockBodyV1::empty();

        {
            let store = StateStore::open(&path).unwrap();
            store.insert_chain_block(block).unwrap();
            store.store_native_block_body(block_id, &body).unwrap();
            store.store_native_block_body(block_id, &body).unwrap();

            assert_eq!(
                store.native_block_body(block_id).unwrap(),
                Some(body.clone())
            );
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert_eq!(reopened.native_block_body(block_id).unwrap(), Some(body));
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_block_body_requires_persisted_block_and_rejects_conflict() {
        use crate::native_block_body::NativeBlockBodyV1;

        let path = temp_state_path("native-block-body-conflict");
        let store = StateStore::open(&path).unwrap();

        let body = NativeBlockBodyV1::empty();
        let err = store
            .store_native_block_body([0x91; 32], &body)
            .unwrap_err();
        assert!(err.contains("unpersisted NIAHCIA block"));

        let block = header([0_u8; 32], 0, [0xff; 32], 2);
        let block_id = block.block_id();
        store.insert_chain_block(block).unwrap();
        store.store_native_block_body(block_id, &body).unwrap();

        let conflicting = NativeBlockBodyV1::new([0x44; 20], Vec::new()).unwrap();
        let err = store
            .store_native_block_body(block_id, &conflicting)
            .unwrap_err();
        assert!(err.contains("different native block body"));

        assert_eq!(store.native_block_body(block_id).unwrap(), Some(body));

        let _ = std::fs::remove_file(path);
    }

    fn sample_state_v2() -> crate::native_state_v2::NativeStateV2 {
        use crate::native_execution::{AccountStateV1, NativeStateV1};
        use crate::native_state_v2::{
            ComputeChannelSettlementPolicyV1, ComputeChannelStateV1, ComputeChannelStatusV1,
            NativeStateV2,
        };

        let mut accounts = NativeStateV1::default();
        accounts.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 1_000,
                nonce: 2,
            },
        );

        let signing_key = k256::ecdsa::SigningKey::from_slice(&[0x42; 32]).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();

        let mut state = NativeStateV2::from_v1(accounts);
        state
            .set_channel(ComputeChannelStateV1 {
                channel_id: [0x22; 32],
                funding_account: [0x11; 20],
                worker_id: [0x33; 32],
                operator_id: [0x44; 32],
                channel_public_key: public_key,
                worker_payment_account: [0x55; 20],
                authorized_amount: 500,
                settled_amount: 0,
                opened_height: 10,
                expiry_height: 20,
                claim_deadline_height: 25,
                refund_available_height: 26,
                service_scope_commitment: [0x66; 32],
                model_scope_commitment: [0x77; 32],
                execution_profile_scope_commitment: [0x88; 32],
                settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
                state: ComputeChannelStatusV1::Open,
            })
            .unwrap();
        state
    }

    #[test]
    fn native_state_v2_snapshot_survives_restart_and_is_idempotent() {
        let path = temp_state_path("native-state-v2-restart");
        let block = header([0_u8; 32], 0, [0xff; 32], 7);
        let block_id = block.block_id();
        let expected = sample_state_v2();

        {
            let store = StateStore::open(&path).unwrap();
            store.insert_chain_block(block).unwrap();
            store
                .store_native_state_v2_snapshot(block_id, &expected)
                .unwrap();
            store
                .store_native_state_v2_snapshot(block_id, &expected)
                .unwrap();

            assert_eq!(
                store.native_state_v2_snapshot(block_id).unwrap(),
                Some(expected.clone())
            );
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert_eq!(
                reopened.native_state_v2_snapshot(block_id).unwrap(),
                Some(expected)
            );
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_state_v2_snapshot_requires_persisted_block() {
        let path = temp_state_path("native-state-v2-requires-block");
        let store = StateStore::open(&path).unwrap();
        let state = sample_state_v2();

        let err = store
            .store_native_state_v2_snapshot([0x91; 32], &state)
            .unwrap_err();
        assert!(err.contains("unpersisted NIAHCIA block"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_state_v1_and_v2_snapshots_cannot_conflict_for_same_block() {
        use crate::native_execution::NativeStateV1;

        let path = temp_state_path("native-state-version-conflict");
        let store = StateStore::open(&path).unwrap();

        let block = header([0_u8; 32], 0, [0xff; 32], 8);
        let block_id = block.block_id();
        store.insert_chain_block(block).unwrap();

        let v1 = NativeStateV1::default();
        store.store_native_state_snapshot(block_id, &v1).unwrap();

        let err = store
            .store_native_state_v2_snapshot(block_id, &sample_state_v2())
            .unwrap_err();
        assert!(err.contains("different native state snapshot"));

        assert!(store.native_state_snapshot(block_id).unwrap().is_some());
        assert!(store.native_state_v2_snapshot(block_id).is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_state_snapshot_requires_persisted_block() {
        use crate::native_execution::NativeStateV1;

        let path = temp_state_path("native-state-requires-block");
        let store = StateStore::open(&path).unwrap();
        let state = NativeStateV1::default();

        let err = store
            .store_native_state_snapshot([0x91; 32], &state)
            .unwrap_err();
        assert!(err.contains("unpersisted NIAHCIA block"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_state_snapshot_survives_restart() {
        use crate::native_execution::{AccountStateV1, NativeStateV1};

        let path = temp_state_path("native-state-restart");
        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();

        let mut expected = NativeStateV1::default();
        expected.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 123_456_789,
                nonce: 4,
            },
        );
        expected.set_account(
            [0x22; 20],
            AccountStateV1 {
                balance: 987_654_321,
                nonce: 9,
            },
        );

        {
            let store = StateStore::open(&path).unwrap();
            store.insert_chain_block(genesis).unwrap();
            store
                .store_native_state_snapshot(genesis_id, &expected)
                .unwrap();

            let loaded = store.native_state_snapshot(genesis_id).unwrap().unwrap();
            assert_eq!(loaded, expected);
            assert_eq!(loaded.state_root(), expected.state_root());
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            let loaded = reopened.native_state_snapshot(genesis_id).unwrap().unwrap();
            assert_eq!(loaded, expected);
            assert_eq!(loaded.state_root(), expected.state_root());
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_state_snapshot_is_idempotent_but_rejects_change() {
        use crate::native_execution::{AccountStateV1, NativeStateV1};

        let path = temp_state_path("native-state-idempotent");
        let store = StateStore::open(&path).unwrap();

        let genesis = header([0_u8; 32], 0, [0xff; 32], 1);
        let genesis_id = genesis.block_id();
        store.insert_chain_block(genesis).unwrap();

        let mut first = NativeStateV1::default();
        first.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 100,
                nonce: 1,
            },
        );

        store
            .store_native_state_snapshot(genesis_id, &first)
            .unwrap();
        store
            .store_native_state_snapshot(genesis_id, &first)
            .unwrap();

        let mut conflicting = first.clone();
        conflicting.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 101,
                nonce: 1,
            },
        );

        let err = store
            .store_native_state_snapshot(genesis_id, &conflicting)
            .unwrap_err();
        assert!(err.contains("different native state snapshot"));

        assert_eq!(
            store.native_state_snapshot(genesis_id).unwrap(),
            Some(first)
        );

        let _ = std::fs::remove_file(path);
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
