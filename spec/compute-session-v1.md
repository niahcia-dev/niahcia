# ComputeSessionV1

Status: **CANDIDATE / pre-alpha sticky bounded worker session**

## Purpose

`ComputeSessionV1` defines the bounded period during which a wallet/client keeps using one selected compute worker for a sequence of related Jobs.

The objective is to preserve low latency, model warmth, and simple single-worker payment-channel accounting without creating permanent worker affinity or winner-take-all scheduling.

The central rule is:

> worker selection may be sticky for efficiency, but every session is bounded and replaceable.

## Candidate canonical fields

```text
ComputeSessionV1
- schema_version
- session_id
- requester_identity
- worker_id
- operator_id
- model_id
- execution_profile_id
- verification_policy_id
- compute_channel_id
- started_height
- expiry_height
- max_jobs
- max_total_spend
- max_idle_blocks
- sequence
- status
```

Exact policy constants remain candidate.

## Session establishment

A session begins only after a worker has been selected from an eligible candidate set under the applicable scheduling policy. For ordinary paid compute, Worker Selection V1 permits the wallet/client to perform this selection locally from signed, expiring advertisements; chain consensus does not choose the worker.

The selected worker MUST satisfy the Job/session requirements at establishment, including:

- model support;
- ExecutionProfile support;
- service status;
- current advertisement validity;
- price/budget ceiling;
- required verification capability;
- bond/accountability requirements where applicable.

The session does not grant the worker chain-consensus authority or ownership of the wallet, Agent, conversation, or memory.

## Sticky execution

Jobs in the same session may continue using the selected worker while all bounds remain valid.

This is preferred over selecting a new worker for every chat prompt because it can:

- keep the model loaded/hot;
- reduce repeated discovery and handshake latency;
- amortize channel setup;
- preserve streaming responsiveness;
- simplify cumulative payment receipts.

Sticky execution MUST NOT imply permanent exclusivity.

## Mandatory bounds

A session MUST be bounded by at least:

- `expiry_height`;
- `max_jobs`;
- `max_total_spend`;
- `max_idle_blocks`;
- ComputeChannel expiry;
- current worker advertisement validity.

The effective session end is the earliest applicable bound.

## Reselection triggers

The wallet/client MUST end or suspend the session and re-run worker selection when the selected worker becomes ineligible or unusable.

Examples include:

- worker advertisement expires;
- worker goes offline;
- execution deadline is missed;
- model/profile support changes;
- price exceeds authorized policy;
- compute channel expires or is exhausted;
- worker enters DRAINING/SUSPENDED/EXITED;
- verification policy requires a new independent participant;
- operator-diversity policy would be violated;
- repeated objectively measurable service failures exceed policy thresholds.

A poor or undesirable LLM answer alone is not proof of protocol misbehavior and does not justify slashing.

## Voluntary rotation

A wallet MAY rotate workers before a mandatory trigger.

Scheduling policy may impose a maximum session age or job count to ensure that long-lived interactive use does not permanently concentrate demand on one worker/operator.

V1 SHOULD prefer bounded periodic reselection rather than random worker changes on every prompt.

## Suggested initial development policy

For early devnet testing, implementations may begin with a simple development profile such as:

```text
max_jobs       = 100
max_session    = 120 blocks
max_idle       = 20 blocks
```

These values are development defaults only and are not production consensus constants.

The protocol should measure:

- cold-start vs warm latency;
- worker failure/reselection time;
- selection distribution across operators;
- session length distribution;
- channel utilization;
- user-visible interruption rate.

Production values should be informed by those measurements.

## Privacy

A ComputeSession binds a sequence of Jobs together from the selected worker's perspective.

Therefore longer sessions improve performance/payment efficiency but increase linkability between prompts handled by that worker.

Wallets SHOULD permit privacy-sensitive users or policies to request shorter sessions or immediate reselection.

Changing workers does not require moving durable conversation state because V1 Agent/chat memory remains wallet/client-local.

## Verification interaction

The primary ComputeSession binds only the primary worker.

Independent verifiers required by a VerificationPolicy MUST be selected separately under the policy's diversity rules. A verifier controlled by the same operator does not satisfy an operator-independence requirement merely because it has a different worker ID.

## Payment interaction

A V1 ComputeSession SHOULD reference one worker-bound `ComputeChannelV1`.

When the session ends, the channel may be settled/closed or kept only if its policy explicitly permits reuse with the same bound worker and scope.

A new worker requires a new worker-bound ComputeChannel.

## Failure continuity

If a worker fails:

```text
wallet-local conversation/memory
      |
      +-- end/suspend old ComputeSession
      +-- preserve Job/result history locally
      +-- select replacement worker
      +-- open new worker-bound channel if needed
      +-- send only required context
      +-- continue
```

No Agent-state migration between workers is required.

## Invariants

1. One ComputeSession has one primary worker/operator.
2. Sessions are bounded and expire.
3. Worker stickiness grants no consensus authority.
4. Durable user memory is not owned by the session worker.
5. Reselection is possible without storage-node participation.
6. A new worker requires fresh eligibility evaluation.
7. Independent verification follows VerificationPolicy diversity rules, not session affinity.
8. Session continuation cannot exceed payment authorization/channel bounds.
9. Session identity may be pseudonymous and need not reveal the durable funding account.
10. Production rotation constants are policy parameters, not silently hard-coded consensus values.

## First milestone

The first implementation should prove:

1. select one eligible worker;
2. establish one ComputeSession;
3. open/bind one ComputeChannel;
4. execute several Jobs while the model remains warm;
5. accumulate usage receipts;
6. simulate worker loss;
7. reselect a different worker;
8. continue from wallet-local context;
9. settle the original channel;
10. confirm no conversation/storage migration was required.
