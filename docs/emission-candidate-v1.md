# NIAHCIA Emission Candidate V1

Status: **numerical candidate for simulation and canonical-vector implementation; not production-final until exact integer vectors pass**

This document turns the architecture in `monetary-policy-v1.md` into a concrete candidate that can be implemented and tested without pretending the economics are final before simulation.

## Candidate constants

- target block interval: `30 seconds`
- atomic units per NIAH: `10^8 aniah`
- main-emission reference ceiling: `41,943,040 NIAH`
- emission shift: `22`
- initial theoretical subsidy: `10 NIAH/block`
- tail subsidy: `0.25 NIAH/block`
- genesis production allocation: `0 NIAH`

The reference ceiling is chosen so that `41,943,040 NIAH >> 22 = 10 NIAH` when expressed in exact atomic-unit integer arithmetic.

## Exact candidate formula

Let:

- `M = 41,943,040 * 10^8` aniah
- `A = cumulative CPU-PoW main-emission subsidy already issued` in aniah
- `TAIL = 25,000,000` aniah

For each candidate canonical block:

`decay_reward = (M - min(A, M)) >> 22`

`cpu_subsidy = max(decay_reward, TAIL)`

All operations are unsigned checked integer operations. `>> 22` floors deterministically at the 8-decimal atomic-unit boundary.

Once `decay_reward <= TAIL`, the subsidy remains exactly `TAIL` forever unless a later explicitly activated consensus version changes the monetary policy.

## Approximate behavior

At the 30-second target there are approximately `1,051,920` blocks/year.

| Elapsed target time | Approx. cumulative CPU issuance | Approx. subsidy |
| --- | ---: | ---: |
| launch | 0 NIAH | 10.00000000 NIAH/block |
| 1 year | 9.30M NIAH | 7.78 NIAH/block |
| 5 years | 29.97M NIAH | 2.85 NIAH/block |
| 10 years | 38.53M NIAH | 0.814 NIAH/block |
| ~14.7 years | 40.894M NIAH | tail begins at 0.25 NIAH/block |
| 25 years | ~43.60M NIAH | 0.25 NIAH/block |

The tail creates approximately `262,980 NIAH/year` at the target interval. The 8-decimal conversion preserves the intended economic curve while making `0.00000001 NIAH` the smallest native amount.

## Reference ceiling and separate reward lanes

`41,943,040 NIAH` is not a maximum supply. It is the reference amount used by the decaying main-emission formula. Tail issuance continues indefinitely.

This formula defines only CPU-PoW consensus subsidy. It does not allocate a percentage to GPU AI workers, storage/service nodes, developers, a foundation, treasury, exchanges, or marketing. Those economic lanes remain separate.

## Required exact vectors before lock

Before production promotion, implementations MUST reproduce canonical 8-decimal vectors for early heights, 100/1,000/100,000 blocks, 1/5/10 target years, the tail boundary, and post-tail blocks. Tests MUST prove checked arithmetic, deterministic floor rounding, reorg-safe accounting, replay consistency, and isolation of devnet acceleration from production constants.

## Review gate

Compare this candidate against at least one slower-decay/later-tail and one faster-decay/earlier-tail profile using the same 30-second target, 8-decimal denomination, zero-premine assumption, and separate CPU/GPU/storage economic lanes before declaring production economics final.
