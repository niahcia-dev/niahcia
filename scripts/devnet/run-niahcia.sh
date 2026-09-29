#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEVNET_DIR="$ROOT/devnet"
CONFIG="$DEVNET_DIR/niahcia.toml"

if [[ ! -f "$DEVNET_DIR/jwt.hex" ]]; then
  echo "missing $DEVNET_DIR/jwt.hex" >&2
  echo "run: scripts/devnet/generate-jwt.sh" >&2
  exit 1
fi

cat > "$CONFIG" <<EOF
network = "devnet"
data_dir = "$DEVNET_DIR/niahcia-data"

reth_engine_api = "http://127.0.0.1:8551"
reth_http_rpc = "http://127.0.0.1:8545"
reth_jwt_path = "$DEVNET_DIR/jwt.hex"

fee_recipient = "0x0000000000000000000000000000000000000000"

mining_rpc_bind = "127.0.0.1:9332"
log_level = "info"
EOF

cd "$ROOT"
exec cargo run -p niahcia -- --config "$CONFIG"
