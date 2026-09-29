# Reth Engine API Connection

NIAHCIA uses Reth as the EVM execution engine.

The NIAHCIA node authenticates to Reth's Engine API using the standard JWT secret file supplied to the execution client.

## Current pre-alpha check

On startup NIAHCIA:

1. loads the configured JWT secret,
2. requires at least 32 bytes of secret material,
3. creates a short-lived HS256 Engine API JWT with an `iat` claim,
4. connects to the configured authenticated Engine API endpoint,
5. calls `engine_exchangeCapabilities`,
6. logs the capabilities returned by Reth,
7. refuses to continue if authentication/connectivity fails.

This is deliberately small: it proves that the NIAHCIA consensus process and Reth can communicate across an authenticated boundary before payload construction is implemented.

## Configuration

```toml
reth_engine_api = "http://127.0.0.1:8551"
reth_jwt_path = "./jwt.hex"
```

The JWT file is hex-encoded and may optionally begin with `0x`.

## Generate a development JWT secret

Linux:

```bash
openssl rand -hex 32 > jwt.hex
chmod 600 jwt.hex
```

Reth and NIAHCIA must use the same JWT secret.

## Expected failure

If Reth is not running, the endpoint is incorrect, or authentication fails, NIAHCIA exits instead of pretending the execution engine is available.

## Boundary

The Engine API client performs transport/authentication and JSON-RPC.

It does not own:

- fork choice
- PoW validation
- RandomX
- difficulty
- block rewards

Those belong to NIAHCIA consensus code.
