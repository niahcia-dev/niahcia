use std::collections::BTreeMap;

pub const NVM1_RUNTIME_ID: u32 = 1;
pub const NVM1_CODE_FORMAT_VERSION: u32 = 1;
pub const NVM1_MAGIC: [u8; 4] = *b"NVM1";

pub const NVM1_MAX_INSTRUCTIONS: u32 = 16_384;
pub const NVM1_MAX_STACK_ITEMS: u16 = 256;
pub const NVM1_INACTIVE_EXECUTION_STEP_LIMIT: u32 = 65_536;
pub const NVM1_MAX_MEMORY_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Nvm1CodeHeader {
    pub instruction_count: u32,
    pub max_stack_items: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedNvm1Code {
    pub header: Nvm1CodeHeader,
    pub instruction_bytes: Vec<u8>,
}

pub fn validate_nvm1_code(code: &[u8]) -> Result<ValidatedNvm1Code, String> {
    const HEADER_LEN: usize = 16;

    if code.len() < HEADER_LEN {
        return Err("NVM1 code is truncated".into());
    }
    if code[..4] != NVM1_MAGIC {
        return Err("NVM1 code has invalid magic".into());
    }

    let format_version = u16::from_be_bytes(code[4..6].try_into().unwrap());
    if format_version != NVM1_CODE_FORMAT_VERSION as u16 {
        return Err(format!(
            "unsupported NVM1 code format version {format_version}"
        ));
    }

    let flags = u16::from_be_bytes(code[6..8].try_into().unwrap());
    if flags != 0 {
        return Err("NVM1 code flags must be zero in format V1".into());
    }

    let instruction_count = u32::from_be_bytes(code[8..12].try_into().unwrap());
    if instruction_count == 0 {
        return Err("NVM1 code must contain at least one instruction".into());
    }
    if instruction_count > NVM1_MAX_INSTRUCTIONS {
        return Err(format!(
            "NVM1 instruction count exceeds {NVM1_MAX_INSTRUCTIONS}"
        ));
    }

    let max_stack_items = u16::from_be_bytes(code[12..14].try_into().unwrap());
    if max_stack_items == 0 || max_stack_items > NVM1_MAX_STACK_ITEMS {
        return Err(format!(
            "NVM1 max_stack_items must be within 1..={NVM1_MAX_STACK_ITEMS}"
        ));
    }

    let reserved = u16::from_be_bytes(code[14..16].try_into().unwrap());
    if reserved != 0 {
        return Err("NVM1 reserved header field must be zero".into());
    }

    let instruction_bytes = &code[HEADER_LEN..];
    validate_instruction_stream(instruction_bytes, instruction_count)?;

    Ok(ValidatedNvm1Code {
        header: Nvm1CodeHeader {
            instruction_count,
            max_stack_items,
        },
        instruction_bytes: instruction_bytes.to_vec(),
    })
}

fn validate_instruction_stream(bytes: &[u8], expected_count: u32) -> Result<(), String> {
    let mut offset = 0usize;
    let mut count = 0u32;

    while offset < bytes.len() {
        let opcode = bytes[offset];
        offset += 1;
        count = count
            .checked_add(1)
            .ok_or_else(|| "NVM1 instruction count overflow".to_string())?;

        let operand_len = match opcode {
            0x00 => 0,  // STOP
            0x01 => 8,  // PUSH_U64
            0x02 => 32, // PUSH_BYTES32
            0x03 => 0,  // POP
            0x04 => 0,  // DUP
            0x05 => 0,  // ADD_U64
            0x06 => 0,  // SUB_U64
            0x07 => 0,  // EQ
            0x08 => 4,  // JUMP instruction index
            0x09 => 4,  // JUMP_IF instruction index
            0x10 => 0,  // INPUT_LEN
            0x11 => 4,  // INPUT_COPY length
            0x12 => 0,  // CALLER
            0x13 => 0,  // CALL_VALUE
            0x20 => 0,  // STORAGE_GET
            0x21 => 0,  // STORAGE_SET
            0x22 => 0,  // STORAGE_DELETE
            0x30 => 0,  // KECCAK256
            0x40 => 0,  // RETURN
            0x41 => 0,  // REVERT
            _ => return Err(format!("unknown NVM1 opcode 0x{opcode:02x}")),
        };

        let end = offset
            .checked_add(operand_len)
            .ok_or_else(|| "NVM1 instruction operand length overflow".to_string())?;
        if end > bytes.len() {
            return Err(format!("truncated operand for NVM1 opcode 0x{opcode:02x}"));
        }
        offset = end;
    }

    if count != expected_count {
        return Err(format!(
            "NVM1 instruction count mismatch: header {expected_count}, decoded {count}"
        ));
    }

    Ok(())
}

pub fn validate_nvm1_jump_targets(code: &[u8]) -> Result<(), String> {
    let validated = validate_nvm1_code(code)?;
    let mut offsets = Vec::with_capacity(validated.header.instruction_count as usize);
    let mut offset = 0usize;

    while offset < validated.instruction_bytes.len() {
        offsets.push(offset);
        let opcode = validated.instruction_bytes[offset];
        offset += 1;
        offset += operand_len(opcode)?;
    }

    let instruction_count = offsets.len() as u32;
    for start in offsets {
        let opcode = validated.instruction_bytes[start];
        if opcode == 0x08 || opcode == 0x09 {
            let operand_start = start + 1;
            let target = u32::from_be_bytes(
                validated.instruction_bytes[operand_start..operand_start + 4]
                    .try_into()
                    .unwrap(),
            );
            if target >= instruction_count {
                return Err(format!(
                    "NVM1 jump target {target} is outside instruction range 0..{instruction_count}"
                ));
            }
        }
    }

    Ok(())
}

fn operand_len(opcode: u8) -> Result<usize, String> {
    match opcode {
        0x00 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x10 | 0x12 | 0x13 | 0x20 | 0x21 | 0x22
        | 0x30 | 0x40 | 0x41 => Ok(0),
        0x01 => Ok(8),
        0x02 => Ok(32),
        0x08 | 0x09 | 0x11 => Ok(4),
        _ => Err(format!("unknown NVM1 opcode 0x{opcode:02x}")),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Nvm1Value {
    U64(u64),
    Bytes32([u8; 32]),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Nvm1Halt {
    Stop,
    Return(Vec<u8>),
    Revert(Vec<u8>),
    Trap(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Nvm1ExecutionContext {
    pub input: Vec<u8>,
    pub caller_payload: [u8; 20],
    pub call_value: u64,
    pub storage: BTreeMap<[u8; 32], [u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nvm1ExecutionResult {
    pub halt: Nvm1Halt,
    pub stack: Vec<Nvm1Value>,
    pub instructions_executed: u32,
    pub committed_storage: Option<BTreeMap<[u8; 32], [u8; 32]>>,
}

#[derive(Clone, Debug)]
struct DecodedInstruction {
    opcode: u8,
    operand: Vec<u8>,
}

pub fn execute_nvm1_core(code: &[u8]) -> Result<Nvm1ExecutionResult, String> {
    execute_nvm1_core_with_context(code, &Nvm1ExecutionContext::default())
}

pub fn execute_nvm1_core_with_context(
    code: &[u8],
    context: &Nvm1ExecutionContext,
) -> Result<Nvm1ExecutionResult, String> {
    let validated = validate_nvm1_code(code)?;
    validate_nvm1_jump_targets(code)?;
    let instructions = decode_instructions(&validated.instruction_bytes)?;

    let mut stack = Vec::<Nvm1Value>::new();
    let mut memory = Vec::<u8>::new();
    let mut storage = context.storage.clone();
    let mut pc = 0usize;
    let mut executed = 0u32;

    loop {
        let Some(instruction) = instructions.get(pc) else {
            return Ok(trap_result(
                stack,
                executed,
                "NVM1 execution fell past final instruction",
            ));
        };
        if executed >= NVM1_INACTIVE_EXECUTION_STEP_LIMIT {
            return Ok(trap_result(
                stack,
                executed,
                "NVM1 inactive execution step limit exceeded",
            ));
        }
        executed += 1;

        match instruction.opcode {
            0x00 => {
                return Ok(Nvm1ExecutionResult {
                    halt: Nvm1Halt::Stop,
                    stack,
                    instructions_executed: executed,
                    committed_storage: Some(storage),
                });
            }
            0x01 => {
                let value = u64::from_be_bytes(instruction.operand[..8].try_into().unwrap());
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::U64(value),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x02 => {
                let value: [u8; 32] = instruction.operand[..32].try_into().unwrap();
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(value),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x03 => {
                if stack.pop().is_none() {
                    return Ok(trap_result(stack, executed, "NVM1 POP stack underflow"));
                }
                pc += 1;
            }
            0x04 => {
                let Some(value) = stack.last().cloned() else {
                    return Ok(trap_result(stack, executed, "NVM1 DUP stack underflow"));
                };
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    value,
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x05 | 0x06 => {
                let Some(rhs) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 arithmetic stack underflow",
                    ));
                };
                let Some(lhs) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 arithmetic stack underflow",
                    ));
                };
                let (Nvm1Value::U64(lhs), Nvm1Value::U64(rhs)) = (lhs, rhs) else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 arithmetic type mismatch",
                    ));
                };
                let value = if instruction.opcode == 0x05 {
                    lhs.checked_add(rhs)
                } else {
                    lhs.checked_sub(rhs)
                };
                let Some(value) = value else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 arithmetic overflow or underflow",
                    ));
                };
                stack.push(Nvm1Value::U64(value));
                pc += 1;
            }
            0x07 => {
                let Some(rhs) = stack.pop() else {
                    return Ok(trap_result(stack, executed, "NVM1 EQ stack underflow"));
                };
                let Some(lhs) = stack.pop() else {
                    return Ok(trap_result(stack, executed, "NVM1 EQ stack underflow"));
                };
                let equal = match (&lhs, &rhs) {
                    (Nvm1Value::U64(a), Nvm1Value::U64(b)) => a == b,
                    (Nvm1Value::Bytes32(a), Nvm1Value::Bytes32(b)) => a == b,
                    _ => return Ok(trap_result(stack, executed, "NVM1 EQ type mismatch")),
                };
                stack.push(Nvm1Value::U64(u64::from(equal)));
                pc += 1;
            }
            0x08 => {
                pc = u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as usize;
            }
            0x09 => {
                let Some(condition) = stack.pop() else {
                    return Ok(trap_result(stack, executed, "NVM1 JUMP_IF stack underflow"));
                };
                let Nvm1Value::U64(condition) = condition else {
                    return Ok(trap_result(stack, executed, "NVM1 JUMP_IF type mismatch"));
                };
                if condition != 0 {
                    pc = u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as usize;
                } else {
                    pc += 1;
                }
            }
            0x10 => {
                let input_len = u64::try_from(context.input.len())
                    .map_err(|_| "NVM1 input length does not fit U64".to_string())?;
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::U64(input_len),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x11 => {
                let Some(offset) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 INPUT_COPY stack underflow",
                    ));
                };
                let Nvm1Value::U64(offset) = offset else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 INPUT_COPY type mismatch",
                    ));
                };
                let offset = usize::try_from(offset)
                    .map_err(|_| "NVM1 INPUT_COPY offset does not fit usize".to_string())?;
                let length =
                    u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as usize;
                let Some(end) = offset.checked_add(length) else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 INPUT_COPY range overflow",
                    ));
                };
                if end > context.input.len() {
                    return Ok(trap_result(stack, executed, "NVM1 INPUT_COPY out of range"));
                }
                let Some(new_memory_len) = memory.len().checked_add(length) else {
                    return Ok(trap_result(stack, executed, "NVM1 memory length overflow"));
                };
                if new_memory_len > NVM1_MAX_MEMORY_BYTES {
                    return Ok(trap_result(stack, executed, "NVM1 memory limit exceeded"));
                }
                memory.extend_from_slice(&context.input[offset..end]);
                pc += 1;
            }
            0x12 => {
                let mut caller = [0u8; 32];
                caller[12..].copy_from_slice(&context.caller_payload);
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(caller),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x13 => {
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::U64(context.call_value),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x20 => {
                let Some(key) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_GET stack underflow",
                    ));
                };
                let Nvm1Value::Bytes32(key) = key else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_GET type mismatch",
                    ));
                };
                let value = storage.get(&key).copied().unwrap_or([0u8; 32]);
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(value),
                    executed,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x21 => {
                let Some(value) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_SET stack underflow",
                    ));
                };
                let Some(key) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_SET stack underflow",
                    ));
                };
                let (Nvm1Value::Bytes32(key), Nvm1Value::Bytes32(value)) = (key, value) else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_SET type mismatch",
                    ));
                };
                storage.insert(key, value);
                pc += 1;
            }
            0x22 => {
                let Some(key) = stack.pop() else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_DELETE stack underflow",
                    ));
                };
                let Nvm1Value::Bytes32(key) = key else {
                    return Ok(trap_result(
                        stack,
                        executed,
                        "NVM1 STORAGE_DELETE type mismatch",
                    ));
                };
                storage.remove(&key);
                pc += 1;
            }
            0x40 => {
                return Ok(Nvm1ExecutionResult {
                    halt: Nvm1Halt::Return(memory),
                    stack,
                    instructions_executed: executed,
                    committed_storage: Some(storage),
                });
            }
            0x41 => {
                return Ok(Nvm1ExecutionResult {
                    halt: Nvm1Halt::Revert(memory),
                    stack,
                    instructions_executed: executed,
                    committed_storage: Some(storage),
                });
            }
            0x30 => {
                return Ok(trap_result(
                    stack,
                    executed,
                    &format!(
                        "NVM1 opcode 0x{:02x} execution semantics are not active",
                        instruction.opcode
                    ),
                ));
            }
            _ => unreachable!("static validation rejects unknown opcodes"),
        }
    }
}

fn decode_instructions(bytes: &[u8]) -> Result<Vec<DecodedInstruction>, String> {
    let mut instructions = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let opcode = bytes[offset];
        offset += 1;
        let len = operand_len(opcode)?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| "NVM1 instruction operand length overflow".to_string())?;
        if end > bytes.len() {
            return Err(format!("truncated operand for NVM1 opcode 0x{opcode:02x}"));
        }
        instructions.push(DecodedInstruction {
            opcode,
            operand: bytes[offset..end].to_vec(),
        });
        offset = end;
    }
    Ok(instructions)
}

fn push_value(
    stack: &mut Vec<Nvm1Value>,
    max_stack_items: u16,
    value: Nvm1Value,
    executed: u32,
) -> Option<Nvm1ExecutionResult> {
    if stack.len() >= max_stack_items as usize {
        return Some(trap_result(
            stack.clone(),
            executed,
            "NVM1 declared stack bound exceeded",
        ));
    }
    stack.push(value);
    None
}

fn trap_result(stack: Vec<Nvm1Value>, executed: u32, message: &str) -> Nvm1ExecutionResult {
    Nvm1ExecutionResult {
        halt: Nvm1Halt::Trap(message.to_string()),
        stack,
        instructions_executed: executed,
        committed_storage: None,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn core_executes_checked_arithmetic_and_equality() {
        let code = module(
            7,
            3,
            &[
                0x01, 0, 0, 0, 0, 0, 0, 0, 7, 0x01, 0, 0, 0, 0, 0, 0, 0, 5, 0x05, 0x01, 0, 0, 0, 0,
                0, 0, 0, 12, 0x07, 0x04, 0x00,
            ],
        );
        let result = execute_nvm1_core(&code).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(result.stack, vec![Nvm1Value::U64(1), Nvm1Value::U64(1)]);
        assert_eq!(result.instructions_executed, 7);
    }

    #[test]
    fn core_jump_if_uses_nonzero_u64_truth() {
        let code = module(
            5,
            2,
            &[
                0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x09, 0, 0, 0, 4, 0x01, 0, 0, 0, 0, 0, 0, 0, 99,
                0x00, 0x00,
            ],
        );
        let result = execute_nvm1_core(&code).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert!(result.stack.is_empty());
        assert_eq!(result.instructions_executed, 3);
    }

    #[test]
    fn core_traps_deterministically_on_underflow_type_and_arithmetic_failure() {
        let underflow = execute_nvm1_core(&module(2, 1, &[0x03, 0x00])).unwrap();
        assert!(matches!(underflow.halt, Nvm1Halt::Trap(_)));

        let mut mixed = vec![0x02];
        mixed.extend_from_slice(&[0u8; 32]);
        mixed.extend_from_slice(&[0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x07, 0x00]);
        let mismatch = execute_nvm1_core(&module(4, 2, &mixed)).unwrap();
        assert_eq!(
            mismatch.halt,
            Nvm1Halt::Trap("NVM1 EQ type mismatch".to_string())
        );

        let overflow = module(
            4,
            2,
            &[
                0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0, 0, 0, 0, 0, 0, 0, 1,
                0x05, 0x00,
            ],
        );
        assert_eq!(
            execute_nvm1_core(&overflow).unwrap().halt,
            Nvm1Halt::Trap("NVM1 arithmetic overflow or underflow".to_string())
        );
    }

    #[test]
    fn core_enforces_declared_stack_bound_and_rejects_deferred_opcodes() {
        let bound = module(3, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x04, 0x00]);
        assert_eq!(
            execute_nvm1_core(&bound).unwrap().halt,
            Nvm1Halt::Trap("NVM1 declared stack bound exceeded".to_string())
        );

        let deferred = execute_nvm1_core(&module(2, 1, &[0x30, 0x00])).unwrap();
        assert_eq!(
            deferred.halt,
            Nvm1Halt::Trap("NVM1 opcode 0x30 execution semantics are not active".to_string())
        );
    }


    #[test]
    fn storage_get_set_delete_and_missing_zero_are_transactional() {
        let key = [0x11u8; 32];
        let old_value = [0x22u8; 32];
        let new_value = [0x33u8; 32];
        let mut storage = BTreeMap::new();
        storage.insert(key, old_value);
        let context = Nvm1ExecutionContext {
            storage,
            ..Nvm1ExecutionContext::default()
        };

        let mut bytes = vec![0x02];
        bytes.extend_from_slice(&key);
        bytes.push(0x20);
        bytes.push(0x02);
        bytes.extend_from_slice(&key);
        bytes.push(0x02);
        bytes.extend_from_slice(&new_value);
        bytes.push(0x21);
        bytes.push(0x02);
        bytes.extend_from_slice(&key);
        bytes.push(0x22);
        bytes.push(0x02);
        bytes.extend_from_slice(&key);
        bytes.push(0x20);
        bytes.push(0x00);

        let result = execute_nvm1_core_with_context(&module(9, 3, &bytes), &context).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(
            result.stack,
            vec![
                Nvm1Value::Bytes32(old_value),
                Nvm1Value::Bytes32([0u8; 32])
            ]
        );
        assert_eq!(result.committed_storage, Some(BTreeMap::new()));
        assert_eq!(context.storage.get(&key), Some(&old_value));
    }

    #[test]
    fn revert_and_trap_do_not_expose_committable_storage() {
        let key = [0x44u8; 32];
        let value = [0x55u8; 32];
        let mut prefix = vec![0x02];
        prefix.extend_from_slice(&key);
        prefix.push(0x02);
        prefix.extend_from_slice(&value);
        prefix.push(0x21);

        let mut revert_bytes = prefix.clone();
        revert_bytes.push(0x41);
        let revert = execute_nvm1_core(&module(4, 2, &revert_bytes)).unwrap();
        assert!(matches!(revert.halt, Nvm1Halt::Revert(_)));
        assert_eq!(revert.committed_storage, None);

        let mut trap_bytes = prefix;
        trap_bytes.push(0x03);
        let trap = execute_nvm1_core(&module(4, 2, &trap_bytes)).unwrap();
        assert!(matches!(trap.halt, Nvm1Halt::Trap(_)));
        assert_eq!(trap.committed_storage, None);
    }

    #[test]
    fn context_exposes_input_length_and_call_value() {
        let code = module(3, 2, &[0x10, 0x13, 0x00]);
        let context = Nvm1ExecutionContext {
            input: vec![1, 2, 3, 4, 5],
            caller_payload: [0u8; 20],
            call_value: 42,
            storage: BTreeMap::new(),
        };
        let result = execute_nvm1_core_with_context(&code, &context).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(result.stack, vec![Nvm1Value::U64(5), Nvm1Value::U64(42)]);
    }

    #[test]
    fn caller_pushes_zero_left_padded_address_payload() {
        let code = module(2, 1, &[0x12, 0x00]);
        let mut payload = [0u8; 20];
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte = (index + 1) as u8;
        }
        let context = Nvm1ExecutionContext {
            input: Vec::new(),
            caller_payload: payload,
            call_value: 0,
            storage: BTreeMap::new(),
        };
        let result = execute_nvm1_core_with_context(&code, &context).unwrap();
        let mut expected = [0u8; 32];
        expected[12..].copy_from_slice(&payload);
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(result.stack, vec![Nvm1Value::Bytes32(expected)]);
    }

    #[test]
    fn input_copy_returns_selected_bytes() {
        let code = module(
            3,
            1,
            &[0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x11, 0, 0, 0, 3, 0x40],
        );
        let context = Nvm1ExecutionContext {
            input: vec![10, 20, 30, 40, 50],
            caller_payload: [0u8; 20],
            call_value: 0,
            storage: BTreeMap::new(),
        };
        let result = execute_nvm1_core_with_context(&code, &context).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Return(vec![20, 30, 40]));
        assert!(result.stack.is_empty());
    }

    #[test]
    fn input_copy_traps_on_range_and_memory_limit() {
        let out_of_range = module(
            3,
            1,
            &[0x01, 0, 0, 0, 0, 0, 0, 0, 2, 0x11, 0, 0, 0, 2, 0x40],
        );
        let context = Nvm1ExecutionContext {
            input: vec![1, 2, 3],
            caller_payload: [0u8; 20],
            call_value: 0,
            storage: BTreeMap::new(),
        };
        assert_eq!(
            execute_nvm1_core_with_context(&out_of_range, &context)
                .unwrap()
                .halt,
            Nvm1Halt::Trap("NVM1 INPUT_COPY out of range".to_string())
        );

        let mut input = vec![0u8; NVM1_MAX_MEMORY_BYTES + 1];
        input[NVM1_MAX_MEMORY_BYTES] = 1;
        let memory_overflow = module(
            5,
            1,
            &[
                0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0x11, 0, 0, 0xff, 0xff, 0x01, 0, 0, 0, 0, 0, 0, 0, 0,
                0x11, 0, 0, 0, 2, 0x40,
            ],
        );
        assert_eq!(
            execute_nvm1_core_with_context(
                &memory_overflow,
                &Nvm1ExecutionContext {
                    input,
                    caller_payload: [0u8; 20],
                    call_value: 0,
                    storage: BTreeMap::new(),
                }
            )
            .unwrap()
            .halt,
            Nvm1Halt::Trap("NVM1 memory limit exceeded".to_string())
        );
    }

    #[test]
    fn revert_returns_current_memory() {
        let code = module(
            3,
            1,
            &[0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0x11, 0, 0, 0, 2, 0x41],
        );
        let context = Nvm1ExecutionContext {
            input: vec![7, 8],
            caller_payload: [0u8; 20],
            call_value: 0,
            storage: BTreeMap::new(),
        };
        assert_eq!(
            execute_nvm1_core_with_context(&code, &context)
                .unwrap()
                .halt,
            Nvm1Halt::Revert(vec![7, 8])
        );
    }

    #[test]
    fn core_traps_on_infinite_jump_at_inactive_step_limit() {
        let code = module(1, 1, &[0x08, 0, 0, 0, 0]);
        let result = execute_nvm1_core(&code).unwrap();
        assert_eq!(
            result.instructions_executed,
            NVM1_INACTIVE_EXECUTION_STEP_LIMIT
        );
        assert_eq!(
            result.halt,
            Nvm1Halt::Trap("NVM1 inactive execution step limit exceeded".to_string())
        );
    }

    #[test]
    fn core_traps_on_fallthrough() {
        let result = execute_nvm1_core(&module(1, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 1])).unwrap();
        assert_eq!(
            result.halt,
            Nvm1Halt::Trap("NVM1 execution fell past final instruction".to_string())
        );
    }

    use super::*;

    fn module(instruction_count: u32, max_stack: u16, instructions: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&NVM1_MAGIC);
        out.extend_from_slice(&(NVM1_CODE_FORMAT_VERSION as u16).to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&instruction_count.to_be_bytes());
        out.extend_from_slice(&max_stack.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(instructions);
        out
    }

    #[test]
    fn validates_minimal_stop_program() {
        let code = module(1, 1, &[0x00]);
        let validated = validate_nvm1_code(&code).unwrap();
        assert_eq!(validated.header.instruction_count, 1);
        assert_eq!(validated.header.max_stack_items, 1);
        assert_eq!(validated.instruction_bytes, vec![0x00]);
    }

    #[test]
    fn rejects_unknown_opcode_and_count_mismatch() {
        assert!(validate_nvm1_code(&module(1, 1, &[0xff])).is_err());
        assert!(validate_nvm1_code(&module(2, 1, &[0x00])).is_err());
    }

    #[test]
    fn jump_targets_are_instruction_indices() {
        let valid = module(
            3,
            2,
            &[
                0x08, 0x00, 0x00, 0x00, 0x02, // JUMP 2
                0x00, // STOP
                0x40, // RETURN
            ],
        );
        validate_nvm1_jump_targets(&valid).unwrap();

        let invalid = module(
            2,
            2,
            &[
                0x08, 0x00, 0x00, 0x00, 0x02, // JUMP 2, outside 0..2
                0x00,
            ],
        );
        assert!(validate_nvm1_jump_targets(&invalid).is_err());
    }

    #[test]
    fn locked_nvm1_code_vector_matches_json() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-contract-nvm1-code-v1.json"
        ))
        .unwrap();
        let fixture = &vectors["fixture"];
        let code = hex::decode(fixture["code_hex"].as_str().unwrap()).unwrap();

        let validated = validate_nvm1_code(&code).unwrap();
        validate_nvm1_jump_targets(&code).unwrap();

        assert_eq!(
            validated.header.instruction_count as u64,
            fixture["instruction_count"].as_u64().unwrap()
        );
        assert_eq!(
            validated.header.max_stack_items as u64,
            fixture["max_stack_items"].as_u64().unwrap()
        );
        assert_eq!(
            hex::encode(&validated.instruction_bytes),
            fixture["instruction_bytes_hex"].as_str().unwrap()
        );
    }
}
