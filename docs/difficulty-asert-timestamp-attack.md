# ASERT Timestamp-Adversary Stress Test

## Status

Research finding. This deliberately models an extreme adversary to expose failure modes before consensus is frozen.

## Setup

- 30-second target interval
- ASERT half-life: 4320 seconds
- current timestamp rule: timestamp > median of previous 11
- current future-drift limit: local adjusted time + 90 seconds
- honest mining for the first 99 blocks
- then the adversarial policy controls every subsequent block
- seeded stochastic PoW solve times

## Policies

### Honest

Header time follows simulated real time.

### Future +90

The miner pushes each block timestamp as far into the permitted future as practical.

Because the offset is bounded rather than cumulative, the initial simulation shows only a small persistent target shift.

### Minimum-MTP

The miner chooses the smallest timestamp still valid under the current rule:

```text
timestamp = median(previous 11 timestamps) + 1
```

This is a worst-case majority-control model.

## Initial result

In a representative 500-block seeded run:

```text
honest:
  post-attack-window mean solve ~30.2 s
  final target ratio          ~1.02

future +90:
  post-attack-window mean solve ~30.0 s
  final target ratio            ~1.03
  header clock offset            ~+90 s

minimum-MTP:
  post-attack-window mean solve ~94.5 s
  final target ratio            ~0.14
  header time falls far behind simulated real time
```

## Interpretation

The +90-second future bound behaves reasonably in this simplified model because the attacker cannot compound the future offset indefinitely.

The **minimum-MTP case is not acceptable as a finished consensus story**. A sustained majority miner can legally slow header-time progression, causing ASERT to believe the chain is far ahead of schedule and therefore harden the target substantially.

This does not imply ASERT should be discarded. Timestamp-based PoW DAAs inherently depend on timestamp validity assumptions, and a miner capable of producing a long majority sequence already has substantial chain control.

But NIAHCIA should not freeze its timestamp/DAA combination without explicitly addressing this behavior.

## Next design questions

Candidates to evaluate include:

- using a more manipulation-resistant time statistic in ASERT,
- constraining how slowly header time may advance relative to deterministic chain history,
- deriving difficulty time from a rolling median while preserving ASERT's anchor behavior,
- quantifying the attacker work required as difficulty hardens,
- modeling partial attacker share rather than 100% consecutive control.

Consensus MUST NOT depend directly on each node's local wall clock for target calculation, because independent nodes must derive the same required target from chain history alone.

## Conclusion

ASERT remains the leading DAA candidate, but timestamp hardening is now a blocker for freezing it.
