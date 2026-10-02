# ComputeWorker

## Status

**Schema V1: legacy candidate field layout retained.**  
**Schema V2: CURRENT CANDIDATE stable worker identity.**

## Purpose

`ComputeWorker` is the durable identity of one compute-serving worker controlled by an Operator.

It is not the worker's live queue, model cache, price list, or network-presence record.

Fast-changing service state belongs in signed, expiring `WorkerAdvertisementV1` objects.

## Identity model

```text
Operator
   |
   +-- ComputeWorker A
   |      +-- WorkerAdvertisement epoch/sequence ...
   |
   +-- ComputeWorker B
          +-- WorkerAdvertisement epoch/sequence ...
```

Worker identity and operator identity are different.

Two worker IDs controlled by one operator do not become independent operators merely because they are separate workers.

## Schema V1

V1 assigned these fields:

```text
1   schema_version
2   worker_id
3   operator_id
4   controller
5   payment_address
6   execution_profiles
7   hardware_capabilities
8   workload_types
9   capacity
10  queue_state
11  models_hot
12  models_warm
13  pricing_policy
14  bond
15  reputation_ref
16  availability_expiry
17  endpoint_descriptor
18  advertisement_signature
19  status
```

Those field meanings remain reserved.

V1 mixed durable identity with dynamic advertisement state and is superseded for new implementation work.

## Schema V2 stable identity

V2 reuses unchanged stable fields:

```text
1   schema_version
2   worker_id
3   operator_id
4   controller
5   payment_address
14  bond
15  reputation_ref
19  status
```

and appends:

```text
20  worker_signing_key
21  created_height
22  metadata_commitment
```

Dynamic capability/availability information is not part of the normal V2 identity encoding.

## Stable fields

`worker_id` identifies the worker across many advertisement refreshes.

`operator_id` identifies the operator controlling the worker.

`controller` authorizes durable worker identity changes according to the applicable authority rules.

`payment_address` is the default durable payment destination or settlement reference where applicable. Worker-bound ComputeChannels may bind more specific payment terms.

`worker_signing_key` authenticates WorkerAdvertisement and other worker-originated protocol objects.

## Bond/accountability

A service bond may be associated with a worker/operator for economic accountability.

Bond size does not grant PoW consensus authority and does not automatically increase worker-selection probability.

Exact production bond requirements remain policy-specific.

## Status

Durable identity status may include:

```text
ACTIVE
SUSPENDED
EXITED
REVOKED
```

This is distinct from temporary availability such as queue-full, draining, endpoint-down, or model-cold states, which belong in WorkerAdvertisement.

## Dynamic state belongs elsewhere

The following are advertisement data, not durable worker identity:

- supported ExecutionProfiles;
- hardware capability claims;
- workload types;
- current capacity/concurrency;
- queue state;
- HOT/WARM/COLD model state;
- pricing offers;
- verification capabilities;
- endpoint descriptor;
- advertisement validity/expiry;
- temporary DRAINING state.

A worker updates those by signing a new WorkerAdvertisementV1.

## Invariants

1. Worker identity is distinct from Operator identity.
2. Durable identity does not expire merely because an advertisement expires.
3. An expired advertisement makes the worker unavailable for new selection, not nonexistent.
4. Queue/model/pricing changes do not require rewriting the durable worker identity.
5. Worker advertisements are authenticated by the worker authority.
6. Base-chain mining eligibility is unrelated to ComputeWorker registration.
7. Bond does not grant consensus authority.
8. Same-operator workers do not satisfy operator-independence requirements.
9. V1 field meanings remain reserved and are not silently repurposed.

## First milestone

Register or configure a durable ComputeWorker identity, then publish repeatedly refreshed WorkerAdvertisementV1 records for discovery and selection.
