# Consensus Development Constants

The current Prototype 0 consensus candidates are:

```text
target block interval   30 seconds
median-time window      11 ancestors
maximum future drift    90 seconds
RandomX epoch length    2048 blocks
RandomX seed lag        64 blocks
```

## RandomX work

The miner hashes exactly the canonical 164-byte `BlockHeaderV1`.

The node supplies the RandomX seed and seed height with `pow_getWork`.

For height `h`:

```text
epoch_start = floor(h / 2048) * 2048
seed_height = max(0, epoch_start - 64)
seed = keccak256("NIAHCIA/RANDOMX-SEED/V1" || seed_block_id)
```

The node now resolves the seed block from persisted NIAHCIA canonical chain history whenever a chain head exists. Only the initial empty-chain/genesis bootstrap still uses the zero block ID as a provisional seed source until genesis/network parameters are frozen. The seed formula itself is implemented and unit-tested.

## Target comparison

RandomX output and target are interpreted as unsigned 256-bit big-endian values:

```text
valid_pow iff pow_hash <= target
```

## Timestamp validation

A candidate timestamp must be greater than median time past and no more than 90 seconds ahead of adjusted node time.

The helper logic is implemented and tested; it is not yet wired into block acceptance because block submission/persistent chain state do not exist yet.

## Difficulty

The protocol repo contains the initial 60-block / 30-minute adjustment candidate.

It remains deliberately marked for simulation rather than being silently frozen into consensus.
