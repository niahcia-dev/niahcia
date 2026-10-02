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
- Storage/service nodes provide measurable service but never fork-choice/finality authority.
- Agents/models/jobs/capabilities/memory/verification/payment are explicit protocol objects.
- Reth supplies EVM execution while NIAHCIA retains chain identity/consensus boundary.
- Key differentiation objectives: **portable agent sovereignty**, **economically maintained self-healing persistence**, and an **ownerless decentralized Primary Agent as a network public good**.

## Non-negotiable boundaries

CPU PoW alone determines canonical chain by cumulative valid work. Service/storage nodes do not vote on canonical chain. Execution hosts are replaceable and do not automatically own/control Agents. Fixed Prototype-0 universal 2-of-3 verification is superseded by policy-driven verification. Storage may preserve ciphertext without decryption/governance/treasury/succession authority. Distinct provider keys do not prove independent durability. NCE/1 deterministic CBOR is canonical serialization foundation.

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

`StorageHealthV1`: `HEALTHY -> DEGRADED -> AT_RISK -> RECOVERING -> HEALTHY`, plus `UNAVAILABLE`/`EXPIRED`. Health is service state, not finality. `niahcia/niahcia#27` remains non-consensus telemetry after CI is green.

`StorageProviderIndependenceV1`: `SAME_OPERATOR`, `SHARED_DOMAIN`, `UNKNOWN`, `EVIDENCE_OF_SEPARATION`; no central KYC/geolocation/cloud authority.

`docs/threat-model.md` covers storage, authority/recovery, migration, signer and duplicate-execution threats.

## Native addresses

Address V1 remains locked pre-alpha: Bech32m; version `0x01`; 22-byte decoded payload; Account `0x00`; Contract `0x01`; HRPs `niah`, `tniah`, `dniah`. KeyAuthority does not alter locked address encoding.

## Monetary representation

Current direction: **8 decimals**; integer consensus/accounting arithmetic only. Fee/supply/denomination/chain-ID/overflow/conversion boundaries require explicit specs/vectors.

## RandomX interoperability

RandomX remains PoW direction. Ordinary/common RandomX miner/pool compatibility is preferred where protocol-safe. Do not change locked RandomX inputs/vectors merely for miner convenience. Dedicated NIAHCIA miner remains lower priority.

## Current blocker

At this handoff, GitHub work associated with **#266** was red/failing. Avoid risky consensus changes until rechecked/resolved. GitHub state is authoritative.

## Safe work while CI is blocked

1. Address V1 interoperability vectors.
2. Denomination/value/fee vectors.
3. Remove stale Prototype-0 fixed 2-of-3 language.
4. Cross-check NCE/1 IDs/domains/signature preimages/storage vectors.
5. Review candidate protocol objects without activating them in implementation.
6. Allocate canonical IDs/fields/domains and vectors before implementation activation.
7. Continue adversarial review of authority, recovery, migration, checkpoint rollback, duplicate execution, side-effect replay, workflow compensation, budget abuse, Primary Agent public-service abuse, and knowledge poisoning.
8. Define deterministic effect receipts/reconciliation evidence.
9. Define `PrimaryAgentServicePolicy` candidate: bounded network-supported baseline resources, provider accounting/compensation boundary, anti-abuse/fairness, without prematurely locking emission percentages.
10. Define canonical Primary Agent knowledge provenance/evidence objects and poisoning fixtures.
11. Keep implementation/protocol/compute documentation synchronized.

Deliberate consensus review still needed for RandomX stock miner/pool interoperability, public-testnet RandomX epoch/seed parameters, remaining monetary constants, genesis/network parameters, and chain-ID finalization.

## Documentation model

- `niahcia/niahcia` — implementation behavior/tests/docs.
- `niahcia/niahcia-protocol` — implementation-independent architecture, formats, interoperability rules, security boundaries, research, vectors.
- `niahcia/niahcia-compute` — replaceable GPU/accelerator execution-host behavior and operational documentation.

Protocol-visible implementation changes update the applicable sources.

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

If asked simply to continue: check GitHub status/#266, read spec status, compare implementation to relevant specs, choose highest-priority safe unresolved work, test if appropriate, update applicable docs, and refresh this handoff. If CI remains blocked, continue safe specification/vector/threat-model work rather than unrelated consensus changes.

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
- `spec/randomx-pow-v1.md`
- `spec/canonical-serialization.md`
- `spec/domain-separation.md`
- `test-vectors/`

---

**Handoff principle:** A new session should be able to read this document, inspect current GitHub state, and continue without relying on chat history.