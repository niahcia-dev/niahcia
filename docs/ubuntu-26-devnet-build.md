# Ubuntu 26.04 native devnet build

## Install prerequisites

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libclang-dev clang cmake curl git

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update stable
```

## Clone and build

```bash
git clone https://github.com/niahcia/niahcia.git
cd niahcia
cargo build --release
```

## Run

```bash
cp config/niahcia.example.toml niahcia.toml
./target/release/niahcia --config niahcia.toml
```

The active reference node uses native NIAHCIA execution/state and does not require Reth, Engine API, or a JWT secret.

## Validate

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

This is pre-alpha devnet guidance, not production deployment guidance.
