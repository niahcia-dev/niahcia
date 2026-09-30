# NIAHCIA Emission Curve Comparison V1

Status: **pre-production economic review; no curve is production-final**

This comparison evaluates the current NIAHCIA CPU-PoW emission candidate against one faster-decay and one slower-decay profile after the denomination lock to 8 decimals.

## Shared assumptions

All three profiles use:

- target block interval: `30 seconds`;
- `1 NIAH = 100,000,000 aniah`;
- initial CPU subsidy: exactly `10 NIAH/block`;
- permanent CPU tail subsidy: exactly `0.25 NIAH/block`;
- production genesis allocation: `0 NIAH`;
- integer-only deterministic arithmetic;
- CPU issuance separate from GPU AI and storage/service compensation.

To preserve the exact 10 NIAH initial reward, each profile uses `M = 10 * 2^shift NIAH`.

## Profiles

| Profile | Shift | Main-emission reference M | Approx. tail start |
| --- | ---: | ---: | ---: |
| Faster decay | 21 | 20,971,520 NIAH | 7.3543 years |
| Current candidate | 22 | 41,943,040 NIAH | 14.7086 years |
| Slower decay | 23 | 83,886,080 NIAH | 29.4172 years |

For each profile:

`decay_reward = (M - min(A, M)) >> shift`

`cpu_subsidy = max(decay_reward, 25,000,000 aniah)`

where `A` is cumulative canonical CPU subsidy already issued.

## Exact 8-decimal simulation checkpoints

The following values come from direct integer recurrence using the candidate formula. They are review values; only a selected production profile should later receive locked interoperability vectors.

| Target time | Faster cumulative | Faster next reward | Current cumulative | Current next reward | Slower cumulative | Slower next reward |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 year | 8,271,917.87710319 | 6.05564218 | 9,303,801.29401329 | 7.78180091 | 9,886,379.59338430 | 8.82145171 |
| 5 years | 19,263,743.86683652 | 0.81433111 | 29,973,967.86764314 | 2.85364917 | 39,074,505.22105771 | 5.34195599 |
| 10 years | 21,142,997.21964502 | 0.25000000 | 38,527,486.71251556 | 0.81433136 | 59,947,933.94606613 | 2.85364938 |
| 25 years | 25,087,697.21964502 | 0.25000000 | 43,600,893.97817847 | 0.25000000 | 80,236,932.36582981 | 0.43501229 |

At the 30-second target, permanent tail issuance is `262,980 NIAH/year` for every profile once its tail floor is active.

## Trade-offs

### Faster decay (`shift=21`)

This profile reduces launch-era issuance most aggressively. The CPU reward is already near the tail after five target years and reaches the permanent floor around year 7.35. It produces the smallest long-run supply of the three profiles, but it also asks fees plus the 0.25 NIAH tail to carry most CPU security much earlier in NIAHCIA's life.

### Current candidate (`shift=22`)

The current profile occupies the middle ground. It retains a material CPU subsidy through the first decade, reaches the tail around year 14.71, and avoids both the early security-floor transition of shift 21 and the much larger multi-decade issuance of shift 23.

### Slower decay (`shift=23`)

This profile preserves substantial CPU issuance for decades. At year 25 the next subsidy is still about 0.435 NIAH and the tail is not expected until about year 29.42. It provides the longest subsidy runway but produces substantially more NIAH and prolongs distribution through mining.

## Review interpretation

The numerical comparison does not itself select a winner. The production decision should explicitly weigh:

1. how long CPU security should receive substantial protocol issuance before relying on the tail plus transaction fees;
2. acceptable early and long-run distribution through CPU mining;
3. desired supply scale after 5, 10, and 25 years;
4. the fact that GPU AI and storage/service compensation are separate economic lanes and therefore should not be hidden inside the CPU subsidy;
5. sensitivity to actual average block interval rather than assuming target time equals wall-clock time.

## Next lock gate

Before CPU rewards are wired into live consensus, choose one profile and then:

1. regenerate its full canonical 8-decimal interoperability vectors;
2. add exact Rust assertions for the selected constants and tail boundary;
3. test issuance rollback across reorgs and replay/restart;
4. define fee destination separately from subsidy issuance;
5. only then connect the selected subsidy to canonical block-state transitions.
