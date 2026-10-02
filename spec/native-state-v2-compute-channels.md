# Native State V2 — Compute Channels

Status: **CANDIDATE / pre-alpha native state successor**

## Purpose

NativeStateV2 extends the accounts-only NativeStateV1 with consensus-committed ComputeChannel state.

NativeStateV1 is already implemented with locked-style byte/root vectors and MUST NOT be silently changed.

The central rule is:

> compute-channel state participates in the same deterministic block state root, persistence, rollback, and reorg behavior as account balances and nonces.

## Versioning

NativeStateV1 remains:

```text
accounts only
state-root domain = NIAHCIA/STATE-ROOT/V1
snapshot version byte = 1
```

NativeStateV2 is a successor:

```text
accounts
+
compute channels
snapshot version byte = 2
```

A node MUST choose the state version from explicit network activation rules. It MUST NOT guess from transaction contents.

## Account component

NativeStateV2 preserves the exact NativeStateV1 account semantics:

```text
AccountId = 20 bytes
balance   = u128 aniah
nonce     = u64
```

The V2 `accounts_root` is exactly the root that the same account map would produce under NativeStateV1.

This intentionally reuses the already tested V1 account commitment as one component of V2.

## ComputeChannelStateV1 fields

Each channel record contains:

```text
channel_id                         32 bytes
funding_account                    20 bytes
worker_id                          32 bytes
operator_id                        32 bytes
channel_public_key                 65 bytes
worker_payment_account             20 bytes
authorized_amount                  u128
settled_amount                     u128
opened_height                      u64
expiry_height                      u64
claim_deadline_height              u64
refund_available_height            u64
service_scope_commitment           32 bytes
model_scope_commitment             32 bytes
execution_profile_scope_commitment 32 bytes
settlement_policy                  u8
state                              u8
```

Integer fields are unsigned big-endian at exact widths.

Candidate state values:

```text
0x00 OPEN
0x01 SETTLED
0x02 REFUNDED
```

Candidate settlement policy values:

```text
0x00 CUMULATIVE_RECEIPT
```

Unknown values are invalid.

## Channel record commitment

For one channel:

```text
channel_record_hash =
  Keccak-256(
    "NIAHCIA/COMPUTE-CHANNEL-STATE/V1" ||
    channel_id ||
    funding_account ||
    worker_id ||
    operator_id ||
    channel_public_key ||
    worker_payment_account ||
    authorized_amount_u128_be ||
    settled_amount_u128_be ||
    opened_height_u64_be ||
    expiry_height_u64_be ||
    claim_deadline_height_u64_be ||
    refund_available_height_u64_be ||
    service_scope_commitment ||
    model_scope_commitment ||
    execution_profile_scope_commitment ||
    settlement_policy_u8 ||
    state_u8
  )
```

Every field affects the commitment.

## Channels root

Channels are ordered lexicographically by raw 32-byte `channel_id`.

For no channels:

```text
channels_root =
  Keccak-256("NIAHCIA/COMPUTE-CHANNELS-ROOT/V1/EMPTY")
```

For a non-empty ordered channel map:

```text
channels_root =
  Keccak-256(
    "NIAHCIA/COMPUTE-CHANNELS-ROOT/V1" ||
    channel_id_0 || channel_record_hash_0 ||
    channel_id_1 || channel_record_hash_1 ||
    ...
  )
```

Channel insertion order MUST NOT affect the root.

## NativeStateV2 root

```text
state_root_v2 =
  Keccak-256(
    "NIAHCIA/STATE-ROOT/V2" ||
    accounts_root ||
    channels_root
  )
```

The state root therefore changes when either account state or channel state changes.

## Canonical snapshot V2

NativeStateV2 snapshot bytes are:

```text
version_u8 = 2

account_count_u64_be
repeated account records, strictly sorted by 20-byte AccountId:
  account_id[20]
  balance_u128_be[16]
  nonce_u64_be[8]

channel_count_u64_be
repeated channel records, strictly sorted by channel_id:
  channel_id[32]
  funding_account[20]
  worker_id[32]
  operator_id[32]
  channel_public_key[65]
  worker_payment_account[20]
  authorized_amount_u128_be[16]
  settled_amount_u128_be[16]
  opened_height_u64_be[8]
  expiry_height_u64_be[8]
  claim_deadline_height_u64_be[8]
  refund_available_height_u64_be[8]
  service_scope_commitment[32]
  model_scope_commitment[32]
  execution_profile_scope_commitment[32]
  settlement_policy_u8[1]
  state_u8[1]
```

Default/zero account records remain forbidden exactly as in V1.

Duplicate or out-of-order account/channel records are invalid.

Trailing bytes are invalid.

## Open transition

ComputeChannelOpen:

1. begins from parent NativeStateV2;
2. validates the V2 native transaction and canonical Open payload;
3. derives funding account from authenticated sender;
4. derives channel_id from the canonical open transaction ID;
5. requires channel_id absence;
6. debits `authorized_amount` from funding account;
7. consumes sender nonce;
8. creates the OPEN channel record;
9. charges native transaction fee according to the active fee policy;
10. commits the complete resulting NativeStateV2 root.

All arithmetic is checked.

## Settle transition

ComputeChannelSettle:

1. begins from parent NativeStateV2;
2. validates channel OPEN state;
3. validates canonical settlement payload and final usage receipt;
4. validates current height <= claim_deadline_height;
5. validates cumulative_spent <= authorized_amount;
6. consumes submitter nonce;
7. credits cumulative_spent to worker_payment_account;
8. credits authorized_amount - cumulative_spent to funding_account;
9. sets settled_amount = cumulative_spent;
10. sets state = SETTLED;
11. charges submitter's native transaction fee;
12. commits resulting NativeStateV2 root.

A SETTLED/REFUNDED channel cannot transition again.

## Refund transition

ComputeChannelRefund:

1. begins from parent NativeStateV2;
2. validates channel OPEN state;
3. validates authenticated sender == funding_account;
4. validates current height >= refund_available_height;
5. consumes sender nonce;
6. credits authorized_amount back to funding_account;
7. leaves settled_amount = 0;
8. sets state = REFUNDED;
9. charges native transaction fee;
10. commits resulting NativeStateV2 root.

## Block atomicity

Compute-channel actions follow the existing native block atomicity rule.

If any transaction in the ordered block fails consensus execution:

```text
accounts rollback
channels rollback
fees rollback
nonces rollback
entire block transition fails
```

No partially opened/settled/refunded channel state may survive a failed block.

## Persistence

The canonical per-block native state snapshot MUST include both:

- account map;
- channel map.

A persisted block's committed state root must match the recomputed NativeStateV2 root exactly.

Channel state MUST NOT live only in an auxiliary database table outside the committed snapshot.

Auxiliary indexes MAY exist for lookup performance, but they are rebuildable caches and never authoritative.

## Reorg behavior

Reorg handling requires no special compensating transaction.

Because every persisted block has its own canonical NativeStateV2 snapshot:

```text
old canonical head
  -> detach blocks
  -> select common ancestor snapshot
  -> attach winning-branch snapshots/transitions
  -> new canonical head
```

Examples:

- if an Open transaction is detached, the locked amount and channel disappear with that detached state;
- if a Settle transaction is detached, worker/refund credits and terminal SETTLED state disappear;
- if the winning branch later includes a different valid settlement, that branch's result becomes canonical;
- if Refund is detached, the channel returns to the ancestor state appropriate to the winning branch.

Off-chain wallets/workers MUST treat settlement finality according to normal chain-confirmation/reorg risk; a receipt is not irrevocably paid merely because one shallow block included settlement.

## Activation / migration

NativeStateV2 requires an explicit activation height or equivalent network parameter.

At activation:

```text
accounts_v2 = accounts_v1
channels_v2 = empty
```

The first V2 state root is therefore derived from:

- the unchanged V1 account map;
- the canonical empty channels root;
- the V2 state-root domain.

This deliberately changes the state root at activation.

The activation rule and expected migration root require canonical vectors before lock.

Before activation, NativeTransaction schema V2 compute actions are invalid.

After activation, nodes MUST apply the same deterministic activation rule.

## Snapshot/restart

A node restarting on a post-activation block MUST decode snapshot version 2 and recover both account and channel state.

A node restarting on a pre-activation block continues to decode snapshot version 1.

Persistence must never decode a V2 snapshot as V1 or vice versa.

## Invariants

1. NativeStateV1 bytes/root remain unchanged.
2. NativeStateV2 commits both accounts and channels.
3. Account component semantics remain V1-compatible.
4. Channel state is consensus state, not an auxiliary service table.
5. Every channel mutation changes channels_root and state_root_v2.
6. Block execution remains atomic across accounts/channels.
7. Reorg rollback uses ordinary per-block canonical state restoration.
8. V2 activation is explicit and deterministic.
9. Compute actions are invalid before activation.
10. Restart/snapshot recovery preserves complete channel state.

## Required vectors/tests

Before lock:

1. V1 account map -> V2 activation migration root;
2. empty V2 snapshot;
3. one open channel snapshot/root;
4. insertion-order independence for channels;
5. open changes account + channel roots;
6. settle credits worker/refund and terminal state;
7. refund restores locked amount and terminal state;
8. duplicate terminal transition rejection;
9. failed second tx rolls back earlier channel mutation in same block;
10. restart from V2 snapshot;
11. reorg detaching Open;
12. reorg detaching Settle;
13. reorg detaching Refund;
14. malformed/out-of-order snapshot rejection.
