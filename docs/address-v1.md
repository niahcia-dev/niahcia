# NIAHCIA Address V1

Status: **locked for pre-alpha interoperability**

This document specifies the canonical human-readable NIAHCIA Address V1 encoding used by nodes, wallets, miners, explorers, SDKs, and other implementations.

## Goals

NIAHCIA addresses are network-explicit, checksummed, human-readable identifiers. The native string form is the user-facing representation. Execution-layer 20-byte hexadecimal addresses are an internal interoperability detail and should not replace the native representation in NIAHCIA user interfaces.

## Encoding

Address V1 uses **Bech32m**.

The encoded data payload is exactly 22 bytes:

| Offset | Size | Field |
| --- | ---: | --- |
| 0 | 1 byte | address version |
| 1 | 1 byte | address kind |
| 2 | 20 bytes | address payload |

The V1 version byte is `0x01`.

Address kinds:

- `0x00` — Account
- `0x01` — Contract

No other V1 kind values are valid.

## Network prefixes

The human-readable part (HRP) identifies the NIAHCIA network:

| Network | HRP | Address prefix |
| --- | --- | --- |
| Mainnet | `niah` | `niah1...` |
| Testnet | `tniah` | `tniah1...` |
| Devnet | `dniah` | `dniah1...` |

An address for one network MUST NOT be accepted as an address for another network when the expected network is known.

## Account derivation

For V1 account addresses, the 20-byte payload is derived from the account public key by hashing the public-key bytes with Keccak-256 and taking the final 20 bytes of the digest.

The native address then encodes:

`0x01 || 0x00 || account_payload_20`

using Bech32m with the HRP for the selected network.

## Contract addresses

A V1 contract address encodes:

`0x01 || 0x01 || contract_payload_20`

using Bech32m with the HRP for the selected network.

The deterministic derivation of `contract_payload_20` is outside the scope of this address-container specification and must be defined by the contract-creation protocol before contract deployment is enabled.

## Validation

A conforming V1 decoder MUST reject an address when any of the following is true:

1. The Bech32m checksum is invalid.
2. The HRP is not a recognized NIAHCIA network HRP.
3. The decoded payload is not exactly 22 bytes.
4. The version byte is not `0x01`.
5. The kind byte is not a defined V1 address kind.
6. The address network does not match a network required by the calling context.
7. The address kind does not match a kind required by the calling context.

Decoders must not silently reinterpret malformed native addresses as hexadecimal addresses or as addresses from another network.

## Mining fee recipients

NIAHCIA mining configuration may use a native account address. On devnet, a native mining fee recipient must therefore be a `dniah1...` **Account** address.

Reth's Engine API consumes the underlying 20-byte execution address. The NIAHCIA node decodes the native address and passes that payload internally to Reth. This conversion does not change the canonical user-facing NIAHCIA address.

Legacy 20-byte hexadecimal fee-recipient configuration remains temporarily supported during pre-alpha devnet migration. It is not the preferred NIAHCIA user-facing address format.

## Canonical interoperability vectors

The Rust implementation in `crates/node/src/address.rs` contains canonical byte-for-byte V1 vectors covering all six network/kind combinations:

- Mainnet Account
- Mainnet Contract
- Testnet Account
- Testnet Contract
- Devnet Account
- Devnet Contract

Those vectors are normative interoperability fixtures. Implementations in other languages must reproduce the exact same encoded strings for the same version, kind, network, and 20-byte payload before being considered V1-compatible.

## Compatibility rule

The meaning of an already-issued V1 address must never be changed. Any future change that alters payload length, kind semantics, derivation semantics, or encoding must use a new address version or another explicitly versioned format rather than silently changing V1.
