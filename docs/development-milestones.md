# Development Milestones

NIAHCIA development is organized around reproducible technical checkpoints rather than obsolete version labels.

## Completed foundation — native chain path

The reference node has moved from the former external-execution prototype to native NIAHCIA execution.

Current green foundation includes deterministic native accounts/state roots, NativeTransaction V1, Transfer execution, native block commitments, atomic persistence, RandomX PoW, cumulative-work fork choice, P2P Version 3, shared native mempool, non-empty mining templates, and restart/reorg handling.

The old Reth/EVM development path is superseded and must not be used as current setup guidance.

## Current milestone — inactive compute settlement proof

NativeStateV2 and NativeTransaction V2 compute-channel functionality is being built behind an inactive boundary.

The milestone is complete only when Open, Settle, and Refund are proven across deterministic validation, exact state-root mutation, nonce/value accounting, stale/duplicate rejection, atomic rollback, persistence/restart, reorg behavior, canonical vectors, reviewed fee/gas accounting, and explicit activation rules.

No V2 compute action is accepted by active mempool/P2P/mining until that boundary is deliberately activated.

## Next milestone — first decentralized AI payment

1. wallet owns spendable NIAH;
2. wallet opens a worker-bound channel;
3. funds are locked exactly once;
4. several AI jobs execute off-chain;
5. wallet signs increasing usage receipts;
6. worker submits the final receipt;
7. worker receives the acknowledged amount;
8. unused value returns to the wallet;
9. a second terminal transition is rejected;
10. CPU-PoW chain progress remains independent of worker availability.

## Release rule

A milestone is complete only when its acceptance target is reproducible from a clean environment with documented commands and green CI.
