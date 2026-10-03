# NIAHCIA Block Header and Mining Work

## Purpose

NIAHCIA uses `BlockHeaderV1` as the native mining-work foundation.

## BlockHeaderV1

The current development header is a fixed 164-byte structure containing version, parent hash, height, timestamp, transaction root, execution root, target, nonce, and extra nonce.

The block ID belongs entirely to NIAHCIA.

## Mining template identity

Miners vary only the search fields allowed by the mining protocol. Template identity binds the non-search fields so stale work cannot silently mutate the transaction or execution commitments.

## Transaction root

The header commits to NIAHCIA-native ordered transaction data.

## Execution root

`execution_root` commits to deterministic native NIAHCIA execution results. It is not an Ethereum/Reth execution hash.

## Target

The consensus header carries the target; human-readable difficulty is derived.

## Interoperability status

The 164-byte development header and vectors remain active pre-alpha fixtures. Production RandomX miner/pool interoperability is still under review, so the exact final mining preimage must not be treated as frozen until that review is complete.
