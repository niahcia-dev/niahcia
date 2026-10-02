#!/usr/bin/env python3
"""Deterministic NIAHCIA difficulty-candidate simulator.

This tool intentionally uses Python integers so the arithmetic model is exact
and unbounded. It is a protocol-development aid, not consensus code.
"""

from __future__ import annotations

import math
import random
from dataclasses import dataclass

TARGET_SECONDS = 30
WINDOW = 60
MIN_SOLVE = 1
MAX_SOLVE = 300
LOW_NUM, LOW_DEN = 7, 8
HIGH_NUM, HIGH_DEN = 9, 8


@dataclass
class Sample:
    solve_time: float
    target: float


def clamp(value: float, low: float, high: float) -> float:
    return min(high, max(low, value))


def normalized_next_target(previous: float, samples: list[Sample]) -> float:
    window = samples[-WINDOW:]
    work_time = sum(clamp(s.solve_time, MIN_SOLVE, MAX_SOLVE) * s.target for s in window)
    raw = work_time / (TARGET_SECONDS * len(window))
    return clamp(raw, previous * LOW_NUM / LOW_DEN, previous * HIGH_NUM / HIGH_DEN)


def rejected_raw_window_target(previous: float, samples: list[Sample]) -> float:
    values = sorted(clamp(s.solve_time, MIN_SOLVE, MAX_SOLVE) for s in samples[-WINDOW:])
    if len(values) == WINDOW:
        values = values[6:-6]
    observed = sum(values) / len(values)
    raw = previous * observed / TARGET_SECONDS
    return clamp(raw, previous * LOW_NUM / LOW_DEN, previous * HIGH_NUM / HIGH_DEN)


def hash_rate_for(name: str, height: int) -> float:
    if name == "steady":
        return 1.0
    if name == "2x":
        return 1.0 if height < 100 else 2.0
    if name == "10x":
        return 1.0 if height < 100 else 10.0
    if name == "90pct_drop":
        return 1.0 if height < 100 else 0.1
    if name == "oscillating":
        return 4.0 if (height // 50) % 2 else 0.25
    raise ValueError(name)


def run(name: str, algorithm, blocks: int = 400, stochastic: bool = False, seed: int = 1):
    rng = random.Random(seed)
    target = 1.0
    samples: list[Sample] = []
    rows = []

    for height in range(1, blocks + 1):
        hashrate = hash_rate_for(name, height)
        expected = TARGET_SECONDS / (hashrate * target)
        solve = rng.expovariate(1.0 / expected) if stochastic else expected
        samples.append(Sample(solve, target))
        target = algorithm(target, samples)
        rows.append((height, hashrate, solve, target))

    return rows


def summarize_step(name: str, algorithm):
    rows = run(name, algorithm)
    checkpoints = (100, 110, 130, 160, 200, 300, 400)
    print(f"\n{name}")
    print("height  solve_s   target_ratio  difficulty_ratio")
    for height in checkpoints:
        _, _, solve, target = rows[height - 1]
        print(f"{height:>6}  {solve:>7.2f}   {target:>12.6f}  {1/target:>16.6f}")


def stochastic_summary(name: str, algorithm, seeds: int = 20):
    mean_solve = []
    mean_target = []
    for seed in range(1, seeds + 1):
        rows = run(name, algorithm, blocks=1500, stochastic=True, seed=seed)
        tail = rows[500:]
        mean_solve.append(sum(row[2] for row in tail) / len(tail))
        mean_target.append(sum(row[3] for row in tail) / len(tail))

    print(
        f"{name:>12}: mean solve={sum(mean_solve)/len(mean_solve):.3f}s "
        f"mean target ratio={sum(mean_target)/len(mean_target):.6f}"
    )


def main():
    print("NIAHCIA difficulty candidate — deterministic step response")
    for scenario in ("steady", "2x", "10x", "90pct_drop"):
        summarize_step(scenario, normalized_next_target)

    print("\nRejected raw-window algorithm — 2x step demonstrates feedback oscillation")
    summarize_step("2x", rejected_raw_window_target)

    print("\nNormalized candidate — seeded stochastic steady-state")
    for scenario in ("steady", "2x", "10x", "90pct_drop", "oscillating"):
        stochastic_summary(scenario, normalized_next_target)

    print("\nASERT research candidate — seeded stochastic mean solve times")
    for half_life in ASERT_HALF_LIVES:
        print(f"half-life={half_life}s")
        for scenario in ("steady", "2x", "10x", "90pct_drop", "oscillating"):
            print(f"  {scenario:>12}: {asert_summary(scenario, half_life):.3f}s")

    print("\nASERT timestamp-adversary stress test — 4320-second half-life")
    for policy in ("honest", "future_90", "minimum_mtp"):
        mean_solve, final_target, clock_offset = timestamp_attack_summary(policy)
        print(
            f"  {policy:>12}: mean solve={mean_solve:.3f}s "
            f"final target={final_target:.6f} "
            f"header-real offset={clock_offset:.1f}s"
        )


# --- ASERT research candidate -------------------------------------------------

ASERT_HALF_LIVES = (2160, 4320, 8640)


def asert_next_target(anchor_target: float, elapsed_time: float, elapsed_blocks: int, half_life: int) -> float:
    """Simulation-only floating-point form. Consensus MUST use fixed-point integers."""
    exponent = (elapsed_time - TARGET_SECONDS * elapsed_blocks) / half_life
    return anchor_target * (2.0 ** exponent)


def run_asert(name: str, blocks: int = 3000, half_life: int = 4320, seed: int = 1):
    rng = random.Random(seed)
    anchor_target = 1.0
    elapsed_time = 0.0
    target = anchor_target
    rows = []

    for height in range(1, blocks + 1):
        hashrate = hash_rate_for(name, height)
        expected = TARGET_SECONDS / (hashrate * target)
        solve = rng.expovariate(1.0 / expected)
        elapsed_time += solve
        next_target = asert_next_target(anchor_target, elapsed_time, height, half_life)
        rows.append((height, hashrate, solve, target, next_target))
        target = next_target

    return rows


def asert_summary(name: str, half_life: int, seeds: int = 20):
    means = []
    for seed in range(1, seeds + 1):
        rows = run_asert(name, half_life=half_life, seed=seed)
        tail = rows[1000:]
        means.append(sum(row[2] for row in tail) / len(tail))
    return sum(means) / len(means)


# --- Timestamp-adversary stress tests -----------------------------------------

def median_time(values):
    window = sorted(values[-11:])
    return window[len(window) // 2]


def run_timestamp_attack(policy: str, blocks: int = 500, seed: int = 1, half_life: int = 4320):
    """Worst-case timestamp stress model for consensus research."""
    rng = random.Random(seed)
    real_time = 0.0
    header_times = []
    target = 1.0
    rows = []

    for height in range(1, blocks + 1):
        expected = TARGET_SECONDS / target
        solve = rng.expovariate(1.0 / expected)
        real_time += solve

        if height < 100 or policy == "honest":
            candidate = int(real_time)
            if header_times:
                candidate = max(candidate, header_times[-1] + 1)
        elif policy == "future_90":
            candidate = int(real_time) + 90
        elif policy == "minimum_mtp":
            candidate = median_time(header_times) + 1
        else:
            raise ValueError(policy)

        if header_times:
            candidate = max(candidate, median_time(header_times) + 1)

        candidate = min(candidate, int(real_time) + 90)
        header_times.append(candidate)

        target = asert_next_target(1.0, candidate, height, half_life)
        rows.append((height, solve, real_time, candidate, target))

    return rows


def timestamp_attack_summary(policy: str):
    rows = run_timestamp_attack(policy)
    tail = rows[100:]
    mean_solve = sum(row[1] for row in tail) / len(tail)
    final = rows[-1]
    return mean_solve, final[4], final[3] - final[2]


# --- Partial attacker / botnet stress tests -----------------------------------

def run_partial_timestamp_attacker(
    attacker_share: float,
    blocks: int = 3000,
    seed: int = 1,
    half_life: int = 4320,
    attack_start: int = 200,
):
    """Model a miner coalition using minimum-MTP timestamps on blocks it wins.

    Honest miners use simulated real time, subject to the same MTP validity
    floor. The target for each block is derived from the already-accepted
    parent chain; a miner cannot choose the target of the block it is mining.
    """
    rng = random.Random(seed)
    real_time = 0.0
    header_times = []
    target = 1.0
    rows = []

    for height in range(1, blocks + 1):
        total_hash = 1.0
        expected = TARGET_SECONDS / (total_hash * target)
        solve = rng.expovariate(1.0 / expected)
        real_time += solve

        attacker_wins = height >= attack_start and rng.random() < attacker_share

        if not header_times:
            header_time = max(1, int(real_time))
        elif attacker_wins:
            header_time = median_time(header_times) + 1
        else:
            header_time = max(int(real_time), median_time(header_times) + 1)

        # Existing consensus future-time bound.
        header_time = min(header_time, int(real_time) + 90)
        header_times.append(header_time)

        # This accepted block affects the target of the NEXT block.
        next_target = asert_next_target(1.0, header_time, height, half_life)
        rows.append(
            (height, solve, real_time, header_time, target, next_target, attacker_wins)
        )
        target = next_target

    return rows


def partial_attacker_summary(attacker_share: float, seeds: int = 20):
    mean_solves = []
    mean_targets = []
    attacker_blocks = []
    max_lags = []

    for seed in range(1, seeds + 1):
        rows = run_partial_timestamp_attacker(attacker_share, seed=seed)
        tail = rows[500:]
        mean_solves.append(sum(row[1] for row in tail) / len(tail))
        mean_targets.append(sum(row[4] for row in tail) / len(tail))
        attacker_blocks.append(sum(1 for row in tail if row[6]) / len(tail))
        max_lags.append(max(row[2] - row[3] for row in tail))

    return {
        "share": attacker_share,
        "mean_solve": sum(mean_solves) / len(mean_solves),
        "mean_target": sum(mean_targets) / len(mean_targets),
        "won_fraction": sum(attacker_blocks) / len(attacker_blocks),
        "mean_max_header_lag": sum(max_lags) / len(max_lags),
    }


def run_botnet_burst(
    botnet_multiplier: float,
    burst_start: int = 500,
    burst_blocks: int = 120,
    blocks: int = 2000,
    seed: int = 1,
    half_life: int = 4320,
):
    """Model temporary stolen CPU hashrate joining and then disappearing.

    A multiplier of 10 means aggregate hashrate is 10x baseline during the
    burst. Timestamps remain honest in this scenario.
    """
    rng = random.Random(seed)
    real_time = 0.0
    header_times = []
    target = 1.0
    rows = []

    burst_end = burst_start + burst_blocks

    for height in range(1, blocks + 1):
        hash_multiplier = (
            botnet_multiplier if burst_start <= height < burst_end else 1.0
        )
        expected = TARGET_SECONDS / (hash_multiplier * target)
        solve = rng.expovariate(1.0 / expected)
        real_time += solve

        if header_times:
            header_time = max(int(real_time), median_time(header_times) + 1)
        else:
            header_time = max(1, int(real_time))
        header_times.append(header_time)

        next_target = asert_next_target(1.0, header_time, height, half_life)
        rows.append((height, solve, hash_multiplier, target, next_target))
        target = next_target

    return rows


def botnet_burst_summary(multiplier: float, seeds: int = 20):
    during = []
    post = []
    final_targets = []

    for seed in range(1, seeds + 1):
        rows = run_botnet_burst(multiplier, seed=seed)
        burst = [r for r in rows if 500 <= r[0] < 620]
        recovery = [r for r in rows if 620 <= r[0] < 900]
        during.append(sum(r[1] for r in burst) / len(burst))
        post.append(sum(r[1] for r in recovery) / len(recovery))
        final_targets.append(rows[-1][4])

    return {
        "multiplier": multiplier,
        "burst_mean_solve": sum(during) / len(during),
        "post_burst_mean_solve": sum(post) / len(post),
        "final_target": sum(final_targets) / len(final_targets),
    }


if __name__ == "__main__":
    main()
    print("\nPartial timestamp attacker — 4320-second half-life")
    for share in (0.10, 0.25, 0.33, 0.40, 0.50, 0.60):
        print(partial_attacker_summary(share))

    print("\nTemporary botnet hashrate bursts — honest timestamps")
    for multiplier in (2.0, 5.0, 10.0, 25.0):
        print(botnet_burst_summary(multiplier))
