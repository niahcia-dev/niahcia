# ServiceNode Security Profile V1

## Status

Draft protocol candidate.

This profile defines security-support roles for NIAHCIA service/storage nodes without granting them consensus authority.

The governing rule is:

> **Service nodes may observe, preserve, relay, and prove. They do not choose the canonical chain.**

Canonical-chain selection remains entirely under NIAHCIA proof-of-work consensus and cumulative-work fork choice.

## Security-support service classes

Initial security-oriented service vocabulary:

```text
ARCHIVE
SNAPSHOT
RELAY
REORG_WATCH
MODEL_STORAGE
AGENT_STORAGE
ARTIFACT_STORAGE
DATASET_STORAGE
INDEXING
VERIFICATION
```

A service node may advertise multiple classes.

### ARCHIVE

Stores historical NIAHCIA block/header data and selected execution artifacts.

Expected capabilities:

- historical header retrieval,
- full block retrieval where retained,
- stale/orphan branch retention,
- RandomX seed-source block retrieval,
- Merkle proof support,
- reorg evidence preservation.

### SNAPSHOT

Serves cryptographically committed state snapshots.

A snapshot provider does not make the snapshot trusted.

Clients MUST verify the snapshot manifest and its anchor against NIAHCIA consensus state.

### RELAY

Provides well-connected block/header relay service.

Relay status grants no authority to alter, suppress, prioritize, or validate consensus beyond what ordinary node rules permit.

### REORG_WATCH

Independently tracks competing branches and records reorganization evidence.

A reorg watcher may report:

```text
old_tip
new_tip
fork_point
old_height
new_height
reorg_depth
old_cumulative_work
new_cumulative_work
first_observed_at
published_at
```

These observations are advisory evidence only.

They MUST NOT override valid cumulative-work fork choice.

## Reorg observations

A service node MAY sign a `ReorgObservationV1` containing observer identity, observed time, old/new tips, fork point, heights, reorg depth, cumulative-work values, sequence, and signature.

The signature proves only that the named service node made the observation. It does not prove that the observation is correct.

Clients MUST verify all referenced chain data independently.

## Chain-tip observations

A service node MAY periodically sign `ChainObservationV1` records containing its observed tip, height, cumulative work, latest timestamp, observation sequence, and signature.

These records can help detect eclipse conditions, network partitions, sudden private-chain publication, abnormal cumulative-work jumps, and inconsistent views across independent peers.

They are never votes.

## Snapshot manifests

A snapshot service MAY publish a `SnapshotManifestV1` containing height, block ID, cumulative work, state root, execution root, chunk commitment, total bytes, provider identity, and signature.

Every chunk MUST be content-addressed.

A client MUST verify:

1. manifest signature,
2. chunk hashes,
3. manifest root,
4. referenced block ID,
5. cumulative-work ancestry,
6. state/execution commitment consistency.

A snapshot is a transport optimization, not a trust shortcut.

## Stale and competing branch retention

Security-oriented archive nodes SHOULD retain recent non-canonical branches.

Initial operational target:

```text
recent stale/orphan branches: 30 days
deep reorg evidence:          indefinite where practical
canonical headers:            permanent
```

These are service-policy targets, not base consensus validity rules.

## Availability evidence

Rewards for storage/security services SHOULD be based on measurable service.

Candidate evidence includes successful random retrieval challenges, proof that named content chunks are retrievable, historical-block retrieval success, snapshot chunk serving, signed availability windows, and independent challenge responses.

Collateral or bond ownership alone MUST NOT earn service rewards.

## Anti-eclipse use

A full node MAY use service nodes as additional independently selected peers.

A node SHOULD compare views from ordinary P2P peers, archival peers, relay peers, and independently selected reorg-watch nodes.

A disagreement is a warning signal. The local node still validates all headers, PoW, targets, and cumulative work itself.

## Explicit non-authority rules

Service nodes MUST NOT gain consensus privilege from service-node status.

Specifically, service nodes do not:

- vote on canonical chain selection,
- veto a heavier valid PoW chain,
- approve or reject blocks by quorum,
- finalize checkpoints,
- replace cumulative-work fork choice,
- determine RandomX validity,
- determine the required target,
- control transaction validity,
- control EVM execution validity.

No threshold such as "60% of service nodes agree" may be used as a consensus acceptance rule.

## Security against service-node capture

Clients SHOULD diversify observations by operator identity, network path, ASN/hosting concentration where observable, geographic region where volunteered, software implementation, and service class.

Multiple service-node identities controlled by one operator MUST NOT be treated as independent evidence merely because their node IDs differ.

## Rewards and penalties

Service rewards should pay for objectively measurable service delivery.

Potential penalties may apply to provably false commitments or failed bonded availability obligations.

Reputation alone MUST NOT determine chain validity.

## Protocol boundary

```text
CPU miners / PoW
    -> consensus authority

service nodes
    -> availability
    -> archival
    -> relay diversity
    -> observation
    -> evidence
    -> snapshots

clients/full nodes
    -> independently verify all consensus claims
```

This separation is intentional and fundamental.


## Proof-of-service protocol

Storage and archive reward eligibility is defined by `proof-of-service-v1.md`.

The initial measurable flow is:

```text
StorageCommitmentV1
        ↓
StorageChallengeV1
        ↓
StorageResponseV1
        ↓
verified evidence
        ↓
ServiceEpochReportV1
```

The candidate service epoch is 720 blocks, approximately six hours at the 30-second target interval.

Proof-of-service is explicitly payment evidence only. It adds no chain work and confers no fork-choice authority.
