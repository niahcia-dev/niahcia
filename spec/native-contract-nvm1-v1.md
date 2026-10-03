# NIAHCIA Native VM Runtime 1 — NVM1 Code Format

Status: **CANDIDATE / VECTORED / INACTIVE**

## Decision

Runtime ID `1` is reserved as the candidate **NIAHCIA Native VM V1 (NVM1)** runtime.

NVM1 is a deliberately small deterministic integer/byte-oriented stack VM designed for consensus execution.

This choice is preferred for Runtime 1 over importing a general WASM or EVM engine because the first NIAHCIA contract runtime should minimize consensus surface, external semantic dependencies, proposal/version drift, floating-point ambiguity, and host integration complexity.

A future WASM/EVM-compatible runtime can still be assigned a separate runtime ID without changing Contract Address V1, ContractCreate payloads, ContractStateV1, or NativeStateV3.

No network currently activates Runtime 1.

## Code container

NVM1 code begins with a fixed 16-byte header:

```text
magic               4 bytes  ASCII "NVM1"
format_version      u16 BE    1
flags               u16 BE    0
instruction_count   u32 BE
max_stack_items     u16 BE
reserved            u16 BE    0
instruction_stream  remaining bytes
```

Rules:

- instruction count MUST be 1..=16,384;
- max stack items MUST be 1..=256;
- flags and reserved fields MUST be zero;
- the instruction stream MUST decode to exactly `instruction_count` instructions;
- unknown opcodes are invalid;
- truncated operands are invalid;
- jump targets are instruction indices, not byte offsets, and MUST be within the decoded instruction range.

## Opcode allocation

Code-format V1 reserves:

```text
00 STOP
01 PUSH_U64        + 8-byte BE operand
02 PUSH_BYTES32    + 32-byte operand
03 POP
04 DUP
05 ADD_U64
06 SUB_U64
07 EQ
08 JUMP            + u32 BE instruction index
09 JUMP_IF         + u32 BE instruction index

10 INPUT_LEN
11 INPUT_COPY       + u32 BE length
12 CALLER
13 CALL_VALUE

20 STORAGE_GET
21 STORAGE_SET
22 STORAGE_DELETE

30 KECCAK256

40 RETURN
41 REVERT
```

The code-format validator locks only opcode identity and operand width.

Execution/stack effects, gas costs, integer overflow behavior, memory representation, storage effects, RETURN/REVERT payload semantics, and call/create behavior are specified separately before activation.

## Determinism boundary

NVM1 V1 contains no floating-point opcode and no implicit host syscall surface.

Network, filesystem, wall clock, environment variables, GPU/AI inference, nondeterministic randomness, threads, SIMD, or host-native pointers are not exposed by this code format.

Any future host capability must be an explicitly versioned deterministic VM surface.

## Static validation

Before ContractCreate may reach runtime execution:

1. ContractCreate payload must decode canonically;
2. runtime registry must select active Runtime ID 1 / code-format version 1;
3. generic and runtime-specific byte limits must pass;
4. NVM1 header must validate;
5. instruction stream must decode exactly;
6. all jump targets must reference valid instruction indices.

Static validation does not execute constructor code and does not prove that runtime execution will succeed.

## Gas boundary

Opcode gas costs are intentionally not assigned by this code-format document.

The next runtime milestone must define deterministic stack effects, execution semantics, resource limits, and gas schedule together so a malformed or expensive program cannot become consensus-valid by accident.

## Vector

The canonical fixture is:

`test-vectors/native-contract-nvm1-code-v1.json`

It locks header parsing, instruction decoding, operand widths, and jump-target indexing.

## Compatibility

Any incompatible change to header layout, opcode identity, operand width, or jump-target indexing requires a successor code-format version or runtime ID.
