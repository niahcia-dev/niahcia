# NIAHCIA Goals

## Primary goal

Build a fully decentralized blockchain network for AI agents where no single company, server, model host, scheduler, storage provider, or website is required for the protocol to function.

NIAHCIA combines:

- CPU-secured blockchain consensus
- EVM smart contracts
- decentralized AI compute
- decentralized storage and service nodes
- persistent, portable AI agents
- independent economic incentives for each network role

## Core goals

1. **Permissionless participation** — anyone may run a full node, CPU miner, compute worker, service node, agent client, or compatible frontend.
2. **CPU-secured consensus** — base-chain security remains independent from AI compute.
3. **Optional AI participation** — miners do not need GPUs; compute workers are a separate role.
4. **Decentralized compute** — AI workloads execute on independently operated workers without a mandatory central scheduler.
5. **Decentralized agents** — agent identity, versions, permissions, economics, and state commitments belong to the network, not a server.
6. **EVM compatibility** — Solidity contracts, wallets, tokens, DAOs, escrow, and existing tooling should work naturally.
7. **Decentralized storage** — models, datasets, agent state, and artifacts are content-addressed and replicated by service nodes.
8. **Independent economics** — CPU miners, compute workers, service nodes, and agents are paid for distinct services.
9. **Fast AI UX** — user-visible responses stream over the AI network without waiting for block settlement.
10. **Verifiable execution** — verification evolves from deterministic redundancy toward optimistic audits, disputes, and cryptographic proofs where practical.
11. **Persistent identity and reputation** — agents, workers, operators, models, and service nodes have durable identities and auditable histories.
12. **Agent autonomy with explicit limits** — tools, budgets, smart contracts, agent-to-agent calls, memory, and scheduled work are governed by capability and spending policies.
13. **Open model and hardware ecosystem** — the protocol must not depend on one LLM, runtime, GPU vendor, or workload type.
14. **Official site without protocol dependence** — one primary website may provide the default experience, but it is only a client of the network.
15. **Future-proof protocol design** — Protocol v1 defines the broad architecture; Prototype 0 proves a minimal end-to-end slice without narrowing the protocol ceiling.

## Ultimate resilience test

If every server operated by the original NIAHCIA developers is turned off, community-operated miners, full nodes, compute workers, service nodes, and alternative clients must still be able to:

- send transactions
- deploy and call contracts
- discover and invoke agents
- execute AI jobs
- retrieve model and agent data
- verify results
- settle worker and service payments

If that is not true, the system is not yet decentralized enough.
