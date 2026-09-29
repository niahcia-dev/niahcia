#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEVNET_DIR="$ROOT/devnet"

mkdir -p "$DEVNET_DIR"

if [[ -e "$DEVNET_DIR/jwt.hex" ]]; then
  echo "JWT already exists: $DEVNET_DIR/jwt.hex"
  exit 0
fi

openssl rand -hex 32 > "$DEVNET_DIR/jwt.hex"
chmod 600 "$DEVNET_DIR/jwt.hex"

echo "Created $DEVNET_DIR/jwt.hex"
