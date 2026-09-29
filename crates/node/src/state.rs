use crate::work::Hash32;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SERVICE_EVIDENCE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("service_evidence_v1");
const SERVICE_EPOCHS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("service_epochs_v1");
const CHAIN_BLOCKS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("chain_blocks_v1");
const CHAIN_META: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("chain_meta_v1");

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

        let db = Database::create(path).map_err(|e| format!("failed to open state database: {e}"))?;

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

    pub fn insert_service_evidence(&self, evidence_key: Hash32) -> Result<bool, String> {
        let write = self
            .db
            .begin_write()
            .map_err(|e| format!("failed to begin service evidence transaction: {e}"))?;

        let inserted = {
            let mut table = write
                .open_table(SERVICE_EVIDENCE)
                .map_err(|e| format!("failed to open service evidence table: {e}"))?;

            if table
                .get(evidence_key.as_slice())
                .map_err(|e| format!("failed to read service evidence key: {e}"))?
                .is_some()
            {
                false
            } else {
                table
                    .insert(evidence_key.as_slice(), [].as_slice())
                    .map_err(|e| format!("failed to persist service evidence key: {e}"))?;
                true
            }
        };

        if inserted {
            write
                .commit()
                .map_err(|e| format!("failed to commit service evidence key: {e}"))?;
        } else {
            write
                .abort()
                .map_err(|e| format!("failed to abort duplicate evidence transaction: {e}"))?;
        }

        Ok(inserted)
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
    use super::StateStore;
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
            assert!(store.insert_service_evidence(evidence_key).unwrap());
            assert!(store.has_service_evidence(evidence_key).unwrap());
            assert!(!store.insert_service_evidence(evidence_key).unwrap());
        }

        {
            let reopened = StateStore::open(&path).unwrap();
            assert!(reopened.has_service_evidence(evidence_key).unwrap());
            assert!(!reopened.insert_service_evidence(evidence_key).unwrap());
        }

        let _ = std::fs::remove_file(path);
    }
}
