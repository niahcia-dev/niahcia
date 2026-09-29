# PoW ↔ Reth Interface

## Design rule

The consensus process treats Reth as the EVM state-transition engine. Reth must independently validate imported blocks; neither process blindly trusts the other.

## Consensus daemon responsibilities

- canonical parent choice
- cumulative-work fork choice
- RandomX PoW
- difficulty calculation
- timestamp validation
- mining template lifecycle
- block-header consensus fields
- peer block propagation
- chain reorganization decisions

## Reth responsibilities

- transaction pool
- transaction ordering policy for payload construction
- EVM execution
- gas and base-fee accounting
- state root
- transaction and receipt roots
- logs and receipts
- Ethereum JSON-RPC

## Candidate custom RPC namespace

```text
pow_getPayloadTemplate
pow_submitMinedPayload
pow_validatePayload
pow_setCanonicalHead
pow_getExecutionStatus
```

Exact wire definitions remain to be specified.

## Mining flow

```text
powd
  |
  | choose parent / timestamp / difficulty
  v
Reth
  |
  | execute candidate transactions
  | produce payload + state commitments
  v
powd
  |
  | mine RandomX over PoW header
  v
FOUND
  |
  | submit completed mined payload
  v
Reth
  |
  | independently validate execution
  v
powd
  |
  | broadcast accepted block
```

Changing the mining nonce must not require rerunning EVM execution.

## Template refresh

Prototype policy should rebuild a candidate on:

- a new canonical parent
- periodic timeout
- material fee-pool change
- explicit invalidation of the current candidate

The starting periodic refresh target is approximately 5 seconds for a ~30 second target block interval, subject to measurement.

## Reorganizations

Fork choice is based on cumulative PoW. When the consensus daemon chooses a new canonical branch, the execution engine must unwind/replay state to the selected canonical head.

AI or service-node state has no special fork-choice authority; it is ordinary EVM state and follows the canonical chain.
