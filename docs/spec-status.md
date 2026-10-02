# NIAHCIA Specification Status Audit

Last reviewed: 2026-09-30

This is the living whole-protocol audit against current design decisions, reference-implementation evidence, and interoperability requirements. It does not itself promote a candidate to normative consensus.

Status vocabulary:

- **LOCKED** — normative behavior for the stated protocol version.
- **CANDIDATE** — intended/proposed behavior still awaiting review, vectors, testing, activation, or another explicit freeze criterion.
- **DESCRIPTIVE** — architecture/rationale; not independent consensus law.
- **IMPLEMENTED (DEV)** — present in the reference implementation; implementation alone does not make it normative.
- **VECTORED** — canonical machine-readable interoperability vectors exist.
- **REVIEW REQUIRED** — a known conflict/open question prevents freeze.
- **SUPERSEDED** — retained only for history/rationale and not current direction.

## Whole-repository inventory

Every current protocol surface is assigned below. A filename's presence does not imply that its behavior is locked.

### Chain, consensus and mining

| Surface | Status | Finding |
| --- | --- | --- |
| `block-header-v1.md` | CANDIDATE + VECTORED + REVIEW REQUIRED | 164-byte header exists in dev implementation/vector, but current PoW-preimage/nonce layout conflicts with the stock-XMRig interoperability objective. |
| `randomx-pow-v1.md` | CANDIDATE + VECTORED + REVIEW REQUIRED | Standard RandomX is retained. Current statement that RandomX hashes the exact 164-byte header must not freeze until stock-XMRig-compatible preimage work is proven. |
| `difficulty-adjustment-v1.md` | SUPERSEDED candidate + VECTORED | Earlier normalized-window controller; retained for history/comparison, not leading V1 direction. |
| `difficulty-asert-v1.md` | CANDIDATE + VECTORED + REVIEW REQUIRED | Leading controller candidate. Timestamp-adversary result blocks freeze. |
| `timestamp-rules-v1.md` | CANDIDATE + REVIEW REQUIRED | Must be reviewed jointly with ASERT; timestamp policy cannot be frozen independently of the known adversarial interaction. |
| `network-parameters-v1.md` | CANDIDATE | Framework exists; production genesis, PoW limit, epoch/seed, activation and concrete network parameters remain unfrozen. |
| `chain-state-v1.md` | CANDIDATE | Chain-state commitment/state definition needs final consensus integration review. |
| `chain-observation-v1.md` | CANDIDATE / NON-AUTHORITY | Observation evidence is advisory; must never become fork-choice voting. |
| `transaction-merkle-v1.md` | CANDIDATE | Commitment format requires implementation/vector alignment before freeze. |

### Native value, identity and transaction primitives

| Surface | Status | Finding |
| --- | --- | --- |
| `monetary-and-fee-primitives-v1.md` | **LOCKED representation primitives** | 8 decimals, aniah, native network IDs, native chain IDs and checked fee arithmetic are locked. Issuance/fee disposition are not. |
| `address-v1.md` | **LOCKED** | Bech32m, 22-byte V1 payload, Account/Contract kinds and `niah`/`tniah`/`dniah` HRPs synchronized from reference implementation docs. |
| `canonical-serialization.md` | CANDIDATE FOUNDATION | NCE/1 deterministic-CBOR design is coherent but its own document requires vectors before freeze. |
| `domain-separation.md` | CANDIDATE FOUNDATION | Must remain synchronized with every ID/signing domain and vector. |
| `hashing-and-identifiers.md` | CANDIDATE FOUNDATION | Requires complete cross-object vector coverage before global freeze. |
| `signatures.md` | CANDIDATE FOUNDATION | Signature algorithms/encodings are interoperability-critical and require complete vector coverage. |
| `object-type-registry.md` | CANDIDATE REGISTRY | Numeric assignments must become immutable when dependent V1 vectors/specs freeze. |
| `field-id-registry.md` | CANDIDATE REGISTRY | Permanent field IDs are already consumed by storage vectors; changes require careful versioning. |

### AI/agent object family

| Surface | Status | Finding |
| --- | --- | --- |
| `object-model.md` | DESCRIPTIVE/CANDIDATE umbrella | Useful map, but not every listed field is frozen. It must not override the individual object specs. |
| `agent.md` | CANDIDATE | Stable identity/governance model remains future-facing. |
| `agent-version.md` | CANDIDATE | Immutable version concept retained; full upgrade/governance semantics still require lock. |
| `model.md` | CANDIDATE | Content/model identity is future-facing; storage/distribution must remain content-addressed and decentralized. |
| `execution-profile.md` | CANDIDATE | Correct place for runtime/version/hardware/determinism semantics; avoids hard-wiring vLLM forever. |
| `operator.md` | CANDIDATE | Operator identity should expose common control without granting cross-subsystem authority. |
| `compute-worker.md` | CANDIDATE | Worker identity/capabilities remain separate from miner consensus authority. |
| `capability.md` | CANDIDATE | Permission/delegation/revocation semantics need further security review. |
| `memory-descriptor.md` | CANDIDATE | Memory descriptor is metadata/commitment architecture; privacy/encryption/access semantics remain open. |
| `payment-plan.md` | CANDIDATE | Flexible budget buckets retained; no global hard-coded percentage split. |
| `result-commitment.md` | CANDIDATE | Canonical result/evidence binding remains required; must align with final verification design. |
| `job.md` | CANDIDATE, UPDATED | Generic workload/state model retained. Fixed 2-of-3 verification assumption removed as superseded. |
| `verification-policy.md` | CANDIDATE + REVIEW REQUIRED, UPDATED | Policy framework retained; fixed network-wide Prototype-0 2-of-3 rule explicitly superseded. Replacement verification design is not yet locked. |

### Service/storage/proof-of-service family

| Surface | Status | Finding |
| --- | --- | --- |
| `service-node.md` | CANDIDATE | Service identity/capabilities do not create chain authority. |
| `service-node-security-v1.md` | CANDIDATE | Correctly preserves the fundamental non-authority boundary: observe/preserve/relay/prove, never choose canonical chain. |
| `proof-of-service-v1.md` | CANDIDATE | Payment-evidence protocol only; no chain work/fork-choice authority. |
| `storage-commitment-v1.md` | CANDIDATE + VECTORED | Locked interoperability fixture exists, but the document still calls the object draft; freeze status should wait for full flow review. |
| `storage-challenge-v1.md` | CANDIDATE + VECTORED | Canonical challenge work is advanced; review with response and epoch-report semantics as one state machine. |
| `storage-response-v1.md` | CANDIDATE + VECTORED | Canonical response work is advanced; verification semantics must remain deterministic. |
| `service-epoch-report-v1.md` | CANDIDATE + VECTORED | Reward/accounting evidence is advanced but economic activation remains separate. |
| `storage-manifest-merkle-v1.md` | CANDIDATE | Manifest commitment primitive; requires final integration/vector review. |
| `storage-range-merkle-v1.md` | CANDIDATE | Range proof primitive; requires final integration/vector review. |
| `availability-attestation-v1.md` | CANDIDATE | Attestation authenticates a claim, not truth/service by itself. |
| `relay-receipt-v1.md` | CANDIDATE | Receipt authenticates evidence and must not imply consensus privilege. |
| `snapshot-manifest-v1.md` | CANDIDATE | Snapshot is transport/bootstrap optimization; clients still verify against PoW consensus. |

### NIPs, research, architecture and tools

| Surface | Status | Finding |
| --- | --- | --- |
| `docs/protocol-architecture-v1.md` | DESCRIPTIVE authority map | Current top-level architecture map. Individual specs control normative behavior. |
| `docs/architecture.md` | DESCRIPTIVE / REVIEW FOR DUPLICATION | Older architecture material should defer to the consolidated architecture map where overlapping. |
| `docs/design-doctrine.md` | DESCRIPTIVE | Project doctrine; must not create hidden consensus rules. |
| `docs/goals.md` | DESCRIPTIVE | Product/protocol objectives, not wire law. |
| `docs/economics.md` | DESCRIPTIVE/CANDIDATE economics | Correctly separates CPU, compute, service and agent economies; mainnet policy remains open. |
| `docs/prototype-0.md` | DEVELOPMENT PLAN / REVIEW REQUIRED | Must be checked for stale fixed 2-of-3 assumptions and other superseded prototype choices. |
| difficulty/botnet/reorg research docs | RESEARCH | Evidence/rationale only; simulation output is not consensus law. |
| `docs/threat-model.md` | DESCRIPTIVE SECURITY | Living threat model; update as protocol surfaces change. |
| `docs/repository-family.md` | DESCRIPTIVE | Repository responsibility map. |
| `NIP-0001.md`, `NIP-0002.md` | PROPOSALS | NIPs do not become normative merely by existing; status/acceptance must be explicit. |
| `tools/difficulty_sim.py`, `tools/reorg_sim.py` | RESEARCH TOOLS | Useful reproducibility tools, not consensus implementations. |

## Vector coverage audit

Current machine-readable vector files cover:

- BlockHeader V1;
- core V1 object/serialization fixtures;
- old difficulty-adjustment candidate;
- ASERT candidate;
- RandomX PoW candidate;
- StorageCommitmentV1;
- StorageChallengeV1;
- StorageResponseV1;
- ServiceEpochReportV1;
- botnet/reorg research outputs.

Important missing/insufficient standalone vector coverage includes the newly locked Address V1 and monetary/fee primitives, plus complete per-object vectors for the broader Agent/Job/Verification/Capability/Payment family. Address fixtures currently live in Rust tests and should be copied into protocol-owned machine-readable vectors.

## Confirmed stale/conflicting assumptions

1. **Fixed 2-of-3 AI verification is superseded.** `verification-policy.md` and `job.md` have been corrected. Any remaining prototype/research references must be labeled historical/development-only rather than current architecture.
2. **PoW input/header layout is not frozen.** Existing BlockHeader/RandomX vectors remain valuable development fixtures but cannot be advertised as final while stock-XMRig compatibility is unresolved.
3. **Old normalized-window difficulty is superseded by the ASERT direction**, while ASERT itself remains blocked from freeze by timestamp-adversary analysis.
4. **Implementation-only locked docs are protocol drift.** Monetary/fee primitives and Address V1 have now been synchronized into `niahcia-protocol`.
5. **A locked vector does not automatically mean an entire draft object/state machine is production-frozen.** Vector stability and protocol activation status are tracked separately.

## Highest-priority gaps

1. Design and prove the stock-XMRig-compatible PoW preimage with node/pool/XMRig vectors.
2. Resolve ASERT + timestamp hardening.
3. Add protocol-owned Address V1 vectors.
4. Add monetary/fee arithmetic and native chain-ID vectors.
5. Audit `prototype-0.md` and older architecture docs for the superseded 2-of-3 design.
6. Complete transaction-envelope/transaction-signing specification alignment with the locked native value/chain-ID rules.
7. Finish replacement VerificationPolicy selection/evidence/dispute design without central scheduler authority.
8. Review the complete proof-of-service state machine as one unit before promoting its advanced vectors to a locked protocol flow.
9. Define explicit freeze criteria for NCE/1, domain separation, signatures and registries because every higher-level object depends on them.
10. Keep both `niahcia-protocol` and `niahcia` documentation synchronized with every protocol-affecting implementation change.

## Documentation synchronization rule

For every protocol-affecting code change:

1. update the affected specification/status here;
2. update canonical vectors where consensus/wire behavior changes;
3. update the reference implementation;
4. update corresponding implementation-facing documentation in `niahcia/niahcia`;
5. explicitly identify superseded behavior rather than leaving contradictory rules.
