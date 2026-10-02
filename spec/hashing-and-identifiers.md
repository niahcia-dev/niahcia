# Hashing and Identifier Derivation

## Status

Draft — Protocol v1 foundation.

This specification defines NIAHCIA protocol digests and permanent identifier derivation.

## Protocol hash

Protocol v1 uses:

```text
Keccak-256
```

for protocol object digests and identifiers that must interoperate directly with EVM contracts.

Reasoning:

- native EVM `keccak256`
- 32-byte output
- mature cross-language implementations
- efficient on-chain verification
- avoids requiring contracts to implement a non-native hash

This choice does not require every storage subsystem to use Keccak-256. Storage manifests MAY support multihash/content-addressing schemes, but protocol identity anchors use Keccak-256 unless another specification explicitly says otherwise.

## Domain-separated digest

No NIAHCIA protocol digest is computed over raw payload bytes alone.

The generic v1 digest is:

```text
digest =
  keccak256(
    "NIAHCIA" ||
    0x00 ||
    purpose ||
    0x00 ||
    network_id ||
    0x00 ||
    canonical_bytes
  )
```

Where:

- `"NIAHCIA"` is ASCII
- separators are the single byte `0x00`
- `purpose` is the assigned ASCII domain string
- `network_id` is the canonical network identifier bytes
- `canonical_bytes` are NCE/1 bytes

The purpose string prevents one valid digest/signature class from being reused as another.

## Initial purpose domains

```text
OBJECT/AGENT
OBJECT/AGENT_VERSION
OBJECT/MODEL
OBJECT/EXECUTION_PROFILE
OBJECT/OPERATOR
OBJECT/COMPUTE_WORKER
OBJECT/SERVICE_NODE
OBJECT/JOB
OBJECT/VERIFICATION_POLICY
OBJECT/CAPABILITY
OBJECT/MEMORY_DESCRIPTOR
OBJECT/PAYMENT_PLAN
OBJECT/RESULT_COMMITMENT

SIGN/AGENT_CONTROL
SIGN/WORKER_ADVERTISEMENT
SIGN/SERVICE_ADVERTISEMENT
SIGN/JOB_REQUEST
SIGN/JOB_ACCEPT
SIGN/RESULT_COMMITMENT
SIGN/CAPABILITY
SIGN/MEMORY_UPDATE
SIGN/P2P_MESSAGE
```

Purpose strings are protocol constants and are case-sensitive.

## Network ID

Every signature and mutable-network object digest MUST be bound to a `network_id`.

Initial conceptual values:

```text
mainnet
testnet/<name>
devnet/<name>
```

The final byte representation and chain ID mapping will be fixed in the network-parameters specification.

Binding signatures to a network prevents a valid testnet authorization from being replayed onto mainnet.

## Identifier classes

NIAHCIA distinguishes two forms of identifiers.

### Content-derived IDs

Used when identity should change whenever canonical immutable content changes.

Examples:

- Model ID
- ExecutionProfile ID
- VerificationPolicy ID
- PaymentPlan ID
- ResultCommitment ID
- immutable AgentVersion definition hash

Derivation:

```text
id = domainSeparatedDigest(canonical_object_without_id_or_signature)
```

### Stable allocated IDs

Used when an identity must survive version changes.

Examples:

- Agent ID
- Operator ID
- ComputeWorker ID
- ServiceNode ID
- Job ID
- Memory ID

Stable IDs are created from explicit creation context rather than mutable object content.

Generic stable derivation:

```text
stable_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    id_domain || 0x00 ||
    network_id || 0x00 ||
    creator_address || 0x00 ||
    creation_nonce
  )
```

The exact nonce source is object-specific.

## EVM-address-created objects

Where an object is created by an EVM account/contract, the recommended creation nonce is a monotonic registry nonce maintained by the authoritative registry contract.

Example:

```text
agent_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/AGENT" || 0x00 ||
    network_id || 0x00 ||
    creator_address || 0x00 ||
    registry_nonce
  )
```

This creates stable non-guess-dependent IDs without making the agent definition itself mutable identity.

## Job IDs

Jobs require uniqueness under high concurrency.

Prototype v1 derivation:

```text
job_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/JOB" || 0x00 ||
    network_id || 0x00 ||
    requester_address || 0x00 ||
    requester_job_nonce
  )
```

The requester nonce is monotonic for on-chain-created jobs.

Off-chain session jobs that later settle on-chain MUST use an equivalent collision-resistant session nonce scheme specified by the session protocol.

## Human-readable form

Canonical IDs are 32-byte binary values.

Default display form:

```text
0x<64 lowercase hex characters>
```

Applications MAY use shortened presentation forms, but shortened IDs MUST NOT be accepted where an exact protocol identifier is required.

## Hash agility

Protocol v1 fixes Keccak-256 for the object classes in this specification.

Future hash algorithms require:

- a new encoding/hash suite version
- explicit object/schema upgrade rules
- no silent reinterpretation of old IDs

Hash agility is achieved by versioning, not by adding an algorithm field to every v1 object.
