# Raw-data and publication policy

## External local state

The local Codex job must create a fresh execution directory outside the Git
worktree, normally:

```text
${XDG_STATE_HOME:-$HOME/.local/state}/vigilode/stage-certificate-telemetry/<UTC_RUN_ID>
```

It retains complete stdout/stderr, compiler output, temporary theorem builds,
Rust target data, exact commands, environment/version records, and failed
attempt history. Existing state directories are never overwritten or removed.

## Files allowed in Git

The follow-up local commit may contain only:

- feature-gated Rust source and focused tests;
- stdlib orchestration/verification source;
- Wolfram, SageMath, Singular, Lean/mathlib, and Rocq proof source;
- Markdown analysis and claim ledgers;
- compact JSON manifests, normalized summaries, and receipts;
- `SHA256SUMS` over checked-in evidence files.

The compact result files may record scalar or short fixed-size vectors needed
to audit the synthetic contract. They must not embed raw iteration histories,
full stdout/stderr, compiler transcripts, or binary artifacts.

## Files forbidden in Git

- raw logs or command transcripts;
- `target/`, `.lake/`, `_build/`, caches, package stores, or vendored tools;
- `.olean`, `.vo`, `.glob`, `.aux`, `.pyc`, object files, executables, cores;
- `.npy`, `.npz`, `.csv`, `.parquet`, HDF5, tarballs, ZIPs, or database files;
- historical candidate, real-client, holdout, one-shot, or solver output;
- copied toolchains or the attached research/coding harness archives.

## Size limits

- maximum single checked-in file in this research directory: 262,144 bytes;
- maximum total checked-in bytes in this research directory: 2,000,000 bytes;
- maximum `analysis_summary.json`: 131,072 bytes;
- maximum `formal_receipt.json`: 131,072 bytes.

Crossing a limit is a packaging failure. Do not compress raw data to evade it.
Summarize it and bind the external bytes by SHA-256 instead.
