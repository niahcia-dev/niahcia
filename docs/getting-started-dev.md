# Developer Quick Start

NIAHCIA is pre-alpha. The current reference node includes native CPU-PoW chain, persistence, mining RPC, and P2P functionality.

## Build

```bash
git clone https://github.com/niahcia/niahcia.git
cd niahcia
cargo build --release
```

## Run

```bash
cargo run -p niahcia
```

Or copy `config/niahcia.example.toml` and pass it with `--config`.

## Check

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The active path does not require Reth, an Ethereum execution client, Engine API, or JWT secret.

NativeTransaction V2 / NativeStateV2 compute-channel work exists behind an inactive boundary and is not yet accepted by the active devnet mempool/P2P/mining path.

See `docs/CURRENT-WORK.md` before continuing development.
