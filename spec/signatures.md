# Signatures and Signing Domains

## Status

Draft — Protocol v1 foundation.

This specification defines how NIAHCIA signs protocol objects and messages.

## Design goals

- EVM-native controller compatibility
- deterministic verification
- replay protection
- separation between protocol authorization and transport identity
- support for offline/hardware wallets
- no raw private keys given to AI models or compute workers unless the key belongs specifically to that worker identity

## Controller signatures

Protocol v1 controller/account signatures use secp256k1 ECDSA.

Requirements common to all V1 secp256k1 signatures:

- low-`s` canonical signatures;
- valid `r` and `s` ranges;
- signatures over a 32-byte domain-separated signing digest;
- verification against the signer/controller identity required by the schema.

V1 supports two schema-selected verification profiles.

### Explicit-key profile

When the schema carries the signer's canonical secp256k1 public key, the
signature uses the fixed-width 64-byte representation `r || s`, where each
component is exactly 32-byte unsigned big-endian.

No recovery identifier is serialized because the public key is supplied
directly.

NativeTransactionV1 uses this profile with a 65-byte uncompressed SEC1 public
key (`0x04 || X || Y`) and a 64-byte `r || s` signature.

### Recoverable profile

A schema requiring public-key/address recovery MAY define a recoverable
secp256k1 signature containing a normalized recovery identifier.

The recovery identifier is required only when the schema explicitly selects
this profile. It MUST NOT be silently appended to an explicit-key signature.

The exact recoverable wire representation MUST be specified by the schema
before that schema is interoperability-locked.

## Signing digest

A signature NEVER signs arbitrary JSON text.

The generic signing digest is:

```text
signing_digest =
  keccak256(
    "NIAHCIA" ||
    0x00 ||
    signing_purpose ||
    0x00 ||
    network_id ||
    0x00 ||
    canonical_signing_payload
  )
```

`canonical_signing_payload` uses NCE/1.

## Replay protection

Every authorization that can be replayed MUST include one or more of:

- monotonic nonce
- object ID
- job ID
- validity start
- expiry/deadline
- chain/network ID
- session ID

A signature without sufficient replay context is invalid for state-changing authorization.

## Signature envelope

The generic signature envelope is:

```text
SignatureEnvelope
- schema_version
- algorithm
- signer
- signing_purpose
- network_id
- nonce
- valid_from
- expires_at
- signature
```

Not every field is required by every message; the signed schema defines the exact set.

## EIP-712 interoperability

Wallet-facing applications MAY expose an EIP-712 representation so users can read what they are signing.

Where EIP-712 is used, the signed EIP-712 data MUST commit to the same protocol-critical values as the NCE/1 signing payload.

An implementation MUST NOT maintain two semantically different authorization formats for the same action.

The protocol's canonical binary representation remains NCE/1; EIP-712 is a wallet UX/interoperability surface.

## Worker identities

A ComputeWorker has a registered controller/payment identity and MAY have a separate operational signing key.

Recommended v1 structure:

- controller: secp256k1 EVM-compatible identity
- worker operational key: secp256k1 v1 default
- P2P transport key: implementation-specific and not automatically trusted for economic authorization

The registry binds an operational worker key to the worker ID/controller.

Rotating the operational key does not change the Worker ID.

## Service-node identities

ServiceNode signing follows the same pattern:

- stable service_node_id
- registered controller
- rotatable operational signing key
- separately rotatable P2P transport identity

## Transport signatures

Libp2p or other transport-level authentication is not a substitute for NIAHCIA protocol signatures.

Economically meaningful messages such as:

- worker advertisements
- job acceptance
- result commitments
- service availability claims
- capability grants

MUST carry protocol-level signatures where their respective specification requires them.

## Signature coverage

A signature MUST cover every field that could change the meaning, recipient, cost, execution target, validity period, or authorization scope of the signed action.

Unsigned metadata MUST NOT be allowed to alter signed semantics.

## Multisig and contract controllers

An Agent/Operator/controller MAY be an EVM smart contract.

For contract-controlled authorization, implementations SHOULD support ERC-1271-style signature validation or an equivalent contract verification path.

The protocol treats the controller as authoritative; it does not require all controllers to be EOAs.

## Key compromise and rotation

Stable object IDs MUST survive operational-key rotation.

Registries SHOULD support:

- controller-authorized key rotation
- explicit activation block/nonce
- revocation of old operational keys
- audit history

Previously valid historical signatures remain attributable to the key that was authorized at the time.
