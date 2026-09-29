# NIAHCIA Repository Family

NIAHCIA is intentionally split into independently maintainable components. The protocol defines the boundaries; repositories implement those boundaries.

## Core repositories

### `niahcia`

Reference blockchain/node implementation.

Responsibilities:

- CPU PoW consensus
- RandomX integration
- Reth/EVM execution integration
- chain P2P
- consensus RPC
- smart-contract deployment support
- node orchestration

### `niahcia-protocol`

Protocol source of truth.

Responsibilities:

- architecture
- protocol object specifications
- NIAHCIA Improvement Proposals (NIPs)
- threat model
- economics
- canonical serialization/hashing rules
- Prototype profiles

## User-facing and operational repositories

### `niahcia-miner`

Dedicated CPU mining software.

Responsibilities:

- RandomX CPU mining
- solo mining
- pool mining protocol support
- CPU tuning/autotuning
- hashrate/latency/temperature telemetry
- failover pools
- mining console/UI
- signed releases and reproducible builds

The miner secures the chain. It is not the AI compute worker.

### `niahcia-compute`

AI compute-worker software.

Responsibilities:

- compute-worker identity
- hardware/runtime capability detection
- vLLM integration
- execution-profile support
- decentralized worker discovery
- AI job execution
- streaming responses
- result commitments
- verification/audit participation
- compute telemetry

A machine may run both `niahcia-miner` and `niahcia-compute`, but they remain independent protocol roles and independent economic services.

### `niahcia-explorer`

Blockchain + AI network explorer.

#### Chain views

- blocks
- transactions
- addresses/accounts
- contracts
- gas
- difficulty
- hashrate
- miner distribution
- reorg/finality indicators

#### AI views

- AI jobs
- execution profiles
- verification status
- model registry
- compute-worker activity
- operator activity

#### Agent views

- agent identity
- immutable versions
- creator/controller
- permissions
- models
- jobs
- usage
- replication/hosting metrics

#### Service views

- service nodes
- stored model replicas
- availability proofs
- storage/retrieval activity

The explorer must read public network state and must never become an authoritative protocol database.

### `niahcia-web`

Official primary user experience.

Responsibilities:

- Ask an Agent
- agent discovery/search
- agent detail pages
- wallet connection
- AI sessions and usage
- job history
- network dashboard
- developer portal
- links into explorer
- model/agent publishing interfaces
- worker/service-node onboarding

The official website is a client of the protocol. The network must remain usable through alternative frontends, SDKs, CLI clients, and direct RPC/P2P interfaces.

### `niahcia.github.io`

Static GitHub Pages project/development site.

Recommended content:

- What is NIAHCIA?
- goals and doctrine
- architecture overview
- GitHub repository directory
- development status
- releases/downloads
- documentation links
- testnet/mainnet links when available

This site should remain small, static, and dependable.

### `.github`

GitHub account/organization presentation and shared community files.

Recommended contents:

- `profile/README.md`
- CONTRIBUTING
- SECURITY
- CODE_OF_CONDUCT
- issue templates
- pull-request template
- shared workflow policy where appropriate

## Naming rule

Repository names describe implementation roles, not marketing layers.

```text
niahcia              chain/node
niahcia-protocol     protocol/specification
niahcia-miner        CPU mining
niahcia-compute      AI compute
niahcia-explorer     chain + AI explorer
niahcia-web          official application
niahcia.github.io    static project site
.github              GitHub profile/community defaults
```

## Separation rule

These boundaries are intentional:

```text
CPU mining != AI compute
Explorer != protocol authority
Official website != network
GitHub Pages != main application
Protocol specification != reference implementation
```

A future monolithic installer may launch several components together, but protocol responsibilities remain separate.

## Recommended creation order

1. `niahcia-protocol`
2. `niahcia`
3. `niahcia-miner`
4. `niahcia-compute`
5. `niahcia-explorer`
6. `niahcia-web`
7. `niahcia.github.io`
8. `.github`

The first two exist today. The others should begin as lightweight skeleton repositories and gain implementation only when their upstream protocol dependencies are stable.
