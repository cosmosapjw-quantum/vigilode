# TDD GREEN and verification summary

The RED contracts in commit 1 were run against the superseded handoff before the repair. They failed on the expected defects: missing path-stable preflight, accepted wrong basename, accepted multiple sidecar records, and a load-bearing raw outer sidecar command.

The canonical package then passed:

```text
control-plane tests                         16/16 PASS
archive validator from handoff root         PASS
archive validator from repository root      PASS
archive validator from unrelated /tmp       PASS
canonical preflight from all three CWDs     PASS
wrong hash                                  REJECTED
withdrawn hash                              REJECTED
wrong archive basename                      REJECTED
multiple active sidecar records             REJECTED
missing archive                             REJECTED
completion schema/example                   PASS
Python compile                              PASS
shell syntax                                PASS
JSON/YAML parse                             PASS
package manifest                            PASS
deterministic tar repack                    byte-identical
production remote mutation                  0
merge / wall timing / Task 2                0 / 0 / 0
```

Load-bearing hashes:

```text
TDD RED log
856d0255c8fd83d6e4cb534dbde8b73f6bcd028ca67cdb121292ae0e71ad0240

TDD GREEN log
e91b67d89b32f57eaff0d23689526040fda12eee544a4b4248d369cbbd3d3623

raw sidecar-CWD reproducer
9746d4e2d71ed5a401401808e5e6c9aa3f1da32de6c377994fd93f9415836ae7

fresh verification log
25332185726849545691e1d06ba4798685c55da1687105d62fc9285cbfe06a99

canonical package
5ddb0f19be010d53187bac00d468c110c49a54b3cf168894207584bee04f1694
```
