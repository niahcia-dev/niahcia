# Difficulty Simulation — Initial Results

## Purpose

This document records the first simulation pass for the NIAHCIA Difficulty Adjustment V1 candidate.

The simulation is reproducible with:

```bash
python3 tools/difficulty_sim.py
```

## Important finding

The originally discussed algorithm:

```text
new_target =
  previous_target
  * trimmed_average(raw_solve_times)
  / 30
```

is **not suitable**.

The reason is structural, not merely parameter tuning: the rolling window contains solve times produced under different historical targets. Applying all of those raw intervals to the current target creates delayed feedback and severe target oscillation.

It was therefore not promoted to consensus.

## Replacement candidate

The current candidate estimates target directly from target-normalized work-time samples:

```text
sample_i =
  historical_target_i
  * bounded_solve_time_i

raw_target =
  average(sample_i) / 30
```

Each sample therefore remembers the amount of target/work context under which that solve time occurred.

## Deterministic step response

Using normalized target 1.0 as the initial equilibrium:

### 2x hash-rate increase

The correct equilibrium target is 0.5.

The normalized candidate converges cleanly to 0.5 by approximately the time one 60-block history window has turned over.

### 10x hash-rate increase

The correct equilibrium target is 0.1.

The normalized candidate converges cleanly to 0.1 without the runaway oscillation seen in the rejected algorithm.

### 90% hash-rate loss

The correct equilibrium target is 10.0.

The normalized candidate converges to 10.0 while the ±12.5% per-block clamp limits abrupt target movement.

## Seeded stochastic test

Twenty fixed random seeds were run for 1,500 simulated blocks per scenario, evaluating the final 1,000 blocks.

The initial simulation produced approximately:

```text
steady       mean solve ~30.5 s
2x hash      mean solve ~30.5 s
10x hash     mean solve ~30.5 s
90% loss     mean solve ~30.5 s
```

The small offset from exactly 30 seconds is consistent with finite stochastic runs and the adaptive estimator.

By comparison, the symmetric 10% trimmed estimator produced an approximately 36.6-second steady-state mean in the same simulation structure because trimming a right-skewed exponential PoW distribution biases the mean downward.

## Conclusion

The simulation invalidated one of our earlier recommendations before it became consensus law.

That is exactly why issue #9 exists.

The target-normalized estimator is now the candidate to test further.

It is **not frozen** yet.

Next tests should focus on:

- oscillating hash rate,
- burst mining,
- adversarial but timestamp-valid behavior,
- startup with very low hash rate,
- long outage recovery,
- exact integer rounding,
- 256-bit boundary cases.
