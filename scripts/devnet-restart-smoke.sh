#!/usr/bin/env bash
set -euo pipefail

ROOT="${NIAHCIA_DEVNET_ROOT:-$HOME/niahcia-devnet}"
NIAHCIA_DIR="${NIAHCIA_DIR:-$ROOT/niahcia}"
RETH_BIN="${RETH_BIN:-$ROOT/reth/target/release/reth}"
RETH_DATA="${RETH_DATA:-$ROOT/reth-data}"
NIAHCIA_DATA="${NIAHCIA_DATA:-$ROOT/niahcia-data}"
JWT="${NIAHCIA_RETH_JWT_PATH:-$ROOT/jwt.hex}"
NIAHCIA_BIN="${NIAHCIA_BIN:-$NIAHCIA_DIR/target/release/niahcia}"
RETH_HTTP="${NIAHCIA_RETH_HTTP_RPC:-http://127.0.0.1:8545}"
RETH_ENGINE="${NIAHCIA_RETH_ENGINE_API:-http://127.0.0.1:8551}"
MINING_HTTP="${NIAHCIA_MINING_RPC_HTTP:-http://127.0.0.1:9332}"
MINING_BIND="${NIAHCIA_MINING_RPC_BIND:-127.0.0.1:9332}"
BLOCKS_BEFORE_RESTART="${NIAHCIA_SMOKE_BLOCKS:-3}"

tmp="$(mktemp -d)"
reth_pid=""
niahcia_pid=""

cleanup() {
  set +e
  [[ -n "$niahcia_pid" ]] && kill -INT "$niahcia_pid" 2>/dev/null
  [[ -n "$reth_pid" ]] && kill -INT "$reth_pid" 2>/dev/null
  wait "$niahcia_pid" 2>/dev/null
  wait "$reth_pid" 2>/dev/null
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

rpc() {
  local url="$1" method="$2" params="$3"
  curl -fsS "$url" -H 'Content-Type: application/json'     --data "$(jq -nc --arg method "$method" --argjson params "$params"       '{jsonrpc:"2.0",id:1,method:$method,params:$params}')"
}

wait_rpc() {
  local url="$1" method="$2" params="$3"
  for _ in $(seq 1 120); do
    if rpc "$url" "$method" "$params" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "RPC did not become ready: $url $method" >&2
  return 1
}

start_reth() {
  "$RETH_BIN" node --chain dev --datadir "$RETH_DATA"     --http --http.addr 127.0.0.1 --http.port 8545 --http.api eth,net,web3     --authrpc.addr 127.0.0.1 --authrpc.port 8551 --authrpc.jwtsecret "$JWT"     --ipcdisable >"$tmp/reth.log" 2>&1 &
  reth_pid=$!
  wait_rpc "$RETH_HTTP" eth_blockNumber '[]'
}

start_niahcia() {
  (
    cd "$NIAHCIA_DIR"
    NIAHCIA_NETWORK=devnet     NIAHCIA_DATA_DIR="$NIAHCIA_DATA"     NIAHCIA_RETH_HTTP_RPC="$RETH_HTTP"     NIAHCIA_RETH_ENGINE_API="$RETH_ENGINE"     NIAHCIA_RETH_JWT_PATH="$JWT"     NIAHCIA_MINING_RPC_BIND="$MINING_BIND"     NIAHCIA_LOG_LEVEL=info     "$NIAHCIA_BIN"
  ) >"$tmp/niahcia.log" 2>&1 &
  niahcia_pid=$!
  wait_rpc "$MINING_HTTP" pow_getWork '{}'
}

stop_niahcia() {
  kill -INT "$niahcia_pid"
  wait "$niahcia_pid"
  niahcia_pid=""
}

stop_reth() {
  kill -INT "$reth_pid"
  wait "$reth_pid"
  reth_pid=""
}

mine_one() {
  local work gen template height result
  work="$(rpc "$MINING_HTTP" pow_getWork '{}')"
  gen="$(jq -r '.result.generation' <<<"$work")"
  template="$(jq -r '.result.template_id' <<<"$work")"
  height="$(jq -r '.result.height' <<<"$work")"
  result="$(rpc "$MINING_HTTP" pow_submitWork     "$(jq -nc --argjson generation "$gen" --arg template_id "$template"       '{generation:$generation,template_id:$template_id,nonce:0,extra_nonce:0}')")"
  jq -e --argjson height "$height"     '.result.accepted == true and .result.became_canonical == true and .result.height == $height'     <<<"$result" >/dev/null
  echo "$height"
}

command -v curl >/dev/null
command -v jq >/dev/null
[[ -x "$RETH_BIN" ]] || { echo "Missing Reth binary: $RETH_BIN" >&2; exit 1; }
[[ -x "$NIAHCIA_BIN" ]] || { echo "Missing NIAHCIA binary: $NIAHCIA_BIN" >&2; exit 1; }
[[ -f "$JWT" ]] || { echo "Missing JWT secret: $JWT" >&2; exit 1; }

echo "Starting persisted devnet..."
start_reth
start_niahcia

initial_height="$(rpc "$MINING_HTTP" pow_getWork '{}' | jq -r '.result.height')"
echo "Initial NIAHCIA work height: $initial_height"

for _ in $(seq 1 "$BLOCKS_BEFORE_RESTART"); do
  echo "Mined NIAHCIA height $(mine_one)"
done

pre_restart_work="$(rpc "$MINING_HTTP" pow_getWork '{}')"
pre_restart_height="$(jq -r '.result.height' <<<"$pre_restart_work")"
pre_restart_parent="$(jq -r '.result.parent_hash' <<<"$pre_restart_work")"
pre_restart_reth="$(rpc "$RETH_HTTP" eth_blockNumber '[]' | jq -r '.result')"

echo "Restart checkpoint: next NIAHCIA=$pre_restart_height Reth=$pre_restart_reth"
stop_niahcia
stop_reth

start_reth
recovered_reth="$(rpc "$RETH_HTTP" eth_blockNumber '[]' | jq -r '.result')"
[[ "$recovered_reth" == "$pre_restart_reth" ]] || {
  echo "Reth persistence mismatch: before=$pre_restart_reth after=$recovered_reth" >&2
  exit 1
}

start_niahcia
post_restart_work="$(rpc "$MINING_HTTP" pow_getWork '{}')"
post_restart_height="$(jq -r '.result.height' <<<"$post_restart_work")"
post_restart_parent="$(jq -r '.result.parent_hash' <<<"$post_restart_work")"
[[ "$post_restart_height" == "$pre_restart_height" ]] || {
  echo "NIAHCIA height recovery mismatch: before=$pre_restart_height after=$post_restart_height" >&2
  exit 1
}
[[ "$post_restart_parent" == "$pre_restart_parent" ]] || {
  echo "NIAHCIA parent recovery mismatch" >&2
  exit 1
}

echo "Recovered NIAHCIA work height $post_restart_height with matching parent"
mined_after_restart="$(mine_one)"
[[ "$mined_after_restart" == "$post_restart_height" ]]

next_height="$(rpc "$MINING_HTTP" pow_getWork '{}' | jq -r '.result.height')"
expected_next="$((post_restart_height + 1))"
[[ "$next_height" -eq "$expected_next" ]] || {
  echo "Next template mismatch: expected=$expected_next actual=$next_height" >&2
  exit 1
}

final_reth_hex="$(rpc "$RETH_HTTP" eth_blockNumber '[]' | jq -r '.result')"
final_reth_dec="$((final_reth_hex))"
expected_reth="$((next_height))"
[[ "$final_reth_dec" -eq "$expected_reth" ]] || {
  echo "Reth height mismatch: expected=$expected_reth actual=$final_reth_dec" >&2
  exit 1
}

echo "PASS: restart recovery and post-restart mining succeeded"
echo "NIAHCIA next height: $next_height"
echo "Reth execution height: $final_reth_dec"
