# NIAHCIA Monetary and Fee Primitives V1

Status: **pre-alpha design lock for implementation**

This document locks the representation rules for NIAHCIA native value, denominations, chain identity, and transaction fee arithmetic. It deliberately does **not** lock the long-term issuance schedule or maximum supply; those are separate monetary-policy decisions and must not be invented implicitly by transaction code.

## Native currency and denominations

The native currency symbol is `NIAH`.

V1 uses exactly **8 decimal places** at the native transaction and monetary boundary:

- `1 NIAH = 10^8 aniah = 100,000,000 aniah`
- `aniah` is the canonical smallest indivisible native unit in V1.
- Consensus, RPC serialization, transaction signing, balances, rewards, and native fees MUST use integer `aniah` values internally.
- Floating-point arithmetic MUST NOT be used for consensus value, balances, rewards, or fees.

The lowercase name `aniah` is a protocol denomination, not an alternate asset.

Human interfaces MAY display decimal NIAH, but conversion to/from `aniah` MUST be exact. Inputs with more than 8 fractional decimal digits MUST be rejected rather than rounded by consensus-facing software.

Execution-engine quantities that use a different internal scale are an implementation-boundary concern. They MUST NOT redefine the native NIAH denomination or leak an 18-decimal assumption into NIAHCIA-native signed objects, RPCs, wallets, rewards, or supply accounting.

## Consensus value representation

TransactionEnvelopeV1 already defines `value` and `max_fee_per_gas` as unsigned 128-bit integers. V1 therefore locks native monetary quantities at the NIAHCIA transaction/protocol boundary to `u128` unless a field is explicitly specified otherwise.

Canonical binary representation of a `u128` monetary field is exactly 16 bytes, unsigned, big-endian.

Implementations MUST use checked integer arithmetic. Overflow or underflow is a hard validation failure; wrapping and saturating arithmetic are forbidden for consensus calculations.

The valid V1 scalar range is:

`0 <= amount <= 2^128 - 1`

No issuance policy may create a canonical balance, transfer, reward, or fee result outside that range.

## Supply accounting

V1 distinguishes representation from monetary policy.

Nodes MUST be able to account for at least genesis-issued native value, block/PoW issuance, other protocol-authorized issuance if later enabled, explicit protocol burns, and circulating/accounted native value.

The invariant is conceptually:

`accounted_supply = genesis_issuance + cumulative_protocol_issuance - cumulative_protocol_burns`

All terms are exact integer `aniah` quantities using checked arithmetic.

This document does **not** choose the NIAHCIA initial block subsidy, emission curve, tail emission, maximum supply, premine/genesis allocation, or reward split. Those require a separately reviewed monetary-policy lock. Implementations MUST NOT infer those policy values from the 8-decimal representation.

## Network identifiers

TransactionEnvelopeV1 reserves `0x00` mainnet, `0x01` testnet, and `0x02` devnet. These values are part of the signed transaction domain and MUST NOT be reused for a different NIAHCIA network.

## NIAHCIA chain IDs

The NIAHCIA `chain_id` is a native unsigned 64-bit consensus identifier included in the signed transaction body.

V1 locks:

- Mainnet: `0x000000004E494148` (`1313423688` decimal; ASCII `NIAH`)
- Testnet: `0x0000000154494148` (`5709054280` decimal; namespace `1` + ASCII `TIAH`)
- Devnet: `0x0000000244494148` (`9735586120` decimal; namespace `2` + ASCII `DIAH`)

A node MUST reject a signed transaction when either its `network` or `chain_id` differs from the node's configured consensus identity. Internal execution-chain identifiers MUST NOT replace or reinterpret the NIAHCIA signed transaction domain.

## Fee arithmetic

TransactionEnvelopeV1 contains `gas_limit: u64` and `max_fee_per_gas: u128`. All native fee-per-gas values are integer `aniah` per gas unit.

`max_fee_reserve = gas_limit * max_fee_per_gas`

`required_balance = value + max_fee_reserve`

The execution engine determines `gas_used`, where `0 <= gas_used <= gas_limit`, and the effective fee must satisfy `effective_fee_per_gas <= max_fee_per_gas`.

`charged_fee = gas_used * effective_fee_per_gas`

`unused_fee_reserve = max_fee_reserve - charged_fee`

The sender is charged exactly `value + charged_fee`. All arithmetic uses checked integers; overflow is invalid.

The execution layer may expose a protocol base fee. Its eventual burn/redirect/producer-compensation policy remains a separate explicit economic decision and MUST NOT be inferred from execution-engine defaults.

## Canonical RPC representation

Native RPC methods SHOULD expose monetary integers as canonical decimal strings to avoid JSON number precision loss.

Examples:

- `"0"`
- `"1"`
- `"100000000"` for 1 NIAH

Human-facing wallets may display `1.00000000 NIAH`; signed and consensus representations remain integer `aniah`.

## Required validation vectors

Before native transaction submission is enabled, implementation tests MUST lock vectors for:

1. `1 NIAH == 100_000_000 aniah`;
2. zero and maximum `u128` serialization;
3. decimal-to-aniah exact conversion;
4. rejection of fractional precision beyond 8 decimals;
5. wrong network rejection;
6. wrong NIAHCIA chain-ID rejection;
7. `gas_limit * max_fee_per_gas` overflow rejection;
8. `value + max_fee_reserve` overflow rejection;
9. insufficient balance at the maximum reserved cost;
10. `gas_used > gas_limit` rejection;
11. `effective_fee_per_gas > max_fee_per_gas` rejection;
12. exact unused-reserve/refund arithmetic;
13. execution-chain ID cannot substitute for NIAHCIA `chain_id`.

## Compatibility rule

Once V1 interoperability vectors are published, the **8-decimal denomination scale**, integer widths, byte order, network identifiers, and three NIAHCIA chain IDs above MUST NOT be reinterpreted. An incompatible change requires an explicitly versioned protocol transition.
