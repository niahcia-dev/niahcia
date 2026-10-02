# Threat Model

This document begins the NIAHCIA threat model. It is intentionally conservative.

## Consensus threats

- majority PoW attacks
- selfish mining
- timestamp manipulation
- difficulty manipulation bugs
- invalid-state block propagation
- chain reorganization attacks

Mitigation work belongs in the consensus specification and must remain independent from AI-layer security.

## Compute threats

- worker returns fabricated output without running the requested model
- worker substitutes a different model or tokenizer
- worker lies about runtime or execution profile
- worker copies another worker's revealed result
- worker accepts jobs and intentionally times out
- Sybil operators register many worker identities
- colluding executor and verifier identities approve invalid work
- hardware/runtime numerical differences cause honest disagreement

Protocol hooks include content-addressed model identity, execution profiles, policy-driven verification, operator identities, worker bonds, audits, challenges, reproducible execution work, and future proof systems.

## Key authority and signer threats

NIAHCIA separates durable identity, signing/control authority, encryption authority, and bounded delegated/session authority. This reduces blast radius only if implementations preserve those boundaries.

Threats include:

- AI runtime reads or exfiltrates a long-term private key;
- prompt injection induces unauthorized payment/signing;
- signer accepts natural-language intent without canonical policy checks;
- compromised signing key attempts unauthorized KeyAuthority rotation;
- attacker replays a stale KeyAuthority epoch;
- attacker races two competing rotations;
- compromised recovery guardian initiates malicious recovery;
- guardian cartel reaches a recovery threshold;
- stale RecoveryPolicy is replayed after guardians/policy change;
- recovery unexpectedly grants access to historical encrypted memory;
- session/capability key is reused after expiry or on another worker/job;
- capability scope is broader than the job requires;
- key material leaks through logs, crash dumps, telemetry, model context, swap, or temporary files.

Required principles:

- long-term signing keys should be isolated from AI runtimes;
- signer inputs are structured canonical operations, not free-form model commands;
- every privileged signing request is checked against current authority epoch, capabilities, limits, destination, and context;
- session authority is short-lived and narrowly scoped;
- recovery is opt-in and versioned;
- no global NIAHCIA recovery key exists;
- recovery normally rotates to new authority rather than reconstructing the old signing key;
- historical decryption access is separately governed;
- secret-bearing logs/telemetry are prohibited.

### Malicious recovery

Recovery is itself a privileged attack surface. A malicious guardian set could steal control even when the normal key remains safe.

Candidate defenses include threshold policies, explicit scopes, policy nonces/versions, optional delays/challenge windows, notification/observation, guardian rotation, and deterministic recovery evidence.

No universal threshold or delay is currently locked. A subject may choose `NONE` and accept irreversible key loss rather than introduce recovery trust.

### Key-epoch rollback

An attacker may present an old but once-valid KeyAuthority state to a new host or signer.

All privileged operations must resolve the currently valid authority epoch from canonical state. Old signatures remain historically valid for their original context but do not resurrect old current authority.

### Capability leakage

A leaked session token/capability should expose only its bounded scope. Capabilities need subject, operation, destination/resource, validity, spend/resource limits, and session/job binding as applicable.

A capability that effectively grants unrestricted signing or decryption defeats the authority hierarchy and must be treated as a master-key equivalent.

## Agent host migration threats

Host migration assumes execution hosts are replaceable and potentially malicious.

Threats include:

- destination host demands/steals master signing key;
- destination requests more memory than authorized;
- old host remains active while new host starts, causing duplicate execution;
- stale checkpoint is presented as current state;
- old authority epoch is used to unlock state;
- source host withholds local state to block migration;
- destination host sees plaintext supplied for ordinary execution;
- session key is replayed on another worker;
- migration causes duplicate payments/tool actions;
- malicious host modifies local memory then tries to publish it as canonical durable state.

Required principles:

- migration is not master-key transfer;
- destination receives only bounded session/capability and memory scope;
- durable checkpoints/storage, not local host disk, are the portability foundation;
- migration authorization binds Agent, AgentVersion/Manifest, current KeyAuthority epoch, destination worker, execution profile, memory scope, nonce, and validity;
- duplicate execution is assumed possible;
- external effects require idempotency/replay protection independent of checkpointing;
- stale checkpoints/authority cannot silently become current;
- ordinary hosts may read plaintext intentionally delivered to them unless a stronger private-compute profile is used.

### Memory unlock after migration

A safe baseline is envelope encryption: durable memory is encrypted with data-encryption keys (DEKs); an authorized session receives only the DEK/material necessary for its allowed memory scope.

The long-term wallet/signing key does not directly decrypt bulk memory and should not be copied to the destination GPU host.

The exact DEK-release mechanism remains open. Candidate future profiles include session-public-key wrapping, threshold release, hardware-backed/attested release, and private-compute mechanisms. No single TEE vendor should become universal protocol authority.

### Duplicate execution

Network partitions, slow failure detection, or malicious hosts can result in old and new hosts executing simultaneously. NIAHCIA must not rely on a global assumption that one Agent equals one running process.

Safety must instead come from unique job/session IDs, current state versions, capability limits, signer policy, idempotent settlement where applicable, and canonical state transitions.

## Service-node and storage threats

Storage is availability/service, not a second consensus system. Storage evidence MUST NOT grant fork-choice, finality, veto, or block-production authority.

Threats include false storage claims, corrupt chunks, disappearance after payment, withholding, eclipse/routing attacks, just-in-time storage, Sybil replicas, correlated failure domains, fake retrieval, collusive traffic, repair farming, extortion, replay, corrupt reconstruction, inflated usage, unauthorized key/plaintext retention, and attempts to derive governance rights from data possession.

### Sybil replicas and correlated failure domains

Distinct provider keys do not prove independent durability. One operator/machine/facility/network can present many identities. Production economics must not equate unique keys with independent failure domains. Unknown independence remains unknown.

### Challenge predictability and replay

Challenges should be unpredictable enough that reacquiring discarded data only for challenge time is impractical. Responses must bind provider, commitment, nonce/context, requested proof/range, and applicable window. Evidence must not be repeatedly paid/replayed.

### Fake retrieval and bandwidth farming

Raw provider-reported byte counts are not trustworthy. NIAHCIA MUST NOT pay production rewards solely from self-reported bandwidth, HTTP counters, or weak colluding-provider/requester claims.

### Repair farming

Recovery economics must not make destruction more profitable than continuous honest storage. Potential defenses include excluding recently failed providers from repair premiums, service-history/bond consequences, bounded repair compensation, and duplicate-payment prevention. Exact economics are open.

### Withholding and corrupt reconstruction

Recovery should start while redundancy is merely degraded. Retrieved chunks are verified against exact commitments. Recovery reproduces the exact object root; semantic similarity is not sufficient.

### Encrypted agent memory and key loss

Storage nodes may preserve ciphertext without decryption authority. Storage does not imply controller, capability, treasury, succession, AgentVersion/Manifest mutation, or plaintext-publication rights.

Perfect ciphertext durability is useless if all authorized decryption capability disappears. Recovery/key policies must make this explicit; storage nodes must never become silent key escrow.

### Object poisoning and resource exhaustion

Human-readable names are discovery metadata, not integrity authority. Exact roots/manifests bind content. Providers need admission/resource limits; discovery of a StorageAgreement does not obligate every node to store it.

### Storage-health manipulation

Until deterministic evidence windows/eligibility are locked, StorageHealthV1 is non-authoritative telemetry and must not create consensus/economic transitions.

### Service-node capture

A service-node cartel may publish false warnings, suppress archives, bias relay, or present a coordinated false chain view. Clients independently verify PoW/headers; service disagreement remains telemetry, not consensus.

## Agent threats

- malicious system instructions
- excessive permissions
- unauthorized spending
- recursive agent-call loops
- malicious tool execution
- silent model/prompt substitution
- creator changes behavior after trust develops

Safeguards include immutable versions, capability permissions, spending limits, call-depth/child budgets, model/config hashes, sandboxed tools, isolated signing, and transparent governance/controller policy.

## Frontend threats

The official website is not protocol authority. A compromised frontend may misrepresent data or construct malicious transactions but must not rewrite chain state, invisibly substitute agent identity/version, become the only path to workers/storage, or custody protocol-required user funds.

## Privacy

Prototype/pre-alpha ordinary compute must assume selected workers can read plaintext job payloads made available to them. Private inference is future capability and must not be implied before a suitable cryptographic/trusted-execution design exists.

Storage confidentiality is separate: providers may preserve ciphertext without decryption. Durable ciphertext does not solve key management, access control, or private compute.

## Botnet and stolen-compute threat

CPU-accessible PoW permits attackers to aggregate unauthorized CPU capacity from compromised machines. Consensus cannot reliably distinguish authorized from stolen compute, so NIAHCIA MUST NOT use IP allowlists, device identity, developer admission servers, or miner registration to decide PoW validity.

Risks include sudden hashrate spikes/disappearance, majority censorship/reorgs, selfish mining, timestamp manipulation, and pool/C2 concentration.

Defenses include cumulative-work fork choice, a DAA stable under abrupt hashrate changes, timestamp rules resistant to manipulation, no implicit mainnet emergency minimum-difficulty reset, anti-eclipse work, miner/pool decentralization, reorg/hashrate telemetry, and no consensus privilege for known miners/pools.

RandomX increases per-miner memory cost and favors general-purpose CPUs but is not botnet prevention. Sustained majority hashpower remains a fundamental Nakamoto-PoW risk.