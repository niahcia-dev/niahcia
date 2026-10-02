# NIAHCIA Transaction Merkle Tree V1

## Status

Draft consensus candidate.

The block header commits to ordered transactions through a NIAHCIA-native binary Merkle tree.

This commitment is independent from Ethereum's Merkle-Patricia transaction trie and from any Reth execution block hash.

## Transaction leaf digest

For each canonical raw transaction byte string `tx`:

```text
tx_digest =
  keccak256(
    "NIAHCIA/TX/V1"
    ||
    tx
  )
```

The Merkle leaf is then:

```text
leaf =
  keccak256(
    "NIAHCIA/MERKLE-LEAF/V1"
    ||
    tx_digest
  )
```

Transaction order is consensus-significant.

## Internal node digest

For a left and right child:

```text
node =
  keccak256(
    "NIAHCIA/MERKLE-NODE/V1"
    ||
    left
    ||
    right
  )
```

Each child is exactly 32 bytes.

## Odd node rule

When a level contains an odd number of nodes, the final node is duplicated:

```text
node(last, last)
```

This rule is applied independently at every tree level.

## Empty block root

A block containing zero transactions has:

```text
transactions_root =
  keccak256(
    "NIAHCIA/MERKLE-EMPTY/V1"
  )
```

## One transaction

A block containing exactly one transaction has:

```text
transactions_root = leaf(tx0)
```

No synthetic sibling is added for a single-leaf tree.

## Independence

An execution engine may use its own internal transaction commitment for execution purposes.

NIAHCIA consensus uses this Merkle root as the transaction commitment in `BlockHeaderV1`.

Two conforming implementations given the same ordered canonical transaction bytes MUST produce the same root.
