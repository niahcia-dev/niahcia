# Service persistence V1

Status: implementation contract for the reference node. This is service-layer accounting state, not consensus authority.

## Purpose

Proof-of-service accounting must survive process restart without allowing previously observed evidence to be counted again. The service state remains in the NIAHCIA-owned `redb` database and is isolated from PoW fork choice.

The existing implementation already persists successful verified evidence in `service_evidence_v1` and reconstructs successful epoch accounting after restart. V1 completes that model by persisting every terminal challenge outcome and finalized epoch reports.

## Tables

### `service_evidence_v1`

Key: 32-byte evidence replay key.

V1 values are tagged records so successful, failed, and deadline-missed outcomes share one replay namespace.

```text
version             u8 = 1
outcome             u8 = 0 success | 1 failure | 2 deadline_missed
epoch_start_height  u64 BE
epoch_end_height    u64 BE
requester_id        [32]   (zero only when genuinely unavailable)
challenge_block_id  [32]   (zero only when genuinely unavailable)
verified_bytes      u64 BE (zero for unsuccessful outcomes)
```

The replay key remains the table key. A key may be inserted exactly once. A later attempt to store a different outcome for the same key is rejected rather than overwritten.

Existing pre-tag successful records must remain readable during the development migration window. New writes use the tagged V1 form.

### `service_epochs_v1`

Key:

```text
service_node_id [32] || epoch_start_height u64 BE || epoch_end_height u64 BE
```

Value: canonical finalized `ServiceEpochReportV1` bytes.

A finalized epoch is immutable. Re-inserting byte-identical data is idempotent; attempting to replace the same epoch key with different report bytes is rejected.

## Reconstruction

Before finalization, restart reconstruction scans terminal evidence for the requested epoch and rebuilds `ServiceEpochAccumulator` using the same `record_success` / `record_failure` rules used in memory.

This must restore:

- challenges passed;
- challenges failed;
- deadlines missed;
- verified bytes served;
- requester diversity;
- challenge-block diversity;
- the complete evidence replay set;
- deterministic evidence root.

A restart must therefore produce the same eligibility result and evidence root as the uninterrupted process.

## Finalization

Finalization order is deliberately one-way:

```text
verified terminal evidence
        -> persisted replay record
        -> reconstructed/current accumulator
        -> FinalizedServiceEpochReport
        -> canonical ServiceEpochReportV1
        -> signature verification
        -> immutable service_epochs_v1 record
```

The finalized report must not be persisted until its service-node identity, report ID, network binding, and signature have been verified.

Finalized reports are accounting artifacts. They do not add chain work, vote on canonical chain selection, veto PoW blocks, or create checkpoint finality.

## Atomicity and crash behavior

Evidence persistence occurs before the in-memory accumulator accepts the event. If the process dies after the database commit but before the in-memory update, restart reconstruction recovers the event exactly once.

If persistence fails, the accumulator must not count the event.

Finalized report persistence is independent of chain-head metadata and must never modify `best_head` or cumulative-work state.

## Required tests

1. success survives restart and cannot replay;
2. ordinary failure survives restart and cannot replay;
3. deadline miss survives restart and cannot replay;
4. mixed outcomes reconstruct identical counters and evidence root;
5. requester and challenge-block diversity survive restart;
6. eligibility before and after restart is identical;
7. finalized report survives restart byte-for-byte;
8. byte-identical report insertion is idempotent;
9. conflicting finalized report for the same node/epoch is rejected;
10. service persistence never changes chain best-head/fork-choice state.

## Follow-on transport boundary

RPC/P2P service challenge transport may submit evidence to this persistence layer only after the existing cryptographic and Merkle-proof verification succeeds. Transport code must not implement a second replay database or a second epoch accumulator.