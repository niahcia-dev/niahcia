# NIAHCIA Native RPC V1

Status: **pre-alpha interface contract**

This document defines the initial NIAHCIA-owned JSON-RPC namespace. It deliberately separates native chain/account interfaces from the `pow_*` mining interface and from Reth's execution-layer RPC.

## Namespace

Native NIAHCIA methods use the `niah_` prefix.

The prefix is protocol-owned. New public NIAHCIA node methods should not use `eth_` names unless the node is intentionally exposing an unmodified execution-layer compatibility endpoint.

Mining methods remain in the `pow_` namespace.

## `niah_getNetworkInfo`

Returns basic native-chain identity and address-format information.

Parameters: none.

Result fields:

- `protocol`: `"NIAHCIA"`
- `network`: native network name (`mainnet`, `testnet`, or `devnet`)
- `address_version`: currently `1`
- `address_hrp`: `niah`, `tniah`, or `dniah`
- `native_address_prefix`: `niah1`, `tniah1`, or `dniah1`

Implementations may add non-consensus informational fields in later revisions. Existing field meanings must not be changed silently.

Example devnet result:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocol": "NIAHCIA",
    "network": "devnet",
    "address_version": 1,
    "address_hrp": "dniah",
    "native_address_prefix": "dniah1"
  }
}
```

## `niah_validateAddress`

Validates and decodes a native NIAHCIA address without converting the user-facing representation to an execution-layer hexadecimal string.

Parameters:

```json
{
  "address": "dniah1..."
}
```

A syntactically valid native address returns:

- `valid`: `true`
- `address`: canonical native string
- `version`: address version
- `network`: decoded network
- `kind`: `account` or `contract`
- `payload`: lowercase hexadecimal representation of the 20-byte payload, without `0x`; this is diagnostic/interoperability data, not the preferred display address
- `network_match`: whether the decoded address belongs to the node's configured network

Example shape:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "valid": true,
    "address": "dniah1...",
    "version": 1,
    "network": "devnet",
    "kind": "account",
    "payload": "0000000000000000000000000000000000000000",
    "network_match": true
  }
}
```

Malformed input is not a JSON-RPC transport failure. The method returns a validation result with `valid: false` and a stable machine-readable reason. Implementations must not reinterpret malformed `niah1`, `tniah1`, or `dniah1` strings as legacy hexadecimal addresses.

Initial reason identifiers:

- `invalid_checksum`
- `unknown_network`
- `invalid_length`
- `unsupported_version`
- `unsupported_kind`
- `malformed_address`

The human-readable error text may evolve; callers should use the reason identifier.

## Network safety

`niah_validateAddress` distinguishes structural validity from suitability for the current node. A valid mainnet address queried against a devnet node remains structurally valid but returns `network_match: false`.

Operations that can move value, set a fee recipient, deploy a contract, or otherwise commit an address to node state MUST reject a network mismatch rather than relying only on structural validity.

## Address presentation

Public NIAHCIA interfaces should return the native Address V1 string wherever an account or contract address is intended for people, wallets, explorers, or NIAHCIA applications.

The underlying 20-byte payload may be exposed where useful for debugging and execution interoperability, but must not be presented as the canonical NIAHCIA address.

## Execution-layer separation

Reth remains the execution engine and may expose Ethereum-compatible RPC independently. Those compatibility methods do not define NIAHCIA's public protocol naming.

NIAHCIA-owned chain, account, service-node, storage, AI-job, and protocol methods belong under NIAHCIA namespaces rather than being disguised as `eth_*` methods.

## Mining separation

`pow_getWork` and `pow_submitWork` remain mining-specific. Wallet/address discovery and validation are intentionally not added to mining work responses.

This keeps the mining protocol small and avoids coupling miners to wallet-facing APIs.

## Versioning rule

V1 method names and existing result-field meanings become compatibility contracts once implemented. Additive fields are permitted. Breaking semantic changes require a new explicitly versioned method or protocol revision.
