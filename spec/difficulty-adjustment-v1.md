# NIAHCIA Difficulty Adjustment V1

## Status

Draft candidate under simulation. **Not frozen consensus.**

The initial raw-interval trimmed-average idea was rejected during simulation because it creates feedback oscillation: historical solve times were produced under different targets, so multiplying the current target by their raw average double-counts prior target changes.

The current candidate instead normalizes every observed solve interval by the target that actually produced that block.

## Constants

```text
TARGET_BLOCK_INTERVAL = 30 seconds
WINDOW_BLOCKS         = 60
MIN_SOLVE_TIME        = 1 second
MAX_SOLVE_TIME        = 300 seconds
MAX_HARDER_STEP       = 12.5 percent
MAX_EASIER_STEP       = 12.5 percent
```

Difficulty is recalculated after every block.

## Samples

For each of the most recent up to 60 non-genesis blocks, define:

```text
solve_time_i =
  block_i.timestamp - block_(i-1).timestamp

bounded_time_i =
  clamp(solve_time_i, 1, 300)

work_time_i =
  target_i * bounded_time_i
```

`target_i` is the 256-bit target carried by the block whose solve interval is being measured.

Using the historical target that actually produced each interval removes the unstable feedback created by applying a new target to old raw solve times.

## Raw next target

For `n` available samples:

```text
raw_target =
  floor(
    sum(work_time_i)
    /
    (30 * n)
  )
```

All arithmetic is integer arithmetic.

The summation MUST use an intermediate width large enough to represent a 256-bit target multiplied by 300 and summed across 60 samples.

No floating point is permitted.

## Per-block clamp

Let `previous_target` be the parent block target.

Harder lower bound:

```text
harder_bound =
  ceil(previous_target * 7 / 8)
```

implemented exactly as:

```text
harder_bound =
  (previous_target * 7 + 7) / 8
```

with integer division.

Easier upper bound:

```text
easier_bound =
  floor(previous_target * 9 / 8)
```

Then:

```text
next_target =
  clamp(raw_target, harder_bound, easier_bound)
```

Finally the result is clamped to the network's absolute target bounds.

## Why there is no trimmed mean

Proof-of-work solve times are naturally right-skewed.

A symmetric trim of the fastest and slowest samples biases the observed mean downward even when miners are perfectly honest. In simulation this made the chain settle above the intended 30-second average.

NIAHCIA therefore uses:

- bounded solve times to limit extreme outliers,
- target-normalized work-time samples,
- a 60-block averaging window,
- a ±12.5% per-block target clamp.

This preserves the statistical mean instead of trimming legitimate slow PoW solves.

## Startup

Before 60 samples exist, the candidate uses all available non-genesis samples.

The network genesis specification defines the initial target.

Whether public mainnet should delay retargeting for a minimum number of startup samples remains open until launch simulations are complete.

## Long outage

There is no implicit mainnet emergency minimum-difficulty rule.

Individual solve intervals are capped at 300 seconds for the estimator. Therefore a multi-hour outage cannot instantly collapse difficulty when one block finally arrives.

Devnet/testnet MAY define an explicit temporary minimum-difficulty policy in their own network parameters. Such a policy is not inherited by mainnet.

## Fork choice

The adjustment algorithm determines the target required for the next block.

Fork choice remains independent:

```text
block_work =
  floor((2^256 - 1) / (target + 1)) + 1

chain_work =
  parent_chain_work + block_work
```

The fully valid branch with greatest cumulative work wins.

## Simulation findings so far

The simulation harness in `tools/difficulty_sim.py` compares both the rejected raw-window algorithm and this target-normalized candidate.

Deterministic step tests show the normalized candidate converging cleanly after 2x, 10x, and 90% hash-rate changes, while the raw-window formulation oscillates severely.

Seeded stochastic tests also show that trimming exponential PoW solve times introduces a persistent block-time bias, while the untrimmed target-normalized estimator remains centered near the target interval.

These are development findings, not sufficient evidence to freeze mainnet consensus.

## Before freezing

The following are still required:

- exact 256-bit/extended-width Rust implementation
- machine-readable arithmetic vectors
- startup-policy decision
- network absolute target bounds
- longer stochastic runs
- adversarial timestamp simulations
- burst/oscillation simulations
- independent implementation reproduction

Until those are complete, this specification remains Draft.
