# NIAHCIA Network Parameters V1

## Status

Native network identity primitives are **LOCKED for V1 interoperability**.

Genesis values, production difficulty parameters, activation heights, and
other launch-specific values remain subject to their individual specifications
and network definitions.

## Network classes

NIAHCIA defines three network classes:

```text
network_id  name     Address V1 HRP
0x00        mainnet  niah
0x01        testnet  tniah
0x02        devnet   dniah
```

The canonical serialized `network_id` is exactly one unsigned byte.

## Native chain IDs

Each network class has a native unsigned 64-bit NIAHCIA chain identifier:

```text
network   chain_id hex         decimal
mainnet   0x000000004E494148   1313423688
testnet   0x0000000154494148   5709054280
devnet    0x0000000244494148   9736028488
```

When a fixed-width byte representation is required, `chain_id` is serialized
as exactly 8 unsigned big-endian bytes.

The pair `(network_id, chain_id)` is the native chain-domain identity used by
protocol components that require both identifiers.

A protocol field specifying only `network_id` serializes exactly the one-byte
identifier. It MUST NOT substitute the network name, Address V1 HRP, chain ID,
genesis ID, or another textual representation.

## Required concrete network parameters

Each concrete network definition MUST assign or reference:

- network name;
- one-byte `network_id`;
- native `chain_id`;
- Address V1 HRP;
- P2P network magic / protocol identity;
- genesis BlockHeaderV1;
- genesis block ID;
- genesis native state commitment;
- initial PoW target;
- PoW limit;
- target block interval;
- RandomX epoch length;
- RandomX seed lag;
- protocol activation heights.

A concrete devnet or testnet MAY define additional versioned development
parameters, but MUST NOT reinterpret the locked V1 network or chain identifiers.

## Network independence

Mainnet, testnet, and devnet are independent NIAHCIA networks.

Each concrete network has its own genesis block and persistent network identity.

No NIAHCIA network definition depends on the state, validators, finality,
genesis, RPC infrastructure, or consensus identity of another blockchain.

## Development genesis shape

The first development genesis remains deliberately minimal:

```text
version           = 1
parent_hash       = 0x00...00
height            = 0
timestamp         = fixed published value
transactions_root = NIAHCIA empty transaction Merkle root
execution_root    = deterministic native state commitment
target            = development pow_limit
nonce             = fixed published value
extra_nonce       = fixed published value
```

The complete genesis serialization and resulting genesis block ID MUST be
published as canonical interoperability data before a public network using
that genesis is treated as stable.

## Mainnet

Mainnet genesis and launch-specific consensus values are not selected during
Prototype 0.

They MUST be generated once, independently reviewed, published in this
repository, and represented by canonical test vectors before mainnet launch.

The locked V1 `network_id` and native `chain_id` do not depend on selection of
those later launch parameters.

## Compatibility rule

The following V1 meanings MUST NOT be silently changed:

- `0x00` means mainnet;
- `0x01` means testnet;
- `0x02` means devnet;
- `network_id` is exactly one unsigned byte when serialized;
- the three native `chain_id` values defined above;
- fixed-width `chain_id` representation is unsigned 64-bit big-endian.

An incompatible change requires an explicitly versioned protocol transition.
