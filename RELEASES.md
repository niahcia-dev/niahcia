# NIAHCIA Releases

This repository is the canonical release distribution point for NIAHCIA.

## Release channels

NIAHCIA uses three release channels:

- **stable** — production-ready release
- **rc** — release candidate
- **dev** — development/test release

Until the network is production-ready, releases will use pre-1.0 version numbers.

## Versioning

NIAHCIA uses semantic-style versioning:

```text
MAJOR.MINOR.PATCH
```

Examples:

```text
v0.1.0
v0.2.0-rc.1
v0.2.1
```

Before `v1.0.0`, breaking changes may occur between minor versions.

## Canonical release page

Latest:

https://github.com/niahcia/niahcia/releases/latest

All releases:

https://github.com/niahcia/niahcia/releases

## Expected release assets

A normal Linux release is expected to publish:

```text
niahcia-node-linux-x86_64.tar.gz
niahcia-miner-linux-x86_64.tar.gz
niahcia-compute-linux-x86_64.tar.gz
niahcia-full-linux-x86_64.tar.gz
SHA256SUMS
release-manifest.json
```

Additional architectures/platforms may be added later.

## Asset naming

Binary archives use:

```text
<component>-<platform>-<architecture>.<archive>
```

Examples:

```text
niahcia-node-linux-x86_64.tar.gz
niahcia-miner-linux-x86_64.tar.gz
niahcia-compute-linux-x86_64.tar.gz
niahcia-full-linux-x86_64.tar.gz
```

Names are lowercase and stable so download tooling can depend on them.

## Checksums

Every downloadable binary archive MUST be listed in `SHA256SUMS`.

Example:

```text
<sha256>  niahcia-node-linux-x86_64.tar.gz
<sha256>  niahcia-miner-linux-x86_64.tar.gz
```

## Release manifest

Every release SHOULD include `release-manifest.json` describing:

- release version
- git commit
- release channel
- build date
- platform/architecture
- component versions
- asset filenames
- SHA-256 hashes

See `release/release-manifest.example.json`.

## Release notes

Every release should clearly state:

- what changed
- required upgrades
- consensus/protocol changes
- compatibility changes
- known issues
- installation/update notes
- checksums and verification instructions

See `release/RELEASE_NOTES_TEMPLATE.md`.

## Source vs binary releases

The source repository and component repositories remain separate.

The main NIAHCIA release page is the normal-user distribution point even when an asset was built from another NIAHCIA repository.
