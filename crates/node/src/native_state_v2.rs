use crate::native_execution::{AccountId, AccountStateV1, NativeStateV1};
use crate::work::{keccak256, Hash32};
use std::collections::BTreeMap;

const CHANNEL_STATE_DOMAIN: &[u8] = b"NIAHCIA/COMPUTE-CHANNEL-STATE/V1";
const CHANNELS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/COMPUTE-CHANNELS-ROOT/V1";
const EMPTY_CHANNELS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/COMPUTE-CHANNELS-ROOT/V1/EMPTY";
const STATE_ROOT_V2_DOMAIN: &[u8] = b"NIAHCIA/STATE-ROOT/V2";

pub type ChannelId = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ComputeChannelSettlementPolicyV1 {
    CumulativeReceipt = 0x00,
}

impl TryFrom<u8> for ComputeChannelSettlementPolicyV1 {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::CumulativeReceipt),
            other => Err(format!("unknown compute channel settlement policy: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ComputeChannelStatusV1 {
    Open = 0x00,
    Settled = 0x01,
    Refunded = 0x02,
}

impl TryFrom<u8> for ComputeChannelStatusV1 {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Open),
            0x01 => Ok(Self::Settled),
            0x02 => Ok(Self::Refunded),
            other => Err(format!("unknown compute channel state: {other}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeChannelStateV1 {
    pub channel_id: ChannelId,
    pub funding_account: AccountId,
    pub worker_id: Hash32,
    pub operator_id: Hash32,
    pub channel_public_key: [u8; 65],
    pub worker_payment_account: AccountId,
    pub authorized_amount: u128,
    pub settled_amount: u128,
    pub opened_height: u64,
    pub expiry_height: u64,
    pub claim_deadline_height: u64,
    pub refund_available_height: u64,
    pub service_scope_commitment: Hash32,
    pub model_scope_commitment: Hash32,
    pub execution_profile_scope_commitment: Hash32,
    pub settlement_policy: ComputeChannelSettlementPolicyV1,
    pub state: ComputeChannelStatusV1,
}

impl ComputeChannelStateV1 {
    pub const CANONICAL_LEN: usize = 363;

    pub fn validate(&self) -> Result<(), String> {
        if self.settled_amount > self.authorized_amount {
            return Err("compute channel settled amount exceeds authorized amount".into());
        }
        if self.opened_height >= self.expiry_height {
            return Err("compute channel expiry must be after open height".into());
        }
        if self.expiry_height > self.claim_deadline_height {
            return Err("compute channel claim deadline precedes expiry".into());
        }
        if self.claim_deadline_height >= self.refund_available_height {
            return Err("compute channel refund height must be after claim deadline".into());
        }

        match self.state {
            ComputeChannelStatusV1::Open if self.settled_amount != 0 => {
                return Err("open compute channel cannot have settled value".into());
            }
            ComputeChannelStatusV1::Refunded if self.settled_amount != 0 => {
                return Err("refunded compute channel cannot have settled value".into());
            }
            _ => {}
        }

        if self.channel_public_key[0] != 0x04 {
            return Err("compute channel public key must be uncompressed SEC1".into());
        }

        Ok(())
    }

    pub fn record_hash(&self) -> Result<Hash32, String> {
        self.validate()?;

        let mut preimage = Vec::with_capacity(CHANNEL_STATE_DOMAIN.len() + Self::CANONICAL_LEN);
        preimage.extend_from_slice(CHANNEL_STATE_DOMAIN);
        self.append_fields(&mut preimage);
        Ok(keccak256(&preimage))
    }

    fn append_fields(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.channel_id);
        out.extend_from_slice(&self.funding_account);
        out.extend_from_slice(&self.worker_id);
        out.extend_from_slice(&self.operator_id);
        out.extend_from_slice(&self.channel_public_key);
        out.extend_from_slice(&self.worker_payment_account);
        out.extend_from_slice(&self.authorized_amount.to_be_bytes());
        out.extend_from_slice(&self.settled_amount.to_be_bytes());
        out.extend_from_slice(&self.opened_height.to_be_bytes());
        out.extend_from_slice(&self.expiry_height.to_be_bytes());
        out.extend_from_slice(&self.claim_deadline_height.to_be_bytes());
        out.extend_from_slice(&self.refund_available_height.to_be_bytes());
        out.extend_from_slice(&self.service_scope_commitment);
        out.extend_from_slice(&self.model_scope_commitment);
        out.extend_from_slice(&self.execution_profile_scope_commitment);
        out.push(self.settlement_policy as u8);
        out.push(self.state as u8);
    }

    fn from_record_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != Self::CANONICAL_LEN {
            return Err("invalid compute channel state record length".into());
        }

        let mut offset = 0usize;
        macro_rules! take {
            ($len:expr) => {{
                let start = offset;
                let end = start + $len;
                offset = end;
                &bytes[start..end]
            }};
        }

        let channel = Self {
            channel_id: take!(32).try_into().unwrap(),
            funding_account: take!(20).try_into().unwrap(),
            worker_id: take!(32).try_into().unwrap(),
            operator_id: take!(32).try_into().unwrap(),
            channel_public_key: take!(65).try_into().unwrap(),
            worker_payment_account: take!(20).try_into().unwrap(),
            authorized_amount: u128::from_be_bytes(take!(16).try_into().unwrap()),
            settled_amount: u128::from_be_bytes(take!(16).try_into().unwrap()),
            opened_height: u64::from_be_bytes(take!(8).try_into().unwrap()),
            expiry_height: u64::from_be_bytes(take!(8).try_into().unwrap()),
            claim_deadline_height: u64::from_be_bytes(take!(8).try_into().unwrap()),
            refund_available_height: u64::from_be_bytes(take!(8).try_into().unwrap()),
            service_scope_commitment: take!(32).try_into().unwrap(),
            model_scope_commitment: take!(32).try_into().unwrap(),
            execution_profile_scope_commitment: take!(32).try_into().unwrap(),
            settlement_policy: ComputeChannelSettlementPolicyV1::try_from(take!(1)[0])?,
            state: ComputeChannelStatusV1::try_from(take!(1)[0])?,
        };

        debug_assert_eq!(offset, Self::CANONICAL_LEN);
        channel.validate()?;
        Ok(channel)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeStateV2 {
    accounts: NativeStateV1,
    channels: BTreeMap<ChannelId, ComputeChannelStateV1>,
}

impl NativeStateV2 {
    pub fn from_v1(accounts: NativeStateV1) -> Self {
        Self {
            accounts,
            channels: BTreeMap::new(),
        }
    }

    pub fn accounts(&self) -> &NativeStateV1 {
        &self.accounts
    }

    pub fn accounts_mut(&mut self) -> &mut NativeStateV1 {
        &mut self.accounts
    }

    pub fn channel(&self, channel_id: ChannelId) -> Option<&ComputeChannelStateV1> {
        self.channels.get(&channel_id)
    }

    pub fn set_channel(&mut self, channel: ComputeChannelStateV1) -> Result<(), String> {
        channel.validate()?;
        self.channels.insert(channel.channel_id, channel);
        Ok(())
    }

    pub fn remove_channel(&mut self, channel_id: ChannelId) -> Option<ComputeChannelStateV1> {
        self.channels.remove(&channel_id)
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn channels_root(&self) -> Result<Hash32, String> {
        if self.channels.is_empty() {
            return Ok(keccak256(EMPTY_CHANNELS_ROOT_DOMAIN));
        }

        let mut preimage =
            Vec::with_capacity(CHANNELS_ROOT_DOMAIN.len() + self.channels.len() * 64);
        preimage.extend_from_slice(CHANNELS_ROOT_DOMAIN);

        for (channel_id, channel) in &self.channels {
            preimage.extend_from_slice(channel_id);
            preimage.extend_from_slice(&channel.record_hash()?);
        }

        Ok(keccak256(&preimage))
    }

    pub fn state_root(&self) -> Result<Hash32, String> {
        let accounts_root = self.accounts.state_root();
        let channels_root = self.channels_root()?;

        let mut preimage = Vec::with_capacity(STATE_ROOT_V2_DOMAIN.len() + 64);
        preimage.extend_from_slice(STATE_ROOT_V2_DOMAIN);
        preimage.extend_from_slice(&accounts_root);
        preimage.extend_from_slice(&channels_root);
        Ok(keccak256(&preimage))
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let account_count = u64::try_from(self.accounts.account_count())
            .map_err(|_| "native state V2 account count exceeds u64".to_string())?;
        let channel_count = u64::try_from(self.channels.len())
            .map_err(|_| "native state V2 channel count exceeds u64".to_string())?;

        let account_bytes = self
            .accounts
            .account_count()
            .checked_mul(44)
            .ok_or_else(|| "native state V2 account snapshot length overflow".to_string())?;
        let channel_bytes = self
            .channels
            .len()
            .checked_mul(ComputeChannelStateV1::CANONICAL_LEN)
            .ok_or_else(|| "native state V2 channel snapshot length overflow".to_string())?;

        let mut out = Vec::with_capacity(
            1usize
                .saturating_add(8)
                .saturating_add(account_bytes)
                .saturating_add(8)
                .saturating_add(channel_bytes),
        );

        out.push(2);
        out.extend_from_slice(&account_count.to_be_bytes());

        for (account, state) in self.accounts.accounts_iter() {
            out.extend_from_slice(account);
            out.extend_from_slice(&state.balance.to_be_bytes());
            out.extend_from_slice(&state.nonce.to_be_bytes());
        }

        out.extend_from_slice(&channel_count.to_be_bytes());
        for channel in self.channels.values() {
            channel.validate()?;
            channel.append_fields(&mut out);
        }

        Ok(out)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 17 {
            return Err("native state V2 snapshot is truncated".into());
        }
        if bytes[0] != 2 {
            return Err(format!(
                "unsupported native state V2 snapshot version: {}",
                bytes[0]
            ));
        }

        let mut offset = 1usize;

        let account_count = read_u64(bytes, &mut offset, "native state V2 account count")?;
        let account_count = usize::try_from(account_count)
            .map_err(|_| "native state V2 account count exceeds platform limits".to_string())?;

        let mut accounts = NativeStateV1::default();
        let mut previous_account: Option<AccountId> = None;

        for _ in 0..account_count {
            let account: AccountId =
                take(bytes, &mut offset, 20, "native state V2 account")?
                    .try_into()
                    .unwrap();
            let balance = u128::from_be_bytes(
                take(bytes, &mut offset, 16, "native state V2 account balance")?
                    .try_into()
                    .unwrap(),
            );
            let nonce = u64::from_be_bytes(
                take(bytes, &mut offset, 8, "native state V2 account nonce")?
                    .try_into()
                    .unwrap(),
            );

            if let Some(previous) = previous_account {
                if account <= previous {
                    return Err("native state V2 accounts are not strictly ordered".into());
                }
            }

            let account_state = AccountStateV1 { balance, nonce };
            if account_state == AccountStateV1::default() {
                return Err("native state V2 contains a default account".into());
            }

            accounts.set_account(account, account_state);
            previous_account = Some(account);
        }

        let channel_count = read_u64(bytes, &mut offset, "native state V2 channel count")?;
        let channel_count = usize::try_from(channel_count)
            .map_err(|_| "native state V2 channel count exceeds platform limits".to_string())?;

        let mut channels = BTreeMap::new();
        let mut previous_channel: Option<ChannelId> = None;

        for _ in 0..channel_count {
            let record = take(
                bytes,
                &mut offset,
                ComputeChannelStateV1::CANONICAL_LEN,
                "native state V2 channel",
            )?;
            let channel = ComputeChannelStateV1::from_record_bytes(record)?;

            if let Some(previous) = previous_channel {
                if channel.channel_id <= previous {
                    return Err("native state V2 channels are not strictly ordered".into());
                }
            }

            previous_channel = Some(channel.channel_id);
            channels.insert(channel.channel_id, channel);
        }

        if offset != bytes.len() {
            return Err("native state V2 snapshot contains trailing bytes".into());
        }

        Ok(Self { accounts, channels })
    }
}

fn take<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    len: usize,
    label: &str,
) -> Result<&'a [u8], String> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("{label} length overflow"))?;
    if end > bytes.len() {
        return Err(format!("{label} is truncated"));
    }
    let out = &bytes[*offset..end];
    *offset = end;
    Ok(out)
}

fn read_u64(bytes: &[u8], offset: &mut usize, label: &str) -> Result<u64, String> {
    Ok(u64::from_be_bytes(
        take(bytes, offset, 8, label)?.try_into().unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(marker: u8) -> ComputeChannelStateV1 {
        let mut public_key = [0_u8; 65];
        public_key[0] = 0x04;
        public_key[1..].fill(marker);

        ComputeChannelStateV1 {
            channel_id: [marker; 32],
            funding_account: [marker.wrapping_add(1); 20],
            worker_id: [marker.wrapping_add(2); 32],
            operator_id: [marker.wrapping_add(3); 32],
            channel_public_key: public_key,
            worker_payment_account: [marker.wrapping_add(4); 20],
            authorized_amount: 1_000,
            settled_amount: 0,
            opened_height: 10,
            expiry_height: 20,
            claim_deadline_height: 25,
            refund_available_height: 26,
            service_scope_commitment: [marker.wrapping_add(5); 32],
            model_scope_commitment: [marker.wrapping_add(6); 32],
            execution_profile_scope_commitment: [marker.wrapping_add(7); 32],
            settlement_policy: ComputeChannelSettlementPolicyV1::CumulativeReceipt,
            state: ComputeChannelStatusV1::Open,
        }
    }

    #[test]
    fn v1_migration_keeps_accounts_and_starts_with_empty_channels() {
        let mut v1 = NativeStateV1::default();
        v1.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 123,
                nonce: 7,
            },
        );

        let v2 = NativeStateV2::from_v1(v1.clone());
        assert_eq!(v2.accounts(), &v1);
        assert_eq!(v2.channel_count(), 0);

        let mut expected = Vec::new();
        expected.extend_from_slice(STATE_ROOT_V2_DOMAIN);
        expected.extend_from_slice(&v1.state_root());
        expected.extend_from_slice(&keccak256(EMPTY_CHANNELS_ROOT_DOMAIN));

        assert_eq!(v2.state_root().unwrap(), keccak256(&expected));
    }

    #[test]
    fn empty_v2_snapshot_round_trips() {
        let state = NativeStateV2::default();
        let bytes = state.canonical_bytes().unwrap();

        assert_eq!(bytes[0], 2);
        assert_eq!(NativeStateV2::from_canonical_bytes(&bytes).unwrap(), state);
    }

    #[test]
    fn channel_snapshot_and_root_round_trip() {
        let mut state = NativeStateV2::default();
        state.set_channel(channel(0x22)).unwrap();

        let bytes = state.canonical_bytes().unwrap();
        let decoded = NativeStateV2::from_canonical_bytes(&bytes).unwrap();

        assert_eq!(decoded, state);
        assert_eq!(decoded.state_root().unwrap(), state.state_root().unwrap());
        assert_eq!(decoded.channels_root().unwrap(), state.channels_root().unwrap());
    }

    #[test]
    fn channel_insertion_order_does_not_change_snapshot_or_root() {
        let mut first = NativeStateV2::default();
        first.set_channel(channel(0x22)).unwrap();
        first.set_channel(channel(0x11)).unwrap();

        let mut second = NativeStateV2::default();
        second.set_channel(channel(0x11)).unwrap();
        second.set_channel(channel(0x22)).unwrap();

        assert_eq!(first.canonical_bytes().unwrap(), second.canonical_bytes().unwrap());
        assert_eq!(first.state_root().unwrap(), second.state_root().unwrap());
    }

    #[test]
    fn rejects_invalid_channel_timeline() {
        let mut invalid = channel(0x22);
        invalid.refund_available_height = invalid.claim_deadline_height;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn rejects_trailing_snapshot_bytes() {
        let mut bytes = NativeStateV2::default().canonical_bytes().unwrap();
        bytes.push(0);
        assert!(NativeStateV2::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn rejects_out_of_order_channel_records() {
        let mut state = NativeStateV2::default();
        state.set_channel(channel(0x11)).unwrap();
        state.set_channel(channel(0x22)).unwrap();
        let mut bytes = state.canonical_bytes().unwrap();

        let channel_count_offset = 1 + 8;
        let channel_count = u64::from_be_bytes(
            bytes[channel_count_offset..channel_count_offset + 8]
                .try_into()
                .unwrap(),
        );
        assert_eq!(channel_count, 2);

        let first = channel_count_offset + 8;
        let second = first + ComputeChannelStateV1::CANONICAL_LEN;
        let left = bytes[first..second].to_vec();
        let right = bytes[second..second + ComputeChannelStateV1::CANONICAL_LEN].to_vec();
        bytes[first..second].copy_from_slice(&right);
        bytes[second..second + ComputeChannelStateV1::CANONICAL_LEN].copy_from_slice(&left);

        assert!(NativeStateV2::from_canonical_bytes(&bytes).is_err());
    }
}
