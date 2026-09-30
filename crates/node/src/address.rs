use bech32::{Bech32m, Hrp};
use sha3::{Digest, Keccak256};
use std::fmt;
use std::str::FromStr;

pub const ADDRESS_PAYLOAD_LEN: usize = 20;
const ADDRESS_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressNetwork {
    Mainnet,
    Testnet,
    Devnet,
}

impl AddressNetwork {
    pub const fn hrp(self) -> &'static str {
        match self {
            Self::Mainnet => "niah",
            Self::Testnet => "tniah",
            Self::Devnet => "dniah",
        }
    }

    fn from_hrp(hrp: &str) -> Result<Self, String> {
        match hrp {
            "niah" => Ok(Self::Mainnet),
            "tniah" => Ok(Self::Testnet),
            "dniah" => Ok(Self::Devnet),
            _ => Err(format!("unknown NIAHCIA address network prefix: {hrp}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AddressKind {
    Account = 0,
    Contract = 1,
}

impl TryFrom<u8> for AddressKind {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Account),
            1 => Ok(Self::Contract),
            _ => Err(format!("unsupported NIAHCIA address kind: {value}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NiahciaAddressV1 {
    pub network: AddressNetwork,
    pub kind: AddressKind,
    pub payload: [u8; ADDRESS_PAYLOAD_LEN],
}

impl NiahciaAddressV1 {
    pub const fn new(
        network: AddressNetwork,
        kind: AddressKind,
        payload: [u8; ADDRESS_PAYLOAD_LEN],
    ) -> Self {
        Self {
            network,
            kind,
            payload,
        }
    }

    pub fn account_from_uncompressed_public_key(
        network: AddressNetwork,
        public_key: &[u8],
    ) -> Result<Self, String> {
        if public_key.len() != 65 || public_key[0] != 0x04 {
            return Err(
                "account public key must be uncompressed SEC1 (65 bytes, 0x04 prefix)".into(),
            );
        }
        let digest = Keccak256::digest(&public_key[1..]);
        let mut payload = [0u8; ADDRESS_PAYLOAD_LEN];
        payload.copy_from_slice(&digest[12..]);
        Ok(Self::new(network, AddressKind::Account, payload))
    }

    fn data(self) -> [u8; ADDRESS_PAYLOAD_LEN + 2] {
        let mut data = [0u8; ADDRESS_PAYLOAD_LEN + 2];
        data[0] = ADDRESS_VERSION;
        data[1] = self.kind as u8;
        data[2..].copy_from_slice(&self.payload);
        data
    }

    pub fn encode(self) -> Result<String, String> {
        let hrp = Hrp::parse(self.network.hrp()).map_err(|e| e.to_string())?;
        bech32::encode::<Bech32m>(hrp, &self.data()).map_err(|e| e.to_string())
    }

    pub fn decode(text: &str) -> Result<Self, String> {
        let (hrp, data) =
            bech32::decode(text).map_err(|e| format!("invalid NIAHCIA address: {e}"))?;
        let network = AddressNetwork::from_hrp(hrp.as_str())?;
        if data.len() != ADDRESS_PAYLOAD_LEN + 2 {
            return Err(format!(
                "invalid NIAHCIA address payload length: {}",
                data.len()
            ));
        }
        if data[0] != ADDRESS_VERSION {
            return Err(format!("unsupported NIAHCIA address version: {}", data[0]));
        }
        let kind = AddressKind::try_from(data[1])?;
        let mut payload = [0u8; ADDRESS_PAYLOAD_LEN];
        payload.copy_from_slice(&data[2..]);
        Ok(Self::new(network, kind, payload))
    }
}

impl fmt::Display for NiahciaAddressV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.encode() {
            Ok(value) => f.write_str(&value),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl FromStr for NiahciaAddressV1 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::decode(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_networks_and_kinds() {
        for network in [
            AddressNetwork::Mainnet,
            AddressNetwork::Testnet,
            AddressNetwork::Devnet,
        ] {
            for kind in [AddressKind::Account, AddressKind::Contract] {
                let address =
                    NiahciaAddressV1::new(network, kind, [0x42; ADDRESS_PAYLOAD_LEN]);
                let encoded = address.encode().unwrap();
                assert_eq!(NiahciaAddressV1::decode(&encoded).unwrap(), address);
                assert!(encoded.starts_with(network.hrp()));
            }
        }
    }

    #[test]
    fn networks_cannot_be_confused() {
        let payload = [7u8; ADDRESS_PAYLOAD_LEN];
        let main = NiahciaAddressV1::new(AddressNetwork::Mainnet, AddressKind::Account, payload)
            .encode()
            .unwrap();
        let dev = NiahciaAddressV1::new(AddressNetwork::Devnet, AddressKind::Account, payload)
            .encode()
            .unwrap();
        assert_ne!(main, dev);
        assert!(main.starts_with("niah1"));
        assert!(dev.starts_with("dniah1"));
    }

    #[test]
    fn checksum_corruption_is_rejected() {
        let address = NiahciaAddressV1::new(
            AddressNetwork::Mainnet,
            AddressKind::Account,
            [0x11; ADDRESS_PAYLOAD_LEN],
        )
        .encode()
        .unwrap();
        let mut bytes = address.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'q' { b'p' } else { b'q' };
        let corrupted = String::from_utf8(bytes).unwrap();
        assert!(NiahciaAddressV1::decode(&corrupted).is_err());
    }

    #[test]
    fn derives_account_payload_from_sec1_public_key() {
        let mut key = [0u8; 65];
        key[0] = 0x04;
        key[1..].copy_from_slice(&[0x22; 64]);
        let address =
            NiahciaAddressV1::account_from_uncompressed_public_key(AddressNetwork::Devnet, &key)
                .unwrap();
        assert_eq!(address.kind, AddressKind::Account);
        assert_eq!(address.network, AddressNetwork::Devnet);
        assert_eq!(address.payload.len(), ADDRESS_PAYLOAD_LEN);
    }
}
