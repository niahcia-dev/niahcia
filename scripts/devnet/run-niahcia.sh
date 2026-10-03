#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEVNET_DIR="$ROOT/devnet"
CONFIG="$DEVNET_DIR/niahcia.toml"

mkdir -p "$DEVNET_DIR"

cat > "$CONFIG" <<EOF
network = "devnet"
data_dir = "$DEVNET_DIR/niahcia-data"
fee_recipient = "0x0000000000000000000000000000000000000000"
mining_rpc_bind = "127.0.0.1:9332"
p2p_bind = "127.0.0.1:9442"
p2p_peers = []
log_level = "info"
EOF

cd "$ROOT"
exec cargo run -p niahcia -- --config "$CONFIG"
