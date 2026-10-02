#!/usr/bin/env python3
"""Research-only private-chain / botnet reorg simulator for NIAHCIA.

This models two independent branches that start from the same accepted tip:

- public branch: baseline honest hashrate = 1.0
- private branch: attacker/botnet hashrate = a multiple of honest baseline

Both branches use honest wall-clock timestamps in this model and independently
follow the 30-second / 4320-second ASERT research candidate.

The simulation compares accumulated work, not block count.

This is not consensus code and does not model networking, selfish mining,
eclipse attacks, or timestamp manipulation.
"""

from __future__ import annotations

import argparse
import random
from dataclasses import dataclass

TARGET_SECONDS = 30.0
HALF_LIFE = 4320.0


@dataclass
class Branch:
    hash_rate: float
    blocks: int = 0
    target: float = 1.0
    work: float = 0.0
    next_time: float = 0.0


def asert_target(elapsed_time: float, blocks: int) -> float:
    return 2.0 ** ((elapsed_time - TARGET_SECONDS * blocks) / HALF_LIFE)


def schedule_next(rng: random.Random, now: float, branch: Branch) -> float:
    if branch.hash_rate <= 0.0:
        return float("inf")

    rate = branch.hash_rate * branch.target / TARGET_SECONDS
    return now + rng.expovariate(rate)


def run_once(attacker_hash: float, duration_seconds: float, seed: int):
    rng = random.Random(seed)
    public = Branch(hash_rate=1.0)
    private = Branch(hash_rate=attacker_hash)

    now = 0.0
    public.next_time = schedule_next(rng, now, public)
    private.next_time = schedule_next(rng, now, private)

    while True:
        event_time = min(public.next_time, private.next_time)
        if event_time > duration_seconds:
            break

        now = event_time

        if public.next_time <= private.next_time:
            public.blocks += 1
            public.work += 1.0 / public.target
            public.target = asert_target(now, public.blocks)
            public.next_time = schedule_next(rng, now, public)
        else:
            private.blocks += 1
            private.work += 1.0 / private.target
            private.target = asert_target(now, private.blocks)
            private.next_time = schedule_next(rng, now, private)

    return {
        "private_heavier": private.work > public.work,
        "public_blocks": public.blocks,
        "private_blocks": private.blocks,
        "public_work": public.work,
        "private_work": private.work,
    }


def summarize(attacker_hash: float, duration_minutes: int, runs: int):
    results = [
        run_once(attacker_hash, duration_minutes * 60.0, seed)
        for seed in range(1, runs + 1)
    ]

    successes = sum(1 for result in results if result["private_heavier"])
    mean_public = sum(result["public_blocks"] for result in results) / runs
    mean_private = sum(result["private_blocks"] for result in results) / runs

    return {
        "attacker_hash_vs_honest": attacker_hash,
        "attacker_share_of_combined_hash": attacker_hash / (1.0 + attacker_hash),
        "duration_minutes": duration_minutes,
        "runs": runs,
        "private_chain_heavier_fraction": successes / runs,
        "mean_public_blocks": mean_public,
        "mean_private_blocks": mean_private,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5000)
    args = parser.parse_args()

    for attacker_hash in (0.25, 0.5, 1.0, 2.0, 5.0, 10.0):
        for minutes in (5, 15, 30):
            print(summarize(attacker_hash, minutes, args.runs))


if __name__ == "__main__":
    main()
