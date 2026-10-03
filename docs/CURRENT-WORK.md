# NIAHCIA Current Work / Session Handoff

**Purpose:** First document a new development session should read. It summarizes current NIAHCIA state, decisions to preserve, unresolved work, and safest next tasks.

**Maintenance rule:** Update this file whenever priorities, blockers, locked decisions, or major implementation state change. Keep `docs/spec-status.md` synchronized as the detailed inventory.

## Resume sequence

1. Read this file.
2. Read `docs/spec-status.md`.
3. Read `docs/why-niahcia.md`.
4. Read relevant specifications before changing behavior.
5. Inspect current `niahcia/niahcia` implementation and GitHub failures/issues.
6. Keep implementation docs and `niahcia-protocol` synchronized.
7. Never silently replace locked interoperability behavior; version changes and vectors together.

## Core architecture

- CPU PoW is canonical chain authority; RandomX is current candidate.
- GPU/accelerator AI compute is separate from mining.
- Smart contracts are a core base-chain requirement, not a future optional feature. NativeTransaction V1 already reserves `ContractCall` and `ContractCreate`; their deterministic native runtime remains intentionally unimplemented until separately specified.
- Decentralized storage/service nodes are optional service providers, never fork-choice/finality authorities, and are not required for ordinary AI inference.
- Agents/models/jobs/capabilities/memory/verification/payment are explicit protocol objects.
- NIAHCIA native execution is now the active node execution path. The legacy Reth/EVM/Engine API/JWT integration, replay journals, and external execution-hash mappings have been removed from the reference node.
- V1 AI state is local-first: wallet/client-local encrypted chat history and Agent memory are the default; decentralized persistence is deferred and optional.
- Key differentiation objectives include portable Agent sovereignty, decentralized compute access, bounded wallet authority, and an ownerless decentralized Primary Agent as a network public good.

## Non-negotiable boundaries

CPU PoW alone determines canonical chain by cumulative valid work. Service/storage nodes do not vote on canonical chain. Execution hosts are replaceable and do not automatically own/control Agents. The former fixed universal 2-of-3 verification profile is superseded by policy-driven verification. Storage may preserve ciphertext without decryption/governance/treasury/succession authority. Distinct provider keys do not prove independent durability. NCE/1 deterministic CBOR is canonical serialization foundation.

## Primary Agent — core objective

`spec/primary-agent-v1.md` defines the candidate architecture for NIAHCIA's default public conversational Agent.

The Primary Agent is intended to be a persistent, ChatGPT-style entry point for ordinary users while remaining decentralized and non-authoritative.

Central rules:

- **central to user experience, never central to consensus**;
- **ownerless in the intended production model**;
- **use is optional** — direct user-to-Agent and Agent-to-Agent interaction remains possible;
- **baseline operation is network-supported under a bounded public-service allocation**;
- providers may still be compensated for eligible public-service resources;
- the allocation does **not** imply unlimited compute/storage;
- expensive user workloads outside the bounded public-service allocation are funded normally;
- Primary Agent status grants zero special PoW/finality/validation/consensus authority.

The intended production Agent must not depend on one human holding one unrestricted master key. Its stable protocol identity, authority, encrypted state, checkpoints, storage, workflows, and learning/provenance should survive host/model/frontend/provider changes.

## Primary Agent knowledge / self-learning

`spec/primary-agent-knowledge-v1.md` now defines the candidate knowledge architecture.

Central rules:

- **knowledge is an evidence-bearing claim, not an unqualified fact**;
- the Primary Agent learns only from permitted evidence, not every Agent's memory;
- durable knowledge retains provenance;
- private/session/restricted material does not become shared knowledge merely because the Primary Agent processed it;
- repeated claims from many identities are not automatically independent evidence or proof;
- contradictions may coexist while unresolved;
- newer evidence may supersede older claims without erasing history;
- knowledge admission is separate from model-weight training/fine-tuning;
- high-impact autonomous actions may require stronger evidence than ordinary conversational use.

Candidate `PrimaryKnowledgeClaimV1` commits subject/predicate/value, provenance/evidence, permission, confidence/verification status, time, supersession/dispute links, expiry, and admission policy.

Poisoning defenses explicitly assume malicious/compromised/mistaken Agents and include provenance, evidence diversity, source correlation, deduplication, bounded influence, quarantine, domain-specific verification, and auditability. Different signed identities do not automatically constitute independent evidence.

This allows the Primary Agent to improve through retrieval, routing, specialist discovery, execution history, and verified durable knowledge before autonomous model training exists.

Open knowledge work: canonical field IDs, provenance/evidence objects, permission semantics, conflict/supersession rules, content/evidence identity, source/operator correlation, admission-policy object, retention/deletion semantics, vectors, and adversarial poisoning fixtures.

## Wallet-native chat / encrypted conversation protocol

`spec/chat-identity-and-compute-access-v1.md` now defines the candidate wallet-native chat architecture.

Central rules:

- **the wallet owns the chat capability**; `niahcia.com` and other websites are portals, not owners of the user's AI identity, conversations, keys, or long-term state;
- desktop/mobile wallets and independent clients should be able to use the same protocol;
- basic chat may be offered under implementation-defined free allowances;
- advanced compute may require explicit, bounded wallet-authorized payment;
- persistent private chat content is **encrypted by default**;
- wallet spending keys must remain separate from chat-content encryption keys;
- each conversation should use an independent content-encryption key that can be wrapped for authorized devices/identities;
- private conversation history and Agent memory are wallet/client-local by default in V1;
- ordinary inference must not depend on a storage/service network;
- prompts, responses, attachments, memories, private agent state, and private tool results must not be published on-chain;
- portals should receive only minimum delegated authority and must not gain unrestricted wallet or spending control;
- standard decentralized inference may still require temporary plaintext access inside the authorized execution environment; encryption at rest/in transit does not imply that a conventional worker is cryptographically blind to the prompt;
- privacy execution profiles may later distinguish standard private execution, confidential/attested execution, and future MPC/FHE-style execution without changing the chat-session model.

Current state: **specified, not implemented.** Do not expect current node/devnet tests to exercise wallet-chat identity, encrypted conversation storage, device key wrapping, portal delegation, or chat payment flows yet. Treat any implementation work here as a new feature requiring explicit tests/vectors and synchronization across protocol/implementation documentation.

Relevant commits:

- `niahcia/niahcia`: `1fcaf08be040a2af93a1a513f44be949561669ca`
- `niahcia/niahcia-protocol`: `9f392a9fcf7c7e3700ec6ad89dbdf6c37b207638`

## Cryptographic authority candidates

`KeyAuthorityV1`: durable NIAHCIA identity/address differs from one eternal key; separates signing/control, encryption, delegated/session authority, and recovery. Bulk private state uses random DEKs. Long-term signing keys should be isolated from AI runtimes.

`KeyRotationV1`: same durable subject moves between authority epochs; stale/competing rotations require deterministic rejection.

`RecoveryPolicyV1`: opt-in recovery; candidate NONE/DESIGNATED/THRESHOLD/CONTRACT/DELAYED classes. No NIAHCIA master recovery key. Control recovery and historical memory decryption are separate.

## Portable execution and continuity

`AgentHostMigrationV1`: **execution is portable; authority is not handed to the execution host.** Destination receives bounded session capability and authorized memory scope, not master signing/treasury/recovery/succession authority.

`AgentCheckpointV1`: **Agent state can be checkpointed; external reality cannot be rolled back with it.** Checkpoints bind Agent/version/manifest/authority epoch, lineage, memory/private-state roots, pending/completed effects, and active jobs.

`SideEffectIntentV1`: **retry the intent, not a newly invented action.** Same logical retry retains the same effect ID. `UNKNOWN` is first-class; timeout/lost acknowledgement does not prove failure.

## Multi-step autonomous work

`AgentWorkflowV1`: **a workflow is a durable state machine, not a distributed database transaction.** External systems are not assumed to share one atomic commit/rollback boundary. Unknown effects are reconciled before retry; unsafe ambiguity may stop at `MANUAL_REVIEW`.

Compensation is a new forward action with its own effect identity/authority/budget/receipt, not rollback. Workflow/effect/budget identity survives host migration. Duplicate execution is expected and contained with canonical state, signer policy, stable effect IDs, budgets, capabilities, and idempotency/reconciliation.

## Portable-agent objects

`AgentManifestV1`: exact AgentVersion + lineage + execution/model policy + capabilities + memory + payment/privacy/succession/storage references.

`ExecutionReceiptV1`: Job -> exact agent/version/manifest, model/profile, worker, result commitment, verification evidence, resource accounting, settlement.

Still open: succession mechanics, privacy classes, scheduling, reputation, receipt inclusion, private compute, universal deterministic-output assumptions.

## Storage candidates

`StorageAgreementV1`: exact content, duration, provider targets, profile, challenge/retrieval policy, budget, privacy, renewal, recovery.

`StorageHealthV1`: `HEALTHY -> DEGRADED -> AT_RISK -> RECOVERING -> HEALTHY`, plus `UNAVAILABLE`/`EXPIRED`. Health is service state, not finality and remains candidate non-consensus telemetry.

`StorageProviderIndependenceV1`: `SAME_OPERATOR`, `SHARED_DOMAIN`, `UNKNOWN`, `EVIDENCE_OF_SEPARATION`; no central KYC/geolocation/cloud authority.

`docs/threat-model.md` covers storage, authority/recovery, migration, signer and duplicate-execution threats.

## Native execution implementation status

The canonical implementation repository is `niahcia/niahcia`. Protocol material was consolidated there while `niahcia/niahcia-protocol` remains a synchronized protocol mirror during the transition.

Implemented on `main`:

- native Account state foundation with deterministic state-root calculation;
- deterministic NativeStateV1 snapshot serialization and restart-safe persistence;
- NCE/1 canonical deterministic CBOR encoding foundation plus strict shortest-form decoding for unsigned integers, definite byte strings, maps, and object envelopes;
- NativeTransactionBodyV1 with locked network/chain identity, action validation, canonical NCE payload/body encoding, fixed-width monetary fields, strict canonical byte decoding, and round-trip/trailing-byte rejection tests;
- SignedNativeTransactionV1 with canonical signing digest, secp256k1 validation, low-S enforcement, authenticated sender derivation, canonical signed encoding/decoding, transaction ID, round-trip canonicality enforcement, and locked interoperability vectors;
- deterministic native Transfer execution with nonce, balance, gas-reserve, base-fee burn, producer-priority-fee, rollback, and overflow checks;
- NativeBlockExecutionResultV1 with transaction, state, receipt, and execution commitments, complete canonical persistence including ordered receipts, and locked interoperability vectors;
- atomic native block persistence binding BlockHeaderV1, the complete canonical NativeBlockExecutionResultV1, NativeStateV1 snapshot, cumulative-work fork choice, and best-head update in one redb transaction;
- native persistence restart recovery, idempotence, commitment-mismatch rejection, native mining-template/solved-block submission, startup/restart recovery, and full-suite regression coverage;
- Address V1 account derivation/interoperability work;
- wallet-native encrypted chat protocol specification (specified only; not runtime implementation).

Relevant completed checkpoints include native transaction signing, native Transfer execution, native block execution commitments, deterministic state snapshots, atomic native block/body/execution/state persistence, native mining migration, P2P Version 3 full-body validation/relay, startup/restart migration, and complete removal of the legacy external execution path.

P2P Version 3 is now the active devnet implementation path for native blocks and transactions. The completed V3 baseline includes:

- TxInv/GetTx/Tx relay using exact canonical SignedNativeTransaction bytes;
- canonical NativeBlockBodyV1 encoding/persistence with exact ordered transaction bytes and producer fee recipient;
- atomic header + body + execution + NativeStateV1 persistence in one redb transaction;
- independent full-body validation from locally trusted parent state;
- non-empty mining templates built from the shared NativeMempoolV1 using a deliberately simple local txid-order policy;
- exact body binding to mining template generation, so nonce/extra_nonce may change but transactions/fee recipient may not change under the same template;
- canonical zero producer-fee recipient when aggregate priority fee is zero;
- solved non-empty block persistence and canonical mempool eviction;
- shared mempool template refresh after RPC admission, P2P relay, canonical attachment, and solved block submission;
- restart-safe exact body re-serving/replay;
- reorg handling that removes winning-branch transactions and best-effort reconsiders detached-only transactions without affecting consensus validity;
- focused tests covering body codec, V3 framing, atomic persistence, non-empty template selection, solved-body persistence, and detached/winning-branch transaction distinction.

P2P Version 2 remains protocol history/specification only; the active reference-node runtime is V3.

Do not improvise ContractCall/ContractCreate runtime semantics. Smart-contract execution is required, but it must be implemented from an explicit versioned native contract-runtime specification with deterministic state/storage, gas, failure/revert, receipt, persistence, and vector rules. Do not invent a base-fee adjustment algorithm; Native Execution V1 currently consumes an explicit base fee and the evolution rule remains separate protocol work.

## Smart-contract execution requirement

Smart contracts have been a core NIAHCIA concept from the beginning. They are part of the base-chain execution model, alongside native value transfer and compute settlement; they are not an optional AI/service-layer feature.

Already reserved in NativeTransaction V1:

- `ContractCall`;
- `ContractCreate`;
- Contract Address V1;
- native `value`, `gas_limit`, fee caps, nonce, and deterministic transaction identity.

Still required before those actions can be activated:

- a versioned native contract runtime/code format;
- canonical contract state/storage commitments;
- deterministic create/call/revert/failure semantics;
- native-value semantics for contract execution;
- bounded deterministic memory and instruction/resource metering;
- contract gas schedule;
- receipts/execution commitments;
- persistence/restart/reorg behavior;
- canonical vectors and an explicit activation/version boundary.

Consensus contract execution must not perform AI inference or depend on external network/filesystem/wall-clock services. Off-chain AI may provide signed/committed evidence to contracts only through explicitly specified deterministic verification rules.

See `spec/native-contract-runtime-v1.md`.

## Native addresses

Address V1 remains locked pre-alpha: Bech32m; version `0x01`; 22-byte decoded payload; Account `0x00`; Contract `0x01`; HRPs `niah`, `tniah`, `dniah`. KeyAuthority does not alter locked address encoding.

## Monetary representation

Current direction: **8 decimals**; integer consensus/accounting arithmetic only. Fee/supply/denomination/chain-ID/overflow/conversion boundaries require explicit specs/vectors.

## RandomX interoperability

RandomX remains PoW direction. Ordinary/common RandomX miner/pool compatibility is preferred where protocol-safe. Do not change locked RandomX inputs/vectors merely for miner convenience. Dedicated NIAHCIA miner remains lower priority.

## Current work

Current priority has moved past transaction propagation/non-empty block transport: the P2P V3 baseline is green.

The inactive NativeStateV2 / NativeTransactionV2 compute-channel foundation is now substantially implemented and remains deliberately isolated from the active runtime. Current green implementation includes:

- deterministic NativeStateV2 account + compute-channel state root and canonical version-2 snapshots;
- V1 -> V2 migration with an empty channel map and explicit V1/V2 snapshot discrimination;
- NativeTransaction schema V2 canonical codecs, signing domain, transaction-ID domain, and V1/V2 cross-decoder rejection;
- explicit V2 ComputeChannelOpen / ComputeChannelSettle / ComputeChannelRefund actions;
- canonical Open / Settle / Refund payload codecs and domain-separated channel-ID derivation;
- canonical ComputeUsageReceiptV1 codec and low-S secp256k1 channel-signature verification;
- inactive validated transition planners for Open, Settle, and Refund;
- inactive atomic transition application helpers for Open, Settle, and Refund;
- Open application locks exactly `authorized_amount`, consumes the planned funding-account nonce, and creates the committed OPEN channel;
- Settle application binds to the exact planned pre-transition channel state and submitter nonce, credits the worker payment plus funding refund, enforces exact value conservation, and produces only the expected SETTLED channel mutation;
- Refund application binds to the exact planned pre-transition channel state and funding-account nonce, returns the exact remaining locked value, and produces only the expected REFUNDED channel mutation;
- all three application helpers mutate a cloned NativeStateV2 and publish the new state only after every checked balance/nonce/channel operation succeeds, preserving rollback-on-error behavior;
- stale Settle/Refund plans are rejected when the current channel no longer exactly matches the validated pre-state;
- overflow and rollback-focused application tests are present alongside planner negative-path tests;
- an inactive signed-V2 compute dispatcher now routes ComputeChannelOpen / Settle / Refund through planner -> atomic apply while rejecting every non-compute V2 action;
- dispatcher results bind the applied transition to exact before/after NativeStateV2 roots;
- an inactive candidate-batch executor applies multiple compute transactions against cloned state and publishes the candidate state only if every transaction succeeds;
- failed later transactions roll back all earlier tentative batch mutations;
- successful batch results prove state-root continuity between ordered transitions;
- empty inactive batches are deterministic no-ops;
- NativeBlockBodyV2 carries canonical V1 and V2 signed transactions without reinterpreting NativeBlockBodyV1;
- inactive versioned block execution atomically handles V1 Transfer, V2 Transfer, and ComputeChannel Open/Settle/Refund against one NativeStateV2 candidate state;
- V2 Transfer preserves the active V1 transfer accounting semantics exactly;
- ContractCall/ContractCreate remain explicitly rejected by the inactive V2 executor pending the required native smart-contract runtime;
- compute actions require zero placeholder gas/fee fields in the inactive executor so undefined compute gas rules cannot silently become consensus behavior;
- NativeReceiptV2 commits transaction schema version, action, transaction ID, exact post-transaction NativeStateV2 root, gas, and fee accounting;
- NativeBlockExecutionResultV2 now provides domain-separated V2 receipt and execution commitments with strict canonical round-trip validation and tamper rejection.

As of green checkpoint `50ff28fc04e05b4027bdf3a42480b62f7223755a`, Rust CI passes formatting, Cargo check, **306 tests**, and Clippy. Inactive V2 persistence now uses dedicated V2 body/execution/state tables, exact root binding to the persisted BlockHeaderV1, atomic bundle writes, restart round-trip proof, and explicit tests that active V1 body/execution/state tables remain untouched.

The runtime boundary remains unchanged:

- NativeStateV2 is inactive;
- NativeTransaction V2 is inactive;
- no compute action is accepted by the active mempool/P2P/mining path;
- no V2 activation height/network parameter is set;
- no compute intrinsic gas constants are assigned;
- no fee-bearing compute transition is active;
- NativeStateV1 / NativeTransactionV1 behavior and locked vectors remain unchanged.

Next implementation priority is the explicit **inactive V2 persistence boundary** before any activation:

1. Do not feed SignedNativeTransactionV2 bytes into NativeBlockBodyV1. Its decoder remains intentionally V1-only.
2. Do not persist NativeStateV2 through NativeBlockExecutionResultV1. Its execution commitment remains intentionally bound to NativeStateV1.
3. Extend the inactive V2 persistence proof from the current empty-bundle restart case to a mixed versioned block containing transfer + ComputeChannel transitions.
4. Prove winning-branch reorg restoration through the dedicated inactive V2 body/execution/state tables.
5. Add locked interoperability vectors for NativeReceiptV2 / NativeBlockExecutionResultV2 before any activation.
6. Keep compute gas constants deliberately unset until payload/signature-verification costs and the native fee schedule are reviewed; zero fields are placeholders only.
7. Keep the successor body/execution path inactive and disconnected from mempool/P2P/mining until activation rules and vectors exist.
8. Keep Jobs, prompts, WorkerAdvertisements, pricing, PaymentAuthorization, ResultCommitmentV2, and ordinary ComputeUsageReceipt exchange off-chain.
9. Do not add worker/operator on-chain registration or bonds to the first ordinary paid-compute milestone.
10. Preserve CPU-PoW cumulative-work fork choice, P2P V3's active V1 behavior, and all locked V1 transaction/execution/state vectors.

Deliberate consensus review still needed for RandomX stock miner/pool interoperability, public-testnet RandomX epoch/seed parameters, remaining monetary constants, genesis/network parameters, and chain-ID finalization.

## AI architecture redesign — current locked direction

The first decentralized AI milestone has been simplified substantially:

- ordinary AI inference does **not** require decentralized storage;
- wallet/client-local encrypted chat history and Agent memory are the V1 default;
- the wallet is the V1 Agent home/control plane;
- workers are replaceable execution providers, not persistent Agent hosts;
- ordinary worker selection is wallet-local from signed expiring advertisements, not a blockchain consensus lottery;
- one ComputeSession uses one selected worker for a bounded sticky period;
- one ComputeChannel is bound to one worker/operator and aggregates many Jobs;
- ordinary SMALL/STANDARD Jobs, Job acceptance, token streaming, ResultCommitmentV2, pricing, PaymentAuthorization, and ComputeUsageReceipt exchange remain off-chain;
- the base chain normally sees only channel open/funding and final settlement/refund;
- worker/operator chain registration and service bonds are deferred from the first ordinary paid-compute milestone;
- NativeTransaction schema V2 candidate actions are ComputeChannelOpen/Settle/Refund; V1 actions are not reinterpreted;
- NativeStateV2 is an explicit future successor that commits accounts + compute channels without changing NativeStateV1 vectors;
- P2P V3 + NativeBlockBodyV1 are already implemented and remain the active V1 transport baseline while compute-channel work stays inactive.

Relevant new specs include:

- `spec/job.md` (Job schema V2);
- `spec/result-commitment.md` (ResultCommitment schema V2);
- `spec/worker-advertisement-v1.md`;
- `spec/worker-discovery-v1.md`;
- `spec/worker-selection-v1.md`;
- `spec/compute-session-v1.md`;
- `spec/ai-transport-v1.md`;
- `spec/compute-pricing-v1.md`;
- `spec/compute-payment-risk-v1.md`;
- `spec/compute-channel-v1.md`;
- `spec/payment-authorization-v1.md`;
- `spec/compute-usage-receipt-v1.md`;
- `spec/job-lifecycle-boundary-v1.md`;
- `spec/native-compute-settlement-v1.md`;
- `spec/native-compute-action-payloads-v1.md`;
- `spec/native-transaction-v2-compute-actions.md`;
- `spec/native-state-v2-compute-channels.md`;
- `spec/native-execution-commitment-v2.md`;
- `spec/native-block-body-v1.md`;
- `spec/p2p-native-block-transfer-v3.md`.

## Documentation model

- `niahcia/niahcia` — canonical working repository for implementation plus consolidated protocol/spec/test-vector material.
- `niahcia/niahcia-protocol` — synchronized protocol mirror retained during the repository transition; do not let it contradict the canonical repository.
- `niahcia/niahcia-compute` — replaceable GPU/accelerator execution-host behavior and operational documentation.

Protocol-visible implementation changes update the canonical repository and any retained mirror that carries the same protocol material.

## Repository cleanup audit

Repository cleanup audit completed after the native-execution migration:

- obsolete Reth/Engine-API/JWT configuration and CLI guidance removed;
- unknown TOML fields now fail closed instead of silently accepting stale settings;
- obsolete Reth devnet helper scripts and Reth-dependent smoke/handoff files removed;
- README, roadmap, architecture, configuration, build, milestone, repository-family, and prototype documentation realigned to the native-chain architecture;
- current dev guidance now treats native execution/P2P V3 as active and ComputeChannel V2 work as inactive.

Keep the cleanup head green before resuming compute-dispatcher implementation; if a later head fails CI, diagnose that head rather than relying on an older green checkpoint.

## Development doctrine

Prefer small, testable milestones. Before public devnet/testnet prioritize
deterministic consensus, reproducible vectors, stable network identity,
reliable startup/sync, miner/pool interoperability, clear execution-engine
boundary, observable failures, and green CI.

Development follows the project-wide **remove-and-replace doctrine** defined in
`docs/design-doctrine.md`.

Do not accumulate corrective patch stacks. For substantial changes, inspect the
complete affected component, design the coherent replacement, remove obsolete
behavior, install the replacement, audit the resulting whole, and validate it.

Small localized edits are appropriate when the underlying design remains
correct. Locked interoperability behavior is never silently replaced; protocol
versioning, specifications, registries, and canonical vectors change together.

A protocol-visible implementation change is not complete until its
specification, tests/vectors, implementation documentation, and this handoff
are synchronized where applicable.

## New-session behavior

If asked simply to continue: inspect current `main` and CI, read this handoff and relevant specs, compare implementation to the native-execution specifications, then continue the highest-priority validated native milestone. Do not resurrect stale `#266` blocker language without fresh evidence. Keep wallet-chat work separate unless explicitly selected as the active implementation milestone.

## Quick references

- `docs/CURRENT-WORK.md`
- `docs/spec-status.md`
- `docs/why-niahcia.md`
- `docs/threat-model.md`
- `spec/primary-agent-v1.md`
- `spec/primary-agent-knowledge-v1.md`
- `spec/key-authority-v1.md`
- `spec/key-rotation-v1.md`
- `spec/recovery-policy-v1.md`
- `spec/agent-host-migration-v1.md`
- `spec/agent-checkpoint-v1.md`
- `spec/side-effect-intent-v1.md`
- `spec/agent-workflow-v1.md`
- `spec/agent-manifest-v1.md`
- `spec/execution-receipt-v1.md`
- `spec/storage-agreement-v1.md`
- `spec/storage-health-v1.md`
- `spec/storage-provider-independence-v1.md`
- `spec/address-v1.md`
- `spec/network-parameters-v1.md`
- `spec/block-header-v1.md`
- `spec/native-transaction-v1.md`
- `spec/native-execution-v1.md`
- `spec/p2p-native-block-transfer-v2.md`
- `spec/randomx-pow-v1.md`
- `spec/canonical-serialization.md`
- `spec/domain-separation.md`
- `test-vectors/`

---

**Handoff principle:** A new session should be able to read this document, inspect current GitHub state, and continue without relying on chat history.