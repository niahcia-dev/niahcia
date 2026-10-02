# WorkerAdvertisementV1

Status: **CANDIDATE / pre-alpha signed expiring compute-service advertisement**

## Purpose

`WorkerAdvertisementV1` is the worker-signed, short-lived description of what a ComputeWorker can serve right now.

It is deliberately separate from durable ComputeWorker identity.

The central rule is:

> worker identity persists; availability and service claims expire.

## Object type

```text
0x0200  WorkerAdvertisement
```

## Candidate canonical fields

```text
1   schema_version
2   advertisement_id
3   worker_id
4   operator_id
5   advertisement_sequence
6   execution_profiles
7   models
8   workload_types
9   hardware_capabilities
10  capacity
11  queue_state
12  model_states
13  price_offer_ids
14  verification_capabilities
15  endpoint_descriptor
16  valid_from
17  expires_at
18  service_status
19  bond_reference
20  advertisement_signature
```

## Sequence and freshness

`advertisement_sequence` is monotonic for one worker identity.

A newer valid advertisement supersedes older service-state claims for new selection.

An old advertisement does not become current again merely because a relay replays it.

Advertisements expire even if no replacement is published.

## Execution profiles and models

`execution_profiles` lists profiles the worker currently claims it can execute.

`models` lists models/versions currently available or serviceable.

`model_states` may distinguish:

```text
HOT
WARM
COLD
```

where:

- HOT: immediately resident/ready in accelerator memory;
- WARM: locally cached and expected to load without remote model acquisition;
- COLD: supported but not currently cached/loaded.

A wallet may use these as bounded selection preferences.

## Capacity and queue state

Advertisement capacity is temporary state.

Examples include:

- maximum supported concurrency;
- currently available slots;
- queue depth/class;
- estimated admission delay where advertised.

These are signed claims, not guarantees.

Wallets may measure observed behavior independently.

## Pricing

`price_offer_ids` references signed ComputePriceOfferV1 objects.

Unsigned mutable pricing is not sufficient for paid Job acceptance.

A price offer and the advertisement referencing it may have independent expiries; both must be valid when required.

## Verification capability

`verification_capabilities` describes whether the worker can participate in applicable verification roles/policies.

Claimed capability does not waive VerificationPolicy diversity or evidence requirements.

## Endpoint

`endpoint_descriptor` follows AI Transport V1.

It binds the worker's currently advertised transport endpoint/key/profile.

Changing endpoint does not change worker identity; it requires a fresh signed advertisement.

## Service status

Temporary service status may include:

```text
AVAILABLE
BUSY
DRAINING
MAINTENANCE
UNAVAILABLE
```

Only states permitted by Worker Selection V1 are eligible for new sessions.

## Bond reference

`bond_reference` may identify the applicable worker/operator service bond or accountability state.

The advertisement cannot create or enlarge a bond by assertion; wallets/full nodes validate the referenced state under the applicable economic rules.

## Discovery

Worker Discovery V1 distributes the original signed advertisement.

Discovery relays are not allowed to rewrite its contents.

Wallets verify advertisement identity, sequence, signature, expiry, and relevant referenced objects locally.

## Privacy

WorkerAdvertisement is public service metadata.

It should not contain customer prompts, session identities, wallet addresses of customers, or Job-specific private information.

## Failure and expiration

If a worker disappears without refreshing:

```text
advertisement expires
   -> worker becomes ineligible for new selection
   -> existing ComputeSession follows its own failure/reselection rules
```

No global slashing follows merely from advertisement expiration.

## Invariants

1. Advertisement state expires.
2. Durable worker identity does not expire with it.
3. Sequence is monotonic per worker.
4. Worker signature authenticates the advertisement.
5. A relay cannot forge current worker state.
6. Pricing references must independently validate.
7. Endpoint changes require a new advertisement.
8. Queue/model availability claims are not permanent identity fields.
9. Same operator identity remains visible for verification-diversity checks.
10. Expiration makes a worker ineligible for new selection without granting consensus consequences.

## First milestone

A worker periodically publishes one signed advertisement containing:

- worker/operator identity;
- one supported model/version;
- one ExecutionProfile;
- capacity;
- HOT/WARM/COLD model state;
- one signed price offer;
- AI Transport endpoint;
- expiry.

Wallets discover, verify, cache, expire, and locally select from these advertisements.
