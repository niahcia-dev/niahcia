# Proof of Service V1

## Status

Draft service-layer protocol.

## Goal

Reward measurable storage/network service without turning service nodes into consensus authorities.

## Proof levels

```text
POS-L1 Availability
  Can retrieve unpredictable committed data.

POS-L2 Retention
  Repeatedly proves unpredictable historical/random ranges
  throughout a promised retention period.

POS-L3 Service
  Reliably serves independent clients/challengers
  with acceptable response behavior.
```

## Core flow

```text
StorageCommitmentV1
        ↓
future NIAHCIA block entropy
        ↓
StorageChallengeV1
        ↓
StorageResponseV1
        ↓
verification
        ↓
ServiceEpochReportV1
        ↓
reward settlement
```

## Challenge selection

Challenge selection MUST depend on future chain data unavailable when the storage commitment was created.

Initial seed:

```text
keccak256(
  "NIAHCIA/STORAGE-CHALLENGE/V1"
  || challenge_block_id
  || commitment_id
)
```

The deterministic selector SHOULD choose both:

- one or more chunk indexes,
- one or more byte ranges inside each selected chunk.

This makes small challenge caches less useful.

## Evidence diversity

A service epoch SHOULD require evidence from more than one source class:

- deterministic protocol-derived challenges,
- independent peer challenges,
- real client retrievals.

No single friendly challenger should be able to manufacture full reward eligibility.

## Service-specific mapping

```text
MODEL_STORAGE
  requires POS-L1 + POS-L2

ARCHIVE
  requires POS-L1 + POS-L2
  plus historical-range coverage

SNAPSHOT
  requires POS-L1
  plus manifest/anchor verification

RELAY
  uses POS-L3-style receipt evidence
  with strict Sybil discounting

REORG_WATCH
  uses independently verifiable signed observations
  rather than storage challenges
```

## Reward accounting

Service rewards SHOULD be calculated per epoch, not per individual challenge.

The candidate service epoch is 720 blocks.

The exact economics and eligibility formula are intentionally separate from this proof format.

## On-chain / off-chain split

On-chain candidates:

- service identity,
- bond,
- compact commitment references,
- epoch summary commitments,
- reward settlement.

Off-chain:

- challenge transport,
- chunk/range bytes,
- Merkle proofs,
- relay receipts,
- large evidence sets,
- observation payloads.

## Non-authority

Proof-of-service affects service payment eligibility only.

It MUST NOT:

- add chain work,
- influence PoW target,
- vote on fork choice,
- finalize blocks,
- veto valid blocks,
- substitute for RandomX.


## Verified response to epoch accounting

The reference node connects a signed storage response to epoch accounting only after all of the following succeed:

1. the challenge signature verifies for the claimed challenger identity,
2. the response signature verifies for the claimed service-node identity,
3. the challenge height falls inside the active service epoch,
4. challenge and commitment IDs match,
5. the response meets the challenge deadline,
6. every requested segment is matched by the corresponding response proof,
7. range proofs authenticate returned bytes to the committed `range_root`,
8. manifest proofs authenticate each `range_root` and chunk tuple to the committed manifest root.

The accepted evidence key is:

```text
evidence_key =
  keccak256(
    "NIAHCIA/SERVICE-EVIDENCE/V1"
    || challenge_id
    || service_node_id
    || response_id
  )
```

Only after that key passes duplicate-replay rejection may the epoch accumulator increment successful challenges, verified bytes, requester diversity, and challenge-block diversity.

This accounting path remains service-layer state. It does not add chain work or participate in fork choice.
