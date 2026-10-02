# ServiceNode

## Purpose

`ServiceNode` represents a bonded provider of persistent network services.

Storage is the initial service, not the permanent limit.

## Canonical fields

```text
ServiceNode
- schema_version
- service_node_id
- operator_id
- controller
- payment_address
- services[]
- storage_capacity
- bandwidth_class
- stored_manifests[]
- availability_state
- endpoint_descriptor
- pricing_policy
- bond
- reputation_ref
- status
```

## Service classes

Initial/future vocabulary:

```text
MODEL_STORAGE
MEMORY_STORAGE
AGENT_STORAGE
ARTIFACT_STORAGE
DATASET_STORAGE
ROUTING
RELAY
INDEXING
ARCHIVE
SNAPSHOT
REORG_WATCH
VERIFICATION
```

A node may advertise several services.

## Storage integrity

Storage services MUST be content-addressed. Retrieval clients verify received content against registered manifests/hashes.

## Availability

Service rewards SHOULD depend on measurable activity such as:

- successful retrievals
- storage availability challenges
- replication commitments
- routing/serving work

Collateral ownership alone MUST NOT entitle a node to service rewards.

## Status

```text
ACTIVE
DRAINING
SUSPENDED
EXITED
```

## Invariants

1. Service nodes do not participate in PoW fork choice by virtue of service-node status.
2. Stored content MUST be independently integrity-verifiable.
3. Service claims MUST be explicit and auditable.
4. Payment for one service class MUST NOT imply entitlement to another.

## Prototype 0

Prototype 0 enables MODEL_STORAGE, model manifest/chunk serving, basic availability tracking, and test retrieval accounting.

## Security support boundary

Service nodes may strengthen availability, archival durability, relay diversity, anti-eclipse observation, snapshot distribution, and reorg forensics.

They do not gain consensus authority from those services.

The detailed security profile is defined in `service-node-security-v1.md`.

Signed chain/reorg observations are defined in `chain-observation-v1.md`.

Verifiable state snapshots are defined in `snapshot-manifest-v1.md`.

A service-node signature proves authorship of an observation or manifest, not correctness of chain selection.


## Identity derivation

The initial service-node identity is derived from a secp256k1 public key using `NIAHCIA/SERVICE-NODE-ID/V1`.

Clients MUST verify that signed service objects are produced by the public key corresponding to the claimed `service_node_id`.

Service-node identity proves authorship only. It does not add chain work, voting power, or fork-choice authority.
