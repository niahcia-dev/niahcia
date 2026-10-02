# Model

## Purpose

`Model` identifies model artifacts independently from friendly names, hosting locations, or any specific compute worker.

## Canonical fields

```text
Model
- schema_version
- model_id
- creator
- name
- architecture
- weights_manifest_hash
- tokenizer_hash
- config_hash
- license_id
- license_hash
- manifest_uri
- supported_workloads[]
- supported_execution_profiles[]
- status
- created_block
```

## Identity

The model's trust boundary is content-addressed.

At minimum, model identity MUST commit to:

- exact model weights or chunk manifest
- tokenizer
- configuration needed for execution

The human-readable `name` MUST NOT be sufficient to identify a model.

## Artifact manifest

`weights_manifest_hash` commits to a manifest describing one or more content-addressed model artifacts/chunks.

`manifest_uri` identifies where that manifest may be retrieved. Multiple mirrors may serve identical content.

## Licensing

`license_id` is a human/machine-readable identifier where possible.

`license_hash` commits to the exact license text or canonical license artifact used at registration.

Registration is not protocol endorsement of legal suitability.

## Status

Initial values:

```text
ACTIVE
DEPRECATED
DISABLED
```

Disabling a model prevents new protocol use where enforced but does not erase historical jobs.

## Invariants

1. Artifact hashes are immutable.
2. A model update creates a new Model identity or explicitly versioned Model object; weights are never silently replaced.
3. Workers MUST verify required artifacts before execution.
4. Service nodes MUST serve content that validates against the registered manifest.

## Prototype 0

Prototype 0 registers one pinned Qwen3-class model and exact tokenizer/config artifacts.
