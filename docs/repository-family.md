# NIAHCIA Repository Family

NIAHCIA is split into independently maintainable components. Repository boundaries describe implementation roles, not consensus authority.

## Core repositories

### `niahcia/niahcia`

Canonical working repository for the reference blockchain/node implementation plus consolidated protocol/spec/test-vector material during pre-alpha development.

Responsibilities include CPU PoW consensus, RandomX integration, native transactions/execution/state, chain P2P, mining/native RPC, persistence/reorg/restart behavior, and inactive compute-settlement integration work.

### `niahcia/niahcia-protocol`

Synchronized protocol mirror retained during the current repository transition. Protocol-affecting changes must not leave this mirror contradicting the canonical working repository.

## Operational repositories

- `niahcia-miner` — dedicated CPU mining software.
- `niahcia-compute` — replaceable AI compute-worker software.
- `niahcia-explorer` — public chain/compute/service explorer, never a protocol authority.
- `niahcia-web` — official portal; it must not own wallet authority, Agent identity, chat keys, or long-term private memory.
- `niahcia.github.io` — static project/development site.
- `.github` — organization presentation and shared community files.

## Separation rules

```text
CPU mining != AI compute
AI compute != consensus
storage/service != consensus
website != wallet/Agent ownership
explorer != protocol authority
execution host != Agent owner
```
