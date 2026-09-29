#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEVNET_DIR="$ROOT/devnet"
JWT="$DEVNET_DIR/jwt.hex"
DATADIR="$DEVNET_DIR/reth-data"

if ! command -v reth >/dev/null 2>&1; then
  echo "reth is not installed or not in PATH" >&2
  exit 1
fi

if [[ ! -f "$JWT" ]]; then
  echo "missing $JWT" >&2
  echo "run: scripts/devnet/generate-jwt.sh" >&2
  exit 1
fi

mkdir -p "$DATADIR"

echo "Starting isolated local Reth execution engine"
echo "  chain spec: built-in dev"
echo "  HTTP RPC:   http://127.0.0.1:8545"
echo "  Engine API: http://127.0.0.1:8551"
echo "  datadir:    $DATADIR"
echo
echo "NOTE: this intentionally uses --chain dev, NOT --dev."
echo "      Reth --dev starts its own local PoA mining loop; NIAHCIA must drive consensus."
echo

exec reth node   --chain dev   --datadir "$DATADIR"   --disable-discovery   --http   --http.addr 127.0.0.1   --http.port 8545   --http.api eth   --authrpc.addr 127.0.0.1   --authrpc.port 8551   --authrpc.jwtsecret "$JWT"
