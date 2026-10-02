# Design Doctrine

These rules are architectural and engineering constraints, not marketing slogans.

## 1. The blockchain is sovereign

AI workers, service nodes, websites, or model hosts may fail independently without stopping consensus or transaction processing.

## 2. Consensus and AI remain separate

CPU miners secure the canonical chain. Compute workers execute AI workloads. AI availability must never be required to validate or extend the base chain.

## 3. Agents belong to the network, not servers

An agent is a stable identity plus immutable versioned definitions and state commitments. Any compatible worker may execute it.

## 4. No mandatory central service

No scheduler, API gateway, database, model repository, frontend, or company-controlled endpoint may become a protocol dependency.

## 5. Pay for measurable service

- CPU miners earn for chain security and block production.
- Compute workers earn for AI execution.
- Service nodes earn for storage, retrieval, availability, routing, and future network services.
- Agents may earn for application-level services.

Collateral alone is not a service.

## 6. Smart contracts are first-class

NIAHCIA targets an EVM-compatible environment so contracts can coordinate escrow, jobs, identities, agents, applications, and agent-to-agent commerce.

## 7. Realtime traffic stays off-chain

Token streaming, job payload transfer, model chunks, and large artifacts use P2P data protocols. The chain provides identity, commitments, escrow, disputes, and settlement.

## 8. Consensus must remain boring

AI popularity, GPU count, storage count, or agent reputation must not influence PoW difficulty or fork choice.

## 9. Everything important is versioned and portable

Models, execution profiles, jobs, agent manifests, storage manifests, verification policies, and capabilities use deterministic versioned representations.

## 10. Protocol v1 describes the future-facing system

Prototype implementations may support only a subset, but unsupported capability classes should not require later protocol redesign.

## 11. Remove and replace — never patch-stack

NIAHCIA development does not accumulate corrective patches on top of obsolete
designs or implementations.

When a component requires substantial correction or redesign:

1. inspect the complete affected component and its dependencies;
2. define the intended replacement as a coherent whole;
3. remove the obsolete implementation or section;
4. install the replacement cleanly;
5. audit the resulting component for stale assumptions, duplicated behavior,
   dead paths, contradictory documentation, and obsolete compatibility logic;
6. validate the complete resulting state with tests, vectors, builds, or
   structural checks appropriate to the component;
7. synchronize protocol specifications, implementation documentation, tests,
   vectors, registries, and session handoff state where applicable.

Do not solve architectural mistakes by repeatedly appending exceptions,
special cases, corrective branches, or text mutations.

Small localized edits remain appropriate when the underlying design is still
correct. The prohibition is against patch stacking as a development strategy,
not against ordinary maintenance.

Consensus-critical or interoperability-locked behavior is never silently
replaced. Such changes require explicit versioning, specification review, and
updated canonical vectors.

This doctrine applies across the NIAHCIA repository family: protocol
specifications, Rust implementation, compute software, websites, scripts,
configuration, tests, documentation, and generated artifacts.

## What NIAHCIA is not

NIAHCIA is not:

- a centralized AI API with a token
- a blockchain where every validator reruns every LLM
- a GPU mining scheme labeled as useful AI
- a single-model or single-runtime network
- a CUDA-only protocol
- a masternode system that rewards collateral without measurable service
- a website whose private database is authoritative network state
