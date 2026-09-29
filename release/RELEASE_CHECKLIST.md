# NIAHCIA Release Checklist

Use this checklist for every public NIAHCIA release.

## Version

- [ ] version selected
- [ ] release channel selected: stable / rc / dev
- [ ] tag name confirmed
- [ ] release title confirmed

## Source revisions

- [ ] core/node commit recorded
- [ ] miner commit recorded
- [ ] compute commit recorded
- [ ] any additional component commits recorded
- [ ] working trees clean

## Build

- [ ] node package built
- [ ] miner package built
- [ ] compute package built
- [ ] full-stack package built if applicable
- [ ] package filenames match release policy
- [ ] binaries report expected version

## Verification

- [ ] packages extracted in clean directory
- [ ] smoke test completed
- [ ] SHA256SUMS generated from final artifacts
- [ ] `sha256sum -c SHA256SUMS` passes
- [ ] release manifest generated
- [ ] manifest hashes match SHA256SUMS

## Release notes

- [ ] highlights written
- [ ] required/recommended/optional upgrade status stated
- [ ] protocol/consensus changes stated explicitly
- [ ] compatibility requirements documented
- [ ] known issues documented
- [ ] upgrade instructions documented

## Publish

- [ ] tag created
- [ ] GitHub Release created from correct tag
- [ ] all binary assets attached
- [ ] SHA256SUMS attached
- [ ] release-manifest.json attached
- [ ] release notes reviewed before publish

## Post-publish verification

- [ ] download every asset from GitHub
- [ ] checksums verified from downloaded files
- [ ] `/releases/latest` resolves correctly when applicable
- [ ] downloads page detects current release
- [ ] README download link resolves correctly
- [ ] changelog updated
- [ ] announcement prepared if appropriate

## Failure rule

If a published artifact is wrong, do not silently replace it under the same version.

Publish a corrected release and document the problem.
