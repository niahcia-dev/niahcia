# NIAHCIA Native Contract State V1

Status: **CANDIDATE / VECTORED / REVIEW REQUIRED / INACTIVE**

## Purpose

This specification defines the deterministic persistent state container required by the native smart-contract runtime without selecting the final VM/instruction technology.

It deliberately preserves locked NativeStateV2 behavior. Contract-capable state is introduced as a successor NativeStateV3 boundary rather than changing NativeStateV2 bytes or roots in place.

## Contract identifier

A contract is keyed by the 20-byte Contract Address V1 payload produced by the locked ContractCreate derivation rule.

The Bech32m text form is not stored in consensus state.

## Contract record

ContractStateV1 contains:

```text
contract_id    20 bytes
balance        u128 big-endian
runtime_id     u32 big-endian
code_len       u32 big-endian
code           code_len opaque bytes
storage_count  u64 big-endian
storage[]      ordered (32-byte key, 32-byte value)
```

The contract code bytes are opaque at this state layer. The eventual runtime specification assigns valid runtime identifiers, code validation rules, and execution semantics.

This state definition therefore does not prematurely select EVM, WASM, or another VM.

## Native value

Each contract record owns an integer `aniah` balance.

Contract value is separate from Account Address V1 balances and from ComputeChannel locked value.

Future ContractCall / ContractCreate execution must move value between authenticated accounts and contract balances atomically according to the active runtime rules.

## Storage

V1 persistent contract storage uses exactly:

```text
key   = 32 bytes
value = 32 bytes
```

Keys are serialized in strict ascending byte order.

The state layer makes no interpretation of key/value contents.

A future runtime may expose higher-level language types, but consensus persistence remains fixed-width canonical bytes for Contract State V1.

## Contract record commitment

Define:

```text
record_hash =
    Keccak-256(
        "NIAHCIA/CONTRACT-STATE/V1" ||
        canonical_contract_record
    )
```

## Contracts root

For zero contracts:

```text
contracts_root =
    Keccak-256("NIAHCIA/CONTRACTS-ROOT/V1/EMPTY")
```

For one or more contracts, concatenate entries in ascending contract-id order:

```text
contracts_root =
    Keccak-256(
        "NIAHCIA/CONTRACTS-ROOT/V1" ||
        contract_id_0 || record_hash_0 ||
        ...
        contract_id_n || record_hash_n
    )
```

A map key that differs from the embedded ContractStateV1 contract ID is invalid.

## NativeStateV3

NativeStateV3 is the candidate contract-capable successor state:

```text
NativeStateV3 {
    base: NativeStateV2,
    contracts: ordered map<ContractId, ContractStateV1>
}
```

Its state root is:

```text
state_root_v3 =
    Keccak-256(
        "NIAHCIA/STATE-ROOT/V3" ||
        native_state_v2_root ||
        contracts_root
    )
```

This preserves the complete V2 accounts + ComputeChannel commitment as one component and adds the contract commitment as a separate component.

## V2 -> V3 migration

The deterministic migration starts with:

```text
base_v3      = exact canonical NativeStateV2
contracts_v3 = empty
```

No Account, ComputeChannel, balance, nonce, or V2 root semantics are rewritten.

## Canonical snapshot

NativeStateV3 snapshot version byte is `0x03`.

The snapshot is:

```text
version          u8 = 3
base_len         u64 big-endian
base_v2          exact canonical NativeStateV2 bytes
contract_count   u64 big-endian
contracts[]:
    record_len   u64 big-endian
    record       canonical ContractStateV1 bytes
```

Contract records MUST be strictly ordered by contract ID.

Storage entries within each record MUST be strictly ordered by storage key.

Trailing bytes are invalid.

## Collision rule

Insertion of a contract whose derived Contract Address V1 payload already has persistent contract state is invalid.

The existing contract record is never overwritten or merged by ContractCreate.

## Locked interoperability vector

The canonical fixture is:

`test-vectors/native-contract-state-v1.json`

and is enforced by the Rust `locked_contract_state_vector_matches_json` test.

It locks:

- ContractStateV1 canonical bytes;
- ContractStateV1 record hash;
- ordered contracts root;
- NativeStateV3 root;
- NativeStateV3 canonical snapshot bytes.

The fixture's `runtime_id = 1` is only a state-format test value. It does not select, define, or activate a VM.

## Runtime boundary

This state model does not activate ContractCreate or ContractCall.

Still separately required:

- runtime/code validation semantics;
- creation payload format;
- call input/output ABI boundary;
- success/revert/trap/out-of-gas rules;
- contract gas schedule;
- nested call/reentrancy policy;
- memory/stack/code/storage limits;
- event/log representation if supported;
- receipts/execution commitment integration;
- persistence/restart/reorg tests;
- activation vectors.

## Compatibility

NativeStateV2 remains unchanged.

Any incompatible change to ContractStateV1 encoding, Contract Storage V1 key/value widths, contracts-root commitment, or NativeStateV3 root/snapshot encoding requires an explicit successor version.
