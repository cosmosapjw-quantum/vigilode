# Reproducibility receipt

The canonical command was executed twice from the same source tree and environment after the output-budget schema clarification.

```text
first SHA256  a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e
second SHA256 a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e
cmp exit       0
```

The second 18 MiB JSON was removed after byte comparison to avoid duplicating the canonical raw
artifact. The stdlib-only analysis script was also rerun and every derived table hash remained
unchanged.
