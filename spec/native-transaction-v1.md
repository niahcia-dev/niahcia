# Native Transaction V1

## Status

Candidate — Protocol V1 foundation.

This specification defines the canonical NIAHCIA native transaction format,
authentication rules, transaction identity, action semantics, and deterministic
state-transition boundary.

It does not define the contract runtime instruction set. Contract runtime
semantics are versioned separately.

## Design principles

Native transactions:

- are canonical NIAHCIA protocol objects;
- use NCE/1 serialization;
- authenticate an exact canonical unsigned body;
- identify the sender cryptographically rather than through a claimed sender
  field;
- commit to the native network and chain;
- use native Account and Contract Address V1 payloads;
- have deterministic replay and nonce rules;
- do not depend on another blockchain transaction format or state model.

## Native transaction body

The canonical unsigned transaction is:

    object_type    = 0x0010
    schema_version = 1

Its NCE/1 payload fields are permanently assigned:

    1   network_id
    2   chain_id
    3   nonce
    4   action
    5   target_payload
    6   value
    7   gas_limit
    8   max_fee_per_gas
    9   max_priority_fee_per_gas
    10  data

The envelope carries `schema_version = 1`; schema version is not duplicated in
the payload.

`network_id`, `chain_id`, `nonce`, `action`, and `gas_limit` use canonical
NCE/1 unsigned-integer encoding.

`target_payload` and `data` use definite-length CBOR byte strings.

`value`, `max_fee_per_gas`, and `max_priority_fee_per_gas` are integer
`aniah` quantities represented as exactly 16 unsigned big-endian bytes inside
NCE/1 CBOR byte strings.

## Actions

V1 defines:

    0x00  Transfer
    0x01  ContractCall
    0x02  ContractCreate

All other action values are invalid in V1.

### Transfer

A Transfer moves native value to an Account.

Requirements:

- `target_payload` is exactly 20 bytes;
- the target is interpreted as an Account Address V1 payload for the
  transaction network;
- `data` is empty.

### ContractCall

A ContractCall invokes an existing Contract.

Requirements:

- `target_payload` is exactly 20 bytes;
- the target is interpreted as a Contract Address V1 payload for the
  transaction network;
- `data` contains the runtime input bytes and MAY be empty.

The active versioned NIAHCIA contract-runtime specification defines runtime
behavior.

### ContractCreate

A ContractCreate creates a new Contract.

Requirements:

- `target_payload` is empty;
- `data` contains the contract-creation payload required by the active
  versioned NIAHCIA contract-runtime specification.

The resulting Contract Address V1 payload is derived according to Address V1.

## Network and chain

`network_id` identifies the NIAHCIA network.

`chain_id` identifies the NIAHCIA chain within the signed transaction domain.

Both MUST match the node's active consensus configuration.

A transaction for another network or chain is invalid.

## Sender

The unsigned body does not contain a sender field.

The authenticated sender is derived from the canonical public key carried by
SignedNativeTransactionV1 according to Address V1 account derivation.

An implementation MUST NOT accept an independently supplied sender identity as
a substitute for the authenticated sender.

## Account nonce

Every Account has a monotonically increasing native transaction nonce.

For admission to execution:

    tx.nonce == current_sender_nonce

is required.

A transaction with a stale or future nonce is not executable in the current
state.

Once a transaction passes pre-execution validation and enters execution, its
sender nonce is consumed exactly once.

Therefore:

- pre-execution invalid transaction: nonce is not consumed;
- successful transaction: nonce is consumed;
- accepted ContractCall or ContractCreate that fails during runtime execution:
  nonce is consumed;
- accepted transaction that exhausts its permitted gas: nonce is consumed.

The nonce transition is checked for overflow.

Registry, object, job, storage, service, and other protocol nonces are separate
domains and MUST NOT be substituted for the native account transaction nonce.

## Value

`value` is denominated in integer `aniah`.

No floating-point representation is consensus-valid.

All balance arithmetic MUST use checked integer arithmetic.

For Transfer, `value` is the amount transferred to the target Account.

For ContractCall and ContractCreate, `value` is the native value made available
to that operation according to the active contract-runtime rules.

## Gas and fees

`gas_limit` is the maximum execution gas authorized by the sender.

`max_fee_per_gas` and `max_priority_fee_per_gas` are integer `aniah`
quantities.

`max_fee_per_gas` caps the sender's total fee per gas unit.

`max_priority_fee_per_gas` caps the sender-authorized priority fee per gas unit
paid to the canonical CPU-PoW block producer under the active V1 fee policy.

The maximum authorized execution charge is:

    gas_limit * max_fee_per_gas

using checked arithmetic.

A transaction that cannot cover its required value and maximum authorized
execution charge is invalid before execution.

Actual fee calculation and fee disposition are defined by the active
versioned monetary and fee policy.

Transfer remains subject to the consensus-defined native transaction cost even
though it does not invoke the contract runtime.

## Data

`data` is a definite-length byte string.

Its interpretation is action-dependent:

- Transfer: MUST be empty;
- ContractCall: runtime input bytes;
- ContractCreate: contract-creation payload.

No independent data-length field is serialized.

## Signed transaction

The canonical signed transaction is:

    object_type    = 0x0011
    schema_version = 1

Its NCE/1 payload fields are:

    1   body
    2   public_key
    3   signature

`body` is a CBOR byte string containing the complete canonical NCE/1
serialization of NativeTransactionBodyV1, including its NCE/1 envelope.

Those exact body bytes are authenticated by the signature.

`public_key` is exactly 65 bytes and MUST be canonical uncompressed SEC1:

    0x04 || X || Y

The key MUST encode a valid secp256k1 public key.

`signature` is exactly 64 bytes:

    r || s

where `r` and `s` are 32-byte unsigned big-endian integers.

V1 requires valid ranges and canonical low-S form.

DER encoding and a recovery identifier are not part of
SignedNativeTransactionV1.

## Canonical decoding requirements

Consensus and wire implementations MUST decode Native Transaction V1 strictly.

For `NativeTransactionBodyV1`:

- the NCE/1 envelope MUST match object type `0x0010` and schema version `1`;
- the payload map MUST contain exactly fields `1..10` in canonical ascending order;
- unsigned integers and lengths MUST use shortest-form NCE/1 encoding;
- `value`, `max_fee_per_gas`, and `max_priority_fee_per_gas` MUST each decode from exactly 16 unsigned big-endian bytes;
- the action value MUST be recognized by Native Transaction V1;
- truncated values, unexpected major types, non-canonical encodings, extra fields, missing fields, and trailing bytes are invalid.

For `SignedNativeTransactionV1`:

- the NCE/1 envelope MUST match object type `0x0011` and schema version `1`;
- the payload map MUST contain exactly fields `1..3` in canonical ascending order;
- field `1 body` MUST contain the complete canonical bytes of one `NativeTransactionBodyV1`;
- the public key and signature MUST satisfy the canonical representations defined below;
- trailing bytes are invalid.

A decoder used for consensus verification MUST be round-trip strict: decoding and then canonically re-encoding the object MUST reproduce the exact original byte sequence. Semantically equivalent but non-canonical byte encodings are rejected rather than normalized for verification.

## Signing digest

The signing purpose is:

    SIGN/NATIVE_TRANSACTION

The digest is:

    signing_digest =
      Keccak-256(
        "NIAHCIA" ||
        0x00 ||
        "SIGN/NATIVE_TRANSACTION" ||
        0x00 ||
        network_id ||
        0x00 ||
        canonical_unsigned_body
      )

`canonical_unsigned_body` is the complete canonical NCE/1 serialization of
NativeTransactionBodyV1.

The `network_id` committed by the signing domain MUST equal the `network_id`
inside the body.

## Transaction identifier

The canonical transaction identifier is derived from the complete canonical
SignedNativeTransactionV1 bytes:

    tx_id =
      Keccak-256(
        "NIAHCIA/TX-ID/V1" ||
        0x00 ||
        canonical_signed_transaction
      )

The transaction identifier is a protocol identifier and is not a sender nonce,
block position, or mutable database identifier.

## Transaction commitment

Blocks commit to ordered native transactions using the canonical transaction
commitment defined by Transaction Merkle V1.

The commitment MUST authenticate transaction order and exact canonical signed
transaction bytes.

## Pre-execution validation

Before a transaction may enter execution, a node MUST validate at least:

1. canonical NCE/1 envelope and payload encoding;
2. object type and schema version;
3. recognized network and matching active network;
4. matching native chain ID;
5. recognized action;
6. action-specific target rules;
7. canonical public key;
8. canonical low-S signature;
9. signature over the exact canonical body;
10. authenticated sender derivation;
11. exact current sender nonce;
12. checked monetary arithmetic;
13. sufficient sender balance for the required value and maximum authorized
    execution charge;
14. consensus size and gas limits.

Failure of pre-execution validation makes the transaction invalid and consumes
neither nonce nor fee.

## Deterministic state transition

A valid transaction enters deterministic native execution.

### Transfer success

A successful Transfer:

1. consumes the sender nonce;
2. debits the transferred value and charged fee from the sender;
3. credits the transferred value to the target Account;
4. applies the versioned fee disposition;
5. commits the resulting state.

### Contract success

A successful ContractCall or ContractCreate:

1. consumes the sender nonce;
2. applies the deterministic contract-runtime transition;
3. applies native value movement required by that transition;
4. charges the actual execution fee;
5. applies the versioned fee disposition;
6. commits the resulting state.

### Contract runtime failure

If an accepted ContractCall or ContractCreate fails during runtime execution:

1. the sender nonce remains consumed;
2. reversible contract state effects are discarded;
3. reversible operation-value effects are discarded;
4. the consensus-defined execution fee is charged;
5. fee disposition is applied;
6. the resulting failure state is committed.

### Out of gas

If execution exhausts `gas_limit`:

1. the sender nonce remains consumed;
2. reversible contract state and operation-value effects are discarded;
3. the transaction is charged according to the full permitted gas consumption
   rule;
4. fee disposition is applied;
5. the resulting failure state is committed.

All state arithmetic MUST be checked.

## Contract creation

Contract creation is a native state transition initiated by
`action = ContractCreate`.

The transaction does not serialize a target contract address.

The resulting contract payload is derived deterministically from the
authenticated sender, native network/chain context, and sender transaction
nonce according to Address V1.

Contract runtime installation and initialization are defined by the active
versioned contract-runtime specification.

## Replay protection

Replay protection is provided jointly by:

- the signing domain;
- `network_id`;
- `chain_id`;
- the authenticated sender;
- the exact sender transaction nonce;
- canonical transaction identity.

A transaction valid on one native network or chain MUST NOT become valid on
another merely because its body bytes are otherwise meaningful there.

## Consensus requirements

Consensus implementations MUST agree on:

- canonical NCE/1 encoding;
- signing digest;
- sender derivation;
- transaction identifier;
- action validation;
- target interpretation;
- nonce validation and consumption;
- checked monetary arithmetic;
- fee arithmetic;
- deterministic native state transition;
- transaction commitment;
- contract-address derivation;
- active contract-runtime version.

Any disagreement in these rules is consensus-critical.

## V1 exclusions

Native Transaction V1 does not define:

- the contract-runtime instruction set;
- contract-runtime memory/storage representation;
- the detailed runtime gas schedule;
- future transaction actions;
- transaction expiry/deadline;
- access-list-style extensions;
- blob/data-availability extensions.

Those require separately versioned protocol specifications.

## Interoperability lock requirements

Before Native Transaction V1 is declared interoperability-locked, canonical
vectors MUST cover at least:

1. one Transfer body;
2. one ContractCall body;
3. one ContractCreate body;
4. signing digest;
5. canonical public key;
6. canonical low-S signature;
7. SignedNativeTransactionV1 bytes;
8. transaction identifier;
9. sender Address V1 derivation;
10. ContractCreate Address V1 derivation;
11. wrong-network rejection;
12. wrong-chain rejection;
13. invalid action rejection;
14. invalid target-length rejection;
15. stale/future nonce rejection;
16. insufficient-balance rejection;
17. checked-overflow rejection;
18. high-S signature rejection;
19. malformed public-key rejection;
20. malformed NCE/1 rejection.

Until those vectors and the dependent native state-transition rules are locked,
this specification remains Candidate.

## Native Transfer V1 gas schedule

A successful `Transfer` action consumes exactly `1,000` gas.

This is a NIAHCIA-native consensus accounting unit. It is not inherited from
Ethereum, EVM, Reth, CPU instruction counts, elapsed execution time, or host
performance.

For V1, `gas_limit >= 1,000` is required for a successful native transfer.

A transfer with `gas_limit < 1,000` is invalid and MUST NOT mutate native
state, consume the sender nonce, transfer value, charge a fee, credit a
producer, or record a protocol burn.

A successful native transfer always reports exactly `1,000` gas used.
Unused gas capacity is not charged.

Contract calls and contract creation do not inherit this fixed transfer cost.
Their metering is defined separately by the applicable native contract
execution protocol.
