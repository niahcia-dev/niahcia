# NIAHCIA Timestamp Rules V1

## Status

Draft consensus candidate.

## Target interval

```text
TARGET_BLOCK_INTERVAL = 30 seconds
```

## Median-time-past

For a non-genesis candidate, gather up to the previous 11 canonical ancestor timestamps.

```text
MTP_WINDOW = 11
```

Sort the available timestamps numerically and use the middle value.

A candidate block is valid only when:

```text
candidate.timestamp > median_time_past
```

During the first 10 blocks, use all available ancestors.

## Future drift

A node MUST reject a candidate whose timestamp is more than:

```text
MAX_FUTURE_DRIFT = 90 seconds
```

ahead of the node's adjusted network time.

For Prototype 0, adjusted network time may initially be local system time. A peer-derived adjusted-time algorithm may be specified before public testnet.

## Purpose

These rules prevent a single miner from:

- moving chain time backward,
- reusing an old timestamp indefinitely,
- pushing chain time arbitrarily far into the future.

## Difficulty interaction

Difficulty adjustment MUST use robust historical timing and MUST NOT trust only the immediately previous block interval.

The difficulty specification defines the exact window and clamps.
