# Native Execution Transition

Status: active implementation work.

NIAHCIA is replacing the current external execution-client subsystem with native NIAHCIA execution. This is a remove-and-replace migration, not a compatibility-layer migration.

## Preserve

The following implementation behavior remains foundational:

- the 164-byte `BlockHeaderV1` layout and canonical encoding;
- block ID and mining-template identity domains;
- `transactions_root` and `execution_root` header commitments;
- CPU proof-of-work cumulative-work fork choice;
- deterministic equal-work block-ID tie breaking;
- chain ancestry/reorganization machinery;
- service evidence persistence.

## Remove

The obsolete subsystem is removed coherently:

- `ExecutionPayloadCommitments` as the external payload-shaped execution model;
- external execution hash mappings;
- external payload/replay journals;
- external fork-choice synchronization;
- external payload building and validation RPC;
- JWT execution-client authentication;
- external execution/public RPC configuration;
- tests whose invariant is that a NIAHCIA block requires an external replay payload or maps to an external execution block.

Do not retain aliases, fallback paths, temporary compatibility adapters, or dead configuration after the native replacement is active.

## Replace

Introduce a native execution module whose deterministic result includes parent/resulting state commitments, receipt commitment, ordered transaction commitment, gas/fee accounting, receipts, and the state delta required for atomic persistence/recovery.

Native block persistence must commit a validated block and its native execution transition atomically. Restart and reorganization recovery must not require an external chain or execution service.

## Implementation sequence

1. Native execution primitives and commitment.
2. Native execution persistence.
3. Mining work construction/submission integration.
4. Startup/canonical-head integration.
5. Delete obsolete execution client/config/dependencies/tests.
6. Repository-wide stale-assumption audit.
7. Full build/test plus restart, mining, competing-chain and reorg validation.

The header is deliberately not redesigned during this transition. Native execution fills the existing `execution_root` consensus boundary.

See `niahcia-protocol/docs/native-execution-transition.md` for the protocol-side replacement boundary.
