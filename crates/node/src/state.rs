use crate::work::Hash32;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SERVICE_EVIDENCE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_evidence_v1");
const SERVICE_EPOCHS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("service_epochs_v1");
const CHAIN_BLOCKS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_blocks_v1");
const CHAIN_META: TableDefinition<&[u8], &[u8]> = TableDefinition::new("chain_meta_v1");

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
