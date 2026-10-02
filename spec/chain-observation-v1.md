# Chain Observation Objects V1

## Status

Draft support-object specification.

These objects carry signed observations made by service nodes. They are not consensus objects and do not alter fork choice.

## ChainObservationV1

```text
ChainObservationV1
- schema_version
- network_id
- observer_service_node_id
- observation_sequence
- observed_at
- tip_block_id
- height
- cumulative_work
- latest_timestamp
- signature
```

The object means only that a service node claims it observed this NIAHCIA tip at this time.

A verifier MUST NOT infer that the observed tip was canonical merely because the object is signed.

## ReorgObservationV1

```text
ReorgObservationV1
- schema_version
- network_id
- observer_service_node_id
- observation_sequence
- observed_at
- old_tip
- new_tip
- fork_point
- old_height
- new_height
- reorg_depth
- old_cumulative_work
- new_cumulative_work
- signature
```

The object records a claimed local observation of a canonical-head transition.

A verifier SHOULD independently fetch and validate the referenced branches.

## Sequence numbers

`observation_sequence` MUST increase monotonically for a given service-node identity.

This helps clients detect replay, duplication, and obvious history rewriting by one observer. It does not create global ordering across observers.

## Signing

Observation objects use the protocol's ordinary signed-object rules.

The signature covers all canonical fields except the signature field itself.

## Retention

REORG_WATCH and ARCHIVE service nodes SHOULD retain all locally observed deep-reorg records, referenced competing branch headers, observation signatures, and enough ancestry to independently reconstruct the fork.

## Client use

Clients MAY aggregate signed observations to improve diagnostics.

Permitted uses include eclipse warnings, partition warnings, sudden private-chain publication warnings, reorg dashboards, operational telemetry, and forensic evidence.

Prohibited consensus use includes counting observation signatures as votes, selecting a canonical chain by observer majority, or rejecting a valid heavier PoW chain because observers disagree with it.
