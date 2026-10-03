# Native Execution Transition

Status: **completed for the active reference-node execution path**.

NIAHCIA replaced the former external execution-client subsystem with native NIAHCIA execution.

## Preserved boundaries

- 164-byte `BlockHeaderV1`;
- block/mining identity domains;
- transaction and execution header commitments;
- CPU-PoW cumulative-work fork choice;
- ancestry/reorganization machinery.

## Removed subsystem

The active node no longer depends on external execution payloads, external execution-hash mappings, replay journals for an external engine, Engine API, JWT execution-client authentication, or Reth/EVM execution configuration.

## Native replacement

The active node owns deterministic native transaction execution, native state, receipts/commitments, atomic block/body/execution/state persistence, mining integration, restart recovery, and P2P validation.

NativeStateV2 / NativeTransaction V2 compute settlement is a separate future activation boundary and does not resurrect the old external execution architecture.
