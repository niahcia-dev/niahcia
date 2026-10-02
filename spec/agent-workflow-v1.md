# AgentWorkflowV1

Status: **CANDIDATE / pre-alpha multi-step workflow model**

## Purpose

`AgentWorkflowV1` defines a durable orchestration model for multi-step Agent work spanning NIAHCIA and external systems.

A workflow may contain compute, payments, contract calls, storage operations, messages, API/tool calls, and agent-to-agent actions. These systems do not share one atomic transaction boundary.

## Central rule

> A workflow is a durable state machine, not a distributed database transaction.

NIAHCIA MUST NOT claim that arbitrary external effects can be atomically committed or rolled back together.

## Candidate canonical fields

```text
AgentWorkflowV1
- schema_version
- workflow_id
- agent_id
- agent_version_id
- agent_manifest_id
- key_authority_epoch
- workflow_definition_root
- initial_checkpoint_id
- capability_root
- budget_limit
- workflow_nonce
- valid_from
- expires_at
- created_block
```

Exact NCE/1 field IDs are not locked.

## Step definition

A workflow definition contains an ordered/dependency graph of versioned steps. A candidate step commits to:

```text
WorkflowStepV1
- step_id
- dependencies
- operation_kind
- request_template_hash
- capability_id
- budget_limit
- retry_policy
- reconciliation_policy
- compensation_policy
- timeout_policy
```

The graph MAY contain parallel independent steps. Cycles require explicit bounded semantics; arbitrary unbounded recursive workflow graphs are not assumed safe.

## Step state

Candidate operational states:

```text
PENDING
READY
AUTHORIZED
SUBMITTED
UNKNOWN
OBSERVED
COMPLETED
FAILED
COMPENSATING
COMPENSATED
MANUAL_REVIEW
EXPIRED
```

`UNKNOWN` is mandatory for effects whose outcome cannot yet be established.

`MANUAL_REVIEW` is mandatory as an available terminal/paused safety state where automatic retry or compensation would be unsafe.

Exact state-transition rules remain profile-specific until locked.

## Side effects

Every externally visible step uses a `SideEffectIntentV1` identity when applicable.

Retries of the same logical action retain the same effect identity. A workflow engine MUST NOT manufacture a new effect merely because a host restarted or a response timed out.

## Reconciliation before retry

For a step in `UNKNOWN`, the workflow engine first attempts reconciliation using available receipts, chain state, service-native idempotency/status APIs, or other versioned evidence.

Only when policy determines retry is safe may the same intent be retried.

If safety cannot be established, the step remains `UNKNOWN` or moves to `MANUAL_REVIEW`; uncertainty is not converted into guessed failure.

## Compensation

Compensation is a new forward action intended to offset an earlier completed action. It is **not rollback**.

Examples:

- refund a payment;
- cancel a reservation if the external system permits it;
- revoke a capability;
- delete/revoke a newly created object where policy permits;
- send a corrective message.

A compensation itself has a unique `SideEffectIntentV1`, authorization, budget, receipts, and failure/UNKNOWN states.

Some actions are irreversible or only partially compensatable. Their workflow definitions must say so.

## Irreversible steps

A workflow SHOULD delay irreversible/high-risk effects until prerequisite reversible/reconcilable work is complete where possible.

Examples include finalized blockchain transfers, publication of secrets, irreversible external submissions, and physical-world actions.

Policy may require explicit human/controller approval before such a step.

## Checkpoints

Workflow progress is included in durable Agent state/checkpoints. Resume reconstructs the workflow from committed state and reconciles any submitted/unknown effects before continuing.

A checkpoint rollback MUST NOT reset a completed external effect to `PENDING`.

## Host migration

Workflow identity and step/effect identities survive host migration. A destination host receives only capabilities needed for currently eligible steps.

Two hosts may race. Canonical workflow state, signer policy, effect identities, budgets, and target idempotency must prevent that race from silently doubling economic actions.

## Budget containment

Workflows need aggregate and per-step limits. A retry of the same effect does not create a fresh budget allocation merely because execution moved to another host.

Compensation budgets are separately accounted so an attacker cannot force unlimited compensating spend by intentionally causing failures.

## Agent-to-agent workflows

An Agent may call/pay another Agent, but each Agent retains independent authority. One Agent cannot roll back another Agent's finalized state merely because its own workflow failed.

Cross-Agent coordination therefore uses explicit intents, receipts, contracts/escrow where appropriate, reconciliation, and compensation rather than assumed shared transactions.

## Failure policy

Workflow definitions should specify what happens when a step fails or remains unknown:

- retry same intent;
- reconcile later;
- compensate prior steps;
- continue along an allowed alternate path;
- expire;
- require controller/manual review.

Silent best-effort guessing is not acceptable for economic/security-sensitive effects.

## Security invariants

1. Workflow state is durable and host-independent.
2. Arbitrary external systems are not treated as one atomic transaction.
3. Same logical retry preserves effect identity.
4. UNKNOWN remains unknown until reconciled or explicitly adjudicated.
5. Compensation is a new auditable action, not history rewrite.
6. Irreversible steps are explicitly identified/policy-bound.
7. Checkpoint rollback cannot replay completed external reality.
8. Migration does not reset workflow/effect identities or budgets.
9. Duplicate hosts cannot obtain fresh spend merely by racing/retrying.
10. Unsafe ambiguity can stop at MANUAL_REVIEW.
11. Agent-to-agent work preserves each Agent's independent authority.
12. Compensation failure is itself represented and reconciled.

## Required vectors before lock

1. linear three-step workflow;
2. dependency graph with parallel independent steps;
3. failed step with safe retry of same effect ID;
4. UNKNOWN step followed by reconciliation;
5. completed step followed by compensation intent;
6. failed compensation;
7. irreversible-step approval fixture;
8. host migration mid-workflow;
9. duplicate-host concurrent retry;
10. checkpoint resume with submitted-but-unacknowledged effect;
11. budget exhaustion/retry containment;
12. cross-Agent workflow fixture;
13. canonical workflow/step IDs.

## Not locked

- universal workflow DSL;
- exact state enum IDs;
- scheduler implementation;
- compensation language;
- human approval UI;
- timeout/retry constants;
- on-chain workflow inclusion;
- universal external receipt format;
- distributed transaction/2PC support (not assumed).
