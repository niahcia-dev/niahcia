# Execution Payload and PoW Work Types

## Purpose

The first NIAHCIA mining boundary separates:

1. execution results produced by Reth,
2. immutable PoW work selected by NIAHCIA consensus,
3. the nonce searched by a CPU miner.

The critical rule is:

> Changing the mining nonce must not require EVM re-execution.

## Execution commitment

`ExecutionPayloadCommitments` contains the execution-layer values that the PoW layer needs to bind into a block candidate:

- parent hash
- fee recipient
- state root
- receipts root
- transactions root
- block number
- gas limit
- gas used
- timestamp
- base fee

The current development encoding is explicit, fixed-order binary data using big-endian integers.

Its Keccak-256 digest becomes the `execution_commitment` carried by a PoW work template.

## PoW work template

`PowWorkTemplate` currently contains:

- version
- block height
- parent hash
- execution commitment
- timestamp
- difficulty
- target

These fields are immutable for one work-template identity.

The template ID is:

```text
keccak256(canonical_work_bytes)
```

## Mining header

The mutable nonce is appended only when producing the bytes to be hashed:

```text
NIAHCIA/POW-HEADER/V1
||
canonical_work_bytes
||
nonce_u64_be
```

Therefore:

```text
same execution
+ same work template
+ different nonce
= no EVM rerun
```

## Domain separation

Current development domains:

```text
NIAHCIA/EXECUTION-COMMITMENT/V1
NIAHCIA/POW-WORK/V1
NIAHCIA/POW-HEADER/V1
```

## Tests

The module currently verifies that:

- execution commitments are deterministic,
- template identity does not depend on the nonce,
- only the final eight header bytes change when the nonce changes,
- changing the execution state commitment changes the work-template identity.

## Status

These are **development types for v0.1.0**, not frozen mainnet consensus serialization.

Before a public consensus network exists, the final block-header encoding and numeric difficulty/target representation must be specified and test-vectored in `niahcia-protocol`.
