# NIAHCIA Specification Status Audit

Last reviewed: 2026-10-02

This file is a living audit of protocol status. Implementation alone does not make behavior normative.

## Status vocabulary

- **LOCKED** — normative for the stated version.
- **CANDIDATE** — intended direction still awaiting review, vectors, testing, activation, or freeze criteria.
- **IMPLEMENTED (DEV)** — present in the reference implementation.
- **VECTORED** — machine-readable interoperability vectors exist.
- **SUPERSEDED** — retained only for history/research.
- **REVIEW REQUIRED** — unresolved design/security/interoperability work prevents freeze.

## Chain / consensus

| Surface | Status | Notes |
| --- | --- | --- |
| BlockHeader V1 | CANDIDATE + VECTORED + IMPLEMENTED | 164-byte development header remains active; stock miner/pool interoperability still needs final review. |
| RandomX PoW V1 | CANDIDATE + VECTORED + IMPLEMENTED | CPU PoW is sole consensus authority; final production preimage/epoch parameters remain open. |
| Difficulty ASERT V1 | CANDIDATE + VECTORED + REVIEW REQUIRED | Timestamp-adversary interaction must be resolved before freeze. |
| Timestamp Rules V1 | CANDIDATE + REVIEW REQUIRED | Must be reviewed with difficulty policy. |
| Network Parameters V1 | CANDIDATE | Production genesis, PoW limits, activation heights, and final chain parameters remain unfrozen. |
| P2P Native Block Transfer V3 | IMPLEMENTED (DEV) | Active devnet transport for native blocks/transactions. |
| P2P V2 | SUPERSEDED | Historical/header-only transport; not active runtime. |

## Native value / identity / execution

| Surface | Status | Notes |
| --- | --- | --- |
| Address V1 | LOCKED + VECTORED + IMPLEMENTED | Bech32m; Account/Contract kinds; main/test/dev HRPs. |
| Monetary representation | LOCKED primitives | 8 decimals and integer aniah accounting; broader issuance policy remains separate. |
| NativeTransaction V1 | IMPLEMENTED + VECTORED | Active transfer path. ContractCall/ContractCreate runtime semantics remain intentionally undefined. |
| Native Execution V1 | IMPLEMENTED + VECTORED | Active deterministic native execution path. |
| NativeState V1 | IMPLEMENTED + VECTORED | Active accounts-only state/snapshot path. |
| NativeBlockBody V1 | IMPLEMENTED | Active canonical non-empty block-body path. |
| NativeBlockBody V2 | IMPLEMENTED (DEV) + INACTIVE | Versioned canonical body for ordered V1/V2 signed transactions; not admitted to active runtime. |
| Native Execution Commitment V2 | IMPLEMENTED (DEV) + VECTOR LOCKED + REVIEW REQUIRED + INACTIVE | Domain-separated ReceiptV2/ExecutionResultV2 commits transaction schema/action, per-transaction post-state roots, aggregate state/gas/fees. `test-vectors/native-execution-v2.json` locks canonical receipt/result bytes and commitments; activation remains open. |
| Native Compute Gas V1 | IMPLEMENTED (DEV) + VECTORED + CANDIDATE + INACTIVE | Open=3,000, Settle=5,000, Refund=2,000 intrinsic gas; transfer-shaped base-fee burn / producer-priority accounting; inactive V2 only. |
| ContractCreate Payload V1 | IMPLEMENTED (DEV) + VECTOR LOCKED + CANDIDATE + INACTIVE | NCE/1 runtime_id/code/init_data boundary; runtime_id zero reserved; code/init each capped at 65,536 bytes; no VM selected or activated. |
| Native Contract State V1 | IMPLEMENTED (DEV) + VECTORED + CANDIDATE + INACTIVE | Contract balance, opaque runtime/code bytes, fixed 32-byte key/value storage, contracts root, and NativeStateV3 successor snapshot/root are defined and vectored without selecting a VM. |
| Native Execution V2 activation boundary | IMPLEMENTED (DEV) + VECTORED + CANDIDATE + REVIEW REQUIRED + INACTIVE | Explicit V1-before / V2-at-and-after height rule plus deterministic V1 -> V2 parent-state migration helper. Locked fixture covers H-1/H/H+1, rejection, restart, and reorg classification. No real activation height is assigned or wired into active runtime. |
| Native smart-contract runtime | REQUIRED + REVIEW REQUIRED + INACTIVE | Smart contracts are a core protocol requirement. ContractCall/ContractCreate are reserved in NativeTransaction V1, but runtime/state/storage/gas/revert/receipt semantics are not yet implemented. |
| NativeState V2 | IMPLEMENTED (DEV) + VECTORED + INACTIVE | Accounts + ComputeChannel state; no activation height set. |
| NativeTransaction V2 | IMPLEMENTED (DEV) + VECTORED + INACTIVE | Explicit ComputeChannel actions; not admitted to active mempool/P2P/mining. |
| Compute action payloads | IMPLEMENTED (DEV) + VECTORED | Canonical Open/Settle/Refund payloads. |
| ComputeUsageReceipt V1 | IMPLEMENTED (DEV) + VECTORED | Canonical signed cumulative usage receipt. |

## Compute settlement

Open, Settle, and Refund planners, inactive atomic application helpers, the signed-V2 compute dispatcher, and candidate-batch atomic execution are implemented and tested.

Current guarantees include:

- exact nonce binding;
- checked integer accounting;
- exact channel pre-state binding for terminal transitions;
- stale-plan rejection;
- duplicate-terminal rejection;
- rollback-on-error via clone-then-commit state mutation;
- deterministic state-root/snapshot round trips;
- non-compute V2 actions are rejected by the inactive compute dispatcher;
- ordered dispatcher results expose exact before/after NativeStateV2 roots;
- multi-transaction candidate batches publish no mutation unless every compute transaction succeeds;
- successful batch transitions form one deterministic state-root chain.

Persistence/restart/reorg proof is now implemented for transition-derived V2 state: Open, Settle, and Refund snapshots survive restart with exact roots, and detached transition effects are removed when canonical state is restored from the winning branch.

Still required before activation:

1. explicit fee/gas schedule;
2. a concrete network activation height;
3. activation/migration interoperability vectors covering H - 1, H, H + 1 and reorgs across H;
4. active-runtime wiring of the V2 boundary without changing locked V1 behavior.

## AI / Agent architecture

Current direction:

- wallet/client-local encrypted chat history and Agent memory are V1 defaults;
- wallet controls Agent identity/capability;
- portals are clients, not owners of identity or memory;
- worker discovery/selection is wallet-local and off-chain for the first milestone;
- jobs/prompts/results/token streams remain off-chain;
- one worker-bound ComputeChannel may aggregate many jobs;
- AI workers receive no consensus authority;
- fixed universal 2-of-3 verification is superseded by policy-driven verification.

Primary Agent, capability, recovery, workflow, execution-receipt, and knowledge objects remain candidate protocol work unless explicitly marked otherwise in their specifications.

## Storage / service architecture

Storage/service protocols are optional service layers and never fork-choice/finality authorities.

The first ordinary AI milestone does not require decentralized storage. Existing storage commitment/challenge/response/service-epoch vectors remain useful candidate interoperability fixtures but do not imply production activation.

## Canonical vectors currently present

Machine-readable vectors include:

- Address V1;
- BlockHeader V1;
- core serialization/object fixtures;
- RandomX PoW;
- difficulty candidates;
- NativeTransaction V2;
- NativeState V2;
- native compute action payloads;
- ComputeUsageReceipt V1;
- storage commitment/challenge/response;
- service epoch report;
- research fixtures for botnet/reorg analysis.

## Repository audit findings resolved

The 2026-10-02 repository audit removed a major source of protocol/documentation drift:

- obsolete Reth/EVM/Engine-API/JWT runtime guidance;
- obsolete Reth devnet helper scripts;
- Reth-dependent smoke/handoff paths;
- stale README/roadmap/architecture/milestone/prototype descriptions;
- silent acceptance of unknown TOML configuration fields.

The active reference node is native-execution only.

## Highest-priority open protocol work

1. Preserve the now-locked NativeReceiptV2 / NativeBlockExecutionResultV2 vectors; incompatible changes require an explicit successor version.
2. Preserve the locked activation/migration fixture in `test-vectors/native-execution-v2-activation.json`; only after remaining fee/gas and runtime review should a concrete network activation parameter be selected.
3. Preserve the vectored Native Contract State V1 / NativeStateV3 and ContractCreate payload boundaries, then define the activated runtime registry/code validation and deterministic call/create/revert semantics without reintroducing an external EVM dependency.
4. Review the now-vectored candidate compute gas/fee schedule together with V2 activation economics before selecting a concrete activation height.
5. Resolve stock miner/pool RandomX interoperability.
6. Resolve difficulty/timestamp hardening.
7. Finalize public-testnet genesis/network/monetary parameters.
8. Keep `niahcia` and the retained `niahcia-protocol` mirror synchronized for protocol-visible changes.

## Documentation rule

Protocol-visible changes require synchronized implementation, specification/status, and vectors when consensus/wire behavior changes. Superseded behavior should be removed or clearly labeled rather than left as a competing current path.
