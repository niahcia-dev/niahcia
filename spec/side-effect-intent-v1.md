# SideEffectIntentV1

Status: **CANDIDATE / pre-alpha replay/idempotency model**

## Purpose

`SideEffectIntentV1` defines a stable identity for an Agent-requested external effect so retries, host migration, duplicate execution, and lost acknowledgements do not automatically create duplicate actions.

This is especially important for payments, contract transactions, messages, storage mutations, tool calls, purchases, and agent-to-agent economic actions.

## Central rule

> Retry the intent, not a newly invented action.

If an Agent is unsure whether an external operation completed, it should reconcile the same `effect_id` rather than generate a semantically similar operation with a new identity.

## Candidate canonical fields

```text
SideEffectIntentV1
- schema_version
- effect_id
- agent_id
- agent_version_id
- agent_manifest_id
- checkpoint_id
- job_id
- session_id
- capability_id
- effect_kind
- target_id
- canonical_request_hash
- value_limit
- fee_limit
- effect_nonce
- valid_from
- expires_at
- created_block
```

Exact fields vary by effect profile; NCE/1 IDs are not locked.

## Effect identity

`effect_id` must commit to the canonical intent rather than host-local retry metadata.

The same logical retry uses the same effect identity. A genuinely new action uses a new effect identity.

## State model

Candidate operational states:

```text
PREPARED
AUTHORIZED
SUBMITTED
OBSERVED
FINALIZED
FAILED
EXPIRED
UNKNOWN
```

`UNKNOWN` is first-class. A timeout or lost response does not prove that an external system failed to perform the operation.

Exact state-transition semantics are effect-profile-specific and not locked by this document.

## Idempotency

Where the target protocol/service supports idempotency keys, `effect_id` SHOULD be used or deterministically mapped to the target's idempotency mechanism.

Where the target does not support idempotency, NIAHCIA cannot universally guarantee exactly-once execution. The Agent/runtime must reconcile target state before retrying, and the capability/signer layer should reject duplicate authorization where it can do so safely.

## Blockchain payments/transactions

For chain-native effects, transaction nonce/state semantics may provide part of replay protection. `effect_id` still provides Agent-level intent identity so migration/recovery logic can determine whether a newly constructed transaction is a retry of an existing intent or a new economic action.

An Agent must not simply issue a second payment because the first transaction acknowledgement was lost.

## Tool/API effects

For external APIs, the adapter should persist:

- effect identity;
- canonical request hash;
- target-provided idempotency key/reference if any;
- submission evidence;
- response/receipt commitment;
- reconciliation result.

Secrets/authentication material must not be embedded in public effect commitments.

## Signer interaction

A signer should receive a structured effect intent and verify current authority/capability, target, value/fee limits, validity, and prior authorization state before signing.

Model-generated prose alone cannot authorize a second payment/tool effect.

## Duplicate execution

Two hosts may concurrently attempt the same effect after a migration race. Both must refer to the same `effect_id` when retrying the same logical operation.

Where NIAHCIA controls settlement, duplicate settlement for the same effect identity should be rejected. For external systems, adapters use available idempotency/reconciliation mechanisms and must accurately represent residual uncertainty.

## Checkpoint interaction

`AgentCheckpointV1` commits pending/completed effect state. A crash after external submission but before local checkpointing is a critical ambiguity case: resume logic must reconcile the effect before deciding whether another submission is safe.

Checkpoint rollback never authorizes replay of an already finalized external effect.

## Security invariants

1. Same logical retry retains the same effect identity.
2. New effect identity means a new potentially chargeable/externally visible action.
3. Timeout/lost acknowledgement does not equal failure.
4. Duplicate execution is assumed possible.
5. Signer/capability checks are independent of model prose.
6. Checkpoint rollback does not roll back external reality.
7. Target-native idempotency is used when available.
8. Exactly-once behavior is not claimed where the target cannot provide it.
9. Secret credentials are not committed in public intent objects.
10. NIAHCIA-controlled settlement should reject duplicate settlement for one effect identity.

## Required vectors before lock

1. canonical effect identity;
2. identical retry -> identical effect ID;
3. changed amount/target -> different effect ID;
4. duplicate NIAHCIA settlement rejection;
5. UNKNOWN after lost acknowledgement;
6. reconciliation to OBSERVED/FINALIZED;
7. expired intent rejection;
8. wrong capability/target rejection;
9. checkpoint pending-effect fixture;
10. concurrent-host retry fixture.

## Not locked

- universal effect enum;
- external adapter APIs;
- finality thresholds per external chain/service;
- retry intervals;
- receipt storage location;
- on-chain intent inclusion;
- compensation/saga semantics for multi-effect workflows.
