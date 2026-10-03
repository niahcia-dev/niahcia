# Node Configuration

NIAHCIA uses TOML configuration plus environment overrides.

The node now uses native NIAHCIA execution. Reth/Engine-API/JWT configuration is obsolete and rejected rather than silently ignored.

## Example

```bash
cp config/niahcia.example.toml niahcia.toml
cargo run -p niahcia -- --config niahcia.toml
```

```toml
network = "devnet"
data_dir = "./data"
fee_recipient = "0x0000000000000000000000000000000000000000"
mining_rpc_bind = "127.0.0.1:9332"
p2p_bind = "127.0.0.1:9442"
p2p_peers = []
log_level = "info"
```

A devnet `dniah1...` account address may also be used as `fee_recipient`; it is normalized internally to its 20-byte account payload.

## Environment overrides

```text
NIAHCIA_NETWORK
NIAHCIA_DATA_DIR
NIAHCIA_FEE_RECIPIENT
NIAHCIA_MINING_RPC_BIND
NIAHCIA_P2P_BIND
NIAHCIA_P2P_PEERS
NIAHCIA_LOG_LEVEL
```

Unknown TOML fields are rejected. This prevents obsolete configuration such as former `reth_*` keys from appearing to work while being ignored.

Current pre-alpha consensus parameters support devnet only.
