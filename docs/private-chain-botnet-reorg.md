# Private-Chain Botnet Reorg Simulation

## Purpose

A temporary botnet is much more dangerous when it does not publish its blocks immediately.

Instead, the attacker can mine a private branch from the current public tip and reveal it only when the private branch has accumulated more work.

NIAHCIA's fork choice is cumulative work, so this is the correct adversarial comparison.

## Model

The initial research model uses:

```text
public honest hashrate = 1.0 baseline
private attacker hash  = configurable multiple of baseline
block target           = 30 seconds
DAA                     = 4320-second ASERT candidate
timestamps              = honest on both branches
fork choice             = greater accumulated work
```

Both branches begin from the same accepted tip.

This model deliberately does **not** yet include:

- timestamp manipulation,
- eclipse attacks,
- selfish-mining publication strategies,
- network propagation advantage,
- pool bribery,
- transaction-specific double-spend timing.

The simulator is:

```text
tools/reorg_sim.py
```

## Monte Carlo results

5,000 deterministic seeds per cell:

| Private botnet hash vs honest network | Combined attacker share | 5 min | 15 min | 30 min |
|---:|---:|---:|---:|---:|
| 0.25x | 20.0% | 1.18% | ~0% | ~0% |
| 0.50x | 33.3% | 10.18% | 1.04% | 0.06% |
| 1.00x | 50.0% | 49.90% | 49.46% | 51.16% |
| 2.00x | 66.7% | 97.04% | 99.88% | ~100% |
| 5.00x | 83.3% | ~100% | ~100% | ~100% |
| 10.00x | 90.9% | ~100% | ~100% | ~100% |

The percentage shown is the fraction of simulations where the attacker's private branch had greater accumulated work at the end of the stated private-mining interval.

## Interpretation

This reinforces a fundamental Nakamoto-consensus boundary:

> once an attacker can temporarily command more hashpower than the honest network, a private-chain reorganization becomes highly probable very quickly.

ASERT does not and should not attempt to "fix" a genuine majority-hash attack.

The correct protocol goals are instead to avoid making sub-majority attacks easier, reduce propagation/eclipsing advantages, and make abnormal hashrate/reorg conditions visible.

### Sub-majority botnet

At approximately one-third of combined hashrate, a five-minute private race still won about 10% of these simplified simulations, but longer races usually fell behind because honest work accumulated faster.

This means short confirmation depths deserve particular attention on a young/low-hashrate network.

### Equal attacker and honest hash

At 50% of combined hashpower the race is essentially a coin flip. More confirmations cannot turn equal sustained hashpower into a safe condition.

### Majority botnet

At roughly two-thirds of combined hashrate, even a five-minute private race was heavier in about 97% of the simulated runs.

This is the regime where ordinary confirmation-count advice stops being an adequate defense.

## Botnet-specific concern at launch

The most dangerous period is likely early network life, when honest RandomX hashrate is small.

A botnet does not need to be enormous in absolute CPU count. It only needs to be large relative to the honest network.

That makes the following launch priorities important:

- broad miner distribution before treating the network as economically secure,
- solo mining and multiple independent pools,
- no developer-controlled dominant pool,
- hashrate and reorg telemetry,
- conservative exchange/bridge confirmation policies,
- anti-eclipse peer topology work,
- no automatic checkpoint authority disguised as decentralization.

## What we should not do

NIAHCIA should not react by creating a permissioned miner list.

A miner identity system would not prove machine authorization and would introduce a centralized consensus gate.

## Next simulation

The next attack model should combine:

```text
private-chain mining
+
temporary botnet burst
+
minimum-valid timestamp strategy
+
publication/reorg event
```

and evaluate:

- reorg depth,
- accumulated-work lead,
- DAA state after the reorg,
- recovery after the botnet disappears.
