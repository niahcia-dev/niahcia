# First Decentralized AI Milestone

The former Reth/EVM + fixed 2-of-3 Prototype 0 profile is superseded.

## Chain

- RandomX CPU PoW;
- cumulative-work fork choice;
- native NIAHCIA transactions/execution/state;
- CPU PoW remains the sole consensus authority.

## AI execution

- replaceable worker runtime such as vLLM;
- worker advertisements and selection remain off-chain;
- wallet chooses a worker without a centralized scheduler;
- one bounded ComputeSession uses a selected worker;
- prompts/results/token streams remain off-chain.

## Payment

- wallet opens one worker-bound ComputeChannel;
- many Jobs may use that channel;
- wallet signs cumulative usage receipts;
- worker submits the final receipt;
- unused authorized value returns to the funding account;
- if no settlement occurs, the wallet may refund after the defined timeout.

## Storage

Ordinary AI inference does not require decentralized storage. Wallet/client-local encrypted history and memory are the V1 default.

## Acceptance test

1. wallet has spendable NIAH;
2. wallet opens a bounded worker ComputeChannel;
3. native state locks the exact authorization;
4. multiple jobs execute off-chain;
5. cumulative usage reaches a known amount;
6. worker settles the final signed receipt;
7. worker receives exactly the acknowledged amount;
8. remainder returns to the wallet;
9. duplicate settlement/refund is rejected;
10. ordinary transfers and CPU-PoW progress continue even if all AI workers disappear.
