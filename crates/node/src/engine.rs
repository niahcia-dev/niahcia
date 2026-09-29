use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct EngineClient {
    endpoint: String,
    jwt_secret: Vec<u8>,
    http: Client,
}

#[derive(Serialize)]
struct JwtClaims {
    iat: usize,
}

impl EngineClient {
    pub fn new(endpoint: String, jwt_path: &Path) -> Result<Self, String> {
        let jwt_secret = load_jwt_secret(jwt_path)?;
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("failed to build Engine API HTTP client: {e}"))?;

        Ok(Self {
            endpoint,
            jwt_secret,
            http,
        })
    }

    pub fn exchange_capabilities(&self) -> Result<Vec<String>, String> {
        let offered = vec![
            "engine_exchangeCapabilities",
            "engine_forkchoiceUpdatedV1",
            "engine_getPayloadV1",
            "engine_newPayloadV1",
        ];

        let result = self.request(
            "engine_exchangeCapabilities",
            json!([offered]),
        )?;

        serde_json::from_value(result)
            .map_err(|e| format!("invalid engine_exchangeCapabilities response: {e}"))
    }

    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let token = self.jwt_token()?;
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(token)
            .json(&body)
            .send()
            .map_err(|e| format!("Engine API request failed: {e}"))?;

        let status = response.status();
        let payload: Value = response
            .json()
            .map_err(|e| format!("Engine API returned non-JSON response ({status}): {e}"))?;

        if !status.is_success() {
            return Err(format!("Engine API HTTP {status}: {payload}"));
        }

        if let Some(error) = payload.get("error") {
            return Err(format!("Engine API JSON-RPC error: {error}"));
        }

        payload
            .get("result")
            .cloned()
            .ok_or_else(|| format!("Engine API response missing result: {payload}"))
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
    use super::load_jwt_secret;
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
}
