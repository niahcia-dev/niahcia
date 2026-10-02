# NIAHCIA Monetary and Fee Primitives V1

Status: **LOCKED representation/interoperability primitives; monetary issuance policy remains separately CANDIDATE**

This specification defines NIAHCIA V1 native value representation, denomination scale, native chain identity, and deterministic transaction fee arithmetic. It deliberately does not freeze the long-term issuance schedule, maximum supply, genesis allocation, reward split, or fee disposition policy.

## Native currency and denomination

The native currency symbol is `NIAH`.

V1 uses exactly **8 decimal places**:

```text
1 NIAH = 100,000,000 aniah
```

`aniah` is the canonical smallest indivisible native unit in V1. Consensus-facing balances, signed transactions, rewards, native fees, RPC values, and supply accounting use integer `aniah` values. Floating-point arithmetic is forbidden for consensus value calculations.

Human interfaces may display decimal NIAH. Conversion must be exact; more than eight fractional decimal digits are rejected rather than rounded.

Execution-engine quantities using another internal scale are an implementation boundary and do not redefine NIAHCIA-native denomination semantics.

## Native scalar representation

Where V1 native transaction/protocol monetary fields are defined as `u128`, their canonical binary representation is exactly 16 unsigned big-endian bytes.

Consensus arithmetic uses checked integers. Overflow or underflow is invalid; wrapping or saturating arithmetic is not permitted to alter a consensus result.

## Supply-accounting representation

Representation is separate from issuance policy. Implementations must be capable of exact accounting for genesis issuance, cumulative protocol-authorized issuance, explicit protocol burns, and resulting accounted supply using integer `aniah` quantities.

Conceptually:

```text
accounted_supply = genesis_issuance + cumulative_protocol_issuance - cumulative_protocol_burns
```

This equation does not itself authorize any issuance or burn mechanism.

## Network identifiers

The V1 signed native transaction domain reserves:

```text
0x00  mainnet
0x01  testnet
0x02  devnet
```

These values must not be reused for a different NIAHCIA network.

## Native NIAHCIA chain IDs

The NIAHCIA native `chain_id` is an unsigned 64-bit consensus identifier included in the signed native transaction body.

V1 locks:

```text
mainnet  0x000000004E494148  1313423688
 testnet 0x0000000154494148  5709054280
 devnet  0x0000000244494148  9736028488
```

A node rejects a signed native transaction when either its network identifier or native `chain_id` differs from the node's configured consensus identity.

An EVM/execution chain ID is a separate execution-domain identifier and must not replace or reinterpret the NIAHCIA native signed-transaction domain.

## Fee arithmetic

For a V1 transaction with:

```text
gas_limit: u64
max_fee_per_gas: u128
value: u128
```

where fee-per-gas values are integer `aniah` per gas unit:

```text
max_fee_reserve = gas_limit * max_fee_per_gas
required_balance = value + max_fee_reserve
```

The execution result provides `gas_used` with:

```text
0 <= gas_used <= gas_limit
```

and the effective fee must satisfy:

```text
effective_fee_per_gas <= max_fee_per_gas
```

Then:

```text
charged_fee = gas_used * effective_fee_per_gas
unused_fee_reserve = max_fee_reserve - charged_fee
sender_charge = value + charged_fee
```

Every operation is checked integer arithmetic. Any overflow, underflow, or violated bound makes the transaction/result invalid.

The disposition of execution base fees or other fee components—burn, producer compensation, treasury, or another explicitly versioned destination—is an economic-policy decision and is not silently inherited from an execution engine.

## RPC representation

Native RPCs should encode monetary integers as canonical decimal strings to avoid JSON-number precision loss.

Examples:

```text
"0"
"1"
"100000000"   # 1 NIAH
```

Human interfaces may display `1.00000000 NIAH`; consensus and signed representations remain integer aniah.

## Required interoperability vectors

The V1 vector set must cover at least:

1. `1 NIAH == 100_000_000 aniah`;
2. zero and maximum `u128` serialization;
3. exact decimal-to-aniah conversion;
4. rejection beyond eight decimal places;
5. wrong-network rejection;
6. wrong-native-chain-ID rejection;
7. multiplication overflow in maximum fee reserve;
8. addition overflow in required balance;
9. insufficient maximum-reserve balance;
10. `gas_used > gas_limit` rejection;
11. effective fee above maximum rejection;
12. exact unused-reserve/refund arithmetic;
13. rejection of an execution chain ID used as a substitute for the native chain ID.

## Compatibility rule

For V1, the eight-decimal denomination scale, canonical smallest-unit meaning, specified integer widths/byte order, network identifiers, and native chain IDs are locked interoperability primitives. They must not be silently reinterpreted. An incompatible change requires an explicitly versioned protocol transition.

## Not locked here

This specification does not freeze:

- mainnet maximum supply;
- genesis/premine allocation;
- block-subsidy curve;
- permanent tail emission;
- miner/compute/storage/verifier reward allocation;
- base-fee burn or redirect policy;
- production EVM chain IDs.

Those belong to separately reviewed economic/network specifications.
