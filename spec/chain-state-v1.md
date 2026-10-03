# NIAHCIA Chain State V1

## Status

Draft storage/validation model.

NIAHCIA maintains its own consensus chain database independently of the execution engine database.

## Minimum stored header metadata

For every known valid or provisionally valid block:

- block ID
- canonical BlockHeaderV1 bytes
- parent block ID
- height
- timestamp
- target
- per-block work
- cumulative work
- transactions root
- execution root
- canonical/orphan status

## Fork choice

Among fully valid branches:

```text
preferred_chain = branch with greatest cumulative work
```

Block count alone does not determine fork choice.

## Validation stages

A candidate block is processed in this order:

1. decode exact BlockHeaderV1,
2. verify supported version,
3. verify parent exists (except genesis),
4. verify height,
5. verify timestamp rules,
6. verify expected target,
7. derive RandomX seed from NIAHCIA history,
8. verify RandomX PoW,
9. verify transaction Merkle root,
10. execute/verify execution result,
11. verify execution root,
12. calculate block work and cumulative work,
13. insert into block tree,
14. change canonical head only if fork-choice rules require it,
15. make the validated native state associated with the winning branch canonical.

## Authority boundary

The NIAHCIA chain database and its committed native state snapshots are authoritative for:

- NIAHCIA parentage;
- PoW validity;
- cumulative work;
- canonical-chain selection;
- reorganization decisions;
- the native state committed by each accepted block.

## Service-node observations

Service nodes may retain competing branches, relay headers, serve snapshots, and publish signed chain/reorg observations.

These are support services only.

The chain database MUST NOT use service-node count, signatures, collateral, reputation, or quorum as fork-choice weight.

A valid heavier PoW chain cannot be vetoed by service nodes.

Conversely, service-node agreement cannot make an invalid or lower-work chain canonical.
