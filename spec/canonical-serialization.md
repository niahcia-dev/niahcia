# Canonical Serialization

## Status

Draft — Protocol v1 foundation.

This specification defines the canonical byte representation used when NIAHCIA protocol objects are hashed, signed, compared, or content-addressed.

The goal is simple:

> The same logical protocol object MUST produce the same canonical bytes on every conforming implementation.

## Encoding choice

Protocol v1 uses **deterministic CBOR** as defined by RFC 8949 deterministic encoding rules, with additional NIAHCIA restrictions in this document.

The canonical encoding is referred to as:

```text
NCE/1 — NIAHCIA Canonical Encoding version 1
```

CBOR is used because it is compact, binary, widely implemented, language-neutral, and supports deterministic map ordering without making JSON text formatting part of protocol identity.

JSON MAY be used for APIs, debugging, RPC, explorer display, and developer tooling, but JSON text is never the canonical hashed representation of a protocol object.

## NCE/1 restrictions

### Maps

- map keys MUST be unsigned integer field identifiers
- map keys MUST be encoded in deterministic ascending canonical order
- duplicate keys are invalid
- unknown keys are handled according to the schema version for that object

Human-readable field names are documentation/API concerns, not canonical wire keys.

Example conceptual mapping:

```text
1 -> schema_version
2 -> agent_id
3 -> creator
4 -> controller
...
```

Each object specification will define its permanent numeric field IDs before the schema is considered stable.

### Integers

- unsigned values MUST use the shortest valid CBOR unsigned representation
- signed values MUST use the shortest valid CBOR signed representation
- arbitrary floating-point values MUST NOT appear in consensus-critical or signed protocol objects unless a later specification explicitly defines their exact representation

Prices, probabilities, rates, percentages, and token amounts MUST use fixed-point integers or rational representations defined by the relevant schema.

### Byte strings

Hashes, addresses, keys, signatures, and binary identifiers are encoded as CBOR byte strings, not hex strings.

Hex is display-only.

### Text strings

- MUST be UTF-8
- MUST be valid Unicode scalar values
- MUST use Unicode Normalization Form C (NFC) before canonical encoding when the field is declared canonical text
- fields intended only for display SHOULD NOT participate in execution-critical hashes unless explicitly required

### Arrays

Array order is significant unless the field specification explicitly declares the collection to be an unordered set.

For unordered sets:

1. encode each element canonically
2. sort by raw encoded byte sequence in ascending lexicographic order
3. encode the resulting ordered array

An implementation MUST NOT rely on insertion order for an unordered set.

### Optional fields

NCE/1 distinguishes:

- field absent
- field present with an explicit value

`null` MUST NOT be used unless a schema explicitly allows null as a meaningful value.

If a field is optional and absent, the numeric map key is omitted entirely.

### Booleans

Only canonical CBOR `false` and `true` are valid.

### Tags

CBOR semantic tags are forbidden in NCE/1 unless explicitly assigned by a future NIAHCIA specification.

### Indefinite-length values

Indefinite-length maps, arrays, byte strings, and text strings are forbidden.

All lengths MUST be definite.

## Schema envelope

Every canonical top-level protocol object is encoded inside a logical envelope:

```text
{
  1: encoding_version,
  2: object_type,
  3: schema_version,
  4: payload
}
```

Where:

- `encoding_version` is `1` for NCE/1
- `object_type` is the permanent numeric NIAHCIA object-type code
- `schema_version` is the object's schema version
- `payload` is the canonical object map

The envelope prevents an identical payload from being interpreted as another object class.

## Excluded derived fields

Fields derived from the canonical object itself MUST NOT be included in the bytes from which they are derived.

Examples:

- `agent_id` is excluded if that particular Agent ID is content-derived
- `profile_hash` is excluded from the bytes used to derive `profile_hash`
- signatures are excluded from the digest they sign
- `commitment_id` is excluded from the bytes used to derive it

Each object specification MUST explicitly state its hash preimage fields.

## Canonical comparison

Protocol equality for hashed objects is equality of canonical bytes or equality of the canonical digest derived from those bytes.

Pretty-printed JSON, database row order, RPC field order, whitespace, or language-specific object layout has no protocol meaning.

## Compatibility

A parser MAY understand future schema versions, but it MUST NOT silently reinterpret unknown fields in a signed/hash-critical object.

If an implementation cannot determine the canonical interpretation of an object version, it MUST reject that object for verification purposes.

## Test vectors

Before Protocol v1 serialization is frozen, the repository MUST include machine-readable test vectors containing:

- logical object fixture
- canonical CBOR bytes
- canonical digest
- expected ID
- expected signing digest where applicable

Independent implementations MUST reproduce the vectors exactly.
