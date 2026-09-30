use crate::address::{AddressKind, AddressNetwork, NiahciaAddressV1};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub network: &'static str,
    pub native_rpc_namespace: &'static str,
    pub address_hrp: &'static str,
    pub address_version: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressValidation {
    pub valid: bool,
    pub network: Option<&'static str>,
    pub kind: Option<&'static str>,
    pub version: Option<u8>,
    pub payload: Option<String>,
    pub network_match: bool,
    pub canonical: Option<String>,
    pub error: Option<String>,
}

pub const fn network_name(network: AddressNetwork) -> &'static str {
    match network {
        AddressNetwork::Mainnet => "mainnet",
        AddressNetwork::Testnet => "testnet",
        AddressNetwork::Devnet => "devnet",
    }
}

pub const fn kind_name(kind: AddressKind) -> &'static str {
    match kind {
        AddressKind::Account => "account",
        AddressKind::Contract => "contract",
    }
}

pub fn get_network_info(network: AddressNetwork) -> NetworkInfo {
    NetworkInfo {
        network: network_name(network),
        native_rpc_namespace: "niah",
        address_hrp: network.hrp(),
        address_version: 1,
    }
}

pub fn validate_address(text: &str, expected_network: AddressNetwork) -> AddressValidation {
    match NiahciaAddressV1::decode(text) {
        Ok(address) => AddressValidation {
            valid: true,
            network: Some(network_name(address.network)),
            kind: Some(kind_name(address.kind)),
            version: Some(1),
            payload: Some(hex::encode(address.payload)),
            network_match: address.network == expected_network,
            canonical: address.encode().ok(),
            error: None,
        },
        Err(error) => AddressValidation {
            valid: false,
            network: None,
            kind: None,
            version: None,
            payload: None,
            network_match: false,
            canonical: None,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_devnet_native_network_info() {
        let info = get_network_info(AddressNetwork::Devnet);
        assert_eq!(info.network, "devnet");
        assert_eq!(info.native_rpc_namespace, "niah");
        assert_eq!(info.address_hrp, "dniah");
        assert_eq!(info.address_version, 1);
    }

    #[test]
    fn validates_native_address_and_reports_canonical_form() {
        let address = NiahciaAddressV1::new(
            AddressNetwork::Devnet,
            AddressKind::Account,
            [0x42; 20],
        );
        let encoded = address.encode().unwrap();
        let result = validate_address(&encoded, AddressNetwork::Devnet);
        assert!(result.valid);
        assert!(result.network_match);
        assert_eq!(result.network, Some("devnet"));
        assert_eq!(result.kind, Some("account"));
        assert_eq!(result.version, Some(1));
        assert_eq!(result.payload, Some(hex::encode([0x42; 20])));
        assert_eq!(result.canonical.as_deref(), Some(encoded.as_str()));
        assert!(result.error.is_none());
    }

    #[test]
    fn valid_foreign_network_address_is_not_a_network_match() {
        let encoded = NiahciaAddressV1::new(
            AddressNetwork::Mainnet,
            AddressKind::Account,
            [0x11; 20],
        )
        .encode()
        .unwrap();
        let result = validate_address(&encoded, AddressNetwork::Devnet);
        assert!(result.valid);
        assert!(!result.network_match);
        assert_eq!(result.network, Some("mainnet"));
    }

    #[test]
    fn malformed_address_is_invalid_without_partial_identity() {
        let result = validate_address("dniah1not-a-valid-address", AddressNetwork::Devnet);
        assert!(!result.valid);
        assert!(!result.network_match);
        assert!(result.network.is_none());
        assert!(result.kind.is_none());
        assert!(result.payload.is_none());
        assert!(result.canonical.is_none());
        assert!(result.error.is_some());
    }
}
