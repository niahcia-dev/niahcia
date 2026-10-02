# Worker Selection V1

Status: **CANDIDATE / pre-alpha client-side decentralized scheduling policy**

## Purpose

Worker Selection V1 defines how wallets/clients choose compute workers without a permanent NIAHCIA scheduler.

For ordinary user-funded compute, worker selection is not a chain-consensus decision and does not require a blockchain randomness beacon.

The central rule is:

> the protocol defines objective eligibility and verifiable worker advertisements; the wallet/client chooses among eligible workers.

## Why selection is not consensus

For normal paid inference:

- the user is purchasing compute with their own authorized funds;
- selecting Worker A instead of Worker B does not change chain validity;
- worker payment comes from the purchaser, not permanent per-job protocol issuance;
- a bad selection primarily harms the purchaser, who can reselect;
- compute workers receive zero PoW consensus authority.

Therefore full nodes do not need to agree on which worker a wallet should choose for an ordinary Job.

This avoids turning worker scheduling into a miner-influenced on-chain lottery.

## Discovery

Wallets obtain signed, expiring WorkerAdvertisement records through Worker Discovery V1 or another compatible decentralized discovery source.

A wallet MUST treat an expired advertisement as ineligible.

Advertisements may include:

- worker_id;
- operator_id;
- supported models;
- supported ExecutionProfiles;
- workload types;
- available capacity/concurrency;
- standing price information;
- verification capabilities;
- bond/accountability state;
- endpoint information;
- expiry.

Discovery mechanisms may evolve independently as long as no permanent centralized scheduler becomes authoritative.

## Objective eligibility filter

Before preference or randomness, the wallet constructs an eligible set using hard requirements.

Examples:

- requested model is supported;
- ExecutionProfile is supported;
- workload class is supported;
- advertisement is current;
- worker status is eligible;
- a signed current ComputePriceOfferV1 exists for the requested service/model/profile;
- quoted/standing maximum price fits the authorization;
- capacity is non-zero;
- required bond/accountability policy is met;
- required verification feature is present.

A worker failing a hard requirement is not made eligible merely by reputation or price.

## Client-side choice

For STANDARD ordinary compute, the wallet MAY choose any eligible worker.

The default wallet policy SHOULD avoid deterministic "always choose the cheapest/fastest/highest reputation" behavior that would unnecessarily centralize demand.

A recommended default is:

```text
eligible set
   -> remove hard failures
   -> construct bounded-score candidate set
   -> cryptographically secure local random choice
```

The random choice is generated locally by the wallet/client and need not be published or agreed by the chain.

## Bounded weighting

Price, observed latency, model warmth, reliability, and reputation MAY influence selection, but their influence SHOULD be bounded.

For example, a wallet may prefer a small candidate set of acceptable workers and then randomly choose among it rather than assigning 99.9% probability to the globally fastest operator.

Exact production weighting is wallet policy, not consensus.

## User choice

Users MAY explicitly select or exclude workers/operators.

NIAHCIA does not need to prevent a user from choosing a preferred service provider when spending the user's own funds.

Wallet defaults should still encourage healthy decentralization and make concentration visible.

## New workers

A valid new worker should have a realistic path to selection.

Wallet defaults SHOULD NOT require extensive historical reputation merely to enter the eligible candidate set.

New workers may receive lower trust for high-assurance workloads until evidence exists, while remaining eligible for suitable STANDARD workloads.

## ComputeSession interaction

Worker Selection establishes the primary worker for a bounded `ComputeSessionV1`.

The wallet may then keep that worker sticky until a session bound or reselection trigger occurs.

The worker is re-evaluated when a new session is created.

## Verification selection is stricter

Independent verification is different from ordinary primary-worker selection.

When `VerificationPolicyV1` claims independence, verifier selection MUST enforce the policy's diversity requirements.

At minimum, different worker IDs controlled by the same `operator_id` do not count as independent operators.

A requester MAY choose a primary worker freely while still being required to obtain qualifying independent verification evidence before a VERIFIED/HIGH-ASSURANCE result is considered satisfied.

Future high-assurance policies may define stronger selection/randomness procedures for verifiers. Those procedures are separate from ordinary STANDARD scheduling.

## Service bonds and Sybil resistance

Service bonds remain useful for accountability and for limiting free worker identities where a policy relies on worker identity.

However, ordinary paid worker selection does not need a bond-weighted lottery.

More bond MUST NOT grant PoW consensus authority or automatically produce proportionally more user jobs.

## Miner influence

Ordinary Worker Selection MUST NOT depend on the current block hash.

Because the wallet chooses locally among eligible providers, CPU miners do not obtain a scheduling lever merely by producing a block.

If a future VerificationPolicy or bootstrap incentive requires protocol-derived unpredictable assignment, that mechanism requires a separate adversarially reviewed randomness specification.

## Failure

If the chosen worker fails:

```text
session suspended/ended
   -> refresh advertisements
   -> recompute eligible set
   -> choose replacement
   -> create new worker-bound channel/session
   -> resend only required wallet-local context
```

Failure does not require Agent-memory migration.

## Invariants

1. No global NIAHCIA scheduler is required.
2. Ordinary paid worker selection is not a consensus decision.
3. Wallets apply hard eligibility before preferences.
4. Expired advertisements are ineligible.
5. Default weighting should be bounded rather than winner-take-all.
6. New eligible workers have a path to receive STANDARD jobs.
7. User choice is allowed.
8. Different worker IDs from one operator do not satisfy operator-independence requirements.
9. Service bond does not grant consensus authority.
10. Current-block-hash scheduling is not required for ordinary compute.
11. High-assurance verifier/random assignment may use a separate stronger future mechanism.

## First milestone

The first implementation should:

1. discover signed expiring worker advertisements;
2. filter by one model/profile/price/capacity requirement set;
3. randomly choose locally among eligible candidates;
4. establish a ComputeSession;
5. record selection distribution for testing;
6. simulate worker failure and reselection;
7. demonstrate that no website or chain miner selected the worker.


## Pricing interaction

Pricing eligibility uses a signed current ComputePriceOfferV1.

Wallets MAY use price as one bounded selection input, but should not automatically collapse all demand onto the cheapest worker.

A worker whose offer would exceed the requester's hard maximum is ineligible for that Job/session.

A later price increase affects future Jobs only and may trigger voluntary reselection.
