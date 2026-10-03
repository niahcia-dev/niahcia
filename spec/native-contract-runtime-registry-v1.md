# NIAHCIA Native Contract Runtime Registry V1

Status: **CANDIDATE / VECTORED / INACTIVE**

## Purpose

This specification defines how a contract runtime identifier becomes eligible for ContractCreate at a particular block height without selecting a VM or instruction set.

The registry is a consensus/runtime-activation boundary. It is not a plugin discovery system and it is not controlled by optional AI or service nodes.

## Descriptor

Each candidate runtime descriptor contains:

```text
runtime_id            u32
code_format_version   u32
activation_height     u64
retirement_height     optional u64
max_code_bytes        u32
max_init_data_bytes   u32
```

Rules:

- runtime ID zero is permanently reserved;
- code-format version zero is permanently reserved;
- runtime IDs are unique within one registry;
- `max_code_bytes` MUST be within 1..=65,536;
- `max_init_data_bytes` MUST be <=65,536;
- if present, `retirement_height` MUST be greater than `activation_height`;
- activation height is inclusive;
- retirement height is exclusive.

A descriptor does not define VM semantics. It only identifies a separately specified runtime/code format and bounds the payload accepted for it.

## Active runtime rule

For descriptor `R` and block height `h`:

```text
active =
    h >= activation_height
    AND
    (retirement_height absent OR h < retirement_height)
```

Unknown runtime IDs are invalid for ContractCreate.

Known but inactive/retired runtime IDs are invalid for ContractCreate.

An empty registry activates no runtime.

## Code-validation boundary

ContractCreate validation is layered:

1. canonical ContractCreatePayload V1 decoding;
2. generic payload bounds;
3. runtime-registry lookup;
4. activation-height check;
5. runtime-specific code/init byte limits;
6. runtime-specific code-format validation;
7. runtime execution/constructor semantics.

This milestone implements and vectors layers 1 through 5.

Layer 6 is intentionally not invented before the VM/code format is selected.

Therefore a registry descriptor being active is necessary but not sufficient for contract execution. The active runtime implementation must still provide deterministic code-format validation and execution semantics.

## Network configuration

No runtime descriptor is installed in the active reference node.

No mainnet, testnet, or devnet runtime activation height is assigned.

The reference implementation's empty registry therefore activates no smart-contract runtime.

Any future network runtime activation requires an explicit protocol/network-parameter decision, runtime specification, code validator, gas schedule, execution vectors, and activation review.

## Interoperability vector

The canonical fixture is:

`test-vectors/native-contract-runtime-registry-v1.json`

It locks registry validation and inclusive/exclusive activation behavior using runtime ID 1 and heights 100/200 only as test values.

Those fixture values are not network parameters and do not select a VM.

## Compatibility

Changing registry activation semantics, identifier widths, descriptor limits, or retirement-height behavior incompatibly requires an explicit successor version.
