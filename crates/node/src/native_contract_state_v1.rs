use crate::work::{keccak256, Hash32};
use std::collections::BTreeMap;

const CONTRACT_STATE_DOMAIN: &[u8] = b"NIAHCIA/CONTRACT-STATE/V1";
const CONTRACTS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/CONTRACTS-ROOT/V1";
const EMPTY_CONTRACTS_ROOT_DOMAIN: &[u8] = b"NIAHCIA/CONTRACTS-ROOT/V1/EMPTY";

pub type ContractId = [u8; 20];
pub type ContractStorageKey = [u8; 32];
pub type ContractStorageValue = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractStateV1 {
    pub contract_id: ContractId,
    pub balance: u128,
    pub runtime_id: u32,
    pub code: Vec<u8>,
    storage: BTreeMap<ContractStorageKey, ContractStorageValue>,
}

impl ContractStateV1 {
    pub fn new(
        contract_id: ContractId,
        balance: u128,
        runtime_id: u32,
        code: Vec<u8>,
    ) -> Self {
        Self {
            contract_id,
            balance,
            runtime_id,
            code,
            storage: BTreeMap::new(),
        }
    }

    pub fn storage(&self, key: ContractStorageKey) -> Option<ContractStorageValue> {
        self.storage.get(&key).copied()
    }

    pub fn set_storage(&mut self, key: ContractStorageKey, value: ContractStorageValue) {
        self.storage.insert(key, value);
    }

    pub fn remove_storage(
        &mut self,
        key: ContractStorageKey,
    ) -> Option<ContractStorageValue> {
        self.storage.remove(&key)
    }

    pub fn storage_count(&self) -> usize {
        self.storage.len()
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let code_len = u32::try_from(self.code.len())
            .map_err(|_| "contract code length exceeds u32".to_string())?;
        let storage_count = u64::try_from(self.storage.len())
            .map_err(|_| "contract storage entry count exceeds u64".to_string())?;

        let storage_bytes = self
            .storage
            .len()
            .checked_mul(64)
            .ok_or_else(|| "contract storage canonical length overflow".to_string())?;
        let mut out = Vec::with_capacity(
            20usize
                .saturating_add(16)
                .saturating_add(4)
                .saturating_add(4)
                .saturating_add(self.code.len())
                .saturating_add(8)
                .saturating_add(storage_bytes),
        );

        out.extend_from_slice(&self.contract_id);
        out.extend_from_slice(&self.balance.to_be_bytes());
        out.extend_from_slice(&self.runtime_id.to_be_bytes());
        out.extend_from_slice(&code_len.to_be_bytes());
        out.extend_from_slice(&self.code);
        out.extend_from_slice(&storage_count.to_be_bytes());
        for (key, value) in &self.storage {
            out.extend_from_slice(key);
            out.extend_from_slice(value);
        }
        Ok(out)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 52 {
            return Err("contract state record is truncated".into());
        }

        let mut offset = 0usize;
        let contract_id = take(bytes, &mut offset, 20, "contract id")?
            .try_into()
            .unwrap();
        let balance = u128::from_be_bytes(
            take(bytes, &mut offset, 16, "contract balance")?
                .try_into()
                .unwrap(),
        );
        let runtime_id = u32::from_be_bytes(
            take(bytes, &mut offset, 4, "contract runtime id")?
                .try_into()
                .unwrap(),
        );
        let code_len = u32::from_be_bytes(
            take(bytes, &mut offset, 4, "contract code length")?
                .try_into()
                .unwrap(),
        );
        let code_len = usize::try_from(code_len)
            .map_err(|_| "contract code length exceeds platform limits".to_string())?;
        let code = take(bytes, &mut offset, code_len, "contract code")?.to_vec();

        let storage_count = u64::from_be_bytes(
            take(bytes, &mut offset, 8, "contract storage count")?
                .try_into()
                .unwrap(),
        );
        let storage_count = usize::try_from(storage_count)
            .map_err(|_| "contract storage count exceeds platform limits".to_string())?;

        let mut storage = BTreeMap::new();
        let mut previous: Option<ContractStorageKey> = None;
        for _ in 0..storage_count {
            let key: ContractStorageKey = take(bytes, &mut offset, 32, "contract storage key")?
                .try_into()
                .unwrap();
            let value: ContractStorageValue =
                take(bytes, &mut offset, 32, "contract storage value")?
                    .try_into()
                    .unwrap();

            if let Some(previous_key) = previous {
                if key <= previous_key {
                    return Err("contract storage keys are not strictly ordered".into());
                }
            }
            previous = Some(key);
            storage.insert(key, value);
        }

        if offset != bytes.len() {
            return Err("contract state record contains trailing bytes".into());
        }

        Ok(Self {
            contract_id,
            balance,
            runtime_id,
            code,
            storage,
        })
    }

    pub fn record_hash(&self) -> Result<Hash32, String> {
        let encoded = self.canonical_bytes()?;
        let mut preimage = Vec::with_capacity(CONTRACT_STATE_DOMAIN.len() + encoded.len());
        preimage.extend_from_slice(CONTRACT_STATE_DOMAIN);
        preimage.extend_from_slice(&encoded);
        Ok(keccak256(&preimage))
    }
}

pub fn contracts_root_v1(
    contracts: &BTreeMap<ContractId, ContractStateV1>,
) -> Result<Hash32, String> {
    if contracts.is_empty() {
        return Ok(keccak256(EMPTY_CONTRACTS_ROOT_DOMAIN));
    }

    let mut preimage =
        Vec::with_capacity(CONTRACTS_ROOT_DOMAIN.len() + contracts.len().saturating_mul(52));
    preimage.extend_from_slice(CONTRACTS_ROOT_DOMAIN);
    for (contract_id, contract) in contracts {
        if contract.contract_id != *contract_id {
            return Err("contract map key does not match contract state id".into());
        }
        preimage.extend_from_slice(contract_id);
        preimage.extend_from_slice(&contract.record_hash()?);
    }
    Ok(keccak256(&preimage))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(marker: u8) -> ContractStateV1 {
        let mut contract = ContractStateV1::new(
            [marker; 20],
            1_234,
            1,
            vec![0x01, marker, 0x03, 0x04],
        );
        contract.set_storage([marker.wrapping_add(1); 32], [marker.wrapping_add(2); 32]);
        contract
    }

    #[test]
    fn contract_state_round_trips_canonically() {
        let contract = contract(0x33);
        let bytes = contract.canonical_bytes().unwrap();
        assert_eq!(ContractStateV1::from_canonical_bytes(&bytes).unwrap(), contract);
        assert_eq!(
            ContractStateV1::from_canonical_bytes(&bytes)
                .unwrap()
                .record_hash()
                .unwrap(),
            contract.record_hash().unwrap()
        );
    }

    #[test]
    fn storage_insertion_order_does_not_change_encoding_or_hash() {
        let mut first = ContractStateV1::new([0x44; 20], 0, 1, vec![0xaa]);
        first.set_storage([0x22; 32], [0x33; 32]);
        first.set_storage([0x11; 32], [0x55; 32]);

        let mut second = ContractStateV1::new([0x44; 20], 0, 1, vec![0xaa]);
        second.set_storage([0x11; 32], [0x55; 32]);
        second.set_storage([0x22; 32], [0x33; 32]);

        assert_eq!(first.canonical_bytes().unwrap(), second.canonical_bytes().unwrap());
        assert_eq!(first.record_hash().unwrap(), second.record_hash().unwrap());
    }

    #[test]
    fn contracts_root_is_order_independent_and_empty_root_is_stable() {
        let empty = BTreeMap::new();
        assert_eq!(
            contracts_root_v1(&empty).unwrap(),
            keccak256(EMPTY_CONTRACTS_ROOT_DOMAIN)
        );

        let mut first = BTreeMap::new();
        first.insert([0x22; 20], contract(0x22));
        first.insert([0x11; 20], contract(0x11));

        let mut second = BTreeMap::new();
        second.insert([0x11; 20], contract(0x11));
        second.insert([0x22; 20], contract(0x22));

        assert_eq!(
            contracts_root_v1(&first).unwrap(),
            contracts_root_v1(&second).unwrap()
        );
    }

    #[test]
    fn decoder_rejects_trailing_bytes() {
        let mut bytes = contract(0x33).canonical_bytes().unwrap();
        bytes.push(0);
        assert!(ContractStateV1::from_canonical_bytes(&bytes).is_err());
    }
}
