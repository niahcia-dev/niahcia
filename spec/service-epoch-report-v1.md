# Service Epoch Report V1

## Status

Draft reward-accounting object.

Service rewards are settled from verified service evidence aggregated over epochs rather than from self-reported uptime.

## Candidate epoch length

```text
SERVICE_EPOCH_BLOCKS = 720
```

At a 30-second target interval this is approximately six hours.

This value is a development candidate, not frozen consensus.

## Canonical fields

```text
ServiceEpochReportV1
- schema_version
- report_id
- service_node_id
- operator_id
- epoch_start_height
- epoch_end_height
- commitments_sampled
- challenges_passed
- challenges_failed
- deadlines_missed
- verified_bytes_served
- distinct_requester_count
- distinct_challenge_block_count
- service_classes
- evidence_root
- eligibility_weight
- created_block
- signature
```

## Principles

- reward accounting MUST be based on verifiable evidence,
- repeated challenges from one friendly identity MUST NOT alone create full eligibility,
- evidence diversity matters,
- collateral or bond ownership alone earns no service reward,
- reputation is advisory and MUST NOT alter chain consensus.

## Evidence root

`evidence_root` commits to the ordered set of challenge/response/retrieval evidence used to calculate the report.

## Eligibility weight

The exact weighting formula is not frozen.

It SHOULD consider:

- challenge success,
- missed deadlines,
- bytes verifiably served,
- requester diversity,
- challenge-block diversity,
- service-class-specific quality metrics.

It MUST NOT include any consensus voting power.


## Replay and diversity accounting

An implementation maintaining an epoch report SHOULD keep a set of unique evidence keys.

Initial replay key:

```text
keccak256(
  "NIAHCIA/SERVICE-EVIDENCE/V1"
  || challenge_id
  || service_node_id
  || response_hash
)
```

The same evidence key MUST NOT be counted twice in one epoch accumulator.

The accumulator SHOULD separately track:

- successful challenges,
- failed challenges,
- missed deadlines,
- total verified bytes served,
- distinct requester identities,
- distinct challenge block IDs,
- total unique evidence records.

These raw counters are evidence-accounting inputs only. The final eligibility-weight formula remains unfrozen.

Persistent storage of replay keys across process restart is required before service rewards are enabled on a public network.


## Conservative development eligibility rule

The first reference-node eligibility rule is intentionally simple and is **not frozen economics**.

A service node is ineligible for an epoch unless all configured minimums are satisfied:

- minimum successful challenge count,
- minimum distinct requester count,
- minimum distinct challenge-block count,
- successful challenges are not outnumbered by failures,
- no missed response deadlines.

When eligible, the development weight is:

```text
max(verified_bytes_served, challenges_passed)
```

This deliberately omits:

- reputation multipliers,
- collateral multipliers,
- operator prestige,
- uptime self-reporting,
- consensus influence.

The rule exists only so the end-to-end accounting path can be tested before economics are designed and frozen.


## Deterministic evidence root

`evidence_root` commits to the unique evidence keys counted in the epoch.

Before tree construction, evidence keys are sorted lexicographically by their 32-byte value.

Leaf:

```text
keccak256(
  "NIAHCIA/SERVICE-EVIDENCE-LEAF/V1"
  || evidence_key
)
```

Internal node:

```text
keccak256(
  "NIAHCIA/SERVICE-EVIDENCE-NODE/V1"
  || left
  || right
)
```

If a tree level has an odd final node, that node is duplicated.

Empty root:

```text
keccak256(
  "NIAHCIA/SERVICE-EVIDENCE-EMPTY/V1"
)
```

Sorting makes the root independent of local evidence-arrival order while duplicate-replay rejection keeps the committed set unique.


## Finalization

The reference node may finalize an in-memory epoch accumulator into a deterministic `FinalizedServiceEpochReport` prior to canonical serialization/signing.

The finalized report binds:

- service_node_id,
- epoch_start_height,
- epoch_end_height,
- challenges_passed,
- challenges_failed,
- deadlines_missed,
- verified_bytes_served,
- distinct_requester_count,
- distinct_challenge_block_count,
- evidence_root,
- eligibility result,
- eligibility_weight.

Finalization does not itself pay rewards and does not make the report consensus-authoritative.

The signed/canonical `ServiceEpochReportV1` object may carry the same accounting values plus protocol metadata such as report ID, operator ID, service classes, created block, and signature.


## Canonical NCE/1 encoding

`ServiceEpochReportV1` uses object type `0x0208` and schema version `1`.

Payload field IDs are the permanent assignments in `field-id-registry.md`.

`service_classes` is an unordered set of protocol-defined ASCII service identifiers such as `ARCHIVE` and `MODEL_STORAGE`. Before NCE/1 array encoding, each text value is canonically encoded, sorted by raw encoded bytes, and duplicates are removed.

`verified_bytes_served` and `eligibility_weight` are unsigned 64-bit values in v1. This avoids CBOR bignum tags, which NCE/1 forbids.

### Report ID

`report_id` is content-derived. Its preimage excludes fields `2 report_id` and `18 signature`, but retains the normal NCE/1 ServiceEpochReport envelope.

```text
report_id =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "ID/SERVICE_EPOCH_REPORT" || 0x00 ||
    network_id || 0x00 ||
    nce1_report_without_report_id_or_signature
  )
```

### Signing digest

The signing preimage includes `report_id` and excludes only field `18 signature`.

```text
signing_digest =
  keccak256(
    "NIAHCIA" || 0x00 ||
    "SIGN/SERVICE_EPOCH_REPORT" || 0x00 ||
    network_id || 0x00 ||
    nce1_report_without_signature
  )
```

The signature therefore commits to the report ID, all accounting fields, evidence root, service classes, created block, and network.

### Network replay protection

The same canonical report produces different report/signing digests on different networks because `network_id` is outside and directly bound into the domain-separated digest.


## secp256k1 signature encoding

The reference implementation signs the 32-byte `SIGN/SERVICE_EPOCH_REPORT` digest using secp256k1 ECDSA.

The wire signature is the fixed-width 64-byte compact form:

```text
r[32] || s[32]
```

DER encoding is not used for `ServiceEpochReportV1`.

Verification MUST:

- recompute and validate `report_id` before signature verification,
- recompute the network-bound signing digest,
- parse the service-node public key as SEC1 secp256k1 bytes,
- require exactly 64 signature bytes,
- reject signatures that do not verify for the supplied public key.

The public-key-to-`service_node_id` binding is a separate service-identity rule and must be defined before public reward settlement is enabled.


## Service-node identity binding

`service_node_id` is bound to the node's secp256k1 public key.

The public key is first normalized to compressed SEC1 form, then:

```text
service_node_id =
  keccak256(
    "NIAHCIA/SERVICE-NODE-ID/V1"
    || compressed_secp256k1_public_key
  )
```

Signature verification MUST reject a report when the supplied public key does not derive the report's `service_node_id`.

This binds authorship to the service identity without granting that identity any consensus privilege.


## Locked interoperability vector

The current cross-implementation compatibility vector is `test-vectors/service-epoch-report-v1.json`. Implementations SHOULD reproduce its compressed public key, `service_node_id`, NCE/1 ID preimage, `report_id`, signing preimage, signing digest, compact secp256k1 signature, and final canonical signed bytes exactly.
