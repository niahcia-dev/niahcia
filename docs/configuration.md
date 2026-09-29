# Node Configuration

NIAHCIA currently uses a small TOML configuration file for the pre-alpha node bootstrap.

## Example

Copy the repository example:

```bash
cp config/niahcia.example.toml niahcia.toml
cargo run -p niahcia -- --config niahcia.toml
```

## Fields

```toml
network = "devnet"
data_dir = "./data"

reth_engine_api = "http://127.0.0.1:8551"
reth_jwt_path = "./jwt.hex"

mining_rpc_bind = "127.0.0.1:9332"
log_level = "info"
```

## Environment overrides

Every current field may be overridden:

```text
NIAHCIA_NETWORK
NIAHCIA_DATA_DIR
NIAHCIA_RETH_ENGINE_API
NIAHCIA_RETH_JWT_PATH
NIAHCIA_MINING_RPC_BIND
NIAHCIA_LOG_LEVEL
```

## Startup lifecycle

The node currently:

1. parses CLI arguments,
2. loads configuration,
3. applies environment overrides,
4. validates configuration,
5. initializes logging,
6. creates the data directory,
7. installs SIGINT/SIGTERM-compatible Ctrl-C handling where supported by the runtime,
8. enters the bootstrap run loop,
9. exits cleanly when shutdown is requested.

Reth connectivity is implemented in the next development issue.
