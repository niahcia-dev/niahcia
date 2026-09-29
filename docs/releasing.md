# Releasing NIAHCIA

This document defines the release procedure for the canonical NIAHCIA distribution.

## 1. Choose a version

Examples:

```text
v0.1.0
v0.2.0-rc.1
v1.0.0
```

The version must be used consistently across:

- Git tag
- release title
- release manifest
- release notes
- packaged assets

## 2. Freeze component revisions

Record the exact commit used for every included component:

```text
niahcia
niahcia-miner
niahcia-compute
```

Explorer and web releases may be included independently when appropriate.

No release asset should be built from an unrecorded working tree.

## 3. Build release assets

Expected Linux assets:

```text
niahcia-node-linux-x86_64.tar.gz
niahcia-miner-linux-x86_64.tar.gz
niahcia-compute-linux-x86_64.tar.gz
niahcia-full-linux-x86_64.tar.gz
```

The exact build process will be automated once the corresponding binaries exist.

## 4. Generate checksums

From the release directory:

```bash
sha256sum niahcia-*.tar.gz > SHA256SUMS
```

Do not manually type checksums.

## 5. Generate release manifest

Create `release-manifest.json` from the final artifacts.

The manifest must reference the same filenames and hashes present in `SHA256SUMS`.

## 6. Verify locally

Before publishing:

```bash
sha256sum -c SHA256SUMS
```

Extract each archive into a clean directory and verify expected binaries are present.

For runnable releases, execute each binary's version command and confirm it reports the intended release version.

## 7. Tag the release

The canonical tag lives in `niahcia/niahcia`.

Example:

```bash
git tag -s v0.1.0 -m "NIAHCIA v0.1.0"
git push origin v0.1.0
```

Signed tags are preferred once the release-signing policy is finalized.

## 8. Create GitHub Release

Use the tag created above.

Release title:

```text
NIAHCIA v0.1.0
```

Use `release/RELEASE_NOTES_TEMPLATE.md` as the starting structure.

Attach:

- all packaged binaries
- `SHA256SUMS`
- `release-manifest.json`

## 9. Verify the public release

After publishing:

- download every asset from GitHub
- run `sha256sum -c SHA256SUMS`
- verify the release page points to the correct tag
- verify `/releases/latest` resolves correctly for stable releases
- verify the NIAHCIA downloads page detects the release

## 10. Do not overwrite published artifacts

A published asset is immutable for operational purposes.

If an artifact is wrong:

1. document the problem,
2. publish a corrected patch or replacement release,
3. do not silently replace a binary while keeping the same version.
