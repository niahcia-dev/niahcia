# VerificationPolicy

## Status

**CANDIDATE — policy framework retained; the former fixed Prototype-0 2-of-3 scheme is SUPERSEDED.**

## Purpose

`VerificationPolicy` defines how execution correctness is evaluated without baking one verification mechanism into the protocol.

## Canonical fields

```text
VerificationPolicy
- schema_version
- policy_id
- type
- executor_count
- agreement_threshold
- challenge_window
- audit_probability
- audit_count
- bond_requirements
- dispute_policy
- hardware_diversity_rules
- operator_diversity_rules
- settlement_delay
- policy_hash
```

## Policy types

```text
NONE
REDUNDANT
OPTIMISTIC
AUDITED
TEE
ZK_PROOF
CUSTOM
```

## REDUNDANT

Multiple independent executions may be compared using canonical result commitments. `executor_count` and `agreement_threshold` are policy parameters, not global constants.

A redundant policy that claims independent verification must account for operator diversity and should account for correlated implementation/hardware/model/runtime failure where applicable.

## OPTIMISTIC

One primary execution may be accepted after a challenge window unless challenged, subject to the policy's evidence, bond, audit, and dispute rules.

## AUDITED

Execution is subject to mandatory or probabilistic independent audits according to the policy.

## TEE / ZK_PROOF

These policy types reserve standardized proof/attestation-driven verification mechanisms. Exact proof systems require separate versioned specifications.

## Disagreement

A disagreement does not automatically imply fraud.

Policies must distinguish:

- timeout/unavailability;
- non-reproducible honest mismatch;
- malformed commitment;
- cryptographically provable protocol violation;
- adjudicated dishonest execution.

Penalty policy must be explicit.

## Invariants

1. A policy referenced by an accepted Job is immutable for that Job.
2. Verification-mechanism evolution does not grant verifiers PoW consensus authority.
3. Independent verification must enforce the diversity properties the policy claims.
4. Result comparison uses canonical serialized commitments.
5. A fixed network-wide `2-of-3` verifier assumption is not part of Protocol V1.
6. Verification and scheduling must not depend on a permanent trusted coordinator.

## Prototype status

The earlier Prototype-0 rule:

```text
executor_count = 3
agreement_threshold = 2
```

is **SUPERSEDED** and must not be treated as the current NIAHCIA verification design.

The replacement verification design remains a candidate and must be specified with explicit eligibility, selection, evidence, diversity, timeout, challenge, dispute, and settlement rules before it is locked. Until then, implementations may use development-only policies for testing but must label them as such.
