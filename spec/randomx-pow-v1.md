# NIAHCIA RandomX Proof-of-Work V1

## Status

Draft consensus candidate for Protocol v1.

This document defines the NIAHCIA-specific rules around RandomX. It does not redefine the RandomX algorithm itself.

## PoW input

The RandomX input is exactly the 164-byte canonical `BlockHeaderV1` byte sequence.

No Reth identifier, Ethereum identifier, JSON representation, length prefix, or hidden salt is added.

```text
pow_input = canonical_block_header_164_bytes
pow_hash  = RandomX(randomx_seed, pow_input)
```

The header includes both miner-controlled fields:

```text
nonce
extra_nonce
```

Changing either field does not require EVM re-execution.

## Target comparison

The 32-byte RandomX output and 32-byte header target are interpreted as unsigned 256-bit **big-endian** integers.

A proof is valid when:

```text
pow_hash <= target
```

Byte-for-byte comparison may be implemented as unsigned lexicographic comparison because both values are fixed-width big-endian 32-byte integers.

## Initial development epoch parameters

Prototype 0 uses the following development parameters:

```text
RANDOMX_EPOCH_LENGTH = 2048 blocks
RANDOMX_SEED_LAG     = 64 blocks
```

At the 30-second target interval, an epoch is approximately 17 hours.

These numbers are development consensus candidates and MUST be benchmarked before public testnet rules are frozen.

## Epoch and seed height

For block height `h`:

```text
epoch_start = floor(h / 2048) * 2048

seed_height =
  max(0, epoch_start - 64)
```

The seed source is the NIAHCIA block ID at `seed_height`.

For early heights where `epoch_start < 64`, the genesis block ID is used.

## Seed derivation

```text
randomx_seed =
  keccak256(
    "NIAHCIA/RANDOMX-SEED/V1"
    ||
    seed_block_id
  )
```

The seed is derived only from NIAHCIA chain history.

It MUST NOT depend on:

- Reth state
- Ethereum block hashes
- beacon-chain randomness
- wall-clock randomness
- miner-provided randomness
- developer servers

## Genesis handling

Genesis itself is not required to satisfy RandomX PoW unless the network-parameters specification explicitly chooses to mine the published genesis header.

For block 1 and subsequent early blocks, the genesis block ID is the RandomX seed source until the first delayed epoch seed becomes available.

## Block work

Fork choice uses cumulative work, not block count.

For target `T`:

```text
block_work = floor((2^256 - 1) / (T + 1)) + 1
```

Equivalent mathematically exact formulations are acceptable if they produce identical integer results for every target.

```text
chain_work = parent_chain_work + block_work
```

The valid chain with the greatest cumulative work is preferred.

## Implementation boundary

RandomX MUST be isolated behind a PoW implementation boundary.

Consensus code should conceptually consume:

```text
seed_for_height(height, chain)
verify_pow(header, seed)
block_work(target)
```

rather than spreading RandomX-specific assumptions through execution, storage, RPC, or P2P code.

## Test vectors

Before public testnet, vectors MUST cover:

- seed height around genesis
- seed height immediately before/after an epoch boundary
- seed derivation
- target comparison
- at least one known valid RandomX header/hash pair
- at least one known invalid RandomX header/hash pair
- block-work calculation


## Locked interoperability vector

The current RandomX conformance fixture is `test-vectors/randomx-pow-v1.json`.

It fixes a 32-byte seed, an exact 164-byte `BlockHeaderV1`, the expected RandomX output, and valid/invalid target comparisons. Seed derivation remains independently testable through the `NIAHCIA/RANDOMX-SEED/V1` domain rule above.
