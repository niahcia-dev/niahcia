use crate::compute_usage_receipt_v1::ComputeUsageReceiptV1;
use crate::native_state_v2::ComputeChannelSettlementPolicyV1;
use crate::nce::{decode_envelope, encode_bytes, encode_envelope, encode_map, encode_unsigned};
use crate::work::{keccak256, Hash32};
use k256::PublicKey;

pub const COMPUTE_CHANNEL_OPEN_PAYLOAD_OBJECT_TYPE: u64 = 0x0014;
pub const COMPUTE_CHANNEL_SETTLE_PAYLOAD_OBJECT_TYPE: u64 = 0x0015;
pub const COMPUTE_CHANNEL_REFUND_PAYLOAD_OBJECT_TYPE: u64 = 0x0016;
pub const COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION: u64 = 1;

const COMPUTE_CHANNEL_ID_DOMAIN: &[u8] = b"NIAHCIA/COMPUTE-CHANNEL-ID/V1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeChannelOpenPayloadV1 {
    pub worker_id: Hash32,
    pub operator_id: Hash32,
    pub channel_public_key: [u8; 65],
    pub worker_payment_account: [u8; 20],
    pub authorized_amount: u128,
    pub expiry_height: u64,
    pub claim_deadline_height: u64,
    pub refund_available_height: u64,
    pub service_scope_commitment: Hash32,
    pub model_scope_commitment: Hash32,
    pub execution_profile_scope_commitment: Hash32,
    pub settlement_policy: ComputeChannelSettlementPolicyV1,
}

impl ComputeChannelOpenPayloadV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.authorized_amount == 0 {
            return Err("compute channel authorized amount must be greater than zero".into());
        }
        if self.expiry_height > self.claim_deadline_height {
            return Err("compute channel claim deadline precedes expiry".into());
        }
        if self.claim_deadline_height >= self.refund_available_height {
            return Err("compute channel refund height must be after claim deadline".into());
        }
        if self.channel_public_key[0] != 0x04 {
            return Err("compute channel public key must be uncompressed SEC1".into());
        }
        PublicKey::from_sec1_bytes(&self.channel_public_key)
            .map_err(|_| "compute channel public key is not a valid secp256k1 point".to_string())?;
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        encode_map(&[
            (1, encode_bytes(&self.worker_id)),
            (2, encode_bytes(&self.operator_id)),
            (3, encode_bytes(&self.channel_public_key)),
            (4, encode_bytes(&self.worker_payment_account)),
            (5, encode_bytes(&self.authorized_amount.to_be_bytes())),
            (6, encode_unsigned(self.expiry_height)),
            (7, encode_unsigned(self.claim_deadline_height)),
            (8, encode_unsigned(self.refund_available_height)),
            (9, encode_bytes(&self.service_scope_commitment)),
            (10, encode_bytes(&self.model_scope_commitment)),
            (11, encode_bytes(&self.execution_profile_scope_commitment)),
            (12, encode_unsigned(self.settlement_policy as u64)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            COMPUTE_CHANNEL_OPEN_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            COMPUTE_CHANNEL_OPEN_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 12 {
            return Err("compute channel open payload must contain exactly twelve fields".into());
        }

        expect_key(&mut reader, 1, "worker_id")?;
        let worker_id = read_fixed::<32>(&mut reader, "worker_id")?;

        expect_key(&mut reader, 2, "operator_id")?;
        let operator_id = read_fixed::<32>(&mut reader, "operator_id")?;

        expect_key(&mut reader, 3, "channel_public_key")?;
        let channel_public_key = read_fixed::<65>(&mut reader, "channel_public_key")?;

        expect_key(&mut reader, 4, "worker_payment_account")?;
        let worker_payment_account = read_fixed::<20>(&mut reader, "worker_payment_account")?;

        expect_key(&mut reader, 5, "authorized_amount")?;
        let authorized_amount =
            u128::from_be_bytes(read_fixed::<16>(&mut reader, "authorized_amount")?);

        expect_key(&mut reader, 6, "expiry_height")?;
        let expiry_height = reader.unsigned()?;

        expect_key(&mut reader, 7, "claim_deadline_height")?;
        let claim_deadline_height = reader.unsigned()?;

        expect_key(&mut reader, 8, "refund_available_height")?;
        let refund_available_height = reader.unsigned()?;

        expect_key(&mut reader, 9, "service_scope_commitment")?;
        let service_scope_commitment = read_fixed::<32>(&mut reader, "service_scope_commitment")?;

        expect_key(&mut reader, 10, "model_scope_commitment")?;
        let model_scope_commitment = read_fixed::<32>(&mut reader, "model_scope_commitment")?;

        expect_key(&mut reader, 11, "execution_profile_scope_commitment")?;
        let execution_profile_scope_commitment =
            read_fixed::<32>(&mut reader, "execution_profile_scope_commitment")?;

        expect_key(&mut reader, 12, "settlement_policy")?;
        let policy_value = reader.unsigned()?;
        let policy_u8 = u8::try_from(policy_value)
            .map_err(|_| "compute channel settlement policy exceeds u8".to_string())?;
        let settlement_policy = ComputeChannelSettlementPolicyV1::try_from(policy_u8)?;

        if !reader.finished() {
            return Err("trailing bytes after compute channel open payload".into());
        }

        let payload = Self {
            worker_id,
            operator_id,
            channel_public_key,
            worker_payment_account,
            authorized_amount,
            expiry_height,
            claim_deadline_height,
            refund_available_height,
            service_scope_commitment,
            model_scope_commitment,
            execution_profile_scope_commitment,
            settlement_policy,
        };
        payload.validate()?;

        if payload.canonical_bytes()? != bytes {
            return Err("compute channel open payload is not canonically encoded".into());
        }

        Ok(payload)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeChannelSettlePayloadV1 {
    pub channel_id: Hash32,
    pub final_usage_receipt: Vec<u8>,
}

impl ComputeChannelSettlePayloadV1 {
    pub fn validate(&self) -> Result<(), String> {
        let receipt = ComputeUsageReceiptV1::from_canonical_bytes(&self.final_usage_receipt)?;
        if receipt.channel_id != self.channel_id {
            return Err("compute channel settle payload receipt channel_id mismatch".into());
        }
        Ok(())
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        encode_map(&[
            (1, encode_bytes(&self.channel_id)),
            (2, encode_bytes(&self.final_usage_receipt)),
        ])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            COMPUTE_CHANNEL_SETTLE_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            COMPUTE_CHANNEL_SETTLE_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 2 {
            return Err("compute channel settle payload must contain exactly two fields".into());
        }

        expect_key(&mut reader, 1, "channel_id")?;
        let channel_id = read_fixed::<32>(&mut reader, "channel_id")?;

        expect_key(&mut reader, 2, "final_usage_receipt")?;
        let final_usage_receipt = reader.bytes()?.to_vec();

        if !reader.finished() {
            return Err("trailing bytes after compute channel settle payload".into());
        }

        let payload = Self {
            channel_id,
            final_usage_receipt,
        };
        payload.validate()?;

        if payload.canonical_bytes()? != bytes {
            return Err("compute channel settle payload is not canonically encoded".into());
        }

        Ok(payload)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeChannelRefundPayloadV1 {
    pub channel_id: Hash32,
}

impl ComputeChannelRefundPayloadV1 {
    pub fn canonical_payload(&self) -> Result<Vec<u8>, String> {
        encode_map(&[(1, encode_bytes(&self.channel_id))])
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        encode_envelope(
            COMPUTE_CHANNEL_REFUND_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
            self.canonical_payload()?,
        )
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = decode_envelope(
            bytes,
            COMPUTE_CHANNEL_REFUND_PAYLOAD_OBJECT_TYPE,
            COMPUTE_ACTION_PAYLOAD_SCHEMA_VERSION,
        )?;

        if reader.map_len()? != 1 {
            return Err("compute channel refund payload must contain exactly one field".into());
        }

        expect_key(&mut reader, 1, "channel_id")?;
        let channel_id = read_fixed::<32>(&mut reader, "channel_id")?;

        if !reader.finished() {
            return Err("trailing bytes after compute channel refund payload".into());
        }

        let payload = Self { channel_id };
        if payload.canonical_bytes()? != bytes {
            return Err("compute channel refund payload is not canonically encoded".into());
        }
        Ok(payload)
    }
}

pub fn derive_compute_channel_id_v1(open_transaction_id_v2: Hash32) -> Hash32 {
    let mut preimage = Vec::with_capacity(COMPUTE_CHANNEL_ID_DOMAIN.len() + 1 + 32);
    preimage.extend_from_slice(COMPUTE_CHANNEL_ID_DOMAIN);
    preimage.push(0);
    preimage.extend_from_slice(&open_transaction_id_v2);
    keccak256(&preimage)
}

fn expect_key(
    reader: &mut crate::nce::NceReader<'_>,
    expected: u64,
    label: &str,
) -> Result<(), String> {
    let actual = reader.unsigned()?;
    if actual != expected {
        return Err(format!(
            "invalid compute action payload {label} field key: expected {expected}, found {actual}"
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
            "compute action payload {label} must be exactly {N} bytes"
        ));
    }
    Ok(bytes.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute_usage_receipt_v1::{
        COMPUTE_USAGE_RECEIPT_OBJECT_TYPE, COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION,
    };
    use crate::nce::{encode_bytes, encode_envelope, encode_map, encode_unsigned};
    use k256::ecdsa::SigningKey;

    fn valid_open() -> ComputeChannelOpenPayloadV1 {
        let signing_key = SigningKey::from_slice(&[0x11; 32]).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let channel_public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();

        ComputeChannelOpenPayloadV1 {
            worker_id: [0x22; 32],
            operator_id: [0x33; 32],
            channel_public_key,
            worker_payment_account: [0x44; 20],
            authorized_amount: 1_000,
            expiry_height: 100,
            claim_deadline_height: 110,
            refund_available_height: 111,
            service_scope_commitment: [0x55; 32],
            model_scope_commitment: [0x66; 32],
            execution_profile_scope_commitment: [0x77; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        }
    }

    fn receipt(channel_id: Hash32) -> Vec<u8> {
        let payload = encode_map(&[
            (1, encode_unsigned(1)),
            (2, encode_bytes(&[0x01; 32])),
            (3, encode_bytes(&channel_id)),
            (4, encode_bytes(&[0x02; 32])),
            (5, encode_bytes(&[0x03; 32])),
            (6, encode_bytes(&[0x04; 32])),
            (7, encode_unsigned(2)),
            (8, encode_bytes(&[0x05; 32])),
            (9, encode_bytes(&100_u128.to_be_bytes())),
            (10, encode_bytes(&[0x06; 32])),
            (11, encode_bytes(&[0x07; 32])),
            (12, encode_bytes(&[0x08; 32])),
            (13, encode_bytes(&25_u128.to_be_bytes())),
            (14, encode_bytes(&[0x09; 32])),
            (15, encode_unsigned(200)),
            (16, encode_bytes(&[0x0a; 64])),
        ])
        .unwrap();

        encode_envelope(
            COMPUTE_USAGE_RECEIPT_OBJECT_TYPE,
            COMPUTE_USAGE_RECEIPT_SCHEMA_VERSION,
            payload,
        )
        .unwrap()
    }

    #[test]
    fn open_round_trips_and_validates() {
        let payload = valid_open();
        let bytes = payload.canonical_bytes().unwrap();
        assert_eq!(
            ComputeChannelOpenPayloadV1::from_canonical_bytes(&bytes).unwrap(),
            payload
        );
    }

    #[test]
    fn open_rejects_invalid_timeline_zero_value_and_bad_key() {
        let mut payload = valid_open();
        payload.authorized_amount = 0;
        assert!(payload.validate().is_err());

        let mut payload = valid_open();
        payload.refund_available_height = payload.claim_deadline_height;
        assert!(payload.validate().is_err());

        let mut payload = valid_open();
        payload.channel_public_key = [0_u8; 65];
        assert!(payload.validate().is_err());
    }

    #[test]
    fn settle_round_trips_with_structurally_valid_receipt() {
        let channel_id = [0x42; 32];
        let payload = ComputeChannelSettlePayloadV1 {
            channel_id,
            final_usage_receipt: receipt(channel_id),
        };
        let bytes = payload.canonical_bytes().unwrap();
        assert_eq!(
            ComputeChannelSettlePayloadV1::from_canonical_bytes(&bytes).unwrap(),
            payload
        );
    }

    #[test]
    fn settle_rejects_receipt_for_different_channel() {
        let payload = ComputeChannelSettlePayloadV1 {
            channel_id: [0x42; 32],
            final_usage_receipt: receipt([0x43; 32]),
        };
        assert!(payload.validate().is_err());
    }

    #[test]
    fn refund_round_trips() {
        let payload = ComputeChannelRefundPayloadV1 {
            channel_id: [0x55; 32],
        };
        let bytes = payload.canonical_bytes().unwrap();
        assert_eq!(
            ComputeChannelRefundPayloadV1::from_canonical_bytes(&bytes).unwrap(),
            payload
        );
    }

    #[test]
    fn native_compute_payload_vectors_match_locked_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-compute-action-payloads-v1.json"
        ))
        .unwrap();

        let channel_vector = &vectors["channel_id_derivation"];
        let open_tx_id: Hash32 =
            hex::decode(channel_vector["open_transaction_id_v2"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap();
        let expected_channel_id = channel_vector["channel_id"].as_str().unwrap();

        assert_eq!(
            hex::encode(derive_compute_channel_id_v1(open_tx_id)),
            expected_channel_id
        );

        let open = &vectors["open"];
        let channel_public_key: [u8; 65] =
            hex::decode(open["channel_public_key"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap();
        let open_payload = ComputeChannelOpenPayloadV1 {
            worker_id: [0x22; 32],
            operator_id: [0x33; 32],
            channel_public_key,
            worker_payment_account: [0x44; 20],
            authorized_amount: 1_000,
            expiry_height: 100,
            claim_deadline_height: 110,
            refund_available_height: 111,
            service_scope_commitment: [0x55; 32],
            model_scope_commitment: [0x66; 32],
            execution_profile_scope_commitment: [0x77; 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
        };

        assert_eq!(
            hex::encode(open_payload.canonical_bytes().unwrap()),
            open["canonical_hex"].as_str().unwrap()
        );

        let channel_id: Hash32 = hex::decode(expected_channel_id)
            .unwrap()
            .try_into()
            .unwrap();

        let receipt_bytes = hex::decode(
            vectors["usage_receipt_structural"]["canonical_hex"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ComputeUsageReceiptV1::from_canonical_bytes(&receipt_bytes)
                .unwrap()
                .channel_id,
            channel_id
        );

        let settle_payload = ComputeChannelSettlePayloadV1 {
            channel_id,
            final_usage_receipt: receipt_bytes,
        };
        assert_eq!(
            hex::encode(settle_payload.canonical_bytes().unwrap()),
            vectors["settle"]["canonical_hex"].as_str().unwrap()
        );

        let refund_payload = ComputeChannelRefundPayloadV1 { channel_id };
        assert_eq!(
            hex::encode(refund_payload.canonical_bytes().unwrap()),
            vectors["refund"]["canonical_hex"].as_str().unwrap()
        );

        assert_eq!(
            ComputeChannelOpenPayloadV1::from_canonical_bytes(
                &hex::decode(open["canonical_hex"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
            open_payload
        );
        assert_eq!(
            ComputeChannelSettlePayloadV1::from_canonical_bytes(
                &hex::decode(vectors["settle"]["canonical_hex"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
            settle_payload
        );
        assert_eq!(
            ComputeChannelRefundPayloadV1::from_canonical_bytes(
                &hex::decode(vectors["refund"]["canonical_hex"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
            refund_payload
        );
    }

    #[test]
    fn channel_id_derivation_is_domain_separated() {
        let tx_id = [0x99; 32];
        let derived = derive_compute_channel_id_v1(tx_id);
        assert_ne!(derived, tx_id);
        assert_ne!(derived, [0_u8; 32]);
    }
}
