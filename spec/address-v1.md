# NIAHCIA Address V1

Status: **LOCKED for V1 interoperability**

NIAHCIA Address V1 is the canonical human-readable address container used by nodes, wallets, miners, explorers, SDKs, and other implementations.

## Encoding

Address V1 uses **Bech32m**. The decoded payload is exactly 22 bytes:

```text
offset  size  field
0       1     version
1       1     kind
2       20    payload
```

V1 version is `0x01`.

Kinds:

```text
0x00 Account
0x01 Contract
```

No other V1 kind is valid.

## Network HRPs

```text
mainnet  niah   niah1...
testnet  tniah  tniah1...
devnet   dniah  dniah1...
```

When the expected network is known, an address from another network is invalid.

## Account payload derivation

For a V1 Account, the secp256k1 public key MUST first be represented as the
65-byte uncompressed SEC1 encoding `0x04 || X || Y`.

The `0x04` SEC1 prefix MUST NOT be included in the hash input. The account
payload is derived as:

    digest = Keccak-256(X || Y)
    account_payload_20 = digest[12..32]

`X || Y` is exactly the 64-byte concatenation of the secp256k1 coordinates.

The native address container encodes:

    0x01 || 0x00 || account_payload_20

using Bech32m and the network HRP.

Compressed SEC1 public-key encoding is not a valid input to the Address V1
account-derivation procedure.

## Contract addresses

A V1 Contract uses kind:

    0x01

and has a native deterministic 20-byte contract payload.

The native address container is:

    0x01 || 0x01 || contract_payload_20

encoded with Bech32m and the HRP for the selected network.

## Contract payload derivation

A ContractCreate transaction derives its contract payload from the
authenticated creator and native transaction context.

Define:

    contract_digest =
        Keccak-256(
            "NIAHCIA/CONTRACT/V1" ||
            0x00 ||
            network_id ||
            0x00 ||
            chain_id_u64_be ||
            0x00 ||
            creator_payload_20 ||
            0x00 ||
            creator_nonce_u64_be
        )

    contract_payload_20 = contract_digest[12..32]

The inputs are exact:

- `"NIAHCIA/CONTRACT/V1"` is the literal ASCII byte sequence;
- each shown `0x00` is exactly one zero byte;
- `network_id` is the canonical network identifier byte sequence defined by
  Network Parameters V1;
- `chain_id_u64_be` is exactly 8 bytes, unsigned big-endian;
- `creator_payload_20` is exactly the authenticated 20-byte Account Address V1
  payload of the transaction sender;
- `creator_nonce_u64_be` is exactly 8 bytes, unsigned big-endian, containing
  the ContractCreate transaction sender nonce.

The Bech32m address text is never part of the derivation input.

The resulting Contract Address V1 is obtained by encoding:

    0x01 || 0x01 || contract_payload_20

with the HRP for the transaction network.

Contract derivation is therefore separated by network, chain, authenticated
creator, and creator transaction nonce.

Registry nonces, object nonces, job nonces, storage nonces, service nonces, and
other protocol nonce domains MUST NOT be substituted for the native account
transaction nonce.

## ContractCreate relationship

Native Transaction V1 ContractCreate serializes no target contract payload.

The contract payload is derived before contract initialization using this
specification.

Derivation of an address does not itself create persistent contract state.
Persistent state exists only when the native state-transition and active
contract-runtime rules commit the creation successfully.

## Validation

A V1 decoder rejects:

1. invalid Bech32m checksum;
2. unknown NIAHCIA HRP;
3. decoded payload not exactly 22 bytes;
4. version other than `0x01`;
5. undefined V1 kind;
6. network mismatch where a network is required;
7. kind mismatch where a kind is required.

Malformed native addresses MUST NOT be silently reinterpreted as hexadecimal
or as another network address.

## Native fee recipients

A native mining fee recipient is an Account Address V1 for the selected
network.

Consensus configuration and user-facing configuration SHOULD use canonical
Address V1 text.

## Interoperability vectors

The existing locked Address V1 container and Account derivation vectors remain
unchanged.

Before native ContractCreate derivation is interoperability-locked, canonical
vectors MUST additionally cover:

1. the complete contract-derivation preimage;
2. the 32-byte contract digest;
3. the resulting 20-byte contract payload;
4. the resulting Bech32m Contract Address V1;
5. mainnet, testnet, and devnet;
6. multiple creator payloads;
7. multiple creator nonces;
8. multiple native chain IDs.

## Compatibility rule

An already-defined V1 address MUST never be silently reinterpreted.

Changes to payload length, kind semantics, Account derivation, Contract
derivation, checksum encoding, or network meaning require an explicitly
versioned protocol transition.

The locked Address V1 container and Account derivation remain unchanged.
