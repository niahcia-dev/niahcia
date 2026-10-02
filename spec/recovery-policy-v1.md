# RecoveryPolicyV1

Status: **CANDIDATE / pre-alpha opt-in recovery model**

## Purpose

`RecoveryPolicyV1` defines explicit, opt-in authority recovery for a durable NIAHCIA identity when normal signing/control authority is unavailable or compromised.

NIAHCIA has no protocol-wide developer, foundation, miner, service-node, storage-node, or website master recovery key.

No recovery policy means no protocol-provided recovery path.

## Design principles

- recovery is configured by the subject/controller before it is needed;
- recovery authorization is distinct from ordinary execution authority;
- recovery does not automatically reveal encrypted memory;
- recovery must be auditable and versioned;
- recovery providers/guardians do not become consensus authorities;
- storage or compute service alone never grants recovery rights.

## Candidate policy forms

A versioned policy may eventually support:

### NONE

No recovery. Loss of all valid authority can permanently lose control.

### DESIGNATED

One explicitly authorized NIAHCIA identity may authorize recovery according to policy constraints.

### THRESHOLD

M-of-N explicitly authorized NIAHCIA identities must approve the recovery transition.

### CONTRACT

A specified contract/governance mechanism may authorize recovery according to its locked semantics.

### DELAYED

Recovery requires valid authorization plus a delay/challenge period before becoming effective.

These are candidate classes; exact semantics and enum values are not locked.

## Candidate canonical fields

```text
RecoveryPolicyV1
- schema_version
- subject_id
- policy_kind
- guardian_set_root
- threshold
- activation_delay
- challenge_window
- recovery_scope
- expiry
- policy_nonce
- created_block
```

## Recovery scope

A policy MUST explicitly define what it can recover. Potential scopes include:

- signing/control authority;
- encryption-wrapping authority for future epochs;
- selected encrypted-key recovery material;
- capability revocation/reset;
- governance/controller transition where separately authorized.

A recovery policy MUST NOT silently imply access to every historical encryption key.

## Encryption-key continuity

Recovering control and recovering old private memory are different operations.

A subject may choose one of several future strategies:

- historical encryption keys are intentionally unrecoverable if lost;
- encrypted key material is recoverable by a threshold policy;
- future key epochs can be rotated but historical data remains inaccessible;
- succession policy grants access to selected protected key material.

The protocol MUST make the selected behavior explicit. Storage nodes must not become implicit key escrow.

## Threshold guardians

For threshold recovery, guardians are NIAHCIA identities or other explicitly versioned authorities. Merely naming N guardians does not guarantee organizational independence.

Threshold recovery should be designed so no single guardian receives the subject's ordinary private key.

Exact threshold cryptography versus multiple signed approvals is not locked here.

## Delays and challenge windows

A recovery delay can provide time for the normal controller to cancel a malicious recovery attempt when the controller is still available.

However, delays can also prevent rapid response to an actively compromised key. Policy should therefore allow different security profiles rather than impose one universal delay.

Exact timing semantics require deterministic chain-time/block-height rules before lock.

## Recovery authorization

A successful recovery authorizes a `KeyRotationV1` transition into a new `KeyAuthorityV1` epoch. Recovery SHOULD NOT disclose or reconstruct the old signing private key.

The safer model is replacement of authority, not resurrection of the same compromised/lost private key.

## Guardian compromise and replacement

Recovery policies themselves must be rotatable/versioned while normal authority remains available.

Changing guardians must require valid current authority and must not retroactively authorize old recovery attempts.

## Security invariants

1. No global NIAHCIA recovery key exists.
2. Recovery is opt-in and policy-bound.
3. Storage/compute/mining participation grants no recovery authority.
4. Recovery normally replaces authority rather than reconstructing the old signing key.
5. Recovery scope is explicit.
6. Recovery of control does not automatically recover historical plaintext/memory.
7. Threshold guardians do not individually receive the subject's ordinary private key.
8. Policy version/nonce prevents stale recovery-policy replay.
9. Recovery events are auditable.
10. A subject may deliberately choose irreversible/no recovery.

## Required vectors before lock

1. NONE policy;
2. designated recovery success/failure;
3. M-of-N threshold approval success;
4. below-threshold rejection;
5. delayed recovery boundary;
6. challenge/cancel case;
7. stale policy nonce rejection;
8. recovery scope enforcement;
9. recovery-authorized KeyRotationV1 fixture;
10. guardian-set update and old-policy replay rejection;
11. cross-implementation recovery fixture.

## Not locked

- threshold signature algorithm;
- guardian discovery/user experience;
- exact delay units;
- recovery fees;
- encrypted key-share scheme;
- social-recovery UI;
- contract recovery ABI;
- succession semantics.
