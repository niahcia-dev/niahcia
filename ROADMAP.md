# NIAHCIA Roadmap

## Milestone 0 — Project foundation

- protocol repository and canonical object specs
- reference implementation repository
- canonical serialization, hashing, IDs, signatures, and domain separation
- repository family defined for miner, compute worker, explorer, official web app, GitHub Pages site, and shared GitHub profile/community files
- contributor/security/release policies
- initial testnet architecture

See `docs/repository-family.md`.

## Milestone 1 — Chain

- CPU PoW
- RandomX
- Reth/EVM execution
- Solidity contracts
- multi-node sync
- reorganizations
- wallets and RPC compatibility
- CPU miner reference implementation
- solo mining first, pool protocol immediately after basic mining stability

## Milestone 2 — Decentralized compute

- compute-worker identity
- capability advertisements
- vLLM execution
- P2P job transport
- streaming responses
- AI escrow and settlement
- canonical execution profiles
- dedicated `niahcia-compute` client

## Milestone 3 — Verification

- redundant verification for initial integration
- commit/reveal
- worker bonds
- operator identities
- random-audit hooks
- optimistic challenge infrastructure

## Milestone 4 — Service network

- service-node registration
- content-addressed model storage
- replication
- parallel retrieval
- availability challenges
- service settlement
- memory-storage primitives

## Milestone 5 — Agents

- AgentRegistry
- immutable agent versions
- persistent memory
- capability-based tools
- budgets
- agent-to-agent invocation
- smart-contract interaction
- autonomous triggers

## Milestone 6 — Public network experience

### Explorer

- blocks / transactions / contracts
- mining and difficulty metrics
- AI jobs and verification
- model registry
- agents and versions
- compute workers/operators
- service nodes/storage

### Main website

- Ask an Agent
- browse/discover agents
- wallet connection
- funded AI sessions
- job/account history
- network dashboard
- developer portal

### GitHub Pages

- static NIAHCIA project/development site
- architecture overview
- repository directory
- docs/downloads/status links

## Ultimate resilience milestone

Turn off every server controlled by the original developers and demonstrate that community-operated nodes can still transact, run contracts, discover agents, execute AI, retrieve required data, verify outcomes, and settle payments.
