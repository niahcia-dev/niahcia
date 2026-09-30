# NIAHCIA Emission Candidate V1 — Exact Vectors

Status: **canonical review vectors for the current candidate; candidate economics remain pre-production**

These vectors apply the exact integer algorithm in `emission-candidate-v1.md` using atomic units (`aniah`). They are intended to catch rounding, implementation, restart/replay, and cross-language disagreements before monetary constants are promoted into live consensus.

Constants:

- `ATOMIC_UNITS_PER_NIAH = 1_000_000_000_000_000_000`
- `M = 41_943_040 * ATOMIC_UNITS_PER_NIAH`
- `EMISSION_SHIFT = 22`
- `TAIL = 250_000_000_000_000_000 aniah`
- target interval = `30 seconds`

For cumulative main-emission subsidy `A` before a block:

`decay_reward = (M - min(A, M)) >> 22`

`cpu_subsidy = max(decay_reward, TAIL)`

The cumulative values below are the amount issued by the first `N` blocks. `next subsidy` is the subsidy for block index `N` when block indexing begins at zero.

| N blocks already issued | Next subsidy (aniah) | Cumulative issued (aniah) |
| ---: | ---: | ---: |
| 0 | 10000000000000000000 | 0 |
| 1 | 9999997615814208984 | 10000000000000000000 |
| 2 | 9999995231628986402 | 19999997615814208984 |
| 10 | 9999976158167669365 | 99999892711707616366 |
| 100 | 9999761584234625756 | 999988198372249749484 |
| 1,000 | 9997616098119343095 | 9998809193646778712736 |
| 100,000 | 9764401110536943055 | 988173364470457593210968 |
| 1,051,920 (~1 target year) | 7781800914129364082 | 9303801298663551709727601 |
| 5,259,600 (~5 target years) | 2853649167386775496 | 29973967882632977986589539 |
| 10,519,200 (~10 target years) | 814331357052723704 | 38527486731788332757278741 |

## Tail transition

The first block whose computed subsidy is exactly the tail floor occurs after **15,472,281** blocks have already been issued under this recurrence.

At that boundary:

- cumulative CPU issuance: `40894464248632975374433207 aniah`
- next subsidy: `250000000000000000 aniah`
- approximate target elapsed time: `14.7086 years`

From this point forward the CPU subsidy remains exactly `250000000000000000 aniah` per canonical block under MonetaryPolicyV1.

## Required implementation assertions

An implementation claiming compatibility with this candidate MUST reproduce every integer above exactly. It MUST also demonstrate that:

1. calculations use unsigned checked integer arithmetic;
2. the right shift floors deterministically;
3. no floating-point quantity participates in consensus;
4. orphaned/reorged blocks do not permanently advance cumulative canonical issuance;
5. replay from genesis reaches identical cumulative issuance;
6. the tail transition occurs at the identical block boundary;
7. tail issuance does not reduce the reference remainder below zero;
8. network-specific test acceleration cannot modify these production-candidate constants.

## Review status

These vectors validate the arithmetic of the current candidate; they do **not** by themselves select it as final production economics. The remaining review gate is to compare this profile against at least one slower-decay/later-tail profile and one faster-decay/earlier-tail profile using the same target interval, atomic denomination, zero-premine assumption, and separate CPU/GPU/storage economic lanes.
