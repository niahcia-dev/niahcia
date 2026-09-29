#!/usr/bin/env bash
set -euo pipefail

RPC="http://127.0.0.1:9332"

response="$(
  curl --fail --silent --show-error "$RPC"     -H 'content-type: application/json'     --data '{"jsonrpc":"2.0","id":1,"method":"pow_getWork","params":[]}'
)"

printf '%s
' "$response"

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
    "height",
    "parent_hash",
    "execution_commitment",
    "timestamp",
    "difficulty",
    "target",
    "nonce_start",
    "nonce_end",
]
missing = [name for name in required if name not in result]
if missing:
    raise SystemExit(f"FAIL: missing fields: {', '.join(missing)}")

zero32 = "00" * 32
if result["execution_commitment"].lower().removeprefix("0x") == zero32:
    raise SystemExit("FAIL: execution commitment is still the zero placeholder")

if result["template_id"].lower().removeprefix("0x") == zero32:
    raise SystemExit("FAIL: template ID is zero")

print()
print("PASS: pow_getWork returned a non-zero execution-backed work template")
print(f"generation:           {result['generation']}")
print(f"height:               {result['height']}")
print(f"template_id:          {result['template_id']}")
print(f"execution_commitment: {result['execution_commitment']}")
PY
