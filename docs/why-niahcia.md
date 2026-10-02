# Why NIAHCIA

Status: **DESCRIPTIVE / pre-alpha project rationale**

This document explains the architectural problem NIAHCIA is trying to solve and why its design differs from simply attaching AI workloads to a blockchain. It is not a claim that every described capability is already implemented or production-ready.

NIAHCIA is currently pre-alpha. Where this document says the network *is designed to*, *should*, or *will*, it describes protocol direction rather than demonstrated production capability.

## The central idea

NIAHCIA is being designed as a decentralized execution and economic protocol for autonomous intelligence.

The goal is not merely a cryptocurrency that rewards AI compute or a marketplace where users rent GPUs. The goal is a decentralized substrate in which chain security, execution, AI compute, storage, verification, agents, capabilities, memory, settlement, and a persistent public AI assistant can cooperate without collapsing into one trusted service.

The defining architectural choice is **separation of responsibilities**.

```text
                         NIAHCIA
                            |
             +--------------+--------------+
             |              |              |
          CPU PoW        AI COMPUTE      STORAGE /
          CONSENSUS       WORKERS         SERVICE
             |              |              |
        chain security    inference,      models,
        and ordering      execution       memory,
             |            and jobs        archival
             +--------------+--------------+
                            |
                       EVM / CONTRACTS
                            |
                          AGENTS
                            |
                     PRIMARY AGENT
                  default public interface
```

These roles can interact economically, but they do not inherit one another's authority.

## 1. AI compute is not consensus

CPU proof-of-work secures canonical history. GPU/accelerator compute executes useful AI workloads. A miner does not need AI hardware to secure the chain, and a GPU operator does not acquire consensus authority by owning substantial compute. Both systems can therefore evolve independently.

## 2. Useful service does not equal consensus authority

Storage/service nodes preserve models, manifests, memory, snapshots, archival material, relay data, availability evidence, and other content-addressed resources. They may earn compensation for measurable service, but they do not vote on canonical chain history. Proof-of-service is reward evidence, not a second consensus mechanism.

## 3. Agents are protocol objects, not accounts on one website

NIAHCIA treats an Agent as a durable, versioned network identity rather than merely a prompt, API session, or process on a company's server.

Protocol concepts include `Agent`, `AgentVersion`, `AgentManifestV1`, `Model`, `ExecutionProfile`, `ComputeWorker`, `Operator`, `ServiceNode`, `Job`, `VerificationPolicy`, `Capability`, `MemoryDescriptor`, `PaymentPlan`, `ResultCommitment`, and `ExecutionReceiptV1`.

This makes agent ownership, upgrades, permissions, tools, memory, compute, verification, payments, delegation, portability, and history explicit protocol relationships rather than hidden application behavior.

## 4. NIAHCIA should have one persistent public Primary Agent

NIAHCIA's intended user-facing center is a persistent **Primary Agent**: an ownerless, network-native conversational assistant that provides a simple default interface for ordinary users.

The Primary Agent may be central to the user experience but is never central to consensus.

An ordinary user should be able to interact with one familiar assistant while that Agent internally discovers specialist Agents, selects eligible compute, retrieves authorized memory, invokes tools/contracts, requests verification, and coordinates workflows.

The user should not need to understand the network's internal `Job`, `ComputeWorker`, `ExecutionProfile`, storage, verification, or scheduling objects merely to ask a question.

Use of the Primary Agent remains optional. Users and Agents may address other Agents directly; the Primary Agent must not become a gatekeeper.

### A protocol public good

The Primary Agent's protected baseline operation is intended to be supported by the decentralized network through a bounded public-service allocation.

This does not mean infrastructure providers receive nothing or that the Agent has infinite resources. The allocation should support baseline conversational inference, replicated checkpoints/state, storage, knowledge/provenance processing, routing, recovery, and availability while providers are compensated through protocol-defined economics.

Large user-requested workloads outside the public-service allocation can still be paid by the requesting user/job/treasury.

The Primary Agent therefore becomes a decentralized public service rather than a permanently hosted corporate chatbot.

### Ownerless and host-independent

The Primary Agent should not belong to the developers, official website, GPU provider, storage provider, or any single human key holder.

Its durable identity should survive execution-host changes, storage-provider failures, frontend disappearance, model upgrades, and key rotation. Production authority should ultimately use decentralized `KeyAuthorityV1` policy rather than one unrestricted human master key.

Primary Agent status grants zero special PoW, finality, transaction-validation, storage-consensus, or chain-governance authority.

If the Primary Agent fails completely, the blockchain continues.

## 5. Network learning should be provenance-aware

The Primary Agent is intended to improve from the wider Agent network, but self-learning must not mean indiscriminately copying every Agent's private memory or treating every generated statement as truth.

Candidate knowledge should cross permission, provenance, verification/evidence, confidence, and admission-policy boundaries before becoming durable Primary Agent knowledge.

This creates a useful long-term distinction: the Primary Agent need not itself be the world's best specialist model at every task. It can learn which Agents, models, execution profiles, tools, and verification mechanisms have produced useful evidence for particular classes of work.

A single user conversation can therefore orchestrate many decentralized specialists while presenting one coherent interface.

Private user/Agent memory is not automatically learning material merely because it traverses NIAHCIA.

## 6. Agent sovereignty means portable authority and state

A decentralized agent should not become property of whichever server currently runs inference. `AgentManifestV1` binds an exact AgentVersion to model/execution policy, capabilities, memory descriptors, payment/privacy/succession policy, storage manifests, and lineage.

The physical GPU is therefore a replaceable execution resource rather than the Agent's identity. Portable memory can remain encrypted and access-controlled.

## 7. Durable identity requires separable cryptographic authority

NIAHCIA is developing `KeyAuthorityV1`, `KeyRotationV1`, and `RecoveryPolicyV1` so durable identity does not mean one eternal private key.

Signing/control authority, encryption authority, and bounded execution/session authority are distinct. Bulk memory should use data-encryption keys rather than direct wallet-key encryption. Long-term signing keys should not normally be exposed to AI runtimes.

This architecture is particularly important for the ownerless Primary Agent: no single execution host or developer should need possession of a permanent unrestricted master key.

## 8. Agents should survive host failure

`AgentHostMigrationV1` treats execution as portable while authority remains with the Agent. A destination host receives only bounded session authority and authorized memory scope.

`AgentCheckpointV1` defines durable resume state. A failed host should not become a portability veto when committed state survives elsewhere.

NIAHCIA assumes duplicate execution can occur during failures/migration and therefore relies on explicit session/job/effect identity, capability limits, state versions, and replay protection rather than assuming one Agent equals one running process.

## 9. External reality requires replay-safe workflows

Agent state can be checkpointed; external reality cannot be rolled back with it.

`SideEffectIntentV1` gives external actions stable identities so a restarted Agent retries the same intent rather than inventing a duplicate payment/tool call merely because an acknowledgement was lost.

`AgentWorkflowV1` treats multi-step autonomous work as a durable state machine rather than pretending arbitrary external systems share one atomic transaction. Compensation is a new auditable forward action, not history rewrite. Ambiguous economic/security-sensitive work can stop for reconciliation or manual review rather than guessing.

## 10. Agent lineage and succession should be verifiable

AgentVersion forms immutable execution history and AgentManifest commits lineage. Succession is intended to become protocol-defined rather than whichever server possesses a copy seizing control. Concrete succession state machines remain open.

## 11. Capability-native security should constrain agents and hosts

An Agent treasury should not require handing every inference host unrestricted wallet authority. Capabilities can constrain tools/services, destinations, budgets, expiration, delegation, model/profile access, memory/storage access, rate limits, and revocation.

Compromising one worker should not automatically compromise the Agent's identity or complete economic authority.

## 12. Compute should be replaceable

NIAHCIA does not enshrine today's AI stack into consensus. vLLM/Qwen/NVIDIA may be useful initial profiles, but model/runtime/hardware behavior belongs behind versioned objects. New runtimes, accelerators, and models should not require redesigning RandomX consensus.

## 13. Verification is a protocol problem

A worker signature proves who claimed work, not correctness. `VerificationPolicy` defines required evidence/agreement. The earlier fixed Prototype-0 2-of-3 assumption is superseded; redundant execution remains one possible policy among auditing, challenges, trusted-execution evidence, specialized proofs, and future techniques.

## 14. Execution receipts make machine work auditable

`ExecutionReceiptV1` can bind a Job to exact AgentVersion/manifest, model, execution profile, worker, result commitment, verification evidence, resource accounting, and settlement without necessarily exposing private prompts/results/memory.

## 15. Privacy should be a job/policy property

Confidential execution is not one global switch. Different work can eventually declare public/encrypted/private/attested requirements. Ordinary hosts may see plaintext deliberately supplied to them until stronger private-compute mechanisms are explicitly implemented.

## 16. Storage should be reconstructable, not hosted

Models, manifests, Agent memory, checkpoints, and execution dependencies should not disappear with one project's server. NIAHCIA favors content addressing, canonical manifests, verifiable chunks, service discovery, durability agreements, and repairable availability.

Storage possession does not grant decryption or governance authority.

## 17. The blockchain should survive AI failure

If every AI worker disappears, PoW consensus should continue. If storage/service nodes disappear, consensus should continue even though higher-level services degrade. If the Primary Agent disappears, the blockchain continues. If the official website disappears, the protocol and Agent identities remain independently addressable.

No subsystem should secretly become the network's control plane.

## 18. Native economics connect independent resource markets

CPU miners provide chain security. Compute workers provide AI execution. Storage/service nodes provide measurable service. Verifiers provide policy-defined verification. Agents/users purchase non-public-service resources.

The Primary Agent adds one special economic category: a bounded protocol public-service allocation for its baseline decentralized operation. This allocation does not grant consensus privilege and must not become an unlimited public-compute resource.

Exact funding/reward mechanics remain open and require explicit economic/security design.

## 19. Smart contracts are part of the substrate

NIAHCIA integrates EVM execution through Reth rather than inventing an isolated contract VM. The consensus daemon remains responsible for NIAHCIA ordering/PoW; Reth provides EVM execution/state validation across an authenticated boundary.

## 20. Common infrastructure should remain common where practical

NIAHCIA should reuse mature infrastructure where safe: standard RandomX, common miner/pool interoperability where possible, EVM via Reth, and versioned AI runtimes rather than unnecessary branded forks.

## 21. Decentralized scheduling should not become a hidden coordinator

Worker discovery, eligibility, assignment, execution, verification, timeout, reassignment, and settlement are protocol concerns. They need not all be written directly into blocks, but no permanent privileged scheduler should become indispensable.

## 22. Protocol-first interoperability

NIAHCIA maintains a separate protocol repository so the Rust implementation is not the sole definition of the network. Interoperability-critical objects require deterministic serialization, domain separation, versioning, and vectors so independent implementations can eventually interoperate.

## What NIAHCIA is not

NIAHCIA is not intended to be:

- a blockchain where every miner must run AI;
- a GPU-rental marketplace with a token attached;
- a storage network controlling finality;
- a single hosted AI API paid with cryptocurrency;
- a fixed 2-of-3 inference voting system;
- permanently tied to one LLM/runtime/accelerator/website;
- a system where an inference host automatically owns the Agent;
- a centralized chatbot whose failure stops the network;
- a Primary Agent with consensus authority;
- an unlimited public compute service without policy/accounting boundaries.

## What success would look like

An ordinary user opens any compatible NIAHCIA interface and talks to the same persistent Primary Agent identity.

The Primary Agent can resolve its distributed authorized state, use its bounded network-supported baseline resources, discover specialists, delegate work, select eligible compute, obtain verification/evidence, maintain provenance-aware durable knowledge, and return a coherent answer.

For expensive work, it can orchestrate a funded Job rather than exceeding its public-service allocation.

Meanwhile CPU miners secure the chain independently; storage providers preserve/repair resources independently; specialist Agents remain directly addressable; compute hosts remain replaceable; and the Primary Agent can migrate between hosts without changing identity.

That composition—not any single AI model—is the core NIAHCIA idea.

## Current reality

NIAHCIA is pre-alpha. `PrimaryAgentV1` is a candidate architecture, not an implemented production service. Its permanent identity, ownerless authority/governance, network-supported resource allocation, provider compensation, abuse resistance, knowledge/provenance system, and upgrade policy remain design/implementation work.

NIAHCIA's present differentiation is therefore an architectural direction being implemented and tested, not a claim of production superiority over existing decentralized AI networks.

## Design test

When considering a future feature, ask:

> Does this make NIAHCIA more independently operable, verifiable, replaceable, portable, least-privileged, and permissionless—or does it quietly create a new central dependency?

For the Primary Agent add a second test:

> Can this remain a persistent public assistant without becoming the network's owner, gatekeeper, consensus authority, or unlimited resource sink?

The objective is not decentralization as branding. The objective is a network whose important roles can actually be replaced by independent participants.