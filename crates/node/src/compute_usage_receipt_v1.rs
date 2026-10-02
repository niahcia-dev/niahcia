use crate::address::AddressNetwork;
use crate::native_transaction::{
    DEVNET_NETWORK_ID, MAINNET_NETWORK_ID, TESTNET_NETWORK_ID,
};
use crate::nce::{decode_envelope, encode_bytes, encode_envelope, encode_map, encode_unsigned};
use crate::work::{keccak256, Hash32};
use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

pub const COMPUTE_USAGE_RECEIPT_OBJECT_TYPE: u64 = 0x0013;
pub const COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION: u64 = 1;

const COMPUTE_USAGE_RECEIPT_SIGNING_PURPOSE: &[u8] = b"SIGN/COMPUTE_USAGE_RECEIPT";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeUsageReceiptV1 {
    pub receipt_id: Hash32,
    pub channel_id: Hash32,
    pub authorization_id: Hash32,
    pub worker_id: Hash32,
    pub operator_id: Hash32,
    pub sequence: u64,
    pub previous_receipt_id: Hash32,
    pub cumulative_spent: u128,
    pub job_id: Hash32,
    pub result_commitment_id: Hash32,
    pub price_offer_id: Hash32,
    pub job_charge: u128,
    pub metering_evidence_hash: Hash32,
    pub expires_at: u64,
    pub channel_signature: Vec<u8>,
}

impl ComputeUsageReceiptV1 {
    pub fn validate_basic(&self) -> Result<(), String> {
        if self.job_charge > self.cumulative_spent {
            return Err("compute usage receipt job charge exceeds cumulative spend".into());
        }
        self.canonical_signature()?;
        Ok(())
    }

    pub fn canonical_payload_without_signature(&self) -> Result<Vec<u8>, String> {
        encode_map(&[
            (1, encode_unsigned(COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION)),
            (2, encode_bytes(&self.receipt_id)),
            (3, encode_bytes(&self.channel_id)),
            (4, encode_bytes(&self.authorization_id)),
            (5, encode_bytes(&self.worker_id)),
            (6, encode_bytes(&self.operator_id)),
            (7, encode_unsigned(self.sequence)),
            (8, encode_bytes(&self.previous_receipt_id)),
            (9, encode_bytes(&self.cumulative_spent.to_be_bytes())),
            (10, encode_bytes(&self.job_id)),
            (11, encode_bytes(&self.result_commitment_id)),
            (12, encode_bytes(&self.price_offer_id)),
            (13, encode_bytes(&self.job_charge.to_be_bytes())),
            (14, encode_bytes(&self.metering_evidence_hash)),
            (15, encode_unsigned(self.expires_at)),
        ])
    }

    pub fn canonical_bytes_without_signature(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            COMPUTE_USAGE_RECEIPT_OBJECT_TYPE,
            COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION,
            self.canonical_payload_without_signature()?,
        )
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        self.canonical_signature()?;
        encode_map(&[
            (1, encode_unsigned(COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION)),
            (2, encode_bytes(&self.receipt_id)),
            (3, encode_bytes(&self.channel_id)),
            (4, encode_bytes(&self.authorization_id)),
            (5, encode_bytes(&self.worker_id)),
            (6, encode_bytes(&self.operator_id)),
            (7, encode_unsigned(self.sequence)),
            (8, encode_bytes(&self.previous_receipt_id)),
            (9, encode_bytes(&self.cumulative_spent.to_be_bytes())),
            (10, encode_bytes(&self.job_id)),
            (11, encode_bytes(&self.result_commitment_id)),
            (12, encode_bytes(&self.price_offer_id)),
            (13, encode_bytes(&self.job_charge.to_be_bytes())),
            (14, encode_bytes(&self.metering_evidence_hash)),
            (15, encode_unsigned(self.expires_at)),
            (16, encode_bytes(&self.channel_signature)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            COMPUTE_USAGE_RECEIPT_OBJECT_TYPE,
            COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn signing_digest(&self, network: AddressNetwork) -> Result<Hash32, String> {
        let network_id = network_id(network);
        let unsigned = self.canonical_bytes_without_signature()?;

        let mut preimage = Vec::with_capacity(
            b"NIAHCIA".len()
                + 1
                + COMPUTE_USAGE_RECEIPT_SIGNING_PURPOSE.len()
                + 1
                + 1
                + 1
                + unsigned.len(),
        );
        preimage.extend_from_slice(b"NIAHCIA");
        preimage.push(0);
        preimage.extend_from_slice(COMPUTE_USAGE_RECEIPT_SIGNING_PURPOSE);
        preimage.push(0);
        preimage.push(network_id);
        preimage.push(0);
        preimage.extend_from_slice(&unsigned);
        Ok(keccak256(&preimage))
    }

    pub fn verify_signature(
        &self,
        network: AddressNetwork,
        channel_public_key: &[u8; 65],
    ) -> Result<(), String> {
        if channel_public_key[0] != 0x04 {
            return Err("compute channel public key must be uncompressed SEC1".into());
        }
        let verifying_key = VerifyingKey::from_sec1_bytes(channel_public_key)
            .map_err(|_| "invalid compute channel secp256k1 public key".to_string())?;
        let signature = self.canonical_signature()?;
        let digest = self.signing_digest(network)?;

        verifying_key
            .verify_prehash(&digest, &signature)
            .map_err(|_| "invalid compute usage receipt channel signature".to_string())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            COMPUTE_USAGE_RECEIPT_OBJECT_TYPE,
            COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 16 {
            return Err("compute usage receipt must contain exactly sixteen fields".into());
        }

        expect_key(&mut reader, 1, "schema_version")?;
        if reader.unsigned()? != COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION {
            return Err("compute usage receipt payload schema_version mismatch".into());
        }

        expect_key(&mut reader, 2, "receipt_id")?;
        let receipt_id = read_fixed::<32>(&mut reader, "receipt_id")?;

        expect_key(&mut reader, 3, "channel_id")?;
        let channel_id = read_fixed::<32>(&mut reader, "channel_id")?;

        expect_key(&mut reader, 4, "authorization_id")?;
        let authorization_id = read_fixed::<32>(&mut reader, "authorization_id")?;

        expect_key(&mut reader, 5, "worker_id")?;
        let worker_id = read_fixed::<32>(&mut reader, "worker_id")?;

        expect_key(&mut reader, 6, "operator_id")?;
        let operator_id = read_fixed::<32>(&mut reader, "operator_id")?;

        expect_key(&mut reader, 7, "sequence")?;
        let sequence = reader.unsigned()?;

        expect_key(&mut reader, 8, "previous_receipt_id")?;
        let previous_receipt_id = read_fixed::<32>(&mut reader, "previous_receipt_id")?;

        expect_key(&mut reader, 9, "cumulative_spent")?;
        let cumulative_spent =
            u128::from_be_bytes(read_fixed::<16>(&mut reader, "cumulative_spent")?);

        expect_key(&mut reader, 10, "job_id")?;
        let job_id = read_fixed::<32>(&mut reader, "job_id")?;

        expect_key(&mut reader, 11, "result_commitment_id")?;
        let result_commitment_id = read_fixed::<32>(&mut reader, "result_commitment_id")?;

        expect_key(&mut reader, 12, "price_offer_id")?;
        let price_offer_id = read_fixed::<32>(&mut reader, "price_offer_id")?;

        expect_key(&mut reader, 13, "job_charge")?;
        let job_charge = u128::from_be_bytes(read_fixed::<16>(&mut reader, "job_charge")?);

        expect_key(&mut reader, 14, "metering_evidence_hash")?;
        let metering_evidence_hash =
            read_fixed::<32>(&mut reader, "metering_evidence_hash")?;

        expect_key(&mut reader, 15, "expires_at")?;
        let expires_at = reader.unsigned()?;

        expect_key(&mut reader, 16, "channel_signature")?;
        let channel_signature = reader.bytes()?.to_vec();

        if !reader.finished() {
            return Err("trailing bytes after compute usage receipt".into());
        }

        let receipt = Self {
            receipt_id,
            channel_id,
            authorization_id,
            worker_id,
            operator_id,
            sequence,
            previous_receipt_id,
            cumulative_spent,
            job_id,
            result_commitment_id,
            price_offer_id,
            job_charge,
            metering_evidence_hash,
            expires_at,
            channel_signature,
        };

        receipt.validate_basic()?;
        if receipt.canonical_bytes()? != bytes {
            return Err("compute usage receipt is not canonically encoded".into());
        }
        Ok(receipt)
    }

    fn canonical_signature(&self) -> Result<Signature, String> {
        if self.channel_signature.len() != 64 {
            return Err("compute usage receipt signature must be exactly 64 bytes".into());
        }
        let signature = Signature::from_slice(&self.channel_signature)
            .map_err(|_| "invalid compute usage receipt secp256k1 signature".to_string())?;
        if signature.normalize_s().is_some() {
            return Err("compute usage receipt signature must use canonical low-S form".into());
        }
        Ok(signature)
    }
}

fn network_id(network: AddressNetwork) -> u8 {
    match network {
        AddressNetwork::Mainnet => MAINNET_NETWORK_ID as u8,
        AddressNetwork::Testnet => TESTNET_NETWORK_ID as u8,
        AddressNetwork::Devnet => DEVNET_NETWORK_ID as u8,
    }
}

fn expect_key(
    reader: &mut crate::nce::NceReader<'_>,
    expected: u64,
    label: &str,
) -> Result<(), String> {
    let actual = reader.unsigned()?;
    if actual != expected {
        return Err(format!(
            "invalid compute usage receipt {label} field key: expected {expected}, found {actual}"
        ));
    }
    Ok(())
}

fn read_fixed<const N: usize>(
    reader: &mut crate::nce::NceReader<'_>,
    label: &str,
) -> Result<[u8; N], String> {
    let bytes = reader.bytes()?;
    if bytes.len() != N {
        return Err(format!(
            "compute usage receipt {label} must be exactly {N} bytes"
        ));
    }
    Ok(bytes.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};

    fn unsigned_receipt() -> ComputeUsageReceiptV1 {
        ComputeUsageReceiptV1 {
            receipt_id: [0x01; 32],
            channel_id: [0x02; 32],
            authorization_id: [0x03; 32],
            worker_id: [0x04; 32],
            operator_id: [0x05; 32],
            sequence: 2,
            previous_receipt_id: [0x06; 32],
            cumulative_spent: 100,
            job_id: [0x07; 32],
            result_commitment_id: [0x08; 32],
            price_offer_id: [0x09; 32],
            job_charge: 25,
            metering_evidence_hash: [0x0a; 32],
            expires_at: 200,
            channel_signature: vec![0; 64],
        }
    }

    fn signed_receipt() -> (ComputeUsageReceiptV1, [u8; 65]) {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();

        let mut receipt = unsigned_receipt();
        let digest = receipt.signing_digest(AddressNetwork::Devnet).unwrap();
        let signature: Signature = signing_key.sign_prehash(&digest).unwrap();
        receipt.channel_signature = signature.to_bytes().to_vec();
        (receipt, public_key)
    }

    #[test]
    fn receipt_round_trips_and_verifies() {
        let (receipt, public_key) = signed_receipt();
        receipt
            .verify_signature(AddressNetwork::Devnet, &public_key)
            .unwrap();

        let bytes = receipt.canonical_bytes().unwrap();
        let decoded = ComputeUsageReceiptV1::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(decoded, receipt);
        decoded
            .verify_signature(AddressNetwork::Devnet, &public_key)
            .unwrap();
    }

    #[test]
    fn receipt_signature_is_network_bound() {
        let (receipt, public_key) = signed_receipt();
        assert!(receipt
            .verify_signature(AddressNetwork::Mainnet, &public_key)
            .is_err());
    }

    #[test]
    fn receipt_rejects_high_s_or_tampered_signature() {
        let (mut receipt, public_key) = signed_receipt();
        receipt.channel_signature[0] ^= 1;
        assert!(receipt
            .verify_signature(AddressNetwork::Devnet, &public_key)
            .is_err());
    }

    #[test]
    fn receipt_rejects_job_charge_above_cumulative_spend() {
        let (mut receipt, _) = signed_receipt();
        receipt.job_charge = receipt.cumulative_spent + 1;
        assert!(receipt.validate_basic().is_err());
    }

    #[test]
    fn receipt_unsigned_preimage_omits_signature_field() {
        let (receipt, _) = signed_receipt();
        let unsigned = receipt.canonical_bytes_without_signature().unwrap();
        let full = receipt.canonical_bytes().unwrap();
        assert_ne!(unsigned, full);
        assert!(unsigned.len() < full.len());
    }
}
