# Protocol v1 Object Model

Protocol v1 defines a future-facing object model. Prototype implementations may support only subsets of fields and capability classes.

## Core objects

```text
ProtocolConfig
Agent
AgentVersion
Model
ExecutionProfile
Operator
ComputeWorker
ServiceNode
Job
VerificationPolicy
Capability
MemoryDescriptor
PaymentPlan
ResultCommitment
ReputationRecord
```

## Agent

Stable identity and governance metadata.

```text
Agent
- agent_id
- creator
- controller
- governance_mode
- current_version
- treasury_address
- status
- created_block
- metadata_uri
```

## AgentVersion

Immutable execution-critical snapshot.

```text
AgentVersion
- agent_id
- version
- definition_hash
- primary_models[]
- fallback_models[]
- execution_profiles[]
- verification_policy
- system_definition_hash
- tool_manifest_hash
- permission_manifest_hash
- memory_policy_hash
- economic_policy_hash
- trigger_policy_hash
- child_agent_policy_hash
- previous_version
- created_block
```

## Model

```text
Model
- model_id
- creator
- name
- architecture
- weights_hash
- tokenizer_hash
- config_hash
- license_hash
- manifest_uri
- supported_workloads[]
- supported_execution_profiles[]
- status
```

## ExecutionProfile

```text
ExecutionProfile
- profile_id
- runtime
- runtime_version
- hardware_requirements
- precision
- quantization
- context_limit
- determinism_mode
- batch_policy
- seed_policy
- supported_workloads[]
- input_format_version
- output_format_version
- profile_hash
```

## Operator

Represents common control above worker/service identities.

```text
Operator
- operator_id
- controller
- bond
- workers[]
- service_nodes[]
- reputation_ref
- created_block
```

## ComputeWorker

```text
ComputeWorker
- worker_id
- operator_id
- payment_address
- execution_profiles[]
- hardware_capabilities[]
- workload_types[]
- capacity
- queue_state
- models_hot[]
- models_warm[]
- bond
- reputation_ref
- availability_expiry
- advertisement_signature
```

## ServiceNode

```text
ServiceNode
- service_node_id
- operator_id
- services[]
- storage_capacity
- bandwidth_class
- stored_manifests[]
- availability_state
- bond
- reputation_ref
- payment_address
```

## Job

A generic job must not assume text-only inference.

```text
Job
- job_id
- requester
- agent_id
- agent_version
- model_id
- execution_profile
- workload_type
- input_manifest_hash
- output_requirements_hash
- verification_policy_id
- payment_plan_id
- resource_requirements
- privacy_requirements
- parent_job_id
- root_job_id
- created_block
- deadline_block
- status
- accepted_result
```

Candidate workload classes include text, vision, audio, embeddings, reranking, tool execution, training, fine-tuning, multi-agent, and custom workloads.

## VerificationPolicy

```text
VerificationPolicy
- policy_id
- type
- executor_count
- agreement_threshold
- challenge_window
- audit_probability
- bond_requirements
- dispute_policy
- hardware_diversity_rules
- operator_diversity_rules
```

Candidate policy types include NONE, REDUNDANT, OPTIMISTIC, AUDITED, TEE, ZK_PROOF, and CUSTOM.

## Capability

```text
Capability
- capability_id
- resource_type
- resource_identifier
- actions[]
- limits
- max_spend
- rate_limit
- expiry
- approval_mode
- delegate_allowed
```

## MemoryDescriptor

```text
MemoryDescriptor
- memory_id
- scope
- owner
- agent_id
- session_id
- root_hash
- storage_manifest
- encryption_policy
- write_policy
- replication_policy
- version
- updated_block
```

Memory scopes may include SESSION, USER, AGENT_GLOBAL, SHARED, PRIVATE, and ARCHIVE.

## PaymentPlan

```text
PaymentPlan
- payment_plan_id
- currency
- max_total
- escrow
- executor_budget
- verifier_budget
- storage_budget
- child_agent_budget
- creator_fee
- protocol_fee
- refund_policy
- settlement_policy
```

## ResultCommitment

```text
ResultCommitment
- job_id
- worker_id
- input_hash
- output_hash
- token_or_artifact_hash
- execution_profile
- runtime_receipt_hash
- storage_location
- signature
- submitted_block
```

## Versioning rule

Every serialization used to derive a persistent identifier or hash must include an explicit schema/version domain separator. Canonical serialization rules will be specified before these structures are treated as consensus- or contract-stable.
