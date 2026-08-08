# Finalization interruption note

Two pre-finalization attempts were interrupted by the execution wrapper:

1. `R10A_FINAL_PAIRED_RERUN_INTERRUPTED.log` stopped after 23 of 72 invocations.
   Its partial output directory was deleted and none of those partial results are used.
2. `R10A_FINAL_VERIFICATION_PRECOMMIT_INTERRUPTED.log` stopped during the release test build.
   It is not completion evidence.

The authoritative benchmark remains the complete guarded exact-head R10-A run recorded in
`R10A_PAIRED_RUN.log`, `R10A_PAIRED_BASELINE_CURRENT.csv`, and the external raw archive whose
SHA-256 is listed in `R10A_KERNEL_AUDIT_SUMMARY.json`.

The authoritative final source verification is `R10A_FINAL_VALIDATION_WORKTREE.log`, which contains
fresh successful fmt, clippy, full release test, and full release build output for the final working tree.
