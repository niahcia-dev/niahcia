# Difficulty Simulation — ASERT Comparison

## Why this comparison was added

The target-normalized rolling estimator improved substantially on the first raw-window formula, but additional periodic-hashrate simulation exposed an important weakness.

With hashrate switching between approximately:

```text
0.25x
and
4x
```

every 50 blocks, the 60-block normalized rolling candidate produced long-run mean block times around **47 seconds** in the initial seeded simulation.

That is too far from the 30-second target.

## Anchor-based comparison

An ASERT-style candidate was then simulated using the same 30-second target.

Candidate half-lives:

```text
2160 s  (72 blocks)
4320 s  (144 blocks)
8640 s  (288 blocks)
```

Across 20 seeded, 3,000-block simulations, evaluating the final 2,000 blocks, approximate mean block times were:

```text
half-life 2160 s
steady       ~30.0 s
2x step      ~30.0 s
10x step     ~30.0 s
90% loss     ~30.0 s
switching    ~30.1 s

half-life 4320 s
steady       ~30.0 s
2x step      ~30.0 s
10x step     ~29.7 s
90% loss     ~30.1 s
switching    ~30.1 s

half-life 8640 s
steady       ~30.0 s
2x step      ~29.4 s
10x step     ~25.7 s
90% loss     ~30.7 s
switching    ~30.3 s
```

The shorter half-lives react more quickly to large permanent hashrate changes.

## Current preference

The **4320-second / 144-block half-life** is the current leading research candidate.

It gives more smoothing than 2160 seconds while responding materially faster than the 8640-second equivalent of a 288-block half-life.

This is still not frozen consensus.

## Important implementation constraint

The simulation uses floating point only to compare controller behavior.

A production ASERT implementation must use a deterministic fixed-point integer approximation and exact test vectors. Floating point must not appear in consensus.

## Next analysis

Before adoption:

- adversarial timestamp behavior,
- very low launch hashrate,
- prolonged outages,
- alternating burst miners,
- pow-limit saturation,
- fixed-point approximation error,
- genesis/anchor definition.
