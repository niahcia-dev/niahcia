# NIAHCIA Block Header V1

## Status

Draft consensus candidate for Protocol v1.

This document defines the first NIAHCIA-owned proof-of-work block header.

The governing rule is:

> **Reth calculates EVM execution results. NIAHCIA defines the block.**

No Ethereum block hash, beacon-chain field, validator field, or Reth-specific identifier is part of NIAHCIA block identity.

## Header fields

The V1 header is fixed-width:

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

Total canonical header size:

```text
164 bytes
```

All integers are unsigned big-endian.

## Canonical field order

The canonical 164-byte representation is exactly:

```text
version_u32_be
||
parent_hash
||
height_u64_be
||
timestamp_u64_be
||
transactions_root
||
execution_root
||
target
||
nonce_u64_be
||
extra_nonce_u64_be
```

No length prefixes, CBOR envelope, JSON representation, padding, or implementation-specific structure layout is used for this consensus header.

This fixed-width header is an explicit consensus-format exception to the general NCE/1 object encoding used by higher-level NIAHCIA protocol objects.

## Block identifier

The block identifier is:

```text
block_id =
  keccak256(
    "NIAHCIA/BLOCK-HEADER/V1"
    ||
    canonical_header_164_bytes
  )
```

The ASCII domain string is included exactly as shown and is case-sensitive.

## Mining template identity

A mining template is the same header with miner-controlled fields treated as mutable:

```text
nonce
extra_nonce
```

Template identity is:

```text
template_id =
  keccak256(
    "NIAHCIA/MINING-TEMPLATE/V1"
    ||
    canonical_header_with_nonce_zero
                         _and_extra_nonce_zero
  )
```

Changing either miner-controlled value MUST NOT require EVM re-execution.

## Field semantics

### version

Consensus header format version.

V1 is encoded as integer `1`.

### parent_hash

The previous **NIAHCIA block ID**.

It is never a Reth or Ethereum block hash.

For genesis:

```text
parent_hash = 0x00...00
```

### height

Genesis is height 0.

Every non-genesis block MUST satisfy:

```text
height = parent.height + 1
```

### timestamp

Unix timestamp in seconds.

Exact median-time and future-drift validity rules are specified separately.

### transactions_root

NIAHCIA-native Merkle commitment to the ordered canonical transaction bytes in the block body.

See `transaction-merkle-v1.md`.

### execution_root

NIAHCIA-native commitment to deterministic EVM execution results.

The execution engine implementation does not define block identity.

### target

Unsigned 256-bit proof-of-work target, encoded as exactly 32 big-endian bytes.

A human-facing "difficulty" value is derived from target and is not a header field.

### nonce

64-bit miner-controlled search value.

### extra_nonce

Second 64-bit miner-controlled search value for pool/work partitioning and expanded search space.

Together, nonce and extra_nonce provide 128 bits of mutable mining search space without rebuilding the execution payload.

## Explicitly excluded

The V1 header does not contain:

- Reth execution block hash
- Ethereum block hash
- Ethereum beacon block root
- `prevRandao`
- Ethereum validator/finality data
- Ethereum difficulty
- Ethereum nonce
- AI work counters
- compute-worker votes
- service-node votes
- miner software version
- transaction count
- chain ID

These values either do not belong to NIAHCIA consensus identity or are committed elsewhere.

## PoW

The PoW algorithm is specified separately.

For Prototype 0 the intended algorithm is RandomX.

The PoW preimage is the canonical NIAHCIA block header. RandomX parameters, seed derivation, and target comparison rules must be fixed before public testnet consensus is declared stable.

## Genesis and network identity

Devnet, testnet, and mainnet MUST have distinct genesis blocks.

The genesis block ID is the root identity anchor for the corresponding NIAHCIA network.

A later network-parameters specification will define chain IDs and whether the genesis ID also participates explicitly in network-domain hashing.

## Upgrade rule

A future incompatible header change requires a new version and activation rule.

V1 bytes MUST never be silently reinterpreted as a later header format.
