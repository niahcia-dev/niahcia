use std::collections::BTreeMap;

use sha3::{Digest, Keccak256};

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
pub enum Nvm1Trap {
    StackUnderflow,
    StackOverflow,
    TypeMismatch,
    ArithmeticOverflowOrUnderflow,
    InputRangeOverflow,
    InputOutOfRange,
    MemoryLengthOverflow,
    MemoryLimitExceeded,
    OutOfGas,
    StepLimitExceeded,
    FellOffEnd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Nvm1Halt {
    Stop,
    Return(Vec<u8>),
    Revert(Vec<u8>),
    Trap(Nvm1Trap),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Nvm1ExecutionContext {
    pub input: Vec<u8>,
    pub caller_payload: [u8; 20],
    pub call_value: u128,
    pub storage: BTreeMap<[u8; 32], [u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nvm1ExecutionResult {
    pub halt: Nvm1Halt,
    pub stack: Vec<Nvm1Value>,
    pub instructions_executed: u32,
    pub gas_used: u64,
    pub committed_storage: Option<BTreeMap<[u8; 32], [u8; 32]>>,
}

#[derive(Clone, Debug)]
struct DecodedInstruction {
    opcode: u8,
    operand: Vec<u8>,
}

fn nvm1_gas_cost(instruction: &DecodedInstruction) -> Result<u64, String> {
    let cost = match instruction.opcode {
        0x00 | 0x03 | 0x04 | 0x40 | 0x41 => 1,
        0x01 | 0x02 | 0x10 | 0x12 | 0x13 => 2,
        0x05..=0x09 => 3,
        0x11 => {
            let length = u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as u64;
            3 + length.div_ceil(32)
        }
        0x20 => 50,
        0x21 => 200,
        0x22 => 100,
        0x30 => 30,
        opcode => return Err(format!("NVM1 gas cost undefined for opcode 0x{opcode:02x}")),
    };
    Ok(cost)
}

pub fn execute_nvm1_core(code: &[u8]) -> Result<Nvm1ExecutionResult, String> {
    execute_nvm1_core_with_context(code, &Nvm1ExecutionContext::default())
}

pub fn execute_nvm1_core_with_context(
    code: &[u8],
    context: &Nvm1ExecutionContext,
) -> Result<Nvm1ExecutionResult, String> {
    execute_nvm1_core_with_context_and_gas(code, context, u64::MAX)
}

pub fn execute_nvm1_core_with_context_and_gas(
    code: &[u8],
    context: &Nvm1ExecutionContext,
    gas_limit: u64,
) -> Result<Nvm1ExecutionResult, String> {
    let validated = validate_nvm1_code(code)?;
    validate_nvm1_jump_targets(code)?;
    let instructions = decode_instructions(&validated.instruction_bytes)?;

    let mut stack = Vec::<Nvm1Value>::new();
    let mut memory = Vec::<u8>::new();
    let mut storage = context.storage.clone();
    let mut pc = 0usize;
    let mut executed = 0u32;
    let mut gas_used = 0u64;

    loop {
        let Some(instruction) = instructions.get(pc) else {
            return Ok(trap_result_with_gas(
                stack,
                executed,
                gas_used,
                Nvm1Trap::FellOffEnd,
            ));
        };
        if executed >= NVM1_INACTIVE_EXECUTION_STEP_LIMIT {
            return Ok(trap_result_with_gas(
                stack,
                executed,
                gas_used,
                Nvm1Trap::StepLimitExceeded,
            ));
        }
        let gas_cost = nvm1_gas_cost(instruction)?;
        if gas_cost > gas_limit.saturating_sub(gas_used) {
            return Ok(trap_result_with_gas(
                stack,
                executed,
                gas_used,
                Nvm1Trap::OutOfGas,
            ));
        }
        gas_used += gas_cost;
        executed += 1;

        match instruction.opcode {
            0x00 => {
                return Ok(Nvm1ExecutionResult {
                    halt: Nvm1Halt::Stop,
                    stack,
                    instructions_executed: executed,
                    gas_used,
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
                    gas_used,
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
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x03 => {
                if stack.pop().is_none() {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                }
                pc += 1;
            }
            0x04 => {
                let Some(value) = stack.last().cloned() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    value,
                    executed,
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x05 | 0x06 => {
                let Some(rhs) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Some(lhs) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let (Nvm1Value::U64(lhs), Nvm1Value::U64(rhs)) = (lhs, rhs) else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
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
                        Nvm1Trap::ArithmeticOverflowOrUnderflow,
                    ));
                };
                stack.push(Nvm1Value::U64(value));
                pc += 1;
            }
            0x07 => {
                let Some(rhs) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Some(lhs) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let equal = match (&lhs, &rhs) {
                    (Nvm1Value::U64(a), Nvm1Value::U64(b)) => a == b,
                    (Nvm1Value::Bytes32(a), Nvm1Value::Bytes32(b)) => a == b,
                    _ => {
                        return Ok(trap_result_with_gas(
                            stack,
                            executed,
                            gas_used,
                            Nvm1Trap::TypeMismatch,
                        ))
                    }
                };
                stack.push(Nvm1Value::U64(u64::from(equal)));
                pc += 1;
            }
            0x08 => {
                pc = u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as usize;
            }
            0x09 => {
                let Some(condition) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Nvm1Value::U64(condition) = condition else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
                    ));
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
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x11 => {
                let Some(offset) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Nvm1Value::U64(offset) = offset else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
                    ));
                };
                let offset = usize::try_from(offset)
                    .map_err(|_| "NVM1 INPUT_COPY offset does not fit usize".to_string())?;
                let length =
                    u32::from_be_bytes(instruction.operand[..4].try_into().unwrap()) as usize;
                let Some(end) = offset.checked_add(length) else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::InputRangeOverflow,
                    ));
                };
                if end > context.input.len() {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::InputOutOfRange,
                    ));
                }
                let Some(new_memory_len) = memory.len().checked_add(length) else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::MemoryLengthOverflow,
                    ));
                };
                if new_memory_len > NVM1_MAX_MEMORY_BYTES {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::MemoryLimitExceeded,
                    ));
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
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x13 => {
                let mut value = [0u8; 32];
                value[16..].copy_from_slice(&context.call_value.to_be_bytes());
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(value),
                    executed,
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x20 => {
                let Some(key) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Nvm1Value::Bytes32(key) = key else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
                    ));
                };
                let value = storage.get(&key).copied().unwrap_or([0u8; 32]);
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(value),
                    executed,
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
            }
            0x21 => {
                let Some(value) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Some(key) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let (Nvm1Value::Bytes32(key), Nvm1Value::Bytes32(value)) = (key, value) else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
                    ));
                };
                storage.insert(key, value);
                pc += 1;
            }
            0x22 => {
                let Some(key) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Nvm1Value::Bytes32(key) = key else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
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
                    gas_used,
                    committed_storage: Some(storage),
                });
            }
            0x41 => {
                return Ok(Nvm1ExecutionResult {
                    halt: Nvm1Halt::Revert(memory),
                    stack,
                    instructions_executed: executed,
                    gas_used,
                    committed_storage: None,
                });
            }
            0x30 => {
                let Some(value) = stack.pop() else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::StackUnderflow,
                    ));
                };
                let Nvm1Value::Bytes32(value) = value else {
                    return Ok(trap_result_with_gas(
                        stack,
                        executed,
                        gas_used,
                        Nvm1Trap::TypeMismatch,
                    ));
                };
                let digest: [u8; 32] = Keccak256::digest(value).into();
                if let Some(result) = push_value(
                    &mut stack,
                    validated.header.max_stack_items,
                    Nvm1Value::Bytes32(digest),
                    executed,
                    gas_used,
                ) {
                    return Ok(result);
                }
                pc += 1;
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
    gas_used: u64,
) -> Option<Nvm1ExecutionResult> {
    if stack.len() >= max_stack_items as usize {
        return Some(trap_result_with_gas(
            stack.clone(),
            executed,
            gas_used,
            Nvm1Trap::StackOverflow,
        ));
    }
    stack.push(value);
    None
}

fn trap_result(stack: Vec<Nvm1Value>, executed: u32, trap: Nvm1Trap) -> Nvm1ExecutionResult {
    trap_result_with_gas(stack, executed, 0, trap)
}

fn trap_result_with_gas(
    _stack: Vec<Nvm1Value>,
    executed: u32,
    gas_used: u64,
    trap: Nvm1Trap,
) -> Nvm1ExecutionResult {
    Nvm1ExecutionResult {
        halt: Nvm1Halt::Trap(trap),
        stack: Vec::new(),
        instructions_executed: executed,
        gas_used,
        committed_storage: None,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn stack_overflow_trap_preserves_gas_already_charged() {
        let mut instructions = vec![0x01];
        instructions.extend_from_slice(&1u64.to_be_bytes());
        instructions.push(0x01);
        instructions.extend_from_slice(&2u64.to_be_bytes());
        let code = module(2, 1, &instructions);
        let context = Nvm1ExecutionContext::default();
        let result = execute_nvm1_core_with_context_and_gas(&code, &context, 4).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::StackOverflow));
        assert_eq!(result.gas_used, 4);
        assert_eq!(result.instructions_executed, 2);
        assert!(result.stack.is_empty());
        assert_eq!(result.committed_storage, None);
    }

    #[test]
    fn ordinary_trap_preserves_gas_already_charged() {
        let code = module(2, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 7, 0x05]);
        let context = Nvm1ExecutionContext::default();
        let result = execute_nvm1_core_with_context_and_gas(&code, &context, 10).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::StackUnderflow));
        assert_eq!(result.gas_used, 5);
        assert_eq!(result.instructions_executed, 2);
        assert_eq!(result.committed_storage, None);
    }

    #[test]
    fn call_value_preserves_full_u128_native_amount() {
        let code = module(2, 1, &[0x13, 0x00]);
        let context = Nvm1ExecutionContext {
            call_value: u128::MAX - 7,
            ..Default::default()
        };
        let result = execute_nvm1_core_with_context_and_gas(&code, &context, 3).unwrap();

        let mut expected = [0u8; 32];
        expected[16..].copy_from_slice(&(u128::MAX - 7).to_be_bytes());

        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(result.stack, vec![Nvm1Value::Bytes32(expected)]);
        assert_eq!(result.gas_used, 3);
    }

    #[test]
    fn gas_meter_charges_exactly_and_traps_before_next_instruction() {
        let code = module(3, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 7, 0x03, 0x00]);
        let context = Nvm1ExecutionContext::default();

        let exact = execute_nvm1_core_with_context_and_gas(&code, &context, 4).unwrap();
        assert_eq!(exact.halt, Nvm1Halt::Stop);
        assert_eq!(exact.gas_used, 4);
        assert_eq!(exact.instructions_executed, 3);

        let short = execute_nvm1_core_with_context_and_gas(&code, &context, 3).unwrap();
        assert_eq!(short.halt, Nvm1Halt::Trap(Nvm1Trap::OutOfGas));
        assert_eq!(short.gas_used, 3);
        assert_eq!(short.instructions_executed, 2);
        assert!(short.stack.is_empty());
        assert_eq!(short.committed_storage, None);
    }

    #[test]
    fn out_of_gas_before_storage_set_discards_working_storage() {
        let key = [0x11; 32];
        let value = [0x22; 32];
        let mut instructions = vec![0x02];
        instructions.extend_from_slice(&key);
        instructions.push(0x02);
        instructions.extend_from_slice(&value);
        instructions.push(0x21);
        instructions.push(0x00);
        let code = module(4, 2, &instructions);
        let context = Nvm1ExecutionContext::default();

        let result = execute_nvm1_core_with_context_and_gas(&code, &context, 203).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::OutOfGas));
        assert_eq!(result.gas_used, 4);
        assert_eq!(result.instructions_executed, 2);
        assert_eq!(result.committed_storage, None);
        assert!(context.storage.is_empty());
    }

    #[test]
    fn gas_bounds_infinite_jump_before_inactive_step_limit() {
        let code = module(1, 1, &[0x08, 0, 0, 0, 0]);
        let context = Nvm1ExecutionContext::default();
        let result = execute_nvm1_core_with_context_and_gas(&code, &context, 9).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::OutOfGas));
        assert_eq!(result.gas_used, 9);
        assert_eq!(result.instructions_executed, 3);
        assert_eq!(result.committed_storage, None);
    }

    #[test]
    fn checked_in_gas_vector_matches_interpreter_schedule() {
        let vector: serde_json::Value = serde_json::from_str(include_str!(
            "../../../test-vectors/native-contract-nvm1-gas-v1.json"
        ))
        .unwrap();
        let schedule = vector["schedule"].as_object().unwrap();
        let expected = [
            ("STOP", 1),
            ("PUSH_U64", 2),
            ("PUSH_BYTES32", 2),
            ("POP", 1),
            ("DUP", 1),
            ("ADD_U64", 3),
            ("SUB_U64", 3),
            ("EQ", 3),
            ("JUMP", 3),
            ("JUMP_IF", 3),
            ("INPUT_LEN", 2),
            ("CALLER", 2),
            ("CALL_VALUE", 2),
            ("STORAGE_GET", 50),
            ("STORAGE_SET", 200),
            ("STORAGE_DELETE", 100),
            ("KECCAK256", 30),
            ("RETURN", 1),
            ("REVERT", 1),
        ];
        for (name, cost) in expected {
            assert_eq!(schedule[name].as_u64(), Some(cost), "{name}");
        }
        assert_eq!(schedule["INPUT_COPY_BASE"].as_u64(), Some(3));
        assert_eq!(schedule["INPUT_COPY_PER_32_BYTES_CEIL"].as_u64(), Some(1));

        let vectors = vector["vectors"].as_array().unwrap();
        let exact = vectors
            .iter()
            .find(|v| v["name"] == "exact-stop-budget")
            .unwrap();
        let instructions = hex::decode(exact["instruction_bytes_hex"].as_str().unwrap()).unwrap();
        let code = module(
            exact["instruction_count"].as_u64().unwrap() as u32,
            exact["max_stack_items"].as_u64().unwrap() as u16,
            &instructions,
        );
        let result = execute_nvm1_core_with_context_and_gas(
            &code,
            &Nvm1ExecutionContext::default(),
            exact["gas_limit"].as_u64().unwrap(),
        )
        .unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(
            result.gas_used,
            exact["expected_gas_used"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(result.instructions_executed),
            exact["expected_instructions_executed"].as_u64().unwrap()
        );

        for name in ["out-of-gas-before-stop", "infinite-jump-bounded-by-gas"] {
            let fixture = vectors.iter().find(|v| v["name"] == name).unwrap();
            let instructions =
                hex::decode(fixture["instruction_bytes_hex"].as_str().unwrap()).unwrap();
            let code = module(
                fixture["instruction_count"].as_u64().unwrap() as u32,
                fixture["max_stack_items"].as_u64().unwrap() as u16,
                &instructions,
            );
            let result = execute_nvm1_core_with_context_and_gas(
                &code,
                &Nvm1ExecutionContext::default(),
                fixture["gas_limit"].as_u64().unwrap(),
            )
            .unwrap();
            assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::OutOfGas), "{name}");
            assert_eq!(
                result.gas_used,
                fixture["expected_gas_used"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                u64::from(result.instructions_executed),
                fixture["expected_instructions_executed"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(result.committed_storage, None, "{name}");
        }
    }

    #[test]
    fn gas_schedule_v1_costs_are_locked() {
        let cases = [
            (vec![0x00], 1),
            (vec![0x01, 0, 0, 0, 0, 0, 0, 0, 1], 2),
            (vec![0x05], 3),
            (vec![0x11, 0, 0, 0, 0], 3),
            (vec![0x11, 0, 0, 0, 1], 4),
            (vec![0x11, 0, 0, 0, 32], 4),
            (vec![0x11, 0, 0, 0, 33], 5),
            (vec![0x20], 50),
            (vec![0x21], 200),
            (vec![0x22], 100),
            (vec![0x30], 30),
            (vec![0x40], 1),
            (vec![0x41], 1),
        ];
        for (bytes, expected) in cases {
            let decoded = decode_instructions(&bytes).unwrap();
            assert_eq!(nvm1_gas_cost(&decoded[0]).unwrap(), expected);
        }
    }

    #[test]
    fn traps_do_not_expose_partially_consumed_stack() {
        let code = module(3, 2, &[0x01, 0, 0, 0, 0, 0, 0, 0, 7, 0x05, 0x00]);
        let result = execute_nvm1_core(&code).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::StackUnderflow));
        assert!(result.stack.is_empty());
        assert_eq!(result.committed_storage, None);
    }

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
        assert_eq!(mismatch.halt, Nvm1Halt::Trap(Nvm1Trap::TypeMismatch));

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
            Nvm1Halt::Trap(Nvm1Trap::ArithmeticOverflowOrUnderflow)
        );
    }

    #[test]
    fn core_enforces_declared_stack_bound() {
        let bound = module(3, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x04, 0x00]);
        assert_eq!(
            execute_nvm1_core(&bound).unwrap().halt,
            Nvm1Halt::Trap(Nvm1Trap::StackOverflow)
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

        let result = execute_nvm1_core_with_context(&module(10, 3, &bytes), &context).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(
            result.stack,
            vec![Nvm1Value::Bytes32(old_value), Nvm1Value::Bytes32([0u8; 32])]
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
    fn keccak256_hashes_exact_bytes32_and_rejects_wrong_operands() {
        let input = [0u8; 32];
        let mut bytes = vec![0x02];
        bytes.extend_from_slice(&input);
        bytes.extend_from_slice(&[0x30, 0x00]);
        let result = execute_nvm1_core(&module(3, 1, &bytes)).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Stop);
        assert_eq!(
            result.stack,
            vec![Nvm1Value::Bytes32([
                0x29, 0x0d, 0xec, 0xd9, 0x54, 0x8b, 0x62, 0xa8, 0xd6, 0x03, 0x45, 0xa9, 0x88, 0x38,
                0x6f, 0xc8, 0x4b, 0xa6, 0xbc, 0x95, 0x48, 0x40, 0x08, 0xf6, 0x36, 0x2f, 0x93, 0x16,
                0x0e, 0xf3, 0xe5, 0x63,
            ])]
        );

        assert!(matches!(
            execute_nvm1_core(&module(1, 1, &[0x30])).unwrap().halt,
            Nvm1Halt::Trap(_)
        ));
        assert!(matches!(
            execute_nvm1_core(&module(3, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 1, 0x30, 0x00]))
                .unwrap()
                .halt,
            Nvm1Halt::Trap(_)
        ));
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
        let mut value = [0u8; 32];
        value[16..].copy_from_slice(&42u128.to_be_bytes());
        assert_eq!(
            result.stack,
            vec![Nvm1Value::U64(5), Nvm1Value::Bytes32(value)]
        );
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
            Nvm1Halt::Trap(Nvm1Trap::InputOutOfRange)
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
            Nvm1Halt::Trap(Nvm1Trap::MemoryLimitExceeded)
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
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::StepLimitExceeded));
    }

    #[test]
    fn core_traps_on_fallthrough() {
        let result = execute_nvm1_core(&module(1, 1, &[0x01, 0, 0, 0, 0, 0, 0, 0, 1])).unwrap();
        assert_eq!(result.halt, Nvm1Halt::Trap(Nvm1Trap::FellOffEnd));
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
