# Relay Receipt V1

## Status

Draft service-evidence object.

Relay receipts provide evidence that one node delivered a valid network object to another peer.

## Canonical fields

```text
RelayReceiptV1
- schema_version
- relay_receipt_id
- object_type
- object_id
- sender_service_node_id
- receiver_peer_id
- received_at_bucket
- transport_session_id
- receiver_signature
```

## Limitations

Relay receipts are Sybil-sensitive.

They SHOULD therefore be low-weight evidence and SHOULD require independent-peer diversity before contributing materially to rewards.

A relay receipt MUST NOT affect block validity, transaction validity, or fork choice.
