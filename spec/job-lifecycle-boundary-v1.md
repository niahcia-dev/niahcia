# Job Lifecycle Boundary V1

Status: **CANDIDATE / pre-alpha off-chain Job lifecycle boundary**

## Purpose

Job Lifecycle Boundary V1 defines which AI Job events normally remain off-chain and which events may require native-chain inclusion.

The central rule is:

> ordinary AI execution is an off-chain service flow; the base chain provides authorization, value settlement, and dispute/high-assurance anchoring when needed.

NIAHCIA MUST NOT require one blockchain transaction for every prompt, stream chunk, Job status transition, or result.

## Ordinary off-chain lifecycle

For a normal user-funded SMALL STANDARD inference Job, the expected path is:

```text
wallet creates Job
  -> worker accepts
  -> worker executes
  -> output streams
  -> worker finalizes ResultCommitment
  -> wallet verifies
  -> wallet signs ComputeUsageReceipt
```

All of those steps MAY remain off-chain.

The chain does not need to record:

- Job creation;
- worker acceptance;
- EXECUTING status;
- token/chunk stream;
- RESPONDED status;
- ordinary ResultCommitment;
- ordinary usage receipt issuance.

These objects remain cryptographically signed/committed even when not published on-chain.

## Chain-visible events

The native chain is used when protocol-enforceable state must change.

Examples include:

- funding/opening a ComputeChannel;
- closing/settling a ComputeChannel;
- releasing/refunding locked channel value;
- registering/revoking authority where the applicable authority model requires chain state;
- adjudicating a dispute that requires chain-enforced value movement;
- publishing evidence required by a HIGH_ASSURANCE VerificationPolicy;
- applying an objectively provable protocol penalty where separately specified;
- other explicit native state transitions.

## Off-chain does not mean unauthenticated

An off-chain Job still uses canonical signed/versioned objects.

The wallet and worker retain evidence such as:

- canonical Job bytes;
- Job acceptance;
- price offer/quote;
- ResultCommitment;
- metering evidence;
- ComputeUsageReceipt;
- relevant VerificationPolicy evidence.

These may later be submitted as dispute or audit evidence if the applicable protocol defines that path.

## Job status

The compact Job lifecycle statuses:

```text
REQUESTED
EXECUTING
RESPONDED
SETTLED
FAILED
DISPUTED
EXPIRED
CANCELLED
```

are logical protocol/application states.

They MUST NOT be interpreted as requiring one native-chain transaction per status change.

For ordinary Jobs, the wallet/worker may derive these states locally from authenticated protocol messages.

## Settlement boundary

A Job may be economically complete off-chain while its cumulative channel payment remains unsettled on-chain.

Conceptually:

```text
Job 1 -> usage receipt
Job 2 -> usage receipt
Job 3 -> usage receipt
...
latest cumulative receipt
    -> one channel settlement transaction
```

This is the expected normal path.

## ResultCommitment publication

A STANDARD Job's ResultCommitment does not need routine on-chain publication.

A VERIFIED Job may also remain off-chain if all required parties can validate the evidence and settlement policy does not require public anchoring.

HIGH_ASSURANCE policies MAY require:

- on-chain commitment;
- challenge window;
- verifier evidence root;
- dispute bond;
- other explicitly specified chain-visible evidence.

Those requirements belong to VerificationPolicy, not to every Job globally.

## Disputes

Ordinary Jobs should complete without chain adjudication.

A dispute protocol MAY accept canonical signed off-chain evidence and make a native state transition based on objective rules.

The base chain MUST NOT be asked to judge whether an LLM answer was semantically good or factually correct.

Disputes should focus on objectively evaluable protocol facts such as:

- signature mismatch;
- wrong Job/result binding;
- wrong worker;
- invalid pricing calculation;
- receipt replay;
- conflicting signed commitments;
- missed explicit deadline where objectively provable;
- malformed verification evidence.

## Availability

Because ordinary Job state is off-chain, losing one worker does not corrupt chain state.

The wallet retains its local conversation/context and may submit a replacement Job to another selected worker.

The failed Job may end as FAILED/EXPIRED locally unless a dispute/settlement rule requires chain action.

## Privacy

Keeping ordinary Jobs off-chain avoids exposing:

- prompt timing;
- session frequency;
- model choice where transport/discovery privacy permits;
- conversation linkage;
- result metadata;
- per-prompt spend.

Channel open/settlement still exposes some economic metadata. V1 does not claim full transaction anonymity.

## Auditability

Wallets and workers SHOULD retain signed Job/payment evidence at least until the related ComputeChannel is settled and any applicable dispute window has expired.

Retention may remain local and encrypted.

No decentralized storage dependency is introduced by this requirement.

## Invariants

1. Ordinary prompt execution does not require a native transaction.
2. Job state transitions are logical states, not mandatory chain events.
3. ComputeChannel settlement aggregates many Jobs.
4. STANDARD ResultCommitments are off-chain by default.
5. HIGH_ASSURANCE policies may explicitly require chain anchoring.
6. Off-chain objects remain canonical and signed.
7. Chain disputes evaluate objective protocol evidence, not semantic answer quality.
8. Worker loss does not corrupt base-chain state.
9. Off-chain audit evidence may remain wallet-local.
10. The base chain remains usable without AI workers.

## First milestone

The first end-to-end AI milestone should place only the following compute-related state on-chain:

```text
ComputeChannel open/funding
ComputeChannel final settlement/refund
```

Everything else for a SMALL STANDARD Job should remain off-chain unless a failure test explicitly exercises dispute handling.

This proves that decentralized AI chat does not require per-prompt blockchain traffic.
