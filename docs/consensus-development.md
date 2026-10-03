# Consensus Development Constants

These are pre-alpha development candidates, not frozen production parameters.

```text
target block interval   approximately 30 seconds
RandomX                 CPU PoW
fork choice             highest cumulative valid work
```

## RandomX

The active devnet implementation uses RandomX and supplies seed/seed-height information through mining work.

The current development header/vector path is useful for testing, but production miner/pool interoperability and final RandomX epoch/seed parameters remain under review.

Do not treat current constants or development vectors as public-network freeze decisions.

## Target comparison

RandomX output and target are interpreted deterministically under the active implementation. Final interoperability vectors must remain synchronized with the production mining preimage.

## Timestamp / difficulty

Timestamp and difficulty policy remain review items. ASERT is the leading difficulty direction, but timestamp-adversary behavior must be resolved before production freeze.

## Network parameters

Production genesis, PoW limits, activation heights, RandomX seed schedule, and final chain ID/network parameters remain unfrozen.
