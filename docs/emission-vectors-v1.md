# NIAHCIA Emission Candidate V1 — Exact 8-Decimal Vectors

Status: **canonical review vectors for the current candidate; candidate economics remain pre-production**

These vectors apply the exact integer algorithm in `emission-candidate-v1.md` using the V1 native denomination of `100,000,000 aniah = 1 NIAH`.

Constants:

- `ATOMIC_UNITS_PER_NIAH = 100_000_000`
- `M = 41_943_040 * ATOMIC_UNITS_PER_NIAH`
- `EMISSION_SHIFT = 22`
- `TAIL = 25_000_000 aniah`
- target interval = `30 seconds`

For cumulative subsidy `A` before a block:

`decay_reward = (M - min(A, M)) >> 22`

`cpu_subsidy = max(decay_reward, TAIL)`

| N blocks already issued | Next subsidy (aniah) | Cumulative issued (aniah) |
| ---: | ---: | ---: |
| 0 | 1000000000 | 0 |
| 1 | 999999761 | 1000000000 |
| 2 | 999999523 | 1999999761 |
| 10 | 999997615 | 9999989267 |
| 100 | 999976158 | 99998819787 |
| 1,000 | 999761609 | 999880918866 |
| 100,000 | 976440111 | 98817336397725 |
| 1,051,920 (~1 target year) | 778180091 | 930380129401329 |
| 5,259,600 (~5 target years) | 285364917 | 2997396786764314 |
| 10,519,200 (~10 target years) | 81433136 | 3852748671251556 |

Human-readable cumulative checkpoints are therefore approximately:

- 1 target year: `9,303,801.29401329 NIAH`
- 5 target years: `29,973,967.86764314 NIAH`
- 10 target years: `38,527,486.71251556 NIAH`

## Tail transition

The first block whose computed subsidy is exactly the tail floor occurs after **15,472,280** blocks have already been issued under the 8-decimal recurrence.

At that boundary:

- cumulative CPU issuance: `4089446397817847 aniah`
- cumulative CPU issuance: `40,894,463.97817847 NIAH`
- next subsidy: `25,000,000 aniah = 0.25000000 NIAH`
- approximate target elapsed time: `14.7086 years`

The one-block difference from the superseded 18-decimal review vector is expected: reducing atomic precision changes deterministic floor rounding. These 8-decimal vectors supersede the earlier 18-decimal vectors.

## Required implementation assertions

An implementation claiming compatibility with this candidate MUST reproduce every integer above exactly and demonstrate checked unsigned arithmetic, deterministic floor rounding, reorg-safe canonical issuance, genesis replay consistency, the identical tail boundary, and isolation of network-specific test acceleration from production constants.

## Review status

These vectors validate the arithmetic of the current 8-decimal candidate. They do not yet select the emission curve as final production economics. The remaining gate is comparison with slower-decay/later-tail and faster-decay/earlier-tail alternatives.
