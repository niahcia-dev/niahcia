# Botnet and Ephemeral Hashrate Simulation

## Purpose

NIAHCIA deliberately targets CPU-accessible proof of work. That improves ordinary participation, but it also means stolen CPU capacity from compromised machines must be treated as a first-class threat.

The protocol cannot tell whether valid RandomX work came from:

- an owner's desktop,
- a datacenter,
- a mining pool,
- or a compromised machine.

A decentralized consensus rule therefore cannot solve botnets by identifying or banning miners without introducing an identity/permission authority.

## Temporary botnet burst model

The simulator models an otherwise honest chain where aggregate hashrate suddenly increases for 120 blocks and then disappears.

The current ASERT research candidate uses:

```text
target interval = 30 seconds
half-life       = 4320 seconds
```

Twenty deterministic random seeds were averaged.

Approximate results:

```text
temporary hash    during burst    after burst/recovery
2x                16.6 s          34.1 s
5x                 7.3 s          37.3 s
10x                3.8 s          38.5 s
25x                1.5 s          39.3 s
```

By the end of the longer simulation, the target returned close to baseline in each case.

## Interpretation

ASERT does what we want structurally:

1. it does not instantly chase a short-lived hashrate spike,
2. blocks become temporarily fast while the burst is present,
3. after the stolen hashrate disappears there is a temporary slow period,
4. the schedule gradually returns toward the 30-second target.

A shorter half-life would react faster to a botnet entering but would also leave honest miners with a larger difficulty hangover when that hashrate disappears.

A longer half-life reduces the hangover but permits a longer fast-block burst.

That tradeoff is now part of the half-life decision.

## Timestamp manipulation with partial attacker share

The simulator also lets an attacker use the minimum timestamp valid under MTP-11 on every block it wins, while honest blocks use real simulated time.

Twenty seeded runs produced approximately:

```text
attacker share   mean solve
10%              30.0 s
25%              30.0 s
33%              30.0 s
40%              30.0 s
50%              30.0 s
60%              30.0 s
```

The average target remained close to baseline in these partial-share runs because honest blocks repeatedly pull header time back toward real time.

However, the maximum temporary header-time lag increased sharply as attacker share rose. That means long lucky attacker streaks and sustained majority control still deserve dedicated reorg/timestamp testing.

The earlier 100%-consecutive-block stress case remains intentionally pathological and showed severe degradation.

## What botnets change

A botnet is more dangerous than ordinary volatile mining when it combines:

- a large fraction of network hashpower,
- coordinated withholding/private mining,
- timestamp manipulation,
- censorship,
- strategic entry and exit.

The next adversarial simulator should combine those behaviors rather than testing hashrate bursts in isolation.

## Protocol position

NIAHCIA should not introduce miner identity or centralized admission as a response to botnets.

The appropriate goals are:

- robust DAA behavior,
- cumulative-work fork choice,
- strong P2P propagation,
- resistance to eclipse attacks,
- monitoring of sudden hashrate concentration,
- decentralized pools and solo-mining support,
- conservative confirmation guidance during abnormal hashrate/reorg conditions.

RandomX remains a CPU-fairness choice, not a claim that unauthorized distributed mining is impossible.
