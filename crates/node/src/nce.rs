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

#[derive(Clone, Copy)]
pub struct NceReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> NceReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "NCE/1 length overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("truncated NCE/1 value".into());
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn head(&mut self) -> Result<(u8, u64), String> {
        let first = self.take(1)?[0];
        let major = first >> 5;
        let add = first & 0x1f;

        let value = match add {
            0..=23 => add as u64,
            24 => {
                let v = self.take(1)?[0] as u64;
                if v < 24 {
                    return Err("non-canonical NCE/1 integer/length encoding".into());
                }
                v
            }
            25 => {
                let v = u16::from_be_bytes(self.take(2)?.try_into().unwrap()) as u64;
                if v <= 0xff {
                    return Err("non-canonical NCE/1 integer/length encoding".into());
                }
                v
            }
            26 => {
                let v = u32::from_be_bytes(self.take(4)?.try_into().unwrap()) as u64;
                if v <= 0xffff {
                    return Err("non-canonical NCE/1 integer/length encoding".into());
                }
                v
            }
            27 => {
                let v = u64::from_be_bytes(self.take(8)?.try_into().unwrap());
                if v <= 0xffff_ffff {
                    return Err("non-canonical NCE/1 integer/length encoding".into());
                }
                v
            }
            _ => return Err("indefinite or reserved NCE/1 additional information".into()),
        };

        Ok((major, value))
    }

    pub fn unsigned(&mut self) -> Result<u64, String> {
        let (major, value) = self.head()?;
        if major != 0 {
            return Err("expected NCE/1 unsigned integer".into());
        }
        Ok(value)
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], String> {
        let (major, len) = self.head()?;
        if major != 2 {
            return Err("expected NCE/1 byte string".into());
        }
        let len = usize::try_from(len)
            .map_err(|_| "NCE/1 byte string length exceeds platform limits".to_string())?;
        self.take(len)
    }

    pub fn map_len(&mut self) -> Result<usize, String> {
        let (major, len) = self.head()?;
        if major != 5 {
            return Err("expected NCE/1 map".into());
        }
        usize::try_from(len).map_err(|_| "NCE/1 map length exceeds platform limits".to_string())
    }
}

pub fn decode_envelope<'a>(
    bytes: &'a [u8],
    expected_object_type: u64,
    expected_schema_version: u64,
) -> Result<NceReader<'a>, String> {
    let mut reader = NceReader::new(bytes);
    let len = reader.map_len()?;
    if len != 4 {
        return Err("NCE/1 envelope must contain exactly four fields".into());
    }

    if reader.unsigned()? != 1 || reader.unsigned()? != NCE_VERSION {
        return Err("invalid NCE/1 envelope version field".into());
    }
    if reader.unsigned()? != 2 || reader.unsigned()? != expected_object_type {
        return Err("unexpected NCE/1 envelope object type".into());
    }
    if reader.unsigned()? != 3 || reader.unsigned()? != expected_schema_version {
        return Err("unexpected NCE/1 envelope schema version".into());
    }
    if reader.unsigned()? != 4 {
        return Err("invalid NCE/1 envelope payload field".into());
    }

    Ok(reader)
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
    fn decoder_rejects_noncanonical_unsigned_encodings() {
        let mut r = NceReader::new(&[0x18, 0x17]);
        assert!(r.unsigned().is_err());

        let mut r = NceReader::new(&[0x19, 0x00, 0xff]);
        assert!(r.unsigned().is_err());

        let mut r = NceReader::new(&[0x1a, 0x00, 0x00, 0xff, 0xff]);
        assert!(r.unsigned().is_err());
    }

    #[test]
    fn decoder_reads_definite_bytes_and_rejects_truncation() {
        let encoded = encode_bytes(&[0xaa, 0xbb]);
        let mut r = NceReader::new(&encoded);
        assert_eq!(r.bytes().unwrap(), &[0xaa, 0xbb]);
        assert!(r.finished());

        let mut r = NceReader::new(&[0x42, 0xaa]);
        assert!(r.bytes().is_err());
    }

    #[test]
    fn envelope_decoder_validates_identity_and_leaves_payload_reader() {
        let payload = encode_map(&[(1, encode_unsigned(42))]).unwrap();
        let encoded = encode_envelope(0x0010, 1, payload).unwrap();

        let mut r = decode_envelope(&encoded, 0x0010, 1).unwrap();
        assert_eq!(r.map_len().unwrap(), 1);
        assert_eq!(r.unsigned().unwrap(), 1);
        assert_eq!(r.unsigned().unwrap(), 42);
        assert!(r.finished());

        assert!(decode_envelope(&encoded, 0x0011, 1).is_err());
        assert!(decode_envelope(&encoded, 0x0010, 2).is_err());
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
