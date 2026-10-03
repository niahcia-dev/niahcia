# NIAHCIA Native RPC V1

Status: **pre-alpha interface contract**

Native NIAHCIA methods use the `niah_` prefix. Mining methods remain under `pow_`.

## niah_getNetworkInfo

Returns protocol/network/address-format information such as protocol name, network, Address V1 version, HRP, and native address prefix.

## niah_validateAddress

Validates and decodes a native NIAHCIA address.

A successful result should expose the canonical native string, version, network, kind, 20-byte diagnostic payload, and whether the address matches the node's configured network.

Malformed input returns a stable validation result rather than being reinterpreted as another address format.

## Network safety

Operations that move value or commit an address to state must reject network mismatches.

## Address presentation

Public wallet/explorer/application interfaces should prefer native Bech32m Address V1 strings. The 20-byte payload is interoperability/debugging data, not the canonical user-facing address.

## Native execution separation

The active reference node uses native NIAHCIA execution. NIAHCIA-owned chain/account/service/AI interfaces belong under NIAHCIA namespaces rather than pretending to be Ethereum `eth_*` endpoints.

## Mining separation

`pow_getWork` and `pow_submitWork` remain mining-specific and intentionally separate from wallet/address RPC.

## Versioning

Once a V1 method is declared stable, existing meanings must not be changed silently. Breaking semantic changes require an explicit versioned revision.
