# Native Execution V2 Activation Boundary

Status: **CANDIDATE / REVIEW REQUIRED / INACTIVE**

## Purpose

This document defines the migration boundary between active Native Execution V1 / NativeStateV1 and the candidate Native Execution V2 / NativeStateV2 path.

It does not assign a real activation height and does not activate V2 behavior.

## Activation parameter

A concrete network that upgrades from V1 to V2 must define one explicit:

```text
native_execution_v2_activation_height
```

The migration form of this activation height must be greater than zero.

For a configured activation height `H`:

- block heights below `H` use Native Execution V1 and NativeStateV1;
- block height `H` and later use the V2 execution/state boundary;
- the parent state used to begin block `H` is the canonical NativeStateV1 state at height `H - 1`, deterministically converted to NativeStateV2.

No current devnet/testnet/mainnet activation height is assigned by this specification.

## Migration rule

At activation height `H`, the canonical parent NativeStateV1 is migrated using the already-vectored V1 -> V2 migration rule:

- every V1 account balance is preserved exactly;
- every V1 account nonce is preserved exactly;
- the ComputeChannel map starts empty;
- the resulting NativeStateV2 root is determined solely by the canonical migrated state.

The migration must use the canonical parent state selected by CPU-PoW cumulative-work fork choice. A node must not migrate an arbitrary side-branch snapshot and treat it as canonical.

## Body and transaction boundary

Before `H`:

- NativeBlockBodyV1 remains the active body format;
- SignedNativeTransactionV1 remains the active transaction schema;
- NativeBlockExecutionResultV1 / NativeStateV1 remain the active persistence boundary.

At and after `H`:

- the candidate successor boundary is NativeBlockBodyV2 + NativeBlockExecutionResultV2 + NativeStateV2;
- NativeBlockBodyV1 bytes are not reinterpreted as V2;
- V1 signed transactions may still be carried explicitly inside the versioned V2 body where allowed by the V2 body rules;
- NativeTransaction V2 compute actions remain subject to their own validation, gas/fee, and activation requirements.

## Smart-contract boundary

This activation boundary does not invent or activate ContractCall / ContractCreate runtime semantics.

Smart contracts remain a required NIAHCIA base-chain feature, but contract execution requires its separately specified native runtime, gas, storage, failure/revert, receipt, persistence, and activation rules.

## Persistence boundary

The inactive reference implementation already has a dedicated atomic V2 insertion path that binds in one redb transaction:

- BlockHeaderV1;
- NativeBlockBodyV2;
- NativeBlockExecutionResultV2;
- NativeStateV2;
- cumulative-work chain record;
- best-head promotion.

That development path remains inactive until a concrete network parameter selects an activation height and the remaining V2 fee/gas rules and activation vectors are locked.

## Required activation vectors

Before activation, interoperability fixtures must cover at least:

1. last V1 block at height `H - 1`;
2. exact canonical NativeStateV1 parent snapshot/root;
3. deterministic migrated NativeStateV2 snapshot/root;
4. first valid V2 block at height `H`;
5. rejection of V2 block/body behavior before `H`;
6. rejection of V1-only block execution rules at/after `H` where superseded;
7. restart at `H - 1`, `H`, and `H + 1`;
8. reorg across the activation boundary;
9. preservation of locked V1 vectors;
10. preservation of locked NativeReceiptV2 / NativeBlockExecutionResultV2 vectors.

## Non-activation statement

The Rust `NativeExecutionActivationV2` helper is an inactive protocol-boundary utility only.

It is not read from NodeConfig, is not connected to mempool/P2P/mining, and does not change active devnet behavior.
