# NIAHCIA Fee Policy V1

Status: **selected pre-production policy; consensus activation pending implementation tests**

This document locks the intended NIAHCIA V1 transaction-fee destination policy while preserving the EIP-1559-style fee-market mechanics provided by the execution layer.

## Decision

NIAHCIA V1 uses two economically distinct fee components:

1. **Base fee** — protocol-computed congestion price; **burned**.
2. **Priority fee** — sender-selected inclusion incentive; **paid to the canonical CPU-PoW block producer**.

The CPU block producer therefore receives:

`cpu_subsidy + priority_fees`

and does **not** receive the base fee.

The base fee does not fund GPU AI workers, storage/service nodes, a treasury, developers, a foundation, or any fixed protocol address.

## Why retain base-fee burning

NIAHCIA does not adopt burning merely because Reth/EVM supports it. It retains the property because paying the protocol-computed base fee directly to the block producer weakens the economic separation between the congestion price and the producer's inclusion incentive and can create incentives to manipulate the fee mechanism.

The priority fee remains the direct market incentive for transaction inclusion and ordering. CPU miners additionally receive the deterministic PoW subsidy defined by `monetary-policy-v1.md` and `monetary.rs`, including the permanent tail subsidy.

This means NIAHCIA does not depend on volatile transaction fees alone for long-run CPU consensus security.

## Canonical arithmetic

For a transaction with:

- `gas_used`
- `base_fee_per_gas`
- `max_fee_per_gas`
- `max_priority_fee_per_gas`

V1 requires:

`max_fee_per_gas >= base_fee_per_gas`

and defines:

`priority_fee_per_gas = min(max_priority_fee_per_gas, max_fee_per_gas - base_fee_per_gas)`

`effective_fee_per_gas = base_fee_per_gas + priority_fee_per_gas`

`base_fee_burn = gas_used * base_fee_per_gas`

`producer_priority_fee = gas_used * priority_fee_per_gas`

`actual_fee = base_fee_burn + producer_priority_fee`

All native NIAH accounting at the NIAHCIA monetary boundary uses integer atomic units (`aniah`) and checked arithmetic. Overflow or underflow is invalid; arithmetic MUST NOT wrap or saturate.

The sender's maximum pre-execution reservation remains:

`gas_limit * max_fee_per_gas`

plus transferred `value`, with unused reservation/refundable gas handled according to the canonical execution rules.

## Supply accounting

Base-fee burning MUST be visible in monetary accounting.

At minimum, clients MUST be able to distinguish:

- `gross_issued_supply` — all consensus-authorized newly created NIAH;
- `protocol_burned_supply` — all NIAH destroyed by consensus fee rules;
- `net_protocol_supply = gross_issued_supply - protocol_burned_supply`.

Priority fees are transfers from transaction senders to block producers and do not change gross or net protocol supply.

CPU subsidies increase `gross_issued_supply`.

Base-fee burns increase `protocol_burned_supply`.

Lost keys are not protocol burns.

## Reorg requirements

Fee accounting is canonical-chain state.

If a block is removed by a reorg, its:

- CPU subsidy issuance;
- base-fee burn accounting; and
- producer priority-fee effects

MUST be reverted with the block's state transition. Applying the replacement canonical block MUST then apply that block's own issuance, burns, and fee transfers exactly once.

No cumulative monetary counter may advance irreversibly before canonicality is established by the normal chain-state machinery.

## Separation from other economic lanes

Transaction base fees and priority fees are not AI-job payments and are not storage/service payments.

V1 MUST NOT automatically split either fee component among CPU miners, GPU workers, storage nodes, service nodes, agents, developers, or a treasury.

AI and storage/service economics remain separately versioned protocols.

## No treasury redirect

V1 intentionally rejects redirecting the base fee to a protocol treasury or fixed address. Such a redirect would create an accumulating consensus-controlled economic pool requiring separate governance, spending authorization, security assumptions, and accounting rules.

A future protocol version may propose a different destination only through an explicit consensus upgrade; it MUST NOT be introduced as an implementation detail.

## Required consensus tests before activation

Before fee policy is treated as production-active, tests MUST cover at least:

1. zero-gas/zero-fee boundary behavior;
2. `max_fee_per_gas < base_fee_per_gas` rejection;
3. priority fee capped by `max_fee_per_gas - base_fee_per_gas`;
4. exact base-fee burn arithmetic;
5. exact priority-fee producer credit;
6. exact sender debit and unused-gas/refund behavior;
7. checked multiplication/addition overflow rejection;
8. block containing multiple transactions;
9. empty block;
10. reorg rollback and replacement of burn/priority accounting;
11. restart/replay reproducing identical monetary counters;
12. devnet/testnet/mainnet using the same arithmetic unless an explicitly versioned test fixture says otherwise.

## V1 economic invariant

For every canonical transaction:

`sender fee paid = protocol base fee burned + producer priority fee received`

subject only to the execution protocol's canonical gas-used/refund calculation.

For every canonical block:

`producer native consensus revenue = CPU subsidy + sum(priority fees)`

The base fee is never producer revenue.

This policy is selected for NIAHCIA V1 but remains pre-production until the implementation and canonical-state/reorg tests are merged and passing.
