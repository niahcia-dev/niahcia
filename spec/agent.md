# Agent

## Purpose

`Agent` is the stable on-chain identity for an AI agent. It is intentionally small. Execution-critical behavior belongs to immutable `AgentVersion` objects.

An Agent is not a server, process, model instance, or worker.

## Canonical fields

```text
Agent
- schema_version
- agent_id
- creator
- controller
- governance_mode
- current_version
- treasury_address
- status
- created_block
- metadata_uri
```

## Field semantics

- `schema_version` — version of the Agent schema.
- `agent_id` — stable unique identifier for the lifetime of the agent.
- `creator` — identity that originally created the agent.
- `controller` — current identity or contract authorized to publish new versions when governance permits.
- `governance_mode` — determines how future versions/controllers may be changed.
- `current_version` — latest version advertised by the Agent record. Historical versions remain addressable.
- `treasury_address` — optional account/contract used for agent revenue and budgets.
- `status` — lifecycle state.
- `created_block` — canonical creation block.
- `metadata_uri` — content-addressed or otherwise integrity-verifiable discovery metadata.

## Governance modes

Initial vocabulary:

```text
IMMUTABLE
CREATOR_CONTROLLED
CONTRACT_CONTROLLED
```

Future governance modes may be added through protocol versioning.

## Status values

```text
ACTIVE
PAUSED
DEPRECATED
REVOKED
```

`REVOKED` does not erase history or old versions.

## Invariants

1. `agent_id` never changes.
2. Previously published AgentVersions are immutable.
3. Updating `current_version` does not invalidate older versions.
4. Execution-critical behavior must not be inferred from mutable metadata.
5. Smart contracts SHOULD pin an exact AgentVersion rather than implicitly follow `current_version`.

## Prototype 0

Prototype 0 needs only stable identity, creator/controller, `current_version`, status, and metadata. The schema nevertheless reserves the full governance model.


## V1 wallet-hosted Agent model

For the first NIAHCIA AI milestone, an Agent MAY be hosted entirely by the user's wallet/client.

In this mode the wallet retains the Agent's private configuration, local encrypted memory, conversation state, capability policy, and spending limits. Remote compute workers execute bounded inference Jobs and return results, but do not become the Agent's persistent host.

A wallet-hosted Agent therefore requires no permanent Agent server, storage node, or worker affinity.

This is compatible with the stable Agent/AgentVersion model: the protocol identity and versioned behavior may remain explicit while private operational state remains local.

Future versions may support independently hosted, always-online, or decentralized autonomous Agents without changing the validity of the wallet-hosted V1 mode.
