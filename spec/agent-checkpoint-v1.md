# AgentCheckpointV1

Status: **CANDIDATE / pre-alpha portable-state model**

## Purpose

`AgentCheckpointV1` defines a durable, content-committed resume point for a NIAHCIA Agent. It lets a compatible execution host determine exactly which Agent state may be resumed after failure, migration, restart, or rescheduling.

A checkpoint is a **state commitment**, not proof that every external side effect before it happened exactly once.

## Central rule

> Agent state can be checkpointed; external reality cannot be rolled back with it.

Payments, messages, tool calls, contract transactions, storage writes, and other external effects therefore require separate replay/idempotency semantics.

## Candidate canonical fields

```text
AgentCheckpointV1
- schema_version
- checkpoint_id
- agent_id
- agent_version_id
- agent_manifest_id
- key_authority_epoch
- parent_checkpoint_id
- checkpoint_sequence
- memory_state_root
- private_state_descriptor_root
- pending_effects_root
- completed_effects_root
- active_job_root
- created_block
- checkpoint_nonce
```

Exact NCE/1 field IDs and inclusion semantics are not locked.

## Checkpoint chain

Checkpoints form an ordered lineage for a durable Agent state branch:

```text
C0 -> C1 -> C2 -> C3
```

A child binds its `parent_checkpoint_id` and monotonically advances `checkpoint_sequence` according to the applicable state-transition policy.

A host cannot make an older checkpoint current merely by replaying it.

## Memory state

`memory_state_root` commits to the exact durable memory/state version needed for resume. Private state may remain encrypted and content-addressed; the checkpoint does not imply public plaintext.

The checkpoint should reference committed durable state, not transient process memory that exists only on one host.

## Pending external effects

A checkpoint must distinguish internal Agent state from externally visible operations that may be pending or completed.

Examples:

- transaction constructed but not submitted;
- transaction submitted but not finalized/observed;
- tool request dispatched with unknown response;
- payment intent authorized but settlement unknown;
- message send requested but acknowledgement unknown.

After a crash, `UNKNOWN` must remain unknown until reconciled. The Agent must not assume failure merely because its local response was lost.

## Resume

A destination host resolves:

1. durable Agent identity;
2. exact AgentVersion/Manifest;
3. currently acceptable KeyAuthority epoch;
4. selected current checkpoint;
5. committed encrypted state referenced by the checkpoint;
6. pending-effect reconciliation state;
7. bounded migration/session authority.

Only then should normal execution resume.

## Checkpoint finality

A checkpoint's existence and a blockchain block's finality are different concepts. The exact rule for when a checkpoint becomes the preferred/current resume point is not locked here.

Implementations must not describe an operational checkpoint as consensus-final unless the applicable protocol actually commits/finalizes it that way.

## Forked Agent state

Concurrent execution may produce competing checkpoint candidates from the same parent. The protocol must not silently merge arbitrary mutable state.

A future state-transition policy must deterministically select/accept an allowed successor or explicitly model branches/merge semantics. Until locked, competing successors are a conflict requiring reconciliation.

## Security invariants

1. Checkpoint binds exact Agent, version, manifest, and state.
2. Checkpoint lineage prevents silent rollback.
3. Durable resume does not depend on the previous execution host.
4. Private state can remain encrypted.
5. Checkpoint does not imply exactly-once external effects.
6. Pending/unknown effects survive restart as unresolved facts.
7. Competing children cannot both silently become one linear current state.
8. Current authority/state policy determines acceptable resume point.
9. Local uncommitted process state is not durable Agent truth.
10. Replay protection for external effects is handled explicitly.

## Required vectors before lock

1. genesis/initial checkpoint;
2. parent -> child checkpoint;
3. sequence increment;
4. stale checkpoint rollback rejection;
5. wrong Agent/Manifest binding rejection;
6. private memory-root fixture;
7. pending-effect root fixture;
8. competing-child conflict fixture;
9. migration resume from checkpoint;
10. canonical serialization/checkpoint ID fixture.

## Not locked

- checkpoint frequency;
- on-chain inclusion;
- storage profile;
- checkpoint fees;
- branch/merge policy;
- garbage collection;
- memory delta encoding;
- checkpoint selection/finality policy.
