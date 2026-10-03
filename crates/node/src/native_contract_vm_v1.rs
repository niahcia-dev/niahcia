pub const NVM1_RUNTIME_ID: u32 = 1;
pub const NVM1_CODE_FORMAT_VERSION: u32 = 1;
pub const NVM1_MAGIC: [u8; 4] = *b"NVM1";

pub const NVM1_MAX_INSTRUCTIONS: u32 = 16_384;
pub const NVM1_MAX_STACK_ITEMS: u16 = 256;

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

#[cfg(test)]
mod tests {
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
