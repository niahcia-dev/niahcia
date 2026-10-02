# AgentVersion

## Purpose

`AgentVersion` is an immutable snapshot of execution-critical agent behavior.

Any change that can alter what an agent does MUST create a new AgentVersion.

## Canonical fields

```text
AgentVersion
- schema_version
- agent_id
- version
- definition_hash
- primary_models[]
- fallback_models[]
- execution_profiles[]
- verification_policy_id
- system_definition_hash
- tool_manifest_hash
- permission_manifest_hash
- memory_policy_hash
- economic_policy_hash
- trigger_policy_hash
- child_agent_policy_hash
- previous_version
- created_block
```

## Version identity

The pair:

```text
(agent_id, version)
```

uniquely identifies a version.

`definition_hash` commits to the canonical serialized version definition.

## Execution-critical changes

A new version is required when any of the following changes:

- model selection or routing
- system definition/instructions
- execution profiles
- verification policy
- tools
- permissions
- memory policy
- economic policy
- trigger behavior
- child-agent behavior

Pure discovery metadata such as display name, icon, or description MAY be updated outside AgentVersion if it cannot affect execution.

## Version chain

`previous_version` links the immutable history. Gaps SHOULD NOT be allowed for normal sequential releases.

## Invariants

1. Published AgentVersions are immutable.
2. `definition_hash` MUST cover every execution-critical field.
3. Version history is append-only.
4. Workers MUST execute the exact requested AgentVersion.
5. A requester MAY explicitly choose a historical version.

## Prototype 0

Prototype 0 may use one model, one system definition, one execution profile, and one verification policy, but the full structure is retained.


## Private local execution state

AgentVersion defines execution-critical behavior, but it does not require private runtime state to be globally hosted.

A wallet-hosted Agent MAY combine an immutable AgentVersion with wallet-local encrypted memory, conversation state, private preferences, and bounded capabilities.

Those private local values are not automatically part of the public AgentVersion unless a field is explicitly committed by the applicable versioned specification.

Remote workers receive only the context and capabilities required for the current Job.
