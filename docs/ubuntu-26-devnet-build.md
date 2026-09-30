# Ubuntu 26.04 devnet build

This is the tested source-build path for the NIAHCIA development stack on Ubuntu 26.04 x86_64.

## 1. Install build prerequisites and Rust

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libclang-dev \
  clang \
  cmake \
  curl \
  git

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update stable

rustc --version
cargo --version
```

The initial tested workstation used Rust 1.98.1.

## 2. Create the devnet workspace

```bash
mkdir -p ~/niahcia-devnet
cd ~/niahcia-devnet
```

## 3. Build Reth v2.7.0

Clone the exact Reth version used by the current devnet:

```bash
git clone --branch v2.7.0 --depth 1 https://github.com/paradigmxyz/reth.git
cd reth
```

Reth v2.7.0's current source graph uses `revmc` / `llvm-sys 221`, so the Ubuntu 26.04 build needs LLVM 22 and Polly:

```bash
sudo apt install -y llvm-22 llvm-22-dev clang-22 libclang-22-dev libpolly-22-dev
```

Expose LLVM 22 to Cargo:

```bash
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22
export PATH="/usr/lib/llvm-22/bin:$PATH"
```

Build Reth:

```bash
cargo build --release --bin reth
```

Verify:

```bash
./target/release/reth --version
```

### LLVM troubleshooting

If the build fails with:

```text
No suitable version of LLVM was found system-wide or pointed to by LLVM_SYS_221_PREFIX
```

install the LLVM 22 packages above and make sure:

```bash
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22
export PATH="/usr/lib/llvm-22/bin:$PATH"
```

are set in the shell used for the build.

If the build instead fails with:

```text
could not find native static library `Polly`
```

install:

```bash
sudo apt install -y libpolly-22-dev
```

then rerun the Reth build with the LLVM environment variables set.

## 4. Build NIAHCIA

From the devnet workspace:

```bash
cd ~/niahcia-devnet
git clone https://github.com/niahcia/niahcia.git
cd niahcia
source "$HOME/.cargo/env"
cargo build --release
```

The NIAHCIA node defaults to the local development endpoints:

```text
network:          devnet
Reth HTTP RPC:    http://127.0.0.1:8545
Reth Engine API:  http://127.0.0.1:8551
mining RPC:       127.0.0.1:9332
JWT path:         ./jwt.hex
data directory:   ./data
```

These defaults keep the first development stack local to the workstation.

## Notes

- This document describes a pre-alpha development environment, not a production deployment.
- Keep Reth and NIAHCIA state in isolated devnet directories.
- Do not reuse production Ethereum/Reth data directories for NIAHCIA testing.
- The LLVM environment exports are shell-local. Re-export them before rebuilding Reth in a new shell, or add an equivalent local development setup.
