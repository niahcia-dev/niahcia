# Availability Attestation V1

## Status

Draft service-evidence object.

An availability attestation records an independently verifiable successful service interaction.

## Canonical fields

```text
AvailabilityAttestationV1
- schema_version
- attestation_id
- service_node_id
- service_class
- requester_id
- object_id
- request_type
- request_started_at
- request_completed_at
- bytes_verified
- evidence_hash
- requester_signature
```

## Use

Availability attestations may contribute to service-epoch evidence.

They are weaker than deterministic protocol challenges because requesters may collude with providers.

Reward logic SHOULD therefore require requester diversity and SHOULD cap the contribution of repeated attestations from the same operator or identity cluster.
