# Downloads

The canonical user-facing download path for NIAHCIA is:

https://github.com/niahcia/niahcia/releases/latest

The project download portal is:

https://niahcia.github.io/downloads.html

## Package selection

| Goal | Package |
|---|---|
| Full node | `niahcia-node-<platform>-<arch>` |
| CPU mining | `niahcia-miner-<platform>-<arch>` |
| AI compute | `niahcia-compute-<platform>-<arch>` |
| Standard operator stack | `niahcia-full-<platform>-<arch>` |

## Verification

Every packaged release must include `SHA256SUMS`.

Linux example:

```bash
sha256sum -c SHA256SUMS
```

Do not trust a binary whose published checksum cannot be verified.

## Source downloads

GitHub automatically provides source archives for every tag.

Those source archives are not the same thing as the packaged NIAHCIA runtime binaries.

Normal users should use the attached release packages.
