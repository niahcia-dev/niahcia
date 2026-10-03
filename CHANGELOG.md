# Changelog

NIAHCIA is pre-alpha and has not published its first tagged binary release. This file tracks major development milestones rather than every internal commit.

## Unreleased

### Added
- native NIAHCIA transaction, execution, state, block-body, persistence, mining-RPC, and P2P foundations;
- P2P Version 3 full-body native block/transaction transport;
- deterministic NativeStateV1 snapshots/state roots and restart/reorg recovery;
- inactive NativeStateV2 with consensus-committed ComputeChannel state;
- inactive NativeTransaction V2 ComputeChannelOpen / ComputeChannelSettle / ComputeChannelRefund codecs, planners, and atomic application helpers;
- ComputeUsageReceipt V1 canonical encoding/signature verification and interoperability vectors;
- wallet-native encrypted chat / Agent architecture specifications;
- canonical release structure, release-note template, manifest format, and checksum policy.

### Changed
- removed the former Reth/EVM/Engine-API execution dependency from the active reference node;
- made CPU Proof-of-Work the sole consensus authority and kept AI/storage/service providers outside fork choice/finality;
- simplified the first AI milestone so ordinary inference uses wallet-local encrypted history/memory and does not require decentralized storage;
- moved active native block/transaction relay from historical P2P V2 to P2P V3.

### Security / correctness
- native block persistence is atomic across header, body, execution result, and state;
- compute Open/Settle/Refund helpers use checked integer accounting and clone-then-commit rollback behavior;
- terminal compute transitions reject stale/duplicate plans;
- unknown TOML configuration fields are rejected rather than silently accepting obsolete settings;
- release binaries, when introduced, must be accompanied by published SHA-256 checksums.

When the first tagged release is published, this section can be converted into a versioned entry:

```text
## [0.1.0] - YYYY-MM-DD
```
