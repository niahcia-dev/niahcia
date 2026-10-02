use crate::address::{AddressKind, AddressNetwork, NiahciaAddressV1};
use serde::Deserialize;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub network: String,
    pub data_dir: PathBuf,
    pub reth_http_rpc: String,
    pub fee_recipient: String,
    pub reth_jwt_path: PathBuf,
    pub mining_rpc_bind: SocketAddr,
    pub p2p_bind: SocketAddr,
    pub p2p_peers: Vec<SocketAddr>,
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            network: "devnet".to_string(),
            data_dir: PathBuf::from("./data"),
            reth_http_rpc: "http://127.0.0.1:8545".to_string(),
            fee_recipient: "0x0000000000000000000000000000000000000000".to_string(),
            reth_jwt_path: PathBuf::from("./jwt.hex"),
            mining_rpc_bind: "127.0.0.1:9332".parse().expect("valid default socket"),
            p2p_bind: "127.0.0.1:9442".parse().expect("valid default P2P socket"),
            p2p_peers: Vec::new(),
            log_level: "info".to_string(),
        }
    }
}

impl NodeConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        let mut cfg = match path {
            Some(path) => {
                let raw = fs::read_to_string(path)
                    .map_err(|e| format!("failed to read config {}: {e}", path.display()))?;
                toml::from_str::<NodeConfig>(&raw)
                    .map_err(|e| format!("failed to parse config {}: {e}", path.display()))?
            }
            None => NodeConfig::default(),
        };

        if let Ok(v) = env::var("NIAHCIA_NETWORK") {
            cfg.network = v;
        }
        if let Ok(v) = env::var("NIAHCIA_DATA_DIR") {
            cfg.data_dir = PathBuf::from(v);
        }
        if let Ok(v) = env::var("NIAHCIA_RETH_HTTP_RPC") {
            cfg.reth_http_rpc = v;
        }
        if let Ok(v) = env::var("NIAHCIA_FEE_RECIPIENT") {
            cfg.fee_recipient = v;
        }
        if let Ok(v) = env::var("NIAHCIA_RETH_JWT_PATH") {
            cfg.reth_jwt_path = PathBuf::from(v);
        }
        if let Ok(v) = env::var("NIAHCIA_MINING_RPC_BIND") {
            cfg.mining_rpc_bind = v
                .parse()
                .map_err(|e| format!("invalid NIAHCIA_MINING_RPC_BIND: {e}"))?;
        }
        if let Ok(v) = env::var("NIAHCIA_P2P_BIND") {
            cfg.p2p_bind = v
                .parse()
                .map_err(|e| format!("invalid NIAHCIA_P2P_BIND: {e}"))?;
        }
        if let Ok(v) = env::var("NIAHCIA_P2P_PEERS") {
            cfg.p2p_peers = v
                .split(',')
                .map(str::trim)
                .filter(|peer| !peer.is_empty())
                .map(|peer| {
                    peer.parse()
                        .map_err(|e| format!("invalid NIAHCIA_P2P_PEERS entry '{peer}': {e}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
        if let Ok(v) = env::var("NIAHCIA_LOG_LEVEL") {
            cfg.log_level = v;
        }

        cfg.validate()?;
        cfg.normalize_fee_recipient_for_execution()?;
        Ok(cfg)
    }

    fn native_fee_recipient(&self) -> Result<Option<NiahciaAddressV1>, String> {
        if self.fee_recipient.starts_with("dniah1")
            || self.fee_recipient.starts_with("tniah1")
            || self.fee_recipient.starts_with("niah1")
        {
            return NiahciaAddressV1::decode(&self.fee_recipient)
                .map(Some)
                .map_err(|e| format!("invalid native fee_recipient: {e}"));
        }
        Ok(None)
    }

    fn validate_fee_recipient(&self) -> Result<(), String> {
        if let Some(address) = self.native_fee_recipient()? {
            if address.network != AddressNetwork::Devnet {
                return Err(format!(
                    "fee_recipient network mismatch: devnet requires a dniah1 address, found {}",
                    address.network.hrp()
                ));
            }
            if address.kind != AddressKind::Account {
                return Err("fee_recipient must be a NIAHCIA account address".into());
            }
            return Ok(());
        }

        let recipient = self
            .fee_recipient
            .strip_prefix("0x")
            .unwrap_or(&self.fee_recipient);
        if recipient.len() != 40 || hex::decode(recipient).is_err() {
            return Err(
                "fee_recipient must be a devnet dniah1 account address or legacy 20-byte hex address"
                    .into(),
            );
        }
        Ok(())
    }

    fn normalize_fee_recipient_for_execution(&mut self) -> Result<(), String> {
        if let Some(address) = self.native_fee_recipient()? {
            // NIAHCIA exposes the Bech32m address at its native boundary, while
            // Reth's Engine API consumes the same account as its 20-byte payload.
            self.fee_recipient = format!("0x{}", hex::encode(address.payload));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        if self.network != "devnet" {
            return Err(format!(
                "unsupported network '{}'; this pre-alpha node currently defines consensus parameters only for devnet",
                self.network
            ));
        }

        if !(self.reth_http_rpc.starts_with("http://")
            || self.reth_http_rpc.starts_with("https://"))
        {
            return Err("reth_http_rpc must start with http:// or https://".into());
        }

        self.validate_fee_recipient()?;

        if self.log_level.trim().is_empty() {
            return Err("log_level must not be empty".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_devnet_account_normalizes_to_execution_payload() {
        let payload = [0x42; 20];
        let native = NiahciaAddressV1::new(AddressNetwork::Devnet, AddressKind::Account, payload)
            .encode()
            .unwrap();
        let mut cfg = NodeConfig {
            fee_recipient: native,
            ..NodeConfig::default()
        };

        cfg.validate().unwrap();
        cfg.normalize_fee_recipient_for_execution().unwrap();
        assert_eq!(cfg.fee_recipient, format!("0x{}", hex::encode(payload)));
    }

    #[test]
    fn native_wrong_network_is_rejected_before_normalization() {
        let native =
            NiahciaAddressV1::new(AddressNetwork::Mainnet, AddressKind::Account, [0x11; 20])
                .encode()
                .unwrap();
        let cfg = NodeConfig {
            fee_recipient: native,
            ..NodeConfig::default()
        };

        assert!(cfg.validate().unwrap_err().contains("network mismatch"));
    }

    #[test]
    fn native_contract_is_rejected_as_fee_recipient() {
        let native =
            NiahciaAddressV1::new(AddressNetwork::Devnet, AddressKind::Contract, [0x22; 20])
                .encode()
                .unwrap();
        let cfg = NodeConfig {
            fee_recipient: native,
            ..NodeConfig::default()
        };

        assert!(cfg
            .validate()
            .unwrap_err()
            .contains("must be a NIAHCIA account address"));
    }
}
