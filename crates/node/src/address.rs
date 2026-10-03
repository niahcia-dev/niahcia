use bech32::{Bech32m, Hrp};
use sha3::{Digest, Keccak256};
use std::fmt;
use std::str::FromStr;

pub const ADDRESS_PAYLOAD_LEN: usize = 20;
const ADDRESS_VERSION: u8 = 1;
const CONTRACT_DERIVATION_DOMAIN: &[u8] = b"NIAHCIA/CONTRACT/V1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressNetwork {
    Mainnet,
    Testnet,
    Devnet,
}

impl AddressNetwork {
    pub const fn network_id(self) -> u8 {
        match self {
            Self::Mainnet => 0x00,
            Self::Testnet => 0x01,
            Self::Devnet => 0x02,
        }
    }

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

    pub fn contract_derivation_preimage(
        network: AddressNetwork,
        chain_id: u64,
        creator_payload: [u8; ADDRESS_PAYLOAD_LEN],
        creator_nonce: u64,
    ) -> Vec<u8> {
        let mut preimage = Vec::with_capacity(
            CONTRACT_DERIVATION_DOMAIN.len() + 1 + 1 + 1 + 8 + 1 + ADDRESS_PAYLOAD_LEN + 1 + 8,
        );
        preimage.extend_from_slice(CONTRACT_DERIVATION_DOMAIN);
        preimage.push(0);
        preimage.push(network.network_id());
        preimage.push(0);
        preimage.extend_from_slice(&chain_id.to_be_bytes());
        preimage.push(0);
        preimage.extend_from_slice(&creator_payload);
        preimage.push(0);
        preimage.extend_from_slice(&creator_nonce.to_be_bytes());
        preimage
    }

    pub fn contract_derivation_digest(
        network: AddressNetwork,
        chain_id: u64,
        creator_payload: [u8; ADDRESS_PAYLOAD_LEN],
        creator_nonce: u64,
    ) -> [u8; 32] {
        let preimage =
            Self::contract_derivation_preimage(network, chain_id, creator_payload, creator_nonce);
        Keccak256::digest(&preimage).into()
    }

    pub fn contract_from_creator(
        network: AddressNetwork,
        chain_id: u64,
        creator_payload: [u8; ADDRESS_PAYLOAD_LEN],
        creator_nonce: u64,
    ) -> Self {
        let digest =
            Self::contract_derivation_digest(network, chain_id, creator_payload, creator_nonce);
        let mut payload = [0u8; ADDRESS_PAYLOAD_LEN];
        payload.copy_from_slice(&digest[12..]);
        Self::new(network, AddressKind::Contract, payload)
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
                let address = NiahciaAddressV1::new(network, kind, [0x42; ADDRESS_PAYLOAD_LEN]);
                let encoded = address.encode().unwrap();
                assert_eq!(NiahciaAddressV1::decode(&encoded).unwrap(), address);
                assert!(encoded.starts_with(network.hrp()));
            }
        }
    }

    #[test]
    fn canonical_zero_payload_vectors_are_stable() {
        let vectors = [
            (
                AddressNetwork::Mainnet,
                AddressKind::Account,
                "niah1qyqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqg6tnyn",
            ),
            (
                AddressNetwork::Mainnet,
                AddressKind::Contract,
                "niah1qyqsqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqquk68t8",
            ),
            (
                AddressNetwork::Testnet,
                AddressKind::Account,
                "tniah1qyqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqugtn3d",
            ),
            (
                AddressNetwork::Testnet,
                AddressKind::Contract,
                "tniah1qyqsqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqgy687e",
            ),
            (
                AddressNetwork::Devnet,
                AddressKind::Account,
                "dniah1qyqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqw26l4w",
            ),
            (
                AddressNetwork::Devnet,
                AddressKind::Contract,
                "dniah1qyqsqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq6xtt66",
            ),
        ];

        for (network, kind, expected) in vectors {
            let address = NiahciaAddressV1::new(network, kind, [0u8; ADDRESS_PAYLOAD_LEN]);
            assert_eq!(address.encode().unwrap(), expected);
            assert_eq!(NiahciaAddressV1::decode(expected).unwrap(), address);
        }
    }

    #[test]
    fn contract_derivation_probe_vector() {
        let creator = hex::decode("1a642f0e3c3af545e7acbd38b07251b3990914f1").unwrap();
        let creator: [u8; ADDRESS_PAYLOAD_LEN] = creator.try_into().unwrap();

        let cases = [
            (AddressNetwork::Mainnet, 0x0000_0000_4E49_4148, 7_u64),
            (AddressNetwork::Testnet, 0x0000_0001_5449_4148, 7_u64),
            (AddressNetwork::Devnet, 0x0000_0002_4449_4148, 7_u64),
        ];
        let actual = cases
            .into_iter()
            .map(|(network, chain_id, nonce)| {
                let preimage = NiahciaAddressV1::contract_derivation_preimage(
                    network, chain_id, creator, nonce,
                );
                let digest = NiahciaAddressV1::contract_derivation_digest(
                    network, chain_id, creator, nonce,
                );
                let address =
                    NiahciaAddressV1::contract_from_creator(network, chain_id, creator, nonce);
                (
                    hex::encode(preimage),
                    hex::encode(digest),
                    hex::encode(address.payload),
                    address.to_string(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                (
                    "4e4941484349412f434f4e54524143542f5631000000000000004e494148001a642f0e3c3af545e7acbd38b07251b3990914f1000000000000000007".to_string(),
                    "TODO".to_string(),
                    "1508350cddc8a39443c7957fbd0e667ef04dfe42".to_string(),
                    "niah1qyq32zp4pnwu3gu5g0re2laapen8auzdlepq2x9wz7".to_string(),
                ),
                (
                    "4e4941484349412f434f4e54524143542f56310001000000000154494148001a642f0e3c3af545e7acbd38b07251b3990914f1000000000000000007".to_string(),
                    "TODO".to_string(),
                    "7829f303530b6bbf75282f5f8c8f19dcdca0c651".to_string(),
                    "tniah1qyqhs20nqdfsk6alw55z7huv3uvaeh9qcegst5axuu".to_string(),
                ),
                (
                    "4e4941484349412f434f4e54524143542f56310002000000000244494148001a642f0e3c3af545e7acbd38b07251b3990914f1000000000000000007".to_string(),
                    "TODO".to_string(),
                    "aeeeb31d0d11c1f819b79f8a39ccbc790f4f2516".to_string(),
                    "dniah1qyq6am4nr5x3rs0crxmelz3eej78jr60y5tqzdp2wa".to_string(),
                ),
            ]
        );
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

    #[test]
    fn account_derivation_vector_is_stable() {
        use k256::ecdsa::SigningKey;

        // Synthetic interoperability key only. Never use for funds.
        let secret = [0x01u8; 32];
        let signing_key = SigningKey::from_slice(&secret).unwrap();
        let public_key = signing_key.verifying_key().to_encoded_point(false);
        let public_key_bytes = public_key.as_bytes();

        assert_eq!(
            hex::encode(public_key_bytes),
            "041b84c5567b126440995d3ed5aaba0565d71e1834604819ff9c17f5e9d5dd078f70beaf8f588b541507fed6a642c5ab42dfdf8120a7f639de5122d47a69a8e8d1"
        );

        let digest = Keccak256::digest(&public_key_bytes[1..]);
        assert_eq!(
            hex::encode(digest),
            "b8a0722ae6cb48cde0b4ae1f1a642f0e3c3af545e7acbd38b07251b3990914f1"
        );

        let expected_payload = hex::decode("1a642f0e3c3af545e7acbd38b07251b3990914f1").unwrap();

        for (network, expected_address) in [
            (
                AddressNetwork::Mainnet,
                "niah1qyqp5ep0pc7r4a29u7kt6w9swfgm8xgfzncsse486g",
            ),
            (
                AddressNetwork::Testnet,
                "tniah1qyqp5ep0pc7r4a29u7kt6w9swfgm8xgfzncsyt480k",
            ),
            (
                AddressNetwork::Devnet,
                "dniah1qyqp5ep0pc7r4a29u7kt6w9swfgm8xgfzncskfytt4",
            ),
        ] {
            let address =
                NiahciaAddressV1::account_from_uncompressed_public_key(network, public_key_bytes)
                    .unwrap();

            assert_eq!(address.payload.as_slice(), expected_payload.as_slice());
            assert_eq!(address.encode().unwrap(), expected_address);
        }
    }
}
