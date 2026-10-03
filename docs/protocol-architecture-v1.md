# NIAHCIA Protocol Architecture V1

The canonical working repository during pre-alpha development is `niahcia/niahcia`. It contains the reference implementation plus consolidated protocol/spec/test-vector material.

`niahcia/niahcia-protocol` remains a synchronized protocol mirror during this transition and must not contradict the canonical working repository.

## Documentation rule

When implementation changes alter protocol behavior:

1. update the affected canonical specification/status in `niahcia/niahcia`;
2. update/add interoperability vectors for consensus- or wire-critical changes;
3. update the reference implementation and implementation-facing documentation;
4. synchronize the retained `niahcia-protocol` mirror where it carries the same material;
5. remove or clearly mark superseded behavior.

Implementation code does not silently redefine locked protocol behavior.

## Current implementation architecture

```text
NIAHCIA reference node
        |
        +-- RandomX PoW / cumulative-work chain selection
        +-- native block / transaction P2P V3
        +-- NativeTransaction V1 / NativeStateV1 active path
        +-- required native smart-contract runtime (inactive / not yet specified)
        +-- native RPC and mining RPC
        +-- inactive NativeTransaction V2 / NativeStateV2 compute settlement
```

Smart contracts are a first-class base-chain requirement. `ContractCall` and `ContractCreate` are already reserved by NativeTransaction V1, but no runtime semantics become consensus-valid until a separately versioned deterministic native contract-runtime specification, vectors, and activation boundary exist.

The base chain must remain valid and usable without any AI worker or storage/service provider.

Wallet/client-local encrypted chat history and Agent memory are the V1 default. Decentralized storage is optional and deferred.

## Interoperability note

Common RandomX miner/pool compatibility remains an explicit objective. The exact production mining preimage and public-network parameters are not frozen merely because development vectors exist.
