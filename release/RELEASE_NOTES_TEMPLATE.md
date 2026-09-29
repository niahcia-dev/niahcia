# NIAHCIA vX.Y.Z

**Release channel:** stable / rc / dev  
**Release date:** YYYY-MM-DD  
**Canonical tag:** `vX.Y.Z`

## Download

Use the attached release assets below.

## Required upgrade

State one of:

- **Required**
- **Recommended**
- **Optional**
- **Not applicable**

Explain why.

## Highlights

- 
- 
- 

## Node / Core

- 

## Miner

- 

## Compute Worker

- 

## Protocol / Consensus

State all protocol, consensus, network, or compatibility changes explicitly.

If none:

> No protocol or consensus changes in this release.

## Compatibility

- supported network:
- database/config migration:
- minimum compatible peer version:
- minimum compatible miner version:
- minimum compatible compute-worker version:

## Known issues

- 

## Upgrade notes

```text
Stop the existing process.
Back up configuration/data as appropriate.
Replace binaries with the new release.
Start the process.
Verify reported version and network sync.
```

Exact commands should be supplied once the runnable software exists.

## Release assets

Expected naming:

```text
niahcia-node-<platform>-<arch>.tar.gz
niahcia-miner-<platform>-<arch>.tar.gz
niahcia-compute-<platform>-<arch>.tar.gz
niahcia-full-<platform>-<arch>.tar.gz
SHA256SUMS
release-manifest.json
```

## Verification

Download `SHA256SUMS` and verify:

```bash
sha256sum -c SHA256SUMS
```

## Component revisions

| Component | Commit |
|---|---|
| niahcia | `<commit>` |
| niahcia-miner | `<commit>` |
| niahcia-compute | `<commit>` |

## Contributors

Thank you to everyone who contributed to this release.
