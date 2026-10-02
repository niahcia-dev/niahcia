# Object Type Registry

## Status

Draft.

NCE/1 top-level envelopes include a permanent numeric `object_type`.

## Initial assignments

```text
0x0001  Agent
0x0002  AgentVersion
0x0003  Model
0x0004  ExecutionProfile
0x0005  Operator
0x0006  ComputeWorker
0x0007  ServiceNode
0x0008  Job
0x0009  VerificationPolicy
0x000A  Capability
0x000B  MemoryDescriptor
0x000C  PaymentPlan
0x000D  ResultCommitment
0x000E  ComputeChannel
0x000F  PaymentAuthorization
0x0010  NativeTransactionBody
0x0011  SignedNativeTransaction

0x0100  ModelManifest
0x0101  MemoryManifest
0x0102  ArtifactManifest
0x0103  DatasetManifest

0x0200  WorkerAdvertisement
0x0201  ServiceAdvertisement
0x0202  JobAcceptance
0x0203  ResultReveal
0x0204  VerificationReceipt
0x0205  StorageCommitment
0x0206  StorageChallenge
0x0207  StorageResponse
0x0208  ServiceEpochReport
0x0209  RelayReceipt
0x020A  AvailabilityAttestation
0x020B  ChainObservation
0x020C  ReorgObservation
0x020D  SnapshotManifest
```

## Allocation policy

- numeric values are never reused
- removed/deprecated object codes remain reserved
- experimental/local types use a separately defined experimental range
- additions require protocol documentation
- changing a schema does not change `object_type`; it increments that object's `schema_version`

## Encoding

`object_type` is encoded as an unsigned integer in NCE/1.

The numeric registry is authoritative; human-readable names are descriptive.
