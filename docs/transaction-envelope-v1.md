# NIAHCIA TransactionEnvelopeV1

Status: **pre-alpha design lock for implementation**

This document defines the first native NIAHCIA transaction envelope. It separates the user-facing NIAHCIA transaction identity and signing domain from the underlying execution engine.

## Design goals

TransactionEnvelopeV1 MUST:

- use native NIAHCIA addresses at wallet and RPC boundaries;
- be network-bound so a signed transaction cannot be replayed across mainnet, testnet, and devnet;
- be chain-bound so the same signed bytes cannot be replayed onto an unrelated chain using the format;
- use deterministic canonical serialization;
- separate signing bytes from the signature itself;
- support ordinary account-to-account value transfer first;
- leave room for contract calls without changing the V1 envelope container;
- keep Reth/EVM transaction representation behind the NIAHCIA node boundary.

## Envelope fields

The canonical V1 logical fields are:

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | `u8` | Envelope version; MUST be `1` |
| `network` | `u8` | NIAHCIA network identifier |
| `chain_id` | `u64` | NIAHCIA chain identifier |
| `nonce` | `u64` | Sender account sequence number |
| `from` | 20 bytes | Sender account payload |
| `to_kind` | `u8` | Destination kind: Account or Contract |
| `to` | 20 bytes | Destination address payload |
| `value` | `u128` | Native value in the smallest denomination |
| `gas_limit` | `u64` | Maximum execution gas |
| `max_fee_per_gas` | `u128` | Maximum fee per gas unit |
| `data_len` | `u32` | Number of bytes in `data` |
| `data` | bytes | Empty for a basic transfer; calldata for execution |
| `signature_scheme` | `u8` | Signature algorithm identifier |
| `signature_len` | `u16` | Number of signature bytes |
| `signature` | bytes | Signature over the V1 signing digest |

The sender is always an Account address in V1. The destination may be an Account or Contract address.

## Canonical serialization

Numeric integers are encoded in fixed-width big-endian form unless explicitly length-prefixed above. No field may have more than one binary representation.

The unsigned signing body is the serialization of every field from `version` through `data`, excluding `signature_scheme`, `signature_len`, and `signature`.

The signing digest is:

`keccak256("NIAHCIA_TX_V1" || unsigned_body)`

The ASCII domain separator is part of the protocol and MUST be included exactly as written.

The signed transaction ID is:

`keccak256("NIAHCIA_TXID_V1" || complete_signed_envelope)`

This gives the native transaction a NIAHCIA-owned identity even when execution is delegated internally.

## Network identifiers

V1 reserves:

- `0x00` — Mainnet
- `0x01` — Testnet
- `0x02` — Devnet

A node MUST reject a transaction whose network identifier does not match the node network.

## Chain ID

`chain_id` is a consensus parameter, not a wallet preference. Exact production chain IDs will be locked separately before public testnet/mainnet. Devnet implementations MUST use the configured/consensus devnet chain ID and MUST reject mismatches.

## Address rules

RPC and wallet APIs should accept native `niah1...`, `tniah1...`, or `dniah1...` addresses. Before constructing the envelope, the node or wallet decodes Address V1 and places the 20-byte payload plus destination kind into the canonical binary envelope.

The `from` address MUST be an Account address and MUST correspond to the public key that validates the signature.

The address network MUST match the transaction network.

## Nonce and replay protection

Each sender has a monotonically increasing account nonce. A transaction is valid only for the sender's current expected nonce according to canonical state.

Replay protection therefore has three independent layers:

1. network identifier;
2. chain ID;
3. sender nonce.

A reorganization may return a previously included transaction to the pending state only if its nonce and all other validity rules remain satisfied on the new canonical chain.

## Signature schemes

The envelope includes an explicit `signature_scheme` byte so signature evolution does not require changing the envelope container.

The first enabled scheme will be locked with canonical cryptographic test vectors before transaction submission is enabled. Unknown or disabled schemes MUST be rejected.

No implementation should infer the signature scheme from signature length.

## Basic transfer

The first transaction type implemented should be a native account-to-account transfer:

- `to_kind = Account`;
- `data_len = 0`;
- `data` is empty;
- `value` may be nonzero.

Contract execution should be enabled only after the basic transfer path, signing, nonce validation, fee validation, persistence, propagation, reorg behavior, and restart recovery are tested.

## Execution boundary

NIAHCIA RPC clients sign and submit TransactionEnvelopeV1, not an Ethereum-branded transaction API.

The NIAHCIA node validates the native envelope and may translate an accepted transaction into the representation required by the configured execution engine. That internal translation MUST NOT change the native signing digest or transaction ID.

Reth is therefore an execution implementation behind the NIAHCIA protocol boundary rather than the owner of the public transaction format.

## Required implementation tests

Before `niah_sendRawTransaction` is enabled, tests MUST cover at least:

- canonical serialization vector;
- canonical signing-digest vector;
- canonical transaction-ID vector;
- valid signature;
- altered-field signature failure;
- wrong-network rejection;
- wrong-chain-ID rejection;
- sender/address mismatch rejection;
- stale nonce rejection;
- future nonce handling;
- insufficient-balance rejection;
- fee overflow/underflow boundaries;
- duplicate transaction submission;
- restart persistence;
- P2P propagation;
- reorg return-to-pending behavior.

## Compatibility rule

Once canonical V1 vectors are locked, their meaning and byte serialization MUST NOT change. An incompatible transaction-format change requires a new envelope version or a separately versioned transaction family.
