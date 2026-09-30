# NIAHCIA SignatureSchemeV1

Status: **pre-alpha design lock for implementation**

This document defines signature scheme identifier `0x01` for native NIAHCIA TransactionEnvelopeV1 account transactions.

## Scheme identifier

`signature_scheme = 0x01` means:

**secp256k1 ECDSA with recoverable sender identity and canonical low-S signatures.**

The explicit scheme byte is part of TransactionEnvelopeV1 so future schemes can be introduced without silently changing V1 signature semantics.

## Curve and keys

Scheme `0x01` uses the SEC 2 `secp256k1` elliptic curve.

Private keys are 32-byte scalars in the valid secp256k1 private-key range.

For account derivation, the canonical public-key input is the 64-byte uncompressed affine public key body:

`X[32] || Y[32]`

The SEC1 `0x04` uncompressed prefix is not included in the bytes hashed for address derivation.

The account payload is:

`last20(keccak256(X || Y))`

The user-facing address is then the Address V1 Account encoding for the selected NIAHCIA network.

## Signing digest

TransactionEnvelopeV1 defines the unsigned transaction body and signing domain.

For scheme `0x01`, ECDSA signs exactly the 32-byte digest:

`keccak256("NIAHCIA_TX_V1" || unsigned_body)`

Implementations MUST NOT apply an additional message prefix, personal-sign prefix, JSON wrapper, or second application-level hash before ECDSA signing.

## Signature encoding

The canonical scheme-`0x01` signature is exactly 65 bytes:

`r[32] || s[32] || recovery_id[1]`

`r` and `s` are unsigned fixed-width 32-byte big-endian integers.

`recovery_id` MUST be a canonical recovery identifier accepted by the NIAHCIA secp256k1 implementation. TransactionEnvelopeV1 chain/network replay protection is carried by the signed body; the recovery byte MUST NOT be overloaded with chain-ID arithmetic.

The envelope therefore contains:

- `signature_scheme = 0x01`
- `signature_len = 65`
- `signature = r || s || recovery_id`

Any other signature length for scheme `0x01` MUST be rejected.

## Low-S rule

Signatures MUST use canonical low-S form.

Let `n` be the secp256k1 group order. A signature is canonical only when:

`1 <= s <= floor(n / 2)`

A node MUST reject a high-S signature rather than normalizing it after transaction submission. Wallets/signers are responsible for producing low-S signatures.

This removes ECDSA signature malleability caused by the `(r, s)` / `(r, n-s)` equivalence.

## Sender recovery and authorization

A validating node:

1. reconstructs the TransactionEnvelopeV1 unsigned body;
2. computes the NIAHCIA V1 signing digest;
3. verifies the canonical signature encoding and low-S rule;
4. recovers the secp256k1 public key using the recovery identifier;
5. derives `last20(keccak256(X || Y))` from the recovered public key;
6. requires that derived payload to equal the envelope `from` payload.

A cryptographically valid signature from a different account MUST be rejected.

The `from` address therefore cannot be chosen independently of the signing key.

## Network and chain binding

The signature authenticates the complete unsigned TransactionEnvelopeV1 body, including its network identifier and chain ID.

A signature valid for one NIAHCIA network or chain ID MUST fail authorization if either field is changed.

Nodes additionally enforce that the transaction network and chain ID match their own consensus configuration.

## Deterministic signing

Wallet implementations SHOULD use deterministic ECDSA nonce generation so signing the same digest with the same private key does not depend on external randomness.

Canonical interoperability vectors will lock the exact expected signature behavior used by the reference implementation before native transaction submission is enabled.

## Transaction identity

The transaction ID is not the ECDSA signature hash. TransactionEnvelopeV1 defines it independently as:

`keccak256("NIAHCIA_TXID_V1" || complete_signed_envelope)`

Because nodes reject noncanonical/high-S signatures, one authorized V1 transaction has one canonical signed representation for a given signing result.

## Future signature schemes

SignatureSchemeV1 does not reserve all future account identities to secp256k1.

Additional schemes may be assigned new `signature_scheme` identifiers after their account-binding, verification, canonical encoding, hardware support, and execution implications are specified and tested.

In particular, device-backed/passkey-oriented signatures may be introduced later without redefining scheme `0x01`.

Unknown or disabled signature-scheme identifiers MUST be rejected.

## Required canonical vectors

Before `niah_sendRawTransaction` is enabled, the reference implementation MUST lock vectors containing at least:

- private key;
- canonical 64-byte public-key body;
- expected Address V1 account payload;
- expected native mainnet/testnet/devnet addresses;
- unsigned TransactionEnvelopeV1 bytes;
- signing digest;
- canonical `r`;
- canonical low-S `s`;
- recovery identifier;
- complete 65-byte signature;
- recovered public key;
- recovered sender payload;
- complete signed envelope bytes;
- native transaction ID.

Negative vectors MUST include:

- high-S signature;
- invalid recovery identifier;
- altered `r` or `s`;
- wrong sender payload;
- changed network;
- changed chain ID;
- changed nonce;
- changed destination;
- changed value;
- changed calldata.

## Compatibility rule

The meaning and canonical encoding of signature scheme `0x01` MUST NOT change after its interoperability vectors are locked. An incompatible cryptographic change requires a new signature-scheme identifier and, if the transaction container itself changes, a new transaction-envelope version.
