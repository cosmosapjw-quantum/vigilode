# Reproduce the bounded audit

The measured baseline is `3064d453091af1dec7f99f0c8afabd98b447ea34`.
Use Rust 1.94.1 and the unchanged Cargo.lock. Native measurements used
`--profile measurement` (no LTO, 16 codegen units), one thread and CPU2.
The raw receipts retain exact commands, hashes and the original absolute paths.

## Numerical contracts

From the repository root:

```bash
cargo test --locked --profile measurement -p rodas5p-core --test lu_workspace_contracts -- --test-threads=1
cargo test --locked --profile measurement -p rodas5p-integrators --test rodas5p_fast_contracts --test partial_t_into_contracts --test fast_dense_contracts
cargo test --locked --profile measurement -p rodas5p-integrators --test work_counter_lane_contracts jvp_calls_equal_user_jvp_callbacks_on_the_matrix_free_lane
cargo test --locked --profile measurement -p vigilode-coupled --test economy_contracts --test economy_robustness
cargo test --locked --profile measurement -p vigilode --lib comparator_dominance_arithmetic_contract
cargo test --locked --profile measurement -p vigilode --test comparator_dominance_contracts --test evidence_switch_contracts --test return_sentinel_contracts
```

Use `--offline` too once the locked dependencies have been fetched. Existing
physical V6/V8 campaigns are not required for these scoped new contracts.

## Theory

`theory/THEORY_AUDIT.md` and `report/theory_section.tex` contain the proofs and
all premises. `theory/cayley_reuse.rs` is a standalone research primitive:

```bash
rustc --edition=2024 --test -O research/rigorous_performance_audit_20261027/theory/cayley_reuse.rs -o /tmp/vigilode-cayley-contracts
/tmp/vigilode-cayley-contracts
python3 research/rigorous_performance_audit_20261027/evidence/dopri_phase_exact.py
```

The shifted-LU experiment requires NumPy and SciPy. Its stored result deliberately
includes two residual-target failures. Reproduction must retain the same target
and report failures rather than modify the threshold.

## Performance comparisons

Build `lu_workspace_bench` in rodas5p-core, `mode_dense_trajectory_20261027` in
rodas5p-integrators, and `audit_adapter_20261027` in vigilode. The dense example
is source-compatible with the baseline. Copy that exact source into a detached
baseline worktree and build it with a **separate Cargo target directory** to
avoid the stale-artifact collision recorded in this audit. Preserve each binary
before building another arm. The audit used a shared dependency target, observed
one stale local-package artifact, and repaired only those local packages; all
receipts are retained.

The orchestration scripts locate baseline/candidate binaries in their enclosing
workspace's `tmp/bin_baseline` and `tmp/bin_candidate`. Adjust those paths for
another checkout. They refuse to overwrite completed result files. The original
first-pass adapter source is also retained under `evidence/adapter_validation`;
its restored bytes match the original build receipt hash. The final example
supports the original CLI and a new fixed-arm CLI.

Example invocations:

```bash
lu_workspace_bench legacy 32 100000
lu_workspace_bench workspace 32 100000
mode_dense_trajectory_20261027 65 20
audit_adapter_20261027 0.4 1 9999 1e-4 0 0 8 16
audit_adapter_20261027 0.4 1 9999 1e-4 0 0 8 16 baseline 1000
audit_adapter_20261027 0.4 1 9999 1e-4 0 0 8 16 optimized 1000
```

The benchmark source is part of the deliverable; bare executable names above
assume they have been put on PATH or replaced with their built paths. The
confirmation protocol was registered after the first noisy pass. No third pass
was performed. CPU times include process setup and diagnostic warmups. Profile
instructions are from Callgrind 3.22.0 in an emulated guest feature environment;
they are not native seconds or a universal host-independent speed ratio.

`evidence/raw_measurements.tar.xz` preserves the full timed outputs, Callgrind
files, protocols and commands. The uncompressed small summaries remain beside
it for review. Baseline and candidate binaries are identified by SHA-256;
the source and build instructions, rather than platform executables, are shipped.

## REVTeX report

Run `bash report/build_report.sh /absolute/output/directory` from this packet,
with a TeX installation containing REVTeX 4.2, AMS packages, natbib, textcase,
hyperref, booktabs, xurl and Latin Modern. The standalone delivery source bundle
includes the unmodified required REVTeX support files and its original source
archive; a normal repository checkout can use its system REVTeX installation.
Two pdfLaTeX passes resolve the references. `report/generate_results.py` rebuilds
tables/plots from saved results and does not rerun scientific experiments.

The delivered report was rendered and inspected across all 12 pages, with
no overfull boxes or unresolved references in the final log. The initial figure
title clipping was corrected before delivery.
