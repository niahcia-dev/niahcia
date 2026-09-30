use crate::consensus::{
    devnet_next_target, randomx_seed, randomx_seed_height, validate_timestamp,
    DEVNET_GENESIS_TARGET, MEDIAN_TIME_WINDOW,
};
use crate::engine::EngineClient;
use crate::pow::RandomXVerifier;
use crate::state::StateStore;
use crate::work::{Address20, BlockHeaderV1, Hash32};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct WorkManager {
    inner: Arc<RwLock<WorkState>>,
}

#[derive(Clone)]
struct WorkState {
    generation: u64,
    header: BlockHeaderV1,
    randomx_seed_height: u64,
    randomx_seed: [u8; 32],
    execution_hash: Hash32,
    solved: bool,
}

impl WorkManager {
    pub fn new(
        header: BlockHeaderV1,
        randomx_seed_height: u64,
        randomx_seed: [u8; 32],
        execution_hash: Hash32,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(WorkState {
                generation: 0,
                header,
                randomx_seed_height,
                randomx_seed,
                execution_hash,
                solved: false,
            })),
        }
    }

    pub fn current(&self) -> (u64, BlockHeaderV1, u64, [u8; 32]) {
        let state = self.inner.read().expect("work state poisoned");
        (
            state.generation,
            state.header.clone(),
            state.randomx_seed_height,
            state.randomx_seed,
        )
    }

    fn submission_candidate(
        &self,
        generation: u64,
        template_id: Hash32,
        nonce: u64,
        extra_nonce: u64,
    ) -> Result<(BlockHeaderV1, Hash32, Hash32), String> {
        let state = self
            .inner
            .read()
            .map_err(|_| "work state poisoned".to_string())?;
        if state.solved {
            return Err("current work template is already solved".into());
        }
        if generation != state.generation || template_id != state.header.mining_template_id() {
            return Err("stale mining work".into());
        }

        let mut header = state.header.clone();
        header.nonce = nonce;
        header.extra_nonce = extra_nonce;
        Ok((header, state.randomx_seed, state.execution_hash))
    }

    fn mark_solved(&self, generation: u64, template_id: Hash32) -> Result<(), String> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| "work state poisoned".to_string())?;
        if generation != state.generation || template_id != state.header.mining_template_id() {
            return Err("mining work changed before acceptance".into());
        }
        state.solved = true;
        Ok(())
    }

    fn is_solved(&self) -> Result<bool, String> {
        self.inner
            .read()
            .map(|state| state.solved)
            .map_err(|_| "work state poisoned".to_string())
    }

    pub fn replace(
        &self,
        header: BlockHeaderV1,
        randomx_seed_height: u64,
        randomx_seed: Hash32,
        execution_hash: Hash32,
    ) -> Result<u64, String> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| "work state poisoned".to_string())?;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| "mining work generation overflow".to_string())?;
        state.header = header;
        state.randomx_seed_height = randomx_seed_height;
        state.randomx_seed = randomx_seed;
        state.execution_hash = execution_hash;
        state.solved = false;
        Ok(state.generation)
    }

    #[cfg(test)]
    pub fn is_stale(&self, generation: u64, template_id: &[u8; 32]) -> bool {
        let state = self.inner.read().expect("work state poisoned");
        generation != state.generation || &state.header.mining_template_id() != template_id
    }
}

pub fn spawn(
    bind: SocketAddr,
    work: WorkManager,
    state: Arc<StateStore>,
    engine: Arc<EngineClient>,
    fee_recipient: Address20,
    running: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, String> {
    let listener =
        TcpListener::bind(bind).map_err(|e| format!("failed to bind mining RPC on {bind}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to configure mining RPC listener: {e}"))?;

    info!(%bind, "mining RPC listening");

    Ok(thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    if let Err(e) = handle_connection(stream, &work, &state, &engine, fee_recipient)
                    {
                        warn!(%peer, error = %e, "mining RPC request failed");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(e) => {
                    error!(error = %e, "mining RPC accept failed");
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }))
}

fn handle_connection(
    mut stream: TcpStream,
    work: &WorkManager,
    state: &StateStore,
    engine: &EngineClient,
    fee_recipient: Address20,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("failed to set RPC read timeout: {e}"))?;

    let mut buf = [0_u8; 16 * 1024];
    let size = stream
        .read(&mut buf)
        .map_err(|e| format!("failed to read RPC request: {e}"))?;

    if size == 0 {
        return Err("empty RPC request".into());
    }

    let request_text = String::from_utf8_lossy(&buf[..size]);
    let body = request_text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "malformed HTTP request".to_string())?;

    let request: Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON-RPC request: {e}"))?;

    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| "JSON-RPC method missing".to_string())?;

    let response = match method {
        "pow_getWork" => {
            if work.is_solved()? {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32001,
                        "message": "current work template is solved; wait for template refresh"
                    }
                })
            } else {
                let (generation, header, randomx_seed_height, randomx_seed) = work.current();
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "development": true,
                        "generation": generation,
                        "template_id": hex::encode(header.mining_template_id()),
                        "version": header.version,
                        "parent_hash": hex::encode(header.parent_hash),
                        "height": header.height,
                        "timestamp": header.timestamp,
                        "transactions_root": hex::encode(header.transactions_root),
                        "execution_root": hex::encode(header.execution_root),
                        "target": hex::encode(header.target),
                        "randomx_seed_height": randomx_seed_height,
                        "randomx_seed": hex::encode(randomx_seed),
                        "nonce_start": 0_u64,
                        "nonce_end": u64::MAX,
                        "extra_nonce_start": 0_u64,
                        "extra_nonce_end": u64::MAX
                    }
                })
            }
        }
        "pow_submitWork" => match submit_work(&request, work, state, Some((engine, fee_recipient)))
        {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Err(message) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32002,
                    "message": message
                }
            }),
        },
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "method not found"
            }
        }),
    };

    let payload = serde_json::to_vec(&response)
        .map_err(|e| format!("failed to encode JSON-RPC response: {e}"))?;

    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(&payload))
        .map_err(|e| format!("failed to write RPC response: {e}"))
}

fn submit_work(
    request: &Value,
    work: &WorkManager,
    state: &StateStore,
    engine: Option<(&EngineClient, Address20)>,
) -> Result<Value, String> {
    let params = request
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| "pow_submitWork params must be an object".to_string())?;

    let generation = params
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| "pow_submitWork generation must be a u64".to_string())?;
    let template_id = parse_hash32_hex(
        params
            .get("template_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "pow_submitWork template_id must be hex".to_string())?,
    )?;
    let nonce = params
        .get("nonce")
        .and_then(Value::as_u64)
        .ok_or_else(|| "pow_submitWork nonce must be a u64".to_string())?;
    let extra_nonce = params
        .get("extra_nonce")
        .and_then(Value::as_u64)
        .ok_or_else(|| "pow_submitWork extra_nonce must be a u64".to_string())?;

    let (header, seed, execution_hash) =
        work.submission_candidate(generation, template_id, nonce, extra_nonce)?;
    let pow_hash = validate_block_candidate(&header, seed, state)?;
    let outcome = state.insert_mined_block_with_execution_outcome(header, execution_hash)?;
    if outcome.current_best == outcome.block.block_id() {
        if let Some((engine, fee_recipient)) = engine {
            engine.set_canonical_head_v3(execution_hash)?;
            install_next_work(work, state, engine, fee_recipient, execution_hash)?;
        } else {
            work.mark_solved(generation, template_id)?;
        }
    } else {
        work.mark_solved(generation, template_id)?;
    }

    let reorg = outcome.reorg.as_ref().map(|reorg| {
        json!({
            "old_head": hex::encode(reorg.old_head),
            "new_head": hex::encode(reorg.new_head),
            "common_ancestor": hex::encode(reorg.common_ancestor),
            "detached": reorg.detached.iter().map(hex::encode).collect::<Vec<_>>(),
            "attached": reorg.attached.iter().map(hex::encode).collect::<Vec<_>>()
        })
    });

    Ok(json!({
        "accepted": true,
        "block_id": hex::encode(outcome.block.block_id()),
        "pow_hash": hex::encode(pow_hash),
        "height": outcome.block.header.height,
        "cumulative_work": outcome.block.chain_work.to_str_radix(10),
        "became_canonical": outcome.current_best == outcome.block.block_id(),
        "previous_best": outcome.previous_best.map(hex::encode),
        "current_best": hex::encode(outcome.current_best),
        "reorg": reorg
    }))
}

fn install_next_work(
    work: &WorkManager,
    state: &StateStore,
    engine: &EngineClient,
    fee_recipient: Address20,
    execution_parent_hash: Hash32,
) -> Result<(), String> {
    let parent = engine.block_by_hash(execution_parent_hash)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock error while refreshing mining work: {e}"))?
        .as_secs();
    let timestamp = now.max(parent.timestamp.saturating_add(1));
    let built = engine.build_payload_v3(&parent, timestamp, fee_recipient)?;
    engine.validate_payload_v3(&built)?;

    let niahcia_parent = state
        .best_chain_head()?
        .ok_or_else(|| "accepted canonical block missing from state".to_string())?;
    let height = niahcia_parent
        .header
        .height
        .checked_add(1)
        .ok_or_else(|| "NIAHCIA height overflow".to_string())?;
    let genesis = state
        .canonical_block_at_height(0)?
        .ok_or_else(|| "canonical chain is missing devnet genesis".to_string())?;
    let target = devnet_next_target(
        genesis.header.timestamp,
        niahcia_parent.header.height,
        niahcia_parent.header.timestamp,
    )?;
    let execution = &built.commitments;
    let header = BlockHeaderV1 {
        version: 1,
        parent_hash: niahcia_parent.block_id(),
        height,
        timestamp: execution.timestamp,
        transactions_root: execution.transactions_root,
        execution_root: execution.commitment_hash(),
        target,
        nonce: 0,
        extra_nonce: 0,
    };
    let seed_height = randomx_seed_height(height);
    let seed_block = state
        .canonical_block_at_height(seed_height)?
        .ok_or_else(|| format!("canonical chain missing RandomX seed block {seed_height}"))?;
    let seed = randomx_seed(seed_block.block_id());
    let execution_hash = built.execution_payload_hash;
    let replay_bytes = engine.encode_replay_payload(&built)?;
    state.store_execution_payload(execution_hash, &replay_bytes)?;
    let next_generation = work.replace(header, seed_height, seed, execution_hash)?;
    info!(
        generation = next_generation,
        height,
        execution_payload_hash = %hex::encode(execution_hash),
        "installed next Reth-backed NIAHCIA mining template"
    );
    Ok(())
}

pub(crate) fn validate_block_candidate(
    header: &BlockHeaderV1,
    seed: Hash32,
    state: &StateStore,
) -> Result<Hash32, String> {
    let adjusted_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock error during block validation: {e}"))?
        .as_secs();

    let mut ancestor_timestamps = Vec::with_capacity(MEDIAN_TIME_WINDOW);
    let expected_target = if header.height == 0 {
        if header.parent_hash != [0_u8; 32] {
            return Err("genesis block must have a zero parent hash".into());
        }
        DEVNET_GENESIS_TARGET
    } else {
        let mut cursor = state
            .load_chain_block(header.parent_hash)?
            .ok_or_else(|| "candidate parent is not persisted".to_string())?;
        if cursor.header.height.checked_add(1) != Some(header.height) {
            return Err("candidate height does not follow persisted parent".into());
        }

        let parent_height = cursor.header.height;
        let parent_timestamp = cursor.header.timestamp;
        let genesis_timestamp = loop {
            ancestor_timestamps.push(cursor.header.timestamp);
            if cursor.header.height == 0 {
                break cursor.header.timestamp;
            }
            let parent = state
                .load_chain_block(cursor.header.parent_hash)?
                .ok_or_else(|| "candidate ancestry references missing parent".to_string())?;
            if ancestor_timestamps.len() < MEDIAN_TIME_WINDOW {
                cursor = parent;
            } else {
                let mut genesis_cursor = parent;
                while genesis_cursor.header.height != 0 {
                    genesis_cursor = state
                        .load_chain_block(genesis_cursor.header.parent_hash)?
                        .ok_or_else(|| {
                            "candidate ancestry references missing parent".to_string()
                        })?;
                }
                break genesis_cursor.header.timestamp;
            }
        };

        devnet_next_target(genesis_timestamp, parent_height, parent_timestamp)?
    };

    if header.target != expected_target {
        return Err(format!(
            "candidate target {} does not match expected devnet target {}",
            hex::encode(header.target),
            hex::encode(expected_target)
        ));
    }

    validate_timestamp(header.timestamp, &ancestor_timestamps, adjusted_time)?;
    RandomXVerifier::new(seed)?.verify_header(header)
}

fn parse_hash32_hex(value: &str) -> Result<Hash32, String> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(raw).map_err(|e| format!("invalid 32-byte hex value: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 32-byte hex value; found {} bytes", v.len()))
}

#[cfg(test)]
mod tests {
    use super::{submit_work, WorkManager};
    use crate::state::StateStore;
    use crate::work::BlockHeaderV1;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "niahcia-mining-{name}-{}-{nonce}.redb",
            std::process::id()
        ))
    }

    fn header(marker: u8) -> BlockHeaderV1 {
        BlockHeaderV1 {
            version: 1,
            parent_hash: [marker; 32],
            height: marker as u64,
            timestamp: 1_800_000_000 + marker as u64,
            transactions_root: [marker.wrapping_add(1); 32],
            execution_root: [marker.wrapping_add(2); 32],
            target: [0xff; 32],
            nonce: 0,
            extra_nonce: 0,
        }
    }

    #[test]
    fn submit_work_independently_verifies_and_persists_block() {
        let path = temp_state_path("submit-valid");
        let store = StateStore::open(&path).unwrap();

        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0_u8; 32],
            height: 0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 0,
            extra_nonce: 0,
        };
        let execution_hash = [0x99; 32];
        store
            .store_execution_payload(execution_hash, b"test-replay-payload")
            .unwrap();
        let manager = WorkManager::new(header.clone(), 0, [0x42; 32], execution_hash);
        let template_id = header.mining_template_id();

        let request = json!({
            "params": {
                "generation": 0,
                "template_id": hex::encode(template_id),
                "nonce": 7,
                "extra_nonce": 9
            }
        });

        let result = submit_work(&request, &manager, &store, None).unwrap();
        assert_eq!(result["accepted"], true);

        let mut solved = header;
        solved.nonce = 7;
        solved.extra_nonce = 9;
        let persisted = store.load_chain_block(solved.block_id()).unwrap().unwrap();
        assert_eq!(persisted.header, solved);
        assert_eq!(result["became_canonical"], true);
        assert_eq!(result["current_best"], hex::encode(solved.block_id()));
        assert!(manager.is_solved().unwrap());

        let duplicate = submit_work(&request, &manager, &store, None).unwrap_err();
        assert_eq!(duplicate, "current work template is already solved");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn submit_work_rejects_future_timestamp_before_persistence() {
        let path = temp_state_path("submit-future-time");
        let store = StateStore::open(&path).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0_u8; 32],
            height: 0,
            timestamp: now + crate::consensus::MAX_FUTURE_DRIFT + 1,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 0,
            extra_nonce: 0,
        };
        let manager = WorkManager::new(header.clone(), 0, [0x42; 32], [0x99; 32]);
        let request = json!({
            "params": {
                "generation": 0,
                "template_id": hex::encode(header.mining_template_id()),
                "nonce": 7,
                "extra_nonce": 9
            }
        });

        assert!(submit_work(&request, &manager, &store, None)
            .unwrap_err()
            .contains("exceeds maximum future time"));
        assert!(store.best_chain_head().unwrap().is_none());
        assert!(!manager.is_solved().unwrap());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn submit_work_rejects_stale_template_before_hashing() {
        let path = temp_state_path("submit-stale");
        let store = StateStore::open(&path).unwrap();
        let header = BlockHeaderV1 {
            version: 1,
            parent_hash: [0_u8; 32],
            height: 0,
            timestamp: 1_800_000_000,
            transactions_root: [0x22; 32],
            execution_root: [0x33; 32],
            target: [0xff; 32],
            nonce: 0,
            extra_nonce: 0,
        };
        let manager = WorkManager::new(header, 0, [0x42; 32], [0x99; 32]);

        let request = json!({
            "params": {
                "generation": 99,
                "template_id": hex::encode([0x55_u8; 32]),
                "nonce": 7,
                "extra_nonce": 9
            }
        });

        assert_eq!(
            submit_work(&request, &manager, &store, None).unwrap_err(),
            "stale mining work"
        );
        assert!(store.best_chain_head().unwrap().is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replacing_work_marks_previous_generation_stale() {
        let manager = WorkManager::new(header(1), 0, [0x33; 32], [0x99; 32]);
        let (generation, current, _, _) = manager.current();
        let old_id = current.mining_template_id();

        assert!(!manager.is_stale(generation, &old_id));

        manager
            .replace(header(2), 0, [0x42; 32], [0x99; 32])
            .unwrap();

        assert!(manager.is_stale(generation, &old_id));
    }
}
