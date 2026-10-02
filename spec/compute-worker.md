# ComputeWorker

## Purpose

`ComputeWorker` represents a network participant offering AI or other supported compute workloads.

The protocol intentionally avoids the term GPU miner.

## Canonical fields

```text
ComputeWorker
- schema_version
- worker_id
- operator_id
- controller
- payment_address
- execution_profiles[]
- hardware_capabilities[]
- workload_types[]
- capacity
- queue_state
- models_hot[]
- models_warm[]
- pricing_policy
- bond
- reputation_ref
- availability_expiry
- endpoint_descriptor
- advertisement_signature
- status
```

## Capabilities

Workers advertise what they can execute, including:

- execution profiles
- workload classes
- available capacity
- currently hot/warm models
- optional performance claims
- standing pricing information

Claims are not automatically trusted. Observed signed job history SHOULD feed reputation and scheduling.

## Model state

```text
HOT   model ready in accelerator memory
WARM  model cached locally and loadable
COLD  model not cached but retrievable
```

Scheduling policies may prefer HOT/WARM workers for interactive jobs.

## Availability

`availability_expiry` makes worker advertisements self-expiring.

A worker must refresh availability to remain eligible.

## Status

```text
ACTIVE
DRAINING
SUSPENDED
EXITED
```

## Invariants

1. Worker identity is distinct from operator identity.
2. A worker MUST NOT be eligible for an execution profile it has not advertised/satisfied.
3. Worker advertisements MUST be signed.
4. Queue/capacity advertisements expire and MUST NOT be treated as permanent state.
5. Base-chain mining eligibility is unrelated to ComputeWorker registration.

## Prototype 0

Prototype 0 workers advertise one vLLM/NVIDIA execution profile, model state, queue availability, payment address, and operator identity.


## Pricing offers

`pricing_policy` SHOULD reference signed, expiring `ComputePriceOfferV1` records defined by ComputePricingV1.

A standing advertisement may expose multiple offers for different model/profile/service combinations.

Workers MUST NOT rely on mutable unsigned pricing that can change after Job acceptance.

A worker MAY reject future Jobs when an offer expires or capacity changes, but already accepted Jobs remain bounded by their accepted `offer_id` and `max_price`.


## Transport endpoint

`endpoint_descriptor` SHOULD follow AI Transport V1 and commit an authenticated, expiring transport endpoint plus worker transport public key/protocol version.

The worker's advertised identity/transport key authenticates the endpoint. Fresh per-ComputeSession traffic keys are derived during transport establishment; long-lived worker identity keys are not direct chat-content encryption keys.


## Discovery propagation

Workers publish signed, expiring advertisements for relay under Worker Discovery V1.

A discovery relay may distribute a worker's advertisement but cannot modify its signed contents or become authoritative for the worker.

The worker SHOULD refresh its advertisement before expiry while it wishes to remain discoverable/eligible.
