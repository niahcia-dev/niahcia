# NIAHCIA Emission Endpoint Comparison V1

Status: **pre-production economic review; endpoint constants are not production-final**

This review isolates the two remaining endpoint choices in the CPU-PoW emission candidate: the launch subsidy and permanent tail subsidy. Decay-speed comparison is documented separately in `emission-curve-comparison-v1.md`.

## Fixed assumptions for this review

- target block interval: `30 seconds`;
- denomination: `1 NIAH = 100,000,000 aniah`;
- production genesis spendable allocation: `0 NIAH`;
- smooth integer decay;
- current middle decay profile: `shift=22`;
- CPU consensus issuance remains separate from GPU AI and storage/service compensation.

For a chosen initial subsidy `R0`, the corresponding smooth-decay reference is:

`M = R0 * 2^22`

This preserves the exact launch subsidy while retaining the same relative decay rate. Consequently, changing `R0` primarily scales the main-emission supply and reward amounts rather than changing the shape's relative half-life.

## Initial subsidy comparison

| Initial CPU subsidy | Main-emission reference M | Approximate 1-year issuance | Approximate 5-year issuance | Approximate 10-year issuance |
| --- | ---: | ---: | ---: | ---: |
| 5 NIAH | 20,971,520 NIAH | ~4.65M | ~14.99M | ~19.26M |
| **10 NIAH candidate** | **41,943,040 NIAH** | **~9.30M** | **~29.97M** | **~38.53M** |
| 20 NIAH | 83,886,080 NIAH | ~18.61M | ~59.95M | ~77.05M |

These comparison values intentionally show scale. Exact production interoperability vectors MUST be regenerated after final endpoint selection because the tail floor can change the late curve.

### Interpretation

A 5 NIAH launch subsidy reduces early distribution and absolute miner revenue by half relative to the current candidate. A 20 NIAH launch subsidy doubles both. None of these values inherently changes PoW security in fiat terms because market value and hashrate are external; the protocol controls only the number of native units offered.

The 10 NIAH candidate provides a simple human-scale launch reward and places the main-emission reference near 42 million NIAH without requiring a premine. There is no protocol requirement that the supply resemble Bitcoin, Ethereum, Monero, or Yerbas.

## Tail subsidy comparison

At a 30-second target there are approximately `1,051,920` target blocks per 365.25-day year.

| Permanent tail | Tail issuance / target year | Tail issuance / target day |
| --- | ---: | ---: |
| 0.10 NIAH/block | 105,192 NIAH | 288 NIAH |
| **0.25 NIAH/block candidate** | **262,980 NIAH** | **720 NIAH** |
| 0.50 NIAH/block | 525,960 NIAH | 1,440 NIAH |

The tail is a long-run CPU security floor, not a supply target. Its percentage monetary expansion declines as cumulative supply grows.

Using the current `shift=22`, 10-NIAH launch candidate, the 0.25-NIAH floor begins around year 14.71. A lower tail permits the smooth-decay phase to continue longer before the floor binds; a higher tail makes the floor bind earlier. Therefore changing the tail requires regeneration of the exact transition height and cumulative issuance vectors.

## Security-budget interpretation

The protocol cannot know the future exchange value of NIAH, electricity prices, CPU efficiency, miner participation, or fee demand. Therefore no fixed native-unit tail can guarantee a particular hashrate or dollar-denominated security budget.

The tail should instead satisfy structural goals:

1. never allow protocol CPU issuance to decay to zero;
2. remain large enough in native units to provide a persistent incentive independent of transaction-fee demand;
3. avoid making permanent monetary expansion unnecessarily large;
4. remain simple enough to audit and explain;
5. remain independent of GPU AI and storage/service reward budgets.

## Candidate assessment

The current endpoint pair remains a coherent production candidate:

- launch subsidy: `10 NIAH/block`;
- tail subsidy: `0.25 NIAH/block`.

Reasons to retain it for the next validation stage:

- 10 NIAH is simple and gives useful early CPU distribution without a production premine;
- 0.25 NIAH is exactly 1/40 of the launch subsidy;
- the current middle decay profile reaches the tail only after roughly 14.7 target years rather than within the first several years;
- permanent target-time tail issuance is about 262,980 NIAH/year and its percentage expansion declines over time;
- neither constant entangles CPU consensus rewards with AI or storage economics.

This assessment is not yet a production consensus activation. It identifies the pair that should receive exact validation unless later simulation exposes an undesirable property.

## Recommended lock candidate

Proceed to exact validation using:

- `DECIMALS = 8`;
- `INITIAL_SUBSIDY = 10 * 100,000,000 aniah`;
- `DECAY_SHIFT = 22`;
- `MAIN_EMISSION_REFERENCE = 41,943,040 * 100,000,000 aniah`;
- `TAIL_SUBSIDY = 25,000,000 aniah`;
- production genesis spendable allocation = `0`.

Before these constants become live consensus policy, the implementation MUST pass canonical vectors, restart/replay accounting, reorg rollback accounting, boundary tests, and independent fee-policy review.
