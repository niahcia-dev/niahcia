# Prototype 0 Implementation Profile

Protocol v1 is intentionally broader than Prototype 0. Prototype 0 exists to prove one complete decentralized path.

## Consensus

- CPU-friendly PoW: RandomX
- target block interval: approximately 30 seconds
- fork choice: highest cumulative work
- execution engine: Reth
- smart-contract environment: EVM / Solidity
- mining reward and gas accounting enabled with test currency

## AI execution

- runtime: vLLM
- workload: text inference
- one pinned Qwen3-class approximately 8B model
- exact model, tokenizer, runtime, and execution-profile hashes
- initial accelerator profile: NVIDIA Ampere class
- one worker execution per GPU job instance
- streaming output over the AI P2P network

## Verification

Initial integration verification:

- deterministic redundant execution
- three selected workers
- two matching canonical token-sequence commitments finalize the result
- commit-before-reveal prevents trivial copying
- no automatic collateral slashing for disagreement in Prototype 0

The interfaces must already allow future replacement with:

- single-executor optimistic verification
- random audits
- bonded challenges
- dispute resolution
- TEE or proof-based verification profiles

## Service nodes

Prototype 0 service-node duties:

- register
- replicate the canonical model
- announce availability
- serve model chunks
- prove basic availability
- receive storage/retrieval accounting

## Required deployment

Minimum demonstration environment:

- 3 independent chain/full nodes capable of CPU mining
- 3 independent compute workers
- 2 service nodes

## Acceptance test

1. Deploy AI contracts.
2. Register the canonical model and execution profile.
3. Replicate model data across both service nodes.
4. Register compute workers.
5. Submit an AI request through an EVM transaction/session flow.
6. Select workers without a centralized scheduler.
7. Workers retrieve input through P2P.
8. Workers obtain and validate the canonical model.
9. vLLM executes independently.
10. The client receives streaming response data.
11. Workers publish commitments.
12. Verification finalizes a canonical output commitment.
13. Compute payments settle independently from CPU mining rewards.
14. Service payments settle independently.
15. A contract/client can read the finalized job result.

## Failure tests

The system must continue appropriately after:

- one CPU miner is stopped
- one compute worker is stopped
- one service node is stopped
- the original bootstrap/developer node is stopped after peers are established
- nodes restart in a different order

The official website is not required to pass the protocol acceptance test.
