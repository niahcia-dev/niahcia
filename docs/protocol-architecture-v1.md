# NIAHCIA Protocol Architecture V1

Status: **pre-alpha architecture map; descriptive unless an incorporated specification says otherwise**

This document is the top-level map of the NIAHCIA protocol. It records the separation of responsibilities already established across the project and points implementers toward the specifications that lock individual wire formats, consensus objects, and economic primitives.

It is intentionally architecture-first. A top-level architecture document must not silently invent consensus rules while implementation and interoperability work are still in progress.

## 1. Core objective

NIAHCIA is a decentralized protocol in which independent resource classes perform distinct jobs:

- CPU proof-of-work establishes chain consensus and canonical ordering.
- Reth provides EVM execution and execution-state validation.
- GPU compute workers execute AI inference jobs and earn compute compensation independently of block mining.
- service/storage nodes distribute and preserve canonical model and protocol data and prove useful service.
- verification mechanisms determine whether off-chain compute results satisfy the job's declared verification policy.
- agents are protocol identities that can discover services, request work, hold permissions, use memory descriptors, and participate in protocol payments as those capabilities are enabled.

No compute-worker, service-node, storage-node, model provider, agent, website, scheduler, or bootstrap host is a second chain-consensus authority.

## 2. Architectural rule: separate authority from service

NIAHCIA deliberately separates **chain authority** from **useful network services**.

CPU PoW answers the consensus question: which valid chain has the greatest accepted cumulative work?

Execution answers: given an ordered execution payload, what state transition does the EVM produce?

AI compute answers: which worker can execute a declared model/runtime/profile job and produce a verifiable result commitment?

Storage/service answers: where can canonical content be retrieved, and did a registered node provide the service it claimed?

These roles may coexist on one physical machine, but their protocol authority does not merge. Running more GPU compute or storage capacity does not grant extra PoW consensus authority. Mining more blocks does not make an AI result correct merely because a miner produced it.

## 3. Chain consensus layer

The NIAHCIA consensus daemon owns chain consensus.

The current pre-alpha direction is:

- proof-of-work: standard RandomX;
- target block interval: approximately 30 seconds;
- fork choice: highest valid cumulative work;
- independent validation by every full node;
- chain P2P for headers, blocks, synchronization, and consensus data;
- authenticated local integration with the execution engine.

The consensus layer must independently validate consensus-critical inputs. External miners and pools are work producers, not trusted validators.

### Mining interoperability

Stock RandomX miner compatibility is a protocol objective. In particular, pool-facing work should be designed so current common RandomX mining software such as stock XMRig can participate without a NIAHCIA-specific miner fork.

The exact XMRig-compatible PoW preimage/blob layout is **not locked by this architecture document**. It requires a separately reviewed interoperability specification and canonical node/miner/pool vectors before production use.

## 4. Execution layer

Reth owns EVM execution and execution state. The NIAHCIA consensus daemon owns ordering and consensus.

The boundary is intentionally explicit:

```text
NIAHCIA consensus
       |
       | authenticated Engine API
       v
Reth execution engine
       |
       +-- EVM transaction execution
       +-- execution payload validation
       +-- execution state
       +-- Ethereum-compatible execution RPC
```

A valid NIAHCIA block must not become valid merely because an execution client accepted an unrelated payload. Consensus commits to the execution result/payload relationship defined by the block protocol, and nodes independently validate that relationship.

The execution engine's internal address, quantity, or chain conventions do not silently redefine NIAHCIA-native signed objects.

## 5. Native identity and addresses

The canonical human-facing address specification is `docs/address-v1.md`.

Address V1 currently locks:

- Bech32m encoding;
- `niah1...` mainnet accounts/contracts;
- `tniah1...` testnet accounts/contracts;
- `dniah1...` devnet accounts/contracts;
- explicit address version and kind bytes;
- a 20-byte execution-compatible payload inside the native address container.

Execution-layer hexadecimal addresses are an interoperability representation, not the preferred native user-facing identity.

Agent, worker, operator, service-node, model, capability, and other protocol-object identifiers are distinct typed identities where their own specifications require them. They must not be conflated merely because an operator also controls a payment account.

## 6. Native value, denominations, and chain identity

The canonical monetary representation specification is `docs/monetary-and-fee-primitives-v1.md`.

V1 currently locks:

- native symbol: `NIAH`;
- exactly 8 decimal places;
- `1 NIAH = 100,000,000 aniah`;
- integer-only native consensus arithmetic;
- `u128` native monetary quantities where specified;
- checked arithmetic with overflow/underflow rejection;
- explicit mainnet/testnet/devnet network identifiers;
- explicit native NIAHCIA chain IDs;
- exact fee reserve, charge, and refund arithmetic.

Representation is separate from monetary policy. The denomination scale does not itself choose maximum supply, initial subsidy, emission curve, genesis allocation, tail emission, or reward split.

## 7. AI compute layer

AI execution is economically and operationally separate from PoW mining.

A compute worker is expected to:

1. possess a network/operator identity;
2. advertise supported execution capabilities/profiles;
3. become eligible for jobs without a permanent central scheduler;
4. retrieve canonical job inputs and model/runtime declarations;
5. verify hashes and execution-profile requirements before execution;
6. execute the workload;
7. stream permitted output to the requester;
8. construct the protocol-defined result commitment;
9. submit the signed commitment/evidence required by the verification policy;
10. receive settlement only according to the job and verification rules.

The first runtime direction uses vLLM for inference, but runtime choice is described by versioned execution profiles rather than being embedded forever into consensus.

GPU ownership does not imply chain authority, and miners are not required to operate GPUs.

## 8. Job lifecycle

The architectural lifecycle is:

```text
request
  -> canonicalize
  -> fund/escrow
  -> make eligible
  -> assign/select worker(s)
  -> execute
  -> commit result/evidence
  -> verify or challenge
  -> finalize
  -> settle
```

Detailed state names, timeouts, assignment rules, retry behavior, verification thresholds, and payment transitions belong in the versioned Job and VerificationPolicy specifications. This document does not substitute for those state-machine definitions.

A scheduler must not become a permanent trusted coordinator. Selection inputs and eligibility rules must be reproducible from protocol-visible state or otherwise verifiable under the applicable job protocol.

## 9. Verification layer

NIAHCIA treats **execution** and **verification** as separate responsibilities.

A result commitment is not automatically correct because a registered worker signed it. VerificationPolicy identifies the evidence and agreement rules required for that class of job.

The protocol should support multiple verification strategies over time rather than embedding one universal `2-of-3` assumption into every AI workload. Verification can evolve by versioned policy while preserving deterministic settlement semantics for jobs created under an older policy.

Disagreement must resolve through explicit protocol states: acceptance, additional verification, challenge, timeout, reassignment, failure, or another versioned outcome. Silent coordinator discretion is not a protocol rule.

## 10. Service and storage layer

Service/storage nodes provide useful network services but are not a second consensus committee.

Their initial responsibilities include:

- canonical model-manifest ingestion;
- content-addressed model/data storage;
- chunk discovery and transfer;
- integrity verification;
- availability/service challenges;
- retrieval/service accounting;
- signed proof-of-service evidence where required.

Proof-of-service demonstrates useful service under its versioned challenge rules. It does **not** choose the canonical chain.

A model or other canonical object should be reconstructable from independently verifiable manifests/chunks rather than depending on one operator's server remaining online.

## 11. P2P separation

NIAHCIA currently distinguishes three logical protocol families:

### Chain P2P

Headers, blocks, synchronization, chain state needed for consensus, and transaction propagation where applicable.

### AI P2P

Worker discovery/advertisements, job-related messages, assignments, input retrieval, output streaming, result commitments, and verification-related traffic.

### Storage P2P

Manifest/chunk discovery, content transfer, availability challenges, and storage/service evidence.

Implementations should reuse secure identity and transport primitives where practical without collapsing the semantics or authority of these protocol families.

## 12. Agent layer

An Agent is a versioned protocol identity, not merely a prompt string or a process currently running on one server.

The architecture reserves explicit concepts for:

- `Agent` — durable identity/ownership root;
- `AgentVersion` — immutable/versioned behavior declaration;
- `Model` — model identity and canonical metadata;
- `ExecutionProfile` — reproducible runtime requirements;
- `Capability` — permission to invoke a class of tool/service/action;
- `MemoryDescriptor` — description/reference rules for agent memory without assuming all memory lives on-chain;
- `PaymentPlan` — declared payment/charging behavior where enabled;
- `Job` — a requested unit of work;
- `ResultCommitment` — commitment/evidence for completed work.

Creator/operator authority, upgrades, delegation, revocation, tool permissions, agent-to-agent payments, discovery, reputation, and memory semantics require explicit versioned rules. They must not depend on one website being the gatekeeper.

## 13. Economic separation

NIAHCIA has multiple economically useful roles, but they are not interchangeable:

- miners are compensated for chain security/block production under the monetary policy;
- compute workers are compensated for accepted AI compute;
- service/storage nodes may be compensated for accepted useful service;
- verifiers may be compensated where a VerificationPolicy provides for it;
- users/agents pay according to the transaction/job/service rules they invoke.

A reward in one subsystem must not implicitly grant authority in another subsystem.

All native settlement ultimately uses the canonical NIAH/aniah representation unless a future explicitly versioned mechanism says otherwise.

## 14. Security boundaries

The architecture assumes participants can fail, disconnect, lie, withhold service, submit stale data, or attempt to game rewards.

Therefore:

- full nodes independently validate PoW and consensus rules;
- Reth independently validates execution payloads/state transitions within the execution boundary;
- AI results require the evidence specified by their verification policy;
- storage/service rewards require protocol-defined proof of useful service;
- signatures authenticate claims but do not make claims true;
- timeouts/reassignment prevent one selected worker from permanently blocking a job;
- no bootstrap node, website, model host, scheduler, miner, worker, or service node should be indispensable after peers have discovered the network and canonical state.

Botnet resistance is not solved by branding RandomX as CPU-friendly. Mining economics, pool concentration, network difficulty, work distribution, telemetry, and attack behavior must be measured during devnet/testnet and addressed explicitly where protocol mechanisms can improve resistance without creating central control.

## 15. Versioning and interoperability

Consensus-critical encodings and state transitions require canonical byte-for-byte or state-transition vectors before they are considered locked for production interoperability.

Once a V1 object has been declared interoperable, implementations must not silently reinterpret it. Incompatible changes require a new object/profile version or an explicitly activated protocol transition.

Specifications should distinguish:

- **normative** — MUST/SHOULD/MAY interoperability rules;
- **candidate** — proposed rules awaiting vectors/review;
- **descriptive** — architecture or implementation explanation;
- **implementation detail** — replaceable code choice that does not define protocol meaning.

This distinction prevents the current Rust implementation from accidentally becoming the only specification of NIAHCIA.

## 16. Current pre-alpha acceptance objective

The smallest complete network should demonstrate:

```text
Chain:
  independent full/mining nodes
        |
        +-- RandomX consensus
        +-- cumulative-work fork choice
        +-- Reth-backed EVM execution

Compute:
  independent GPU workers
        |
        +-- decentralized eligibility/assignment
        +-- declared execution profiles
        +-- result commitments
        +-- verification

Storage/service:
  independent service nodes
        |
        +-- canonical manifests/chunks
        +-- retrieval
        +-- proof of useful service

End-to-end:
  submit job -> execute -> verify -> finalize -> settle
```

The network must continue making appropriate progress when individual miners, compute workers, service nodes, or the original developer/bootstrap host disappear.

## 17. Specification map

Current source-of-truth documents include:

- `docs/address-v1.md` — native Address V1 container and network HRPs.
- `docs/monetary-and-fee-primitives-v1.md` — NIAH/aniah representation, network/chain IDs, fee arithmetic.
- `docs/implementation.md` — Prototype 0 workstreams and acceptance topology.
- `docs/mining-rpc.md` — native mining RPC behavior.
- `docs/consensus-development.md` — consensus-development constraints and process.
- `docs/devnet-smoke-test.md` — current devnet smoke-test procedures.
- emission/fee policy documents under `docs/` — candidate and locked economic details according to each document's own status header.

Future top-level protocol documents should link back here rather than duplicate architectural authority boundaries.

## 18. Non-goals of this architecture map

This document does not itself lock:

- the final XMRig-compatible PoW preimage layout;
- production genesis parameters;
- final monetary issuance/reward split unless separately locked;
- a universal AI verification algorithm;
- production slashing parameters;
- one mandatory model/runtime forever;
- distributed training semantics;
- private inference or zkML;
- the complete autonomous-agent permission/memory/payment state machines.

Those require their own versioned specifications and interoperability tests.
