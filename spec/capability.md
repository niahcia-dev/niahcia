# Capability

## Purpose

`Capability` is the permission primitive for tools, contracts, agents, storage, wallets, and other external actions.

Agents receive explicit capabilities rather than unrestricted ambient authority.

## Canonical fields

```text
Capability
- schema_version
- capability_id
- subject
- resource_type
- resource_identifier
- actions[]
- limits
- max_spend
- rate_limit
- valid_from
- expiry
- approval_mode
- delegate_allowed
- revocation_ref
- capability_hash
```

## Resource classes

Examples:

```text
WEB
STORAGE
BLOCKCHAIN
CONTRACT
WALLET
AGENT
COMPUTE
CUSTOM
```

## Actions

Examples include:

```text
READ
WRITE
INVOKE
EXECUTE
SPEND
SIGN_REQUEST
STORE
DELETE
```

Exact action vocabulary is resource-specific and versioned.

## Approval modes

```text
AUTO
USER_APPROVAL
CONTROLLER_APPROVAL
CONTRACT_POLICY
DENY
```

## Spending

An LLM or worker MUST NOT receive raw wallet private keys merely because an agent has spending authority.

Signing SHOULD be isolated in a component that validates Capability constraints before authorizing a transaction.

## Delegation

If `delegate_allowed` is false, child agents/tools cannot inherit the capability.

Delegated capabilities MUST be equal to or narrower than their parent authority.

## Invariants

1. Capabilities are deny-by-default.
2. Delegation cannot increase authority.
3. Spending and write capabilities MUST have explicit limits or policy references.
4. Expired/revoked capabilities are invalid even if cached by a worker.
5. Tool execution is sandboxed independently from permission authorization.

## Prototype 0

Prototype 0 may use no tools or read-only capabilities, but the full capability model is part of Protocol v1.
