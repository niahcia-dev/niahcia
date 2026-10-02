/// NIAHCIA Canonical Encoding version 1 (NCE/1).
///
/// This module implements the consensus-critical deterministic CBOR subset
/// required by native NIAHCIA protocol objects.
///
/// Supported values:
/// - unsigned integers
/// - definite-length byte strings
/// - definite-length arrays
/// - definite-length maps
///
/// Maps are emitted in caller-supplied order. Consensus callers MUST provide
/// keys in canonical ascending order.
pub const NCE_VERSION: u64 = 1;

fn encode_head(major: u8, value: u64, out: &mut Vec<u8>) {
    let prefix = major << 5;

    match value {
        0..=23 => out.push(prefix | value as u8),
        24..=0xff => {
            out.push(prefix | 24);
            out.push(value as u8);
        }
        0x100..=0xffff => {
            out.push(prefix | 25);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(prefix | 26);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push(prefix | 27);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

pub fn encode_unsigned(value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_head(0, value, &mut out);
    out
}

pub fn encode_bytes(value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    encode_head(2, value.len() as u64, &mut out);
    out.extend_from_slice(value);
    out
}

pub fn encode_array(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    encode_head(4, items.len() as u64, &mut out);

    for item in items {
        out.extend_from_slice(item);
    }

    out
}

pub fn encode_map(entries: &[(u64, Vec<u8>)]) -> Result<Vec<u8>, String> {
    for pair in entries.windows(2) {
        if pair[0].0 >= pair[1].0 {
            return Err("NCE/1 map keys must be strictly ascending".into());
        }
    }

    let mut out = Vec::new();
    encode_head(5, entries.len() as u64, &mut out);

    for (key, value) in entries {
        encode_head(0, *key, &mut out);
        out.extend_from_slice(value);
    }

    Ok(out)
}

pub fn encode_envelope(
    object_type: u64,
    schema_version: u64,
    payload: Vec<u8>,
) -> Result<Vec<u8>, String> {
    encode_map(&[
        (1, encode_unsigned(NCE_VERSION)),
        (2, encode_unsigned(object_type)),
        (3, encode_unsigned(schema_version)),
        (4, payload),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_boundaries_are_shortest_form() {
        assert_eq!(encode_unsigned(0), vec![0x00]);
        assert_eq!(encode_unsigned(23), vec![0x17]);
        assert_eq!(encode_unsigned(24), vec![0x18, 0x18]);
        assert_eq!(encode_unsigned(255), vec![0x18, 0xff]);
        assert_eq!(encode_unsigned(256), vec![0x19, 0x01, 0x00]);
        assert_eq!(encode_unsigned(65535), vec![0x19, 0xff, 0xff]);
        assert_eq!(encode_unsigned(65536), vec![0x1a, 0x00, 0x01, 0x00, 0x00]);
        assert_eq!(
            encode_unsigned(u32::MAX as u64),
            vec![0x1a, 0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(
            encode_unsigned(u32::MAX as u64 + 1),
            vec![0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn byte_strings_are_definite_length() {
        assert_eq!(encode_bytes(&[]), vec![0x40]);
        assert_eq!(encode_bytes(&[0xaa, 0xbb]), vec![0x42, 0xaa, 0xbb]);
    }

    #[test]
    fn arrays_are_definite_length() {
        let encoded = encode_array(&[encode_unsigned(1), encode_unsigned(2), encode_unsigned(3)]);

        assert_eq!(encoded, vec![0x83, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn map_keys_must_be_strictly_ascending() {
        assert!(encode_map(&[(1, encode_unsigned(10)), (2, encode_unsigned(20)),]).is_ok());

        assert!(encode_map(&[(2, encode_unsigned(20)), (1, encode_unsigned(10)),]).is_err());

        assert!(encode_map(&[(1, encode_unsigned(10)), (1, encode_unsigned(20)),]).is_err());
    }

    #[test]
    fn map_encoding_is_byte_exact() {
        let encoded = encode_map(&[(1, encode_unsigned(10)), (2, encode_bytes(&[0xaa]))]).unwrap();

        assert_eq!(encoded, vec![0xa2, 0x01, 0x0a, 0x02, 0x41, 0xaa]);
    }

    #[test]
    fn envelope_encoding_is_byte_exact() {
        let payload = encode_map(&[(1, encode_unsigned(42))]).unwrap();

        let encoded = encode_envelope(0x0010, 1, payload).unwrap();

        assert_eq!(
            encoded,
            vec![0xa4, 0x01, 0x01, 0x02, 0x10, 0x03, 0x01, 0x04, 0xa1, 0x01, 0x18, 0x2a,]
        );
    }
}
