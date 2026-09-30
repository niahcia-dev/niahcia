# NIAHCIA Emission Candidate V1

Status: **numerical candidate for simulation and canonical-vector implementation; not production-final until exact integer vectors pass**

This document turns the architecture in `monetary-policy-v1.md` into a concrete candidate that can be implemented and tested without pretending the economics are final before simulation.

## Candidate constants

- target block interval: `30 seconds`
- atomic units per NIAH: `10^18 aniah`
- main-emission reference ceiling: `41,943,040 NIAH`
- emission shift: `22`
- initial theoretical subsidy: `10 NIAH/block`
- tail subsidy: `0.25 NIAH/block`
- genesis production allocation: `0 NIAH`

The reference ceiling is chosen so that:

`41,943,040 NIAH >> 22 = 10 NIAH`

when expressed in exact atomic-unit integer arithmetic.

## Exact candidate formula

Let:

- `M = 41,943,040 * 10^18` aniah
- `A = cumulative CPU-PoW main-emission subsidy already issued` in aniah
- `TAIL = 0.25 * 10^18` aniah

For each candidate canonical block:

`decay_reward = (M - min(A, M)) >> 22`

`cpu_subsidy = max(decay_reward, TAIL)`

All operations are unsigned checked integer operations. `>> 22` is an integer right shift and therefore rounds downward deterministically.

Once `decay_reward <= TAIL`, the subsidy remains exactly `TAIL` forever unless a later explicitly activated consensus version changes the monetary policy.

`A` tracks CPU main-emission subsidy for this formula. Tail issuance does not make `M - A` negative and does not terminate tail emission.

## Approximate behavior

The values below are analytical approximations for understanding the candidate. Canonical values MUST come from the exact integer implementation/vector generator.

At the 30-second target there are approximately `1,051,920` blocks/year.

| Elapsed target time | Approx. cumulative CPU issuance | Approx. subsidy |
| --- | ---: | ---: |
| launch | 0 NIAH | 10.0000 NIAH/block |
| 1 year | 9.30M NIAH | 7.78 NIAH/block |
| 5 years | 29.97M NIAH | 2.85 NIAH/block |
| 10 years | 38.53M NIAH | 0.814 NIAH/block |
| ~14.7 years | 40.894M NIAH | tail begins at 0.25 NIAH/block |
| 15 years | ~40.97M NIAH | 0.25 NIAH/block |
| 25 years | ~43.60M NIAH | 0.25 NIAH/block |

The tail creates approximately `262,980 NIAH/year` at the target block interval.

At the expected tail transition supply of roughly `40.894M NIAH`, that is about `0.64%` gross annual issuance initially. Because the tail is constant in NIAH while supply continues growing, the percentage rate declines over time.

## Why this shape is a useful candidate

The candidate deliberately avoids both extremes:

- it does not dump nearly all supply into the first year or two;
- it does not keep a large launch-era subsidy for decades;
- it gives CPU miners substantial early compensation while the network is young;
- it transitions gradually rather than using large halving cliffs;
- it reaches a modest permanent security floor after roughly fifteen target years;
- the arithmetic is exceptionally simple to reproduce across implementations.

The design is inspired by the useful property of remaining-supply smooth-emission formulas, but these constants and NIAHCIA's 30-second target are NIAHCIA-specific.

## What the reference ceiling means

`41,943,040 NIAH` is **not a maximum supply**.

It is the reference amount used by the decaying main-emission formula. Because the tail floor activates before the mathematical decay reaches zero and then continues indefinitely, total issued supply eventually exceeds this number.

RPCs and explorers MUST NOT label this value `max_supply`.

## Separate reward lanes remain unchanged

This formula defines only the CPU-PoW consensus subsidy.

It does not allocate a percentage to:

- GPU AI workers;
- storage nodes;
- service nodes;
- developers;
- a foundation;
- a treasury;
- exchanges;
- marketing.

AI and proof-of-service compensation remain separately accounted economic lanes as defined in `monetary-policy-v1.md`.

## Required exact vectors before lock

Before this candidate can become the production monetary vector, an implementation MUST generate and commit exact values for at least:

1. height 0 subsidy;
2. heights 1 through 10;
3. height 100;
4. height 1,000;
5. height 100,000;
6. approximately 1 target year;
7. approximately 5 target years;
8. approximately 10 target years;
9. the final block above the tail floor;
10. the first tail-floor block;
11. at least 10 blocks after tail activation;
12. cumulative issuance at every checkpoint;
13. the same calculations using an independent reference implementation.

Tests MUST also prove:

- no overflow in atomic-unit arithmetic;
- no underflow in remaining-emission arithmetic;
- deterministic floor rounding;
- reorgs restore issuance accounting correctly;
- orphaned blocks do not contribute to canonical issued supply;
- restart/replay derives the identical subsidy and cumulative issuance;
- devnet acceleration, if used, cannot leak into production constants.

## Review gates

Before declaring this production-final, compare at least two nearby alternatives against this candidate:

- a slower-decay / later-tail profile;
- a faster-decay / earlier-tail profile.

Compare early miner distribution, 1/5/10/25-year issuance, initial tail inflation, and sensitivity to actual average block time.

If this candidate wins that review, promote these constants into a canonical `MonetaryPolicyV1` implementation and lock the generated interoperability vectors. If not, replace this candidate document before public production economics are promised.
