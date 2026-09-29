# NIAHCIA Block Header and Mining Work

## Purpose

NIAHCIA now uses the provisional Protocol v1 block header directly as the mining-work foundation.

The critical rule remains:

> Changing miner-controlled search values must not require EVM re-execution.

## BlockHeaderV1

The node implements the 164-byte fixed-width header defined by `niahcia-protocol/spec/block-header-v1.md`:

```text
version             u32
parent_hash         bytes32
height              u64
timestamp           u64
transactions_root   bytes32
execution_root      bytes32
target              bytes32
nonce               u64
extra_nonce         u64
```

All integer fields are big-endian.

## Block identity

```text
block_id =
  keccak256(
    "NIAHCIA/BLOCK-HEADER/V1"
    ||
    canonical_header_164_bytes
  )
```

The block ID belongs entirely to NIAHCIA.

Reth's execution payload block hash is not included.

## Mining template identity

Mining work is an unfinished `BlockHeaderV1`.

The miner may vary:

```text
nonce
extra_nonce
```

The template ID is computed after zeroing those two fields:

```text
keccak256(
  "NIAHCIA/MINING-TEMPLATE/V1"
  ||
  header_with_zero_nonce_and_extra_nonce
)
```

Therefore changing either miner value does not alter template identity and does not require execution to run again.

## Transaction root

The header now carries a NIAHCIA-native binary Merkle root over the ordered raw transaction bytes.

It does not use Reth's execution block hash and does not use Ethereum's transaction trie as NIAHCIA block identity.

## Execution root

`execution_root` commits to the execution results NIAHCIA requires from the EVM engine, including state and receipt roots plus execution metadata.

Reth is currently adapter #1 for producing those results.

## Target

Only the 256-bit target is consensus header data.

Human-readable difficulty is derived from target and is no longer carried as a separate mining-template field.

## Status

This is the agreed Protocol v1 consensus candidate.

RandomX seed derivation, timestamp validity, target adjustment, genesis/network parameters, and final execution-record encoding remain to be specified before a public testnet is considered consensus-stable.
