# ExecutionProfile

## Purpose

`ExecutionProfile` defines the reproducible environment in which a workload is expected to execute.

It separates model identity from runtime, hardware, numerical, and determinism requirements.

## Canonical fields

```text
ExecutionProfile
- schema_version
- profile_id
- runtime
- runtime_version
- runtime_build_hash
- hardware_requirements
- precision
- quantization
- context_limit
- determinism_mode
- batch_policy
- seed_policy
- generation_policy
- supported_workloads[]
- input_format_version
- output_format_version
- profile_hash
```

## Runtime

Examples may include:

```text
VLLM
LLAMA_CPP
SGLANG
TENSORRT_LLM
CUSTOM
```

The protocol MUST remain runtime-neutral.

## Hardware requirements

Requirements may constrain:

- accelerator vendor/family
- compute capability
- minimum VRAM
- CPU features
- RAM
- multi-device topology
- approved kernel/backend class

Self-reported hardware is advisory unless independently attestable. Eligibility ultimately depends on successful execution of the profile.

## Determinism

`determinism_mode` declares the reproducibility expectations of the profile.

Possible classes:

```text
NONE
BEST_EFFORT
SAME_HARDWARE_CLASS
CROSS_HARDWARE_REPRODUCIBLE
PROOF_VERIFIED
```

## Output commitment

For token-generating workloads, the profile MUST specify the canonical token/output representation used for result commitments.

## Invariants

1. `profile_hash` covers every execution-relevant field.
2. A runtime or determinism-changing modification creates a new profile.
3. Workers MUST advertise explicit supported profiles.
4. Jobs MUST reference a concrete profile or a policy that deterministically resolves to one.

## Prototype 0

Prototype 0 uses a pinned vLLM version/build, one NVIDIA Ampere-class profile, fixed generation settings, and canonical token-sequence output hashing.


## Long-running workloads

ExecutionProfile may describe TRAINING/FINE_TUNING/CUSTOM workloads, but protocol support for a workload type does not imply that V1 payment/checkpoint semantics are ready for production use.

Long-running workloads SHOULD NOT be enabled for production settlement until their interruption, checkpoint, restart, artifact commitment, staged payment, and metering rules are explicitly specified.

The first end-to-end milestone remains focused on bounded inference workloads.
