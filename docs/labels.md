# Issue and Pull Request Labels

NIAHCIA uses labels to keep development and generated release notes organized.

The main release-note configuration in `.github/release.yml` expects the following labels.

## Release categories

```text
breaking
consensus
node
core
miner
mining
compute
ai
protocol
security
documentation
docs
skip-changelog
duplicate
```

## Recommended additional workflow labels

```text
bug
enhancement
needs-reproduction
needs-design
blocked
good-first-issue
help-wanted
release
packaging
performance
networking
rpc
native-execution
randomx
compute-settlement
testing
ci
```

## Meaning

- `breaking` — user/operator compatibility break
- `consensus` — affects chain validity, fork choice, difficulty, block rules, or consensus-critical validation
- `node` / `core` — node/reference implementation
- `miner` / `mining` — CPU mining software or mining interfaces
- `compute` / `ai` — AI compute-worker software or AI job execution
- `protocol` — protocol/specification-visible change
- `security` — security hardening or vulnerability-related change
- `documentation` / `docs` — documentation only
- `skip-changelog` — intentionally omitted from generated release notes
- `duplicate` — duplicate issue/PR

## Important

Applying a label does not make a change safe.

Any change labeled `consensus`, `breaking`, `protocol`, or `security` deserves explicit review before merge and should be called out in release notes.
