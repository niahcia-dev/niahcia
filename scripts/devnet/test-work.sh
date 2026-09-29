#!/usr/bin/env bash
set -euo pipefail

RPC="http://127.0.0.1:9332"

response="$(
  curl --fail --silent --show-error "$RPC" \
    -H 'content-type: application/json' \
    --data '{"jsonrpc":"2.0","id":1,"method":"pow_getWork","params":[]}'
)"

printf '%s\n' "$response"

python3 - "$response" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
result = payload.get("result")
if not isinstance(result, dict):
    raise SystemExit("FAIL: JSON-RPC result missing")

required = [
    "generation",
    "template_id",
    "version",
    "parent_hash",
    "height",
    "timestamp",
    "transactions_root",
    "execution_root",
    "target",
    "randomx_seed_height",
    "randomx_seed",
    "nonce_start",
    "nonce_end",
    "extra_nonce_start",
    "extra_nonce_end",
]
missing = [name for name in required if name not in result]
if missing:
    raise SystemExit(f"FAIL: missing fields: {', '.join(missing)}")

zero32 = "00" * 32
if result["execution_root"].lower().removeprefix("0x") == zero32:
    raise SystemExit("FAIL: execution root is still the zero placeholder")

if result["template_id"].lower().removeprefix("0x") == zero32:
    raise SystemExit("FAIL: template ID is zero")

print()
print("PASS: pow_getWork returned a NIAHCIA BlockHeaderV1 mining template")
print(f"generation:        {result['generation']}")
print(f"height:            {result['height']}")
print(f"template_id:       {result['template_id']}")
print(f"transactions_root: {result['transactions_root']}")
print(f"execution_root:    {result['execution_root']}")
print(f"randomx_seed:      {result['randomx_seed']}")
print(f"seed_height:       {result['randomx_seed_height']}")
PY
