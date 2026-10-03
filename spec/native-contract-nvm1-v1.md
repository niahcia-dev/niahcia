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


## Candidate execution semantics — core stack/control slice

Status: **CANDIDATE / INACTIVE**

NVM1 uses a typed operand stack. The only V1 stack value classes in this core slice are:

- `U64`: an unsigned 64-bit integer;
- `Bytes32`: exactly 32 uninterpreted bytes.

There are no implicit conversions between value classes.

### Core stack effects

```text
STOP            [] -> halt success
PUSH_U64 x      [] -> [U64(x)]
PUSH_BYTES32 x  [] -> [Bytes32(x)]
POP             [a] -> []
DUP             [a] -> [a, a]
ADD_U64         [U64(a), U64(b)] -> [U64(a + b)]
SUB_U64         [U64(a), U64(b)] -> [U64(a - b)]
EQ              [a, b] -> [U64(a == b ? 1 : 0)]
JUMP i          [] -> pc=i
JUMP_IF i       [U64(cond)] -> pc=i when cond != 0, otherwise fall through
```

For binary operations the rightmost displayed item is the top of stack. Therefore
`SUB_U64` computes `a - b`, not `b - a`.

`EQ` requires both operands to have the same value class. Comparing unlike value
classes traps rather than coercing either operand.

### Arithmetic

`ADD_U64` and `SUB_U64` use checked unsigned arithmetic. Overflow or underflow
traps. Arithmetic never wraps.

### Stack bounds

Every instruction must have all required operands with the required value classes.
Stack underflow or a type mismatch traps.

Execution must never exceed the module header's declared `max_stack_items`.
Exceeding that declared bound traps even when the absolute protocol maximum of 256
would not otherwise be exceeded.

### Program counter and control flow

The program counter is an instruction index. Execution begins at instruction index
0. Static validation guarantees encoded JUMP/JUMP_IF targets are valid instruction
indices.

A taken JUMP/JUMP_IF sets the next instruction index exactly to its target. An
untaken JUMP_IF advances to the following instruction.

Falling past the final instruction without executing STOP, RETURN, or REVERT traps.
This avoids an implicit success rule not represented by bytecode.

### Core termination/failure classes

For the core interpreter milestone:

- `STOP` is successful termination with empty return bytes;
- `RETURN` and `REVERT` remain reserved but are not executable until their
  payload/memory semantics are locked;
- stack underflow, stack overflow, type mismatch, arithmetic overflow/underflow,
  unsupported-yet-reserved execution surfaces, and fall-through past the final
  instruction are deterministic traps.

A trap is not a node/process error. It is a deterministic VM execution result that
all validating nodes must reproduce.

### Deliberately deferred from this slice

The following opcodes retain their locked identities but their execution semantics
remain inactive until their respective surfaces are specified together:

- INPUT_LEN, INPUT_COPY, CALLER, CALL_VALUE;
- STORAGE_GET, STORAGE_SET, STORAGE_DELETE;
- KECCAK256;
- RETURN, REVERT.

Gas accounting is also deferred from this core slice. No gas constants are implied
by these semantics.


## Candidate execution semantics — call context and byte memory

Status: **CANDIDATE / INACTIVE**

The inactive NVM1 interpreter uses one bounded linear byte-memory buffer. Memory begins empty and may grow only through deterministic VM operations. Runtime 1 remains inactive while gas costs are unspecified.

The candidate V1 memory ceiling is **65,536 bytes**. Any operation whose resulting memory length would exceed that ceiling traps. Reads beyond the current memory length trap; memory is never implicitly zero-extended by a read.

### Call context

Each execution receives immutable call input bytes and the full native `u128` call value.

```text
INPUT_LEN          [] -> [U64(input_length)]
CALL_VALUE         [] -> [U64(call_value)]
INPUT_COPY n       [U64(input_offset)] -> append input[input_offset..input_offset+n] to memory
```

CALL_VALUE preserves the complete native `u128` transaction value. It is encoded as a `Bytes32` value with 16 leading zero bytes followed by the 16-byte big-endian unsigned amount. This avoids truncating NIAHCIA's native value domain to `u64`.

For `INPUT_COPY n`, the immediate `u32` operand is the byte count. The stack supplies the input offset. The offset plus length must be wholly within call input; otherwise execution traps. A zero-length copy is valid when the offset is at most the input length. Successful copy consumes the offset and appends exactly the selected bytes to memory.

### Return and revert data

```text
RETURN             [] -> halt success with the complete current memory buffer
REVERT             [] -> halt revert with the complete current memory buffer
```

Neither opcode consumes stack values. `STOP` remains successful termination with empty return data regardless of memory contents.

This deliberately small model avoids introducing arbitrary memory addressing before NVM1 needs it. A future code-format/runtime version may add explicit memory load/store operations if contract workloads require them.

### Still deferred

`CALLER` remains deferred until the exact address-to-stack representation is locked. Storage operations, KECCAK256, and gas accounting also remain inactive.


## Candidate CALLER representation

Status: **CANDIDATE / INACTIVE**

`CALLER` exposes the authenticated caller's **20-byte Address V1 payload**, not Bech32m text and not the 22-byte version/kind address container.

NVM1 represents that payload as `Bytes32` by left-padding with exactly 12 zero bytes:

```text
CALLER [] -> [Bytes32(0x000000000000000000000000 || caller_payload_20)]
```

The final 20 bytes are exactly the Address V1 payload. The leading 12 bytes MUST be zero. This representation is identical for account and contract callers; caller kind remains transaction/execution-context metadata and is not encoded into the `Bytes32` value.

This rule avoids introducing a third stack type solely for native addresses while preserving the complete canonical 20-byte payload without text encoding or truncation.


## Candidate persistent storage opcode semantics

Status: **CANDIDATE / INACTIVE**

NVM1 persistent storage operates directly on the current contract's Contract State V1 storage map. Both keys and values are exactly `Bytes32`, matching the canonical 32-byte key/value state representation.

```text
STORAGE_GET     [Bytes32(key)] -> [Bytes32(value)]
STORAGE_SET     [Bytes32(key), Bytes32(value)] -> []
STORAGE_DELETE  [Bytes32(key)] -> []
```

For `STORAGE_GET`, the rightmost displayed item is the top of stack. A missing key returns exactly 32 zero bytes; absence is therefore observationally equivalent to a stored all-zero value through `STORAGE_GET` alone.

`STORAGE_SET` inserts or replaces the exact 32-byte value at the exact 32-byte key. `STORAGE_DELETE` removes the key when present and is a successful no-op when absent.

All three operations require `Bytes32` operands. Stack underflow or another value class traps.

### Execution isolation

The interpreter executes against a working copy/view of the current contract storage. Persistent state is committed only if the enclosing contract execution terminates successfully under the native state-transition rules.

`REVERT` and deterministic VM traps discard all storage writes/deletes produced by that execution. They MUST NOT partially mutate persistent Contract State V1.

This section does not activate storage execution. Gas/resource charging and the enclosing state-transition commit boundary remain required before Runtime ID 1 activation.


## Candidate KECCAK256 opcode semantics

Status: **CANDIDATE / INACTIVE**

`KECCAK256` hashes exactly one `Bytes32` stack value using legacy Keccak-256 and replaces it with the 32-byte digest:

```text
KECCAK256 [Bytes32(value)] -> [Bytes32(keccak256(value))]
```

The hash input is exactly the 32 bytes contained in the operand, in their existing byte order. No length prefix, domain tag, text encoding, ABI encoding, memory bytes, or other implicit data is added.

The opcode requires one `Bytes32` operand. Stack underflow or a non-`Bytes32` operand traps. The output is exactly the raw 32-byte Keccak-256 digest as `Bytes32`.

This intentionally narrow primitive keeps NVM1 hashing deterministic without introducing arbitrary memory-range hashing. Contracts that need to hash shorter structured values must first represent or derive the exact 32-byte preimage through protocol-defined operations. A later runtime version may add explicit byte-memory hashing if required.

This section does not activate Runtime ID 1. Gas/resource charging remains required before activation.


## Gas schedule V1

Status: **CANDIDATE / INACTIVE**.

NVM1 gas is deterministic protocol accounting. It is measured against the enclosing NativeTransaction `gas_limit`; NVM1 does not define a second contract-specific gas-limit field.

GasScheduleV1 charges before an instruction mutates VM state. If the remaining gas is less than the complete charge for the next instruction, execution halts with `OutOfGas`. The failing instruction MUST NOT partially execute. Out-of-gas is a trap: persistent storage changes are discarded.

| Operation | Gas |
| --- | ---: |
| STOP, POP, DUP | 1 |
| PUSH_U64, PUSH_BYTES32 | 2 |
| ADD_U64, SUB_U64, EQ | 3 |
| JUMP, JUMP_IF | 3 |
| INPUT_LEN, CALLER, CALL_VALUE | 2 |
| INPUT_COPY | 3 + ceil(length / 32) |
| STORAGE_GET | 50 |
| STORAGE_SET | 200 |
| STORAGE_DELETE | 100 |
| KECCAK256 | 30 |
| RETURN, REVERT | 1 |

The V1 costs are native NIAHCIA schedule constants, not host timing measurements and not inherited EVM constants.

`gas_used` starts at zero and increases only for instructions whose complete charge was successfully reserved. Therefore an instruction rejected for insufficient remaining gas does not add its charge to `gas_used`.

The existing inactive execution-step limit remains a defensive implementation ceiling while NVM1 is inactive. Gas is the consensus resource bound intended for activation; the inactive step ceiling is not part of GasScheduleV1.

GasScheduleV1 is versioned with NVM1. Changing any cost or accounting rule requires an explicit successor schedule/runtime rule and must not reinterpret already-executed blocks.
