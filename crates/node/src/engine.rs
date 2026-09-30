use crate::work::{keccak256, Address20, ExecutionPayloadCommitments, Hash32};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct LatestBlock {
    pub hash: Hash32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct BuiltExecutionPayload {
    pub commitments: ExecutionPayloadCommitments,
    pub execution_payload_hash: Hash32,
    execution_payload: Value,
    parent_beacon_block_root: Hash32,
    execution_requests: Value,
}

#[derive(Debug)]
pub struct EngineClient {
    engine_endpoint: String,
    public_endpoint: String,
    jwt_secret: Vec<u8>,
    http: Client,
}

#[derive(Serialize)]
struct JwtClaims {
    iat: usize,
}

impl EngineClient {
    pub fn new(
        engine_endpoint: String,
        public_endpoint: String,
        jwt_path: &Path,
    ) -> Result<Self, String> {
        let jwt_secret = load_jwt_secret(jwt_path)?;
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("failed to build Reth HTTP client: {e}"))?;

        Ok(Self {
            engine_endpoint,
            public_endpoint,
            jwt_secret,
            http,
        })
    }

    pub fn exchange_capabilities(&self) -> Result<Vec<String>, String> {
        let offered = vec![
            "engine_exchangeCapabilities",
            "engine_forkchoiceUpdatedV3",
            "engine_getPayloadV4",
            "engine_newPayloadV4",
        ];

        let result = self.engine_request("engine_exchangeCapabilities", json!([offered]))?;

        serde_json::from_value(result)
            .map_err(|e| format!("invalid engine_exchangeCapabilities response: {e}"))
    }

    pub fn block_by_hash(&self, hash: Hash32) -> Result<LatestBlock, String> {
        let result = self.public_request("eth_getBlockByHash", json!([hex32(hash), false]))?;
        if result.is_null() {
            return Err(format!("Reth block {} not found", hex32(hash)));
        }
        Ok(LatestBlock {
            hash: parse_hash32(field_str(&result, "hash")?)?,
            timestamp: parse_quantity(field_str(&result, "timestamp")?)?,
        })
    }

    pub fn latest_block(&self) -> Result<LatestBlock, String> {
        let result = self.public_request("eth_getBlockByNumber", json!(["latest", false]))?;

        Ok(LatestBlock {
            hash: parse_hash32(field_str(&result, "hash")?)?,
            timestamp: parse_quantity(field_str(&result, "timestamp")?)?,
        })
    }

    pub fn build_payload_v3(
        &self,
        parent: &LatestBlock,
        timestamp: u64,
        fee_recipient: Address20,
    ) -> Result<BuiltExecutionPayload, String> {
        if timestamp <= parent.timestamp {
            return Err(format!(
                "payload timestamp {timestamp} must be greater than parent timestamp {}",
                parent.timestamp
            ));
        }

        let parent_hex = hex32(parent.hash);
        let fee_hex = format!("0x{}", hex::encode(fee_recipient));
        let zero32 = format!("0x{}", "00".repeat(32));

        let forkchoice = json!({
            "headBlockHash": parent_hex,
            "safeBlockHash": parent_hex,
            "finalizedBlockHash": parent_hex
        });

        let attributes = json!({
            "timestamp": quantity(timestamp),
            "prevRandao": zero32,
            "suggestedFeeRecipient": fee_hex,
            "withdrawals": [],
            "parentBeaconBlockRoot": zero32
        });

        let update = self.engine_request(
            "engine_forkchoiceUpdatedV3",
            json!([forkchoice, attributes]),
        )?;

        let status = update
            .pointer("/payloadStatus/status")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("forkchoice response missing payloadStatus.status: {update}"))?;

        if status != "VALID" {
            return Err(format!(
                "Reth rejected forkchoice update with status {status}: {update}"
            ));
        }

        let payload_id = update
            .get("payloadId")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("forkchoice response missing payloadId: {update}"))?;

        let envelope = self.engine_request("engine_getPayloadV4", json!([payload_id]))?;
        let payload = envelope
            .get("executionPayload")
            .ok_or_else(|| format!("engine_getPayloadV4 missing executionPayload: {envelope}"))?;

        let parent_hash = parse_hash32(field_str(payload, "parentHash")?)?;
        if parent_hash != parent.hash {
            return Err("Reth payload parentHash does not match requested parent".into());
        }

        let execution_payload_hash = parse_hash32(field_str(payload, "blockHash")?)?;
        let execution_requests = envelope
            .get("executionRequests")
            .cloned()
            .ok_or_else(|| format!("engine_getPayloadV4 missing executionRequests: {envelope}"))?;
        let transactions_root = transaction_merkle_root(payload)?;

        Ok(BuiltExecutionPayload {
            commitments: ExecutionPayloadCommitments {
                execution_parent_hash: parent_hash,
                fee_recipient: parse_address20(field_str(payload, "feeRecipient")?)?,
                state_root: parse_hash32(field_str(payload, "stateRoot")?)?,
                receipts_root: parse_hash32(field_str(payload, "receiptsRoot")?)?,
                transactions_root,
                block_number: parse_quantity(field_str(payload, "blockNumber")?)?,
                gas_limit: parse_quantity(field_str(payload, "gasLimit")?)?,
                gas_used: parse_quantity(field_str(payload, "gasUsed")?)?,
                timestamp: parse_quantity(field_str(payload, "timestamp")?)?,
                base_fee_per_gas: parse_u256(field_str(payload, "baseFeePerGas")?)?,
            },
            execution_payload_hash,
            execution_payload: payload.clone(),
            parent_beacon_block_root: [0_u8; 32],
            execution_requests,
        })
    }

    pub fn validate_payload_v3(&self, built: &BuiltExecutionPayload) -> Result<(), String> {
        let result = self.engine_request(
            "engine_newPayloadV4",
            json!([
                built.execution_payload,
                [],
                hex32(built.parent_beacon_block_root),
                built.execution_requests
            ]),
        )?;
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("engine_newPayloadV4 response missing status: {result}"))?;

        if status != "VALID" {
            return Err(format!(
                "Reth did not validate execution payload; status {status}: {result}"
            ));
        }

        let validated_hash = result
            .get("latestValidHash")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!("VALID engine_newPayloadV3 response missing latestValidHash: {result}")
            })?;
        if parse_hash32(validated_hash)? != built.execution_payload_hash {
            return Err(format!(
                "Reth VALID latestValidHash {validated_hash} does not match built payload {}",
                hex32(built.execution_payload_hash)
            ));
        }

        Ok(())
    }

    pub fn set_canonical_head_v3(&self, head: Hash32) -> Result<(), String> {
        let head_hex = hex32(head);
        let forkchoice = json!({
            "headBlockHash": head_hex,
            "safeBlockHash": head_hex,
            "finalizedBlockHash": head_hex
        });
        let result =
            self.engine_request("engine_forkchoiceUpdatedV3", json!([forkchoice, null]))?;
        let status = result
            .pointer("/payloadStatus/status")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("forkchoice response missing payloadStatus.status: {result}"))?;
        if status != "VALID" {
            return Err(format!(
                "Reth rejected canonical head with status {status}: {result}"
            ));
        }
        Ok(())
    }

    fn engine_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let token = self.jwt_token()?;
        self.request(&self.engine_endpoint, method, params, Some(token))
    }

    fn public_request(&self, method: &str, params: Value) -> Result<Value, String> {
        self.request(&self.public_endpoint, method, params, None)
    }

    fn request(
        &self,
        endpoint: &str,
        method: &str,
        params: Value,
        bearer: Option<String>,
    ) -> Result<Value, String> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let mut request = self.http.post(endpoint).json(&body);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .map_err(|e| format!("{method} request to {endpoint} failed: {e}"))?;

        let status = response.status();
        let payload: Value = response
            .json()
            .map_err(|e| format!("{method} returned non-JSON response ({status}): {e}"))?;

        if !status.is_success() {
            return Err(format!("{method} HTTP {status}: {payload}"));
        }

        if let Some(error) = payload.get("error") {
            return Err(format!("{method} JSON-RPC error: {error}"));
        }

        payload
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{method} response missing result: {payload}"))
    }

    fn jwt_token(&self) -> Result<String, String> {
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("system clock error: {e}"))?
            .as_secs() as usize;

        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("JWT".to_string());

        encode(
            &header,
            &JwtClaims { iat },
            &EncodingKey::from_secret(&self.jwt_secret),
        )
        .map_err(|e| format!("failed to create Engine API JWT: {e}"))
    }
}

fn transaction_merkle_root(payload: &Value) -> Result<Hash32, String> {
    const TX_DOMAIN: &[u8] = b"NIAHCIA/TX/V1";
    const EMPTY_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-EMPTY/V1";
    const LEAF_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-LEAF/V1";
    const NODE_DOMAIN: &[u8] = b"NIAHCIA/MERKLE-NODE/V1";

    let transactions = payload
        .get("transactions")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("execution payload missing transactions array: {payload}"))?;

    if transactions.is_empty() {
        return Ok(keccak256(EMPTY_DOMAIN));
    }

    let mut level = Vec::with_capacity(transactions.len());

    for transaction in transactions {
        let raw = transaction
            .as_str()
            .ok_or_else(|| "execution transaction must be a hex string".to_string())?;
        let decoded =
            hex::decode(strip_hex(raw)).map_err(|e| format!("invalid transaction hex: {e}"))?;

        let mut tx_preimage = Vec::with_capacity(TX_DOMAIN.len() + decoded.len());
        tx_preimage.extend_from_slice(TX_DOMAIN);
        tx_preimage.extend_from_slice(&decoded);
        let tx_digest = keccak256(&tx_preimage);

        let mut leaf_preimage = Vec::with_capacity(LEAF_DOMAIN.len() + 32);
        leaf_preimage.extend_from_slice(LEAF_DOMAIN);
        leaf_preimage.extend_from_slice(&tx_digest);
        level.push(keccak256(&leaf_preimage));
    }

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));

        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { pair[0] };

            let mut node_preimage = Vec::with_capacity(NODE_DOMAIN.len() + 64);
            node_preimage.extend_from_slice(NODE_DOMAIN);
            node_preimage.extend_from_slice(&left);
            node_preimage.extend_from_slice(&right);
            next.push(keccak256(&node_preimage));
        }

        level = next;
    }

    Ok(level[0])
}

fn field_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or invalid {field}: {value}"))
}

fn strip_hex(value: &str) -> &str {
    value.strip_prefix("0x").unwrap_or(value)
}

fn parse_hash32(value: &str) -> Result<Hash32, String> {
    let bytes = hex::decode(strip_hex(value)).map_err(|e| format!("invalid 32-byte hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 32 bytes, found {}", v.len()))
}

fn parse_address20(value: &str) -> Result<Address20, String> {
    let bytes = hex::decode(strip_hex(value)).map_err(|e| format!("invalid address hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 20-byte address, found {}", v.len()))
}

fn parse_quantity(value: &str) -> Result<u64, String> {
    u64::from_str_radix(strip_hex(value), 16)
        .map_err(|e| format!("invalid hexadecimal quantity {value}: {e}"))
}

fn parse_u256(value: &str) -> Result<Hash32, String> {
    let raw = strip_hex(value);
    if raw.len() > 64 {
        return Err(format!("quantity exceeds 256 bits: {value}"));
    }

    let padded = format!("{raw:0>64}");
    parse_hash32(&padded)
}

fn quantity(value: u64) -> String {
    format!("0x{value:x}")
}

fn hex32(value: Hash32) -> String {
    format!("0x{}", hex::encode(value))
}

fn load_jwt_secret(path: &Path) -> Result<Vec<u8>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read Reth JWT secret {}: {e}", path.display()))?;
    let trimmed = raw.trim().strip_prefix("0x").unwrap_or(raw.trim());

    let bytes = hex::decode(trimmed)
        .map_err(|e| format!("Reth JWT secret {} is not valid hex: {e}", path.display()))?;

    if bytes.len() < 32 {
        return Err(format!(
            "Reth JWT secret {} must contain at least 32 bytes; found {}",
            path.display(),
            bytes.len()
        ));
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{load_jwt_secret, parse_quantity, parse_u256, transaction_merkle_root};
    use serde_json::json;
    use std::fs;

    #[test]
    fn jwt_secret_accepts_0x_prefixed_hex() {
        let path = std::env::temp_dir().join(format!(
            "niahcia-jwt-test-{}-{}.hex",
            std::process::id(),
            "prefixed"
        ));
        fs::write(&path, format!("0x{}\n", "11".repeat(32))).unwrap();

        let secret = load_jwt_secret(&path).unwrap();
        assert_eq!(secret, vec![0x11; 32]);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn jwt_secret_rejects_short_values() {
        let path = std::env::temp_dir().join(format!(
            "niahcia-jwt-test-{}-{}.hex",
            std::process::id(),
            "short"
        ));
        fs::write(&path, "abcd\n").unwrap();

        let error = load_jwt_secret(&path).unwrap_err();
        assert!(error.contains("at least 32 bytes"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_engine_quantities() {
        assert_eq!(parse_quantity("0x2a").unwrap(), 42);
        assert_eq!(parse_quantity("0x0").unwrap(), 0);
    }

    #[test]
    fn transaction_merkle_root_is_niahcia_native() {
        let payload = json!({
            "transactions": [
                "0x010203",
                "0xaabb"
            ]
        });

        let first = transaction_merkle_root(&payload).unwrap();
        let second = transaction_merkle_root(&payload).unwrap();
        assert_eq!(first, second);

        let changed = json!({
            "transactions": [
                "0x010203",
                "0xaabc"
            ]
        });
        assert_ne!(first, transaction_merkle_root(&changed).unwrap());
    }

    #[test]
    fn empty_transaction_root_is_stable() {
        let payload = json!({"transactions": []});
        let first = transaction_merkle_root(&payload).unwrap();
        let second = transaction_merkle_root(&payload).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn pads_u256_quantity_to_32_bytes() {
        let value = parse_u256("0x1").unwrap();
        assert_eq!(value[31], 1);
        assert!(value[..31].iter().all(|byte| *byte == 0));
    }
}
