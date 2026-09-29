# Developer Quick Start

NIAHCIA is pre-alpha. This document currently covers only the first runnable Rust bootstrap.

## Requirements

- Rust stable toolchain
- Cargo

## Build

```bash
git clone https://github.com/niahcia/niahcia.git
cd niahcia
cargo build
```

## Run

```bash
cargo run -p niahcia
```

Expected output:

```text
NIAHCIA 0.1.0-dev
pre-alpha node bootstrap
No network services are implemented yet.
```

## Version

```bash
cargo run -p niahcia -- --version
```

Expected:

```text
niahcia 0.1.0-dev
```

## Help

```bash
cargo run -p niahcia -- --help
```

## Scope

This bootstrap does not yet include:

- Reth
- Engine API
- RandomX
- P2P
- EVM RPC
- mining RPC
- chain state

Those are added incrementally under the v0.1.0 development issues.
