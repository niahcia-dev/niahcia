use crate::native_contract_state_v1::{contracts_root_v1, ContractId, ContractStateV1};
use crate::native_state_v2::NativeStateV2;
use crate::work::{keccak256, Hash32};
use std::collections::BTreeMap;

const STATE_ROOT_V3_DOMAIN: &[u8] = b"NIAHCIA/STATE-ROOT/V3";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeStateV3 {
    base: NativeStateV2,
    contracts: BTreeMap<ContractId, ContractStateV1>,
}

impl NativeStateV3 {
    pub fn from_v2(base: NativeStateV2) -> Self {
        Self {
            base,
            contracts: BTreeMap::new(),
        }
    }

    pub fn base(&self) -> &NativeStateV2 {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut NativeStateV2 {
        &mut self.base
    }

    pub fn contract(&self, contract_id: ContractId) -> Option<&ContractStateV1> {
        self.contracts.get(&contract_id)
    }

    pub fn contract_mut(&mut self, contract_id: ContractId) -> Option<&mut ContractStateV1> {
        self.contracts.get_mut(&contract_id)
    }

    pub fn insert_contract(&mut self, contract: ContractStateV1) -> Result<(), String> {
        if self.contracts.contains_key(&contract.contract_id) {
            return Err("contract state already exists at derived contract id".into());
        }
        self.contracts.insert(contract.contract_id, contract);
        Ok(())
    }

    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }

    pub fn contracts_root(&self) -> Result<Hash32, String> {
        contracts_root_v1(&self.contracts)
    }

    pub fn state_root(&self) -> Result<Hash32, String> {
        let base_root = self.base.state_root()?;
        let contracts_root = self.contracts_root()?;
        let mut preimage = Vec::with_capacity(STATE_ROOT_V3_DOMAIN.len() + 64);
        preimage.extend_from_slice(STATE_ROOT_V3_DOMAIN);
        preimage.extend_from_slice(&base_root);
        preimage.extend_from_slice(&contracts_root);
        Ok(keccak256(&preimage))
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let base = self.base.canonical_bytes()?;
        let base_len = u64::try_from(base.len())
            .map_err(|_| "native state V3 base snapshot exceeds u64".to_string())?;
        let contract_count = u64::try_from(self.contracts.len())
            .map_err(|_| "native state V3 contract count exceeds u64".to_string())?;

        let mut records = Vec::with_capacity(self.contracts.len());
        let mut records_len = 0usize;
        for contract in self.contracts.values() {
            let record = contract.canonical_bytes()?;
            records_len = records_len
                .checked_add(8)
                .and_then(|value| value.checked_add(record.len()))
                .ok_or_else(|| "native state V3 contract snapshot length overflow".to_string())?;
            records.push(record);
        }

        let mut out = Vec::with_capacity(
            1usize
                .saturating_add(8)
                .saturating_add(base.len())
                .saturating_add(8)
                .saturating_add(records_len),
        );
        out.push(3);
        out.extend_from_slice(&base_len.to_be_bytes());
        out.extend_from_slice(&base);
        out.extend_from_slice(&contract_count.to_be_bytes());
        for record in records {
            let record_len = u64::try_from(record.len())
                .map_err(|_| "native state V3 contract record exceeds u64".to_string())?;
            out.extend_from_slice(&record_len.to_be_bytes());
            out.extend_from_slice(&record);
        }
        Ok(out)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 17 {
            return Err("native state V3 snapshot is truncated".into());
        }
        if bytes[0] != 3 {
            return Err(format!(
                "unsupported native state V3 snapshot version: {}",
                bytes[0]
            ));
        }

        let mut offset = 1usize;
        let base_len = read_u64(bytes, &mut offset, "native state V3 base length")?;
        let base_len = usize::try_from(base_len)
            .map_err(|_| "native state V3 base length exceeds platform limits".to_string())?;
        let base = NativeStateV2::from_canonical_bytes(take(
            bytes,
            &mut offset,
            base_len,
            "native state V3 base snapshot",
        )?)?;

        let contract_count = read_u64(bytes, &mut offset, "native state V3 contract count")?;
        let contract_count = usize::try_from(contract_count)
            .map_err(|_| "native state V3 contract count exceeds platform limits".to_string())?;

        let mut contracts = BTreeMap::new();
        let mut previous: Option<ContractId> = None;
        for _ in 0..contract_count {
            let record_len =
                read_u64(bytes, &mut offset, "native state V3 contract record length")?;
            let record_len = usize::try_from(record_len).map_err(|_| {
                "native state V3 contract record length exceeds platform limits".to_string()
            })?;
            let contract = ContractStateV1::from_canonical_bytes(take(
                bytes,
                &mut offset,
                record_len,
                "native state V3 contract record",
            )?)?;

            if let Some(previous_id) = previous {
                if contract.contract_id <= previous_id {
                    return Err("native state V3 contracts are not strictly ordered".into());
                }
            }
            previous = Some(contract.contract_id);
            contracts.insert(contract.contract_id, contract);
        }

        if offset != bytes.len() {
            return Err("native state V3 snapshot contains trailing bytes".into());
        }

        Ok(Self { base, contracts })
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
    use crate::native_contract_state_v1::ContractStateV1;

    fn contract(marker: u8) -> ContractStateV1 {
        let mut contract = ContractStateV1::new([marker; 20], 1_234, 1, vec![0x01, marker, 0x03]);
        contract.set_storage([marker.wrapping_add(1); 32], [marker.wrapping_add(2); 32]);
        contract
    }

    #[test]
    fn v2_migration_preserves_base_and_starts_without_contracts() {
        let base = NativeStateV2::default();
        let state = NativeStateV3::from_v2(base.clone());
        assert_eq!(state.base(), &base);
        assert_eq!(state.contract_count(), 0);
    }

    #[test]
    fn contract_snapshot_round_trips_with_exact_root() {
        let mut state = NativeStateV3::from_v2(NativeStateV2::default());
        state.insert_contract(contract(0x33)).unwrap();

        let bytes = state.canonical_bytes().unwrap();
        let decoded = NativeStateV3::from_canonical_bytes(&bytes).unwrap();

        assert_eq!(decoded, state);
        assert_eq!(decoded.state_root().unwrap(), state.state_root().unwrap());
    }

    #[test]
    fn contract_state_probe_vector() {
        let mut contract = ContractStateV1::new([0x33; 20], 1_234, 1, vec![0x01, 0x02, 0x03, 0x04]);
        contract.set_storage([0x44; 32], [0x55; 32]);

        let contract_bytes = contract.canonical_bytes().unwrap();
        let contract_hash = contract.record_hash().unwrap();

        let mut state = NativeStateV3::from_v2(NativeStateV2::default());
        state.insert_contract(contract).unwrap();

        let actual = (
            hex::encode(contract_bytes),
            hex::encode(contract_hash),
            hex::encode(state.contracts_root().unwrap()),
            hex::encode(state.state_root().unwrap()),
            hex::encode(state.canonical_bytes().unwrap()),
        );

        assert_eq!(
            actual,
            (
                "TODO".to_string(),
                "TODO".to_string(),
                "TODO".to_string(),
                "TODO".to_string(),
                "TODO".to_string(),
            )
        );
    }

    #[test]
    fn duplicate_contract_id_is_rejected_without_replacement() {
        let mut state = NativeStateV3::default();
        state.insert_contract(contract(0x33)).unwrap();
        let before = state.clone();

        assert!(state.insert_contract(contract(0x33)).is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn contract_insertion_order_does_not_change_snapshot_or_root() {
        let mut first = NativeStateV3::default();
        first.insert_contract(contract(0x22)).unwrap();
        first.insert_contract(contract(0x11)).unwrap();

        let mut second = NativeStateV3::default();
        second.insert_contract(contract(0x11)).unwrap();
        second.insert_contract(contract(0x22)).unwrap();

        assert_eq!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
        assert_eq!(first.state_root().unwrap(), second.state_root().unwrap());
    }

    #[test]
    fn rejects_trailing_snapshot_bytes() {
        let mut bytes = NativeStateV3::default().canonical_bytes().unwrap();
        bytes.push(0);
        assert!(NativeStateV3::from_canonical_bytes(&bytes).is_err());
    }
}
