# Native Devnet Smoke Test

This smoke test exercises the current native NIAHCIA node. No external execution engine is required.

## Build

```bash
cargo build --release
```

## Start one node

```bash
rm -rf /tmp/niahcia-smoke
NIAHCIA_DATA_DIR=/tmp/niahcia-smoke \
NIAHCIA_MINING_RPC_BIND=127.0.0.1:19332 \
NIAHCIA_P2P_BIND=127.0.0.1:19442 \
./target/release/niahcia
```

## Query mining work

```bash
curl -s http://127.0.0.1:19332 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"pow_getWork","params":{}}'
```

A successful response returns a native work template.

## Restart proof

Stop the node cleanly, start it again with the same data directory, and require the persisted canonical head/native state snapshot to load without any external replay service.

## Two-node proof

Run a second node with a distinct data directory and ports, with `NIAHCIA_P2P_PEERS=127.0.0.1:19442`. The peer must validate received native blocks independently and follow the higher cumulative-work chain.

This does not activate NativeTransaction V2 compute settlement or prove production network parameters.
