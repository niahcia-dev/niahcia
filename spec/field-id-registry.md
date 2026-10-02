# Canonical Field ID Registry

## Status

Draft — Protocol v1 foundation.

This document assigns permanent numeric field identifiers used by NCE/1 canonical maps.

## Rules

- field IDs are unsigned integers
- field IDs are permanent once assigned
- removed/deprecated IDs remain reserved
- IDs are never reused for another semantic meaning within the same object type
- new fields append new IDs unless a specification explicitly reserves a range
- payload field IDs are defined by each object's registered schema
- where an object schema includes a payload `schema_version`, it MUST equal the top-level NCE/1 envelope schema version
- an object schema MAY rely solely on the top-level NCE/1 envelope schema version and therefore omit a duplicate payload `schema_version`

---

## Agent — object type 0x0001

```text
1   schema_version
2   agent_id
3   creator
4   controller
5   governance_mode
6   current_version
7   treasury_address
8   status
9   created_block
10  metadata_uri
```

## AgentVersion — object type 0x0002

```text
1   schema_version
2   agent_id
3   version
4   definition_hash
5   primary_models
6   fallback_models
7   execution_profiles
8   verification_policy_id
9   system_definition_hash
10  tool_manifest_hash
11  permission_manifest_hash
12  memory_policy_hash
13  economic_policy_hash
14  trigger_policy_hash
15  child_agent_policy_hash
16  previous_version
17  created_block
```

## Model — object type 0x0003

```text
1   schema_version
2   model_id
3   creator
4   name
5   architecture
6   weights_manifest_hash
7   tokenizer_hash
8   config_hash
9   license_id
10  license_hash
11  manifest_uri
12  supported_workloads
13  supported_execution_profiles
14  status
15  created_block
```

## ExecutionProfile — object type 0x0004

```text
1   schema_version
2   profile_id
3   runtime
4   runtime_version
5   runtime_build_hash
6   hardware_requirements
7   precision
8   quantization
9   context_limit
10  determinism_mode
11  batch_policy
12  seed_policy
13  generation_policy
14  supported_workloads
15  input_format_version
16  output_format_version
17  profile_hash
```

## Operator — object type 0x0005

```text
1   schema_version
2   operator_id
3   controller
4   payment_address
5   bond
6   status
7   workers
8   service_nodes
9   reputation_ref
10  created_block
```

## ComputeWorker — object type 0x0006

### Schema V1 — legacy candidate fields

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

### Schema V2 — stable identity additions

V2 reuses stable V1 fields only with unchanged meanings and appends:

```text
20  worker_signing_key
21  created_height
22  metadata_commitment
```

V1 dynamic-state field IDs remain reserved even when omitted from normal V2 identity encoding.

## ServiceNode — object type 0x0007

```text
1   schema_version
2   service_node_id
3   operator_id
4   controller
5   payment_address
6   services
7   storage_capacity
8   bandwidth_class
9   stored_manifests
10  availability_state
11  endpoint_descriptor
12  pricing_policy
13  bond
14  reputation_ref
15  status
```

## Job — object type 0x0008

### Schema V1 — superseded candidate; IDs remain reserved

```text
1   schema_version
2   job_id
3   requester
4   agent_id
5   agent_version
6   model_id
7   execution_profile_id
8   workload_type
9   input_manifest_hash
10  output_requirements_hash
11  verification_policy_id
12  payment_plan_id
13  resource_requirements
14  privacy_requirements
15  scheduling_policy
16  parent_job_id
17  root_job_id
18  created_block
19  deadline_block
20  status
21  accepted_result_id
```

### Schema V2 — current candidate additions

V2 may reuse V1 fields only with the same semantic meaning. New V2 semantics append:

```text
22  input_commitment
23  encrypted_input_descriptor
24  model_version
25  payment_authorization_id
26  price_offer_id
27  quote_id
28  max_price
29  payment_risk_mode
30  compute_session_id
31  submitted_height
32  assignment_deadline
33  execution_deadline
34  result_destination
35  resource_limits
36  requester_signature
```

V1 IDs 9, 13, 15, 18, 19, 20, and 21 remain permanently reserved with their V1 meanings even when omitted from normal V2 encoding.

## VerificationPolicy — object type 0x0009

```text
1   schema_version
2   policy_id
3   type
4   executor_count
5   agreement_threshold
6   challenge_window
7   audit_probability
8   audit_count
9   bond_requirements
10  dispute_policy
11  hardware_diversity_rules
12  operator_diversity_rules
13  settlement_delay
14  policy_hash
```

## Capability — object type 0x000A

```text
1   schema_version
2   capability_id
3   subject
4   resource_type
5   resource_identifier
6   actions
7   limits
8   max_spend
9   rate_limit
10  valid_from
11  expiry
12  approval_mode
13  delegate_allowed
14  revocation_ref
15  capability_hash
```

## MemoryDescriptor — object type 0x000B

```text
1   schema_version
2   memory_id
3   scope
4   owner
5   agent_id
6   session_id
7   root_hash
8   storage_manifest_hash
9   encryption_policy
10  read_policy
11  write_policy
12  replication_policy
13  version
14  previous_root
15  updated_block
```

## PaymentPlan — object type 0x000C

```text
1   schema_version
2   payment_plan_id
3   currency
4   max_total
5   escrow_amount
6   executor_budget
7   verifier_budget
8   storage_budget
9   routing_budget
10  child_agent_budget
11  creator_fee
12  protocol_fee
13  refund_policy
14  settlement_policy
15  expiry_policy
16  plan_hash
```

## ResultCommitment — object type 0x000D

```text
1   schema_version
2   commitment_id
3   job_id
4   worker_id
5   operator_id
6   model_id
7   execution_profile_id
8   input_hash
9   output_hash
10  token_or_artifact_hash
11  runtime_receipt_hash
12  output_manifest_hash
13  output_location
14  started_at
15  completed_at
16  submitted_block
17  nonce_or_salt_commitment
18  signature
```

## Derived-field rule

The assigned field ID exists even when the field is excluded from a particular hash/signing preimage.

Examples:

- `profile_id` and `profile_hash` retain their field IDs but are omitted while deriving those values
- `commitment_id` and `signature` retain their IDs but are omitted while deriving/signing a ResultCommitment
- a stable `agent_id` is included in normal Agent serialization, but its creation derivation uses the separate `ID/AGENT` formula

Exclusion is a preimage rule, not a renumbering rule.


## WorkerAdvertisement — object type 0x0200

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

## StorageCommitment — object type 0x0205

```text
1   schema_version
2   commitment_id
3   service_node_id
4   operator_id
5   service_class
6   object_id
7   manifest_root
8   chunk_count
9   total_bytes
10  retained_from_block
11  retained_until_block
12  commitment_nonce
13  created_block
14  signature
```

## StorageChallenge — object type 0x0206

```text
1   schema_version
2   challenge_id
3   commitment_id
4   challenge_block_id
5   challenge_height
6   challenge_seed
7   requested_ranges
8   issued_at
9   response_deadline
10  challenger_id
11  signature
```

## StorageResponse — object type 0x0207

```text
1   schema_version
2   response_id
3   challenge_id
4   commitment_id
5   service_node_id
6   answered_at
7   range_proofs
8   response_bytes_hash
9   signature
```

## ServiceEpochReport — object type 0x0208

```text
1   schema_version
2   report_id
3   service_node_id
4   operator_id
5   epoch_start_height
6   epoch_end_height
7   commitments_sampled
8   challenges_passed
9   challenges_failed
10  deadlines_missed
11  verified_bytes_served
12  distinct_requester_count
13  distinct_challenge_block_count
14  service_classes
15  evidence_root
16  eligibility_weight
17  created_block
18  signature
```

## RelayReceipt — object type 0x0209

```text
1   schema_version
2   relay_receipt_id
3   object_type
4   object_id
5   sender_service_node_id
6   receiver_peer_id
7   received_at_bucket
8   transport_session_id
9   receiver_signature
```

## AvailabilityAttestation — object type 0x020A

```text
1   schema_version
2   attestation_id
3   service_node_id
4   service_class
5   requester_id
6   object_id
7   request_type
8   request_started_at
9   request_completed_at
10  bytes_verified
11  evidence_hash
12  requester_signature
```

## ChainObservation — object type 0x020B

```text
1   schema_version
2   network_id
3   observer_service_node_id
4   observation_sequence
5   observed_at
6   tip_block_id
7   height
8   cumulative_work
9   latest_timestamp
10  signature
```

## ReorgObservation — object type 0x020C

```text
1   schema_version
2   network_id
3   observer_service_node_id
4   observation_sequence
5   observed_at
6   old_tip
7   new_tip
8   fork_point
9   old_height
10  new_height
11  reorg_depth
12  old_cumulative_work
13  new_cumulative_work
14  signature
```

## SnapshotManifest — object type 0x020D

```text
1   schema_version
2   network_id
3   height
4   block_id
5   cumulative_work
6   state_root
7   execution_root
8   chunk_size
9   chunk_count
10  chunk_manifest_root
11  total_bytes
12  created_at
13  provider_service_node_id
14  signature
```


## ComputeChannel — object type 0x000E

```text
1   schema_version
2   channel_id
3   funding_account
4   channel_public_key
5   worker_id
6   operator_id
7   authorized_amount
8   service_scope
9   model_scope
10  execution_profile_scope
11  opened_height
12  expiry_height
13  settlement_policy
14  sequence
15  status
```

## PaymentAuthorization — object type 0x000F

```text
1   schema_version
2   authorization_id
3   channel_id
4   authorized_subject
5   service_type
6   model_scope
7   execution_profile_scope
8   max_per_job
9   max_total
10  max_jobs
11  valid_from
12  expires_at
13  delegation_depth
14  parent_authorization_id
15  nonce
16  signature
```

## NativeTransactionBody — object type 0x0010

```text
1   network_id
2   chain_id
3   nonce
4   action
5   target_payload
6   value
7   gas_limit
8   max_fee_per_gas
9   max_priority_fee_per_gas
10  data
```

## SignedNativeTransaction — object type 0x0011

```text
1   body
2   public_key
3   signature
```

For `SignedNativeTransaction` schema version 1, field `1 body` is a CBOR byte
string containing the complete canonical NCE/1 serialization of
`NativeTransactionBody` object type `0x0010`.

Those embedded bytes are exactly the unsigned-body bytes authenticated by the
transaction signature.

## StorageChallenge requested_ranges nested encoding

For `StorageChallenge` schema version 1, field `7 requested_ranges` is an array of four-element arrays:

```text
[ chunk_index, segment_index, offset, length ]
```

The positions are permanent within schema version 1. They are not map-field IDs and therefore do not consume global field-registry numbers.


## StorageResponse range_proofs nested encoding

For `StorageResponse` schema version 1, field `7 range_proofs` is an array of ten-element arrays:

```text
[
  chunk_index,
  segment_index,
  offset,
  length,
  returned_bytes,
  chunk_length,
  chunk_hash,
  range_root,
  range_proof,
  manifest_proof
]
```

Each `range_proof` and `manifest_proof` is an array of:

```text
[ sibling_bytes32, sibling_is_left_bool ]
```

These positions are permanent within schema version 1 and do not consume global field-registry numbers.


## ComputeSession — object type 0x0012

```text
1   schema_version
2   session_id
3   requester_identity
4   worker_id
5   operator_id
6   model_id
7   execution_profile_id
8   verification_policy_id
9   compute_channel_id
10  started_height
11  expiry_height
12  max_jobs
13  max_total_spend
14  max_idle_blocks
15  sequence
16  status
```


## ComputeUsageReceipt — object type 0x0013

```text
1   schema_version
2   receipt_id
3   channel_id
4   authorization_id
5   worker_id
6   operator_id
7   sequence
8   previous_receipt_id
9   cumulative_spent
10  job_id
11  result_commitment_id
12  price_offer_id
13  job_charge
14  metering_evidence_hash
15  expires_at
16  channel_signature
```
