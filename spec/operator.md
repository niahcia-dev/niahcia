# Operator

## Purpose

`Operator` groups resources under common administrative/economic control.

A worker address is not assumed to equal an independent operator.

## Canonical fields

```text
Operator
- schema_version
- operator_id
- controller
- payment_address
- bond
- status
- workers[]
- service_nodes[]
- reputation_ref
- created_block
```

## Why Operator exists

Without an operator layer, one entity may register many worker identities and appear independent during executor/verifier selection.

Operator identity enables protocol policies such as:

- do not intentionally select multiple workers from the same declared operator for independent verification
- track aggregate failure/collusion evidence
- apply operator-level eligibility rules
- support multiple physical resources under one controller

Operator identity does not fully solve hidden Sybil ownership; it provides a necessary protocol primitive.

## Bond

The `bond` field references or represents role-neutral operator collateral where enabled.

Worker- and service-specific collateral MAY remain separate.

## Status

```text
ACTIVE
SUSPENDED
EXITING
EXITED
```

## Invariants

1. A worker/service node MUST reference exactly one current operator.
2. Operator changes MUST be explicit and auditable.
3. Operator identity MUST NOT grant base-chain consensus power.
4. Reputation evidence SHOULD preserve historical operator relationships.

## Prototype 0

Prototype 0 uses Operator primarily to prevent deliberate same-operator selection in redundant verification.
