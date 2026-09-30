#!/usr/bin/env bash
set -euo pipefail

ROOT="${NIAHCIA_DEVNET_ROOT:-$HOME/niahcia-devnet}"
NIAHCIA_DIR="${NIAHCIA_DIR:-$ROOT/niahcia}"
RETH_BIN="${RETH_BIN:-$ROOT/reth/target/release/reth}"
NIAHCIA_BIN="${NIAHCIA_BIN:-$NIAHCIA_DIR/target/release/niahcia}"
JWT="${NIAHCIA_RETH_JWT_PATH:-$ROOT/jwt.hex}"
SMOKE_ROOT="${NIAHCIA_INTERRUPTED_SYNC_ROOT:-$ROOT/interrupted-sync-smoke}"
A_RETH_DATA="$SMOKE_ROOT/reth-a"; B_RETH_DATA="$SMOKE_ROOT/reth-b"
A_DATA="$SMOKE_ROOT/niahcia-a"; B_DATA="$SMOKE_ROOT/niahcia-b"
A_RETH_HTTP="http://127.0.0.1:18545"; A_RETH_ENGINE="http://127.0.0.1:18551"
A_MINING="http://127.0.0.1:19332"; A_P2P="127.0.0.1:19442"
B_RETH_HTTP="http://127.0.0.1:19545"; B_RETH_ENGINE="http://127.0.0.1:19551"
B_MINING="http://127.0.0.1:20332"; B_P2P="127.0.0.1:20442"
tmp="$(mktemp -d)"; a_reth_pid=""; b_reth_pid=""; a_pid=""; b_pid=""

cleanup() {
  set +e
  for pid in "$b_pid" "$a_pid" "$b_reth_pid" "$a_reth_pid"; do [[ -n "$pid" ]] && kill -INT "$pid" 2>/dev/null; done
  for pid in "$b_pid" "$a_pid" "$b_reth_pid" "$a_reth_pid"; do [[ -n "$pid" ]] && wait "$pid" 2>/dev/null; done
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

rpc() {
  local url="$1" method="$2" params="$3"
  curl -fsS "$url" -H 'Content-Type: application/json' --data "$(jq -nc --arg method "$method" --argjson params "$params" '{jsonrpc:"2.0",id:1,method:$method,params:$params}')"
}

wait_rpc() {
  local url="$1" method="$2" params="$3"
  for _ in $(seq 1 120); do if rpc "$url" "$method" "$params" >/dev/null 2>&1; then return 0; fi; sleep 1; done
  echo "RPC did not become ready: $url $method" >&2; return 1
}

start_reth() {
  local name="$1" data="$2" http_port="$3" engine_port="$4" p2p_port="$5"
  "$RETH_BIN" node --chain dev --datadir "$data" --port "$p2p_port" --disable-discovery --http --http.addr 127.0.0.1 --http.port "$http_port" --http.api eth,net,web3 --authrpc.addr 127.0.0.1 --authrpc.port "$engine_port" --authrpc.jwtsecret "$JWT" --ipcdisable >"$tmp/reth-$name.log" 2>&1 &
  local pid=$!; if [[ "$name" == "a" ]]; then a_reth_pid="$pid"; else b_reth_pid="$pid"; fi
  for _ in $(seq 1 120); do
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "Reth $name exited during startup" >&2
      cat "$tmp/reth-$name.log" >&2
      return 1
    fi
    if rpc "http://127.0.0.1:$http_port" eth_blockNumber '[]' >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "Execution RPC did not become ready for Reth $name on port $http_port" >&2
  cat "$tmp/reth-$name.log" >&2
  return 1
}

start_node() {
  local name="$1" data="$2" reth_http="$3" reth_engine="$4" mining="$5" p2p="$6" peers="$7"
  (cd "$NIAHCIA_DIR"; NIAHCIA_NETWORK=devnet NIAHCIA_DATA_DIR="$data" NIAHCIA_RETH_HTTP_RPC="$reth_http" NIAHCIA_RETH_ENGINE_API="$reth_engine" NIAHCIA_RETH_JWT_PATH="$JWT" NIAHCIA_MINING_RPC_BIND="${mining#http://}" NIAHCIA_P2P_BIND="$p2p" NIAHCIA_P2P_PEERS="$peers" NIAHCIA_LOG_LEVEL=info "$NIAHCIA_BIN") >"$tmp/niahcia-$name.log" 2>&1 &
  local pid=$!; if [[ "$name" == "a" ]]; then a_pid="$pid"; else b_pid="$pid"; fi
  wait_rpc "$mining" pow_getWork '{}'
}

stop_a() { kill -INT "$a_pid"; wait "$a_pid"; a_pid=""; }
stop_b() { kill -INT "$b_pid"; wait "$b_pid"; b_pid=""; }

mine_on() {
  local mining="$1"
  local work gen template height result
  work="$(rpc "$mining" pow_getWork '{}')"; gen="$(jq -r '.result.generation' <<<"$work")"; template="$(jq -r '.result.template_id' <<<"$work")"; height="$(jq -r '.result.height' <<<"$work")"
  result="$(rpc "$mining" pow_submitWork "$(jq -nc --argjson generation "$gen" --arg template_id "$template" '{generation:$generation,template_id:$template_id,nonce:0,extra_nonce:0}')")"
  jq -e '.result.accepted == true and .result.became_canonical == true' <<<"$result" >/dev/null; echo "$height"
}

mine_a() { mine_on "$A_MINING"; }
mine_b() { mine_on "$B_MINING"; }

wait_equal_work() {
  for _ in $(seq 1 120); do
    local a_work b_work a_height b_height a_parent b_parent
    a_work="$(rpc "$A_MINING" pow_getWork '{}')"; b_work="$(rpc "$B_MINING" pow_getWork '{}')"
    a_height="$(jq -r '.result.height' <<<"$a_work")"; b_height="$(jq -r '.result.height' <<<"$b_work")"
    a_parent="$(jq -r '.result.parent_hash' <<<"$a_work")"; b_parent="$(jq -r '.result.parent_hash' <<<"$b_work")"
    if [[ "$a_height" == "$b_height" && "$a_parent" == "$b_parent" ]]; then echo "$a_height"; return 0; fi
    sleep 1
  done
  echo "Node B did not reach Node A canonical NIAHCIA tip" >&2; cat "$tmp/niahcia-b.log" >&2; return 1
}

command -v curl >/dev/null; command -v jq >/dev/null
[[ -x "$RETH_BIN" ]] || { echo "Missing Reth binary: $RETH_BIN" >&2; exit 1; }
[[ -x "$NIAHCIA_BIN" ]] || { echo "Missing NIAHCIA binary: $NIAHCIA_BIN" >&2; exit 1; }
[[ -f "$JWT" ]] || { echo "Missing JWT secret: $JWT" >&2; exit 1; }
rm -rf "$SMOKE_ROOT"; mkdir -p "$SMOKE_ROOT"

BACKLOG="${NIAHCIA_INTERRUPTED_SYNC_BLOCKS:-132}"
INTERRUPT_AT="${NIAHCIA_INTERRUPTED_SYNC_AT:-10}"
if (( BACKLOG <= 128 )); then
  echo "NIAHCIA_INTERRUPTED_SYNC_BLOCKS must exceed 128" >&2
  exit 1
fi

echo "Starting independent execution nodes..."
start_reth a "$A_RETH_DATA" 18545 18551 30303
start_reth b "$B_RETH_DATA" 19545 19551 30304
echo "Starting A and building a $BACKLOG-block synchronization backlog..."
start_node a "$A_DATA" "$A_RETH_HTTP" "$A_RETH_ENGINE" "$A_MINING" "$A_P2P" ""
for _ in $(seq 1 "$BACKLOG"); do
  height="$(mine_a)"
  if (( height % 20 == 0 || height + 1 == BACKLOG )); then echo "A backlog reached next height $((height + 1))"; fi
done

echo "Starting B and waiting for partial first-batch progress..."
start_node b "$B_DATA" "$B_RETH_HTTP" "$B_RETH_ENGINE" "$B_MINING" "$B_P2P" "$A_P2P"
for _ in $(seq 1 240); do
  b_height="$(rpc "$B_MINING" pow_getWork '{}' | jq -r '.result.height')"
  if (( b_height >= INTERRUPT_AT && b_height < BACKLOG )); then break; fi
  sleep 0.1
done
if (( b_height < INTERRUPT_AT )); then
  echo "B did not make partial sync progress before timeout (height=$b_height)" >&2
  exit 1
fi
echo "B reached next work height $b_height; stopping A during synchronization..."
stop_a

sleep 2
b_after="$(rpc "$B_MINING" pow_getWork '{}' | jq -r '.result.height')"
if (( b_after < b_height )); then
  echo "B lost persisted sync progress after peer interruption: before=$b_height after=$b_after" >&2
  exit 1
fi
echo "B remained healthy with persisted progress at next work height $b_after"

echo "Restarting A with its existing state; B must resume without restart..."
start_node a "$A_DATA" "$A_RETH_HTTP" "$A_RETH_ENGINE" "$A_MINING" "$A_P2P" ""
final_height="$(wait_equal_work)"
a_reth="$(rpc "$A_RETH_HTTP" eth_blockNumber '[]' | jq -r '.result')"
b_reth="$(rpc "$B_RETH_HTTP" eth_blockNumber '[]' | jq -r '.result')"
[[ "$a_reth" == "$b_reth" ]] || { echo "Execution canonical height mismatch after interrupted-sync recovery: A=$a_reth B=$b_reth" >&2; exit 1; }

echo "PASS: interrupted multi-batch sync resumed without restarting B"
echo "NIAHCIA next work height: $final_height"
echo "Execution canonical height: $a_reth"
