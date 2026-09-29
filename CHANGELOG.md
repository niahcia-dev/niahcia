# Changelog

All notable NIAHCIA changes will be documented here.

NIAHCIA is currently pre-alpha. The changelog will begin with the first runnable development release.

## Unreleased

### Added
- canonical release structure
- release notes template
- release manifest format
- checksum policy
- generated release-note categories

### Changed
- primary repository README now points normal users to the canonical release path

### Security
- published release artifacts are treated as immutable
- SHA-256 checksum publication is required for binary archives

---

When the first tagged release is published, the Unreleased section will be converted into a versioned entry:

```text
## [0.1.0] - YYYY-MM-DD
```
