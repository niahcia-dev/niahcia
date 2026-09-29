# Local Reth Devnet Smoke Test

This is the first live integration test for the NIAHCIA execution/mining boundary.

The test is deliberately one-machine and local-only.

## What it proves

A successful run proves:

```text
local Reth
    |
    | authenticated Engine API
    v
NIAHCIA
    |
    | execution results
    v
NIAHCIA-native execution commitment
    |
    v
PowWorkTemplate
    |
    v
pow_getWork
```

It does **not** prove RandomX, mined-block submission, multi-node consensus, P2P, difficulty adjustment, or reorg handling.

## Important Reth mode choice

The smoke test uses:

```text
reth node --chain dev
```

and intentionally does **not** use:

```text
reth node --dev
```

Reth's `--dev` option starts Reth's own local proof-of-authority mining behavior. NIAHCIA must be the consensus driver, so the test uses the built-in `dev` chain specification without enabling Reth's dev-mining mode.

Reth's CLI documents `dev` as a built-in chain and separately documents `--dev` as the mode that runs a local PoA consensus engine.

## Requirements

- Linux development host
- current Rust toolchain
- `reth` in `PATH`
- `openssl`
- `curl`
- `python3`

## Terminal 1 — generate JWT

From the repository root:

```bash
scripts/devnet/generate-jwt.sh
```

This creates:

```text
devnet/jwt.hex
```

The file is development-only and must never be committed.

## Terminal 1 — start Reth

```bash
scripts/devnet/run-reth.sh
```

Expected local endpoints:

```text
HTTP JSON-RPC  http://127.0.0.1:8545
Engine API     http://127.0.0.1:8551
```

The script disables discovery so this first test stays isolated.

## Terminal 2 — verify Reth RPC

```bash
curl -s http://127.0.0.1:8545 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}'
```

Then:

```bash
curl -s http://127.0.0.1:8545 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_getBlockByNumber","params":["latest",false]}'
```

## Terminal 2 — start NIAHCIA

```bash
scripts/devnet/run-niahcia.sh
```

The script writes an ignored development configuration at:

```text
devnet/niahcia.toml
```

NIAHCIA should:

1. authenticate to Reth's Engine API,
2. read the local execution head,
3. request an execution payload,
4. compute its own transaction commitment,
5. compute its own execution commitment,
6. create a NIAHCIA PoW work template,
7. expose it on port 9332.

## Terminal 3 — test mining work

```bash
scripts/devnet/test-work.sh
```

Success requires a JSON-RPC result and a non-zero `execution_commitment`.

The script prints `PASS` only if those checks succeed.

## Expected first failure mode

The most likely first integration failure is Reth rejecting the current Engine API V3 payload attributes because NIAHCIA has no Ethereum beacon chain and intentionally supplies zero values for PoS-specific fields.

If that happens, capture the exact NIAHCIA and Reth error output.

Do not work around the failure by connecting to Ethereum, an Ethereum consensus client, or a public RPC provider. The correct fix is to adapt the local execution-engine interface while preserving NIAHCIA's independent consensus.

## Reset

Stop both processes and remove the local state:

```bash
rm -rf devnet/reth-data devnet/niahcia-data devnet/niahcia.toml
```

Keep `devnet/jwt.hex` if you want to reuse the same local JWT secret.

## Security

All ports in these scripts bind to `127.0.0.1`.

Do not expose the Engine API or mining development RPC to the public Internet.
