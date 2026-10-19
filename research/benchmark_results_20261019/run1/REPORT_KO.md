# VigilODE first benchmark, 2026-10-19 (Korean report)

**Evidence status:** Original 154-row measurement CSV published unchanged. This report is a reconstruction from that CSV and the prior in-conversation benchmark description, not a byte copy of the original long report.

## Frozen setup

- Repository: cosmosapjw-quantum/vigilode.
- Measured source commit: `ccaeaa50beb4580505a36d25ccb919ec3ca38c7f` on `claude/nice-dijkstra-208xq0`.
- Rust 1.94.1 / LLVM 21.1.8; measurement optimization level 3, LTO off, executable CGU=1 after an original CGU=16 linker failure. Benchmarks used 1 thread and CPU affinity. Host: AMD EPYC 9V74 as exposed by a shared KVM VM.
- Endpoint cases: Robertson (n=3), HIRES (n=8), van der Pol mu=1000 (n=2), Brusselator-1D 50 (n=100), Brusselator-1D 200 (n=400).
- rtol = 1e-3,...,1e-9; atol = rtol*1e-4 for Robertson/HIRES and atol = rtol for the others; h0 = 1e-6; endpoint output.
- 154 tested (problem, arm, rtol) rows. The original run description states 1 warmup + 7 timing replicates per cell. Full timing raw attempts are **not** included; the CSV contains summary statistics.
- Accuracy reference was independently produced with tighter SciPy Radau and cross-checked against LSODA. Exact reference data and cross-check logs are not contained in this publication; the CSV records the corresponding numerical metrics.

## Machine-checkable results

All 154 CSV rows have `status=completed` and `deterministic=True`. The largest endpoint weighted RMS plus the saved empirical reference spread is **0.9917799399710141**.

For each problem and arm below, the selected row is the *fastest measured median among rows with* `empirical_error_plus_spread <= 1e-5`. Times are **ms per call**, not process times. The best specialized arm is stated, not implied to be fastest under every accuracy target or on every machine.

| Problem | Selected rtol | Base direct ms | Fast ms | Specialized arm | Specialized ms | Base/specialized |
|---|---:|---:|---:|---|---:|---:|
| Robertson | 1e-5 | 0.228635 | 0.023524 | fast-small | 0.018197 | 12.56x |
| HIRES | 1e-5 | 1.199540 | 0.189898 | fast-small-static | 0.140957 | 8.51x |
| Van der Pol (mu=1000) | 1e-6 | 3.228738 | 0.313410 | fast-small | 0.208466 | 15.49x |
| Brusselator n=100 | 1e-6 | 18.581235 | 2.744597 | fast-banded | 1.730129 | 10.74x |
| Brusselator n=400 | 1e-6 | 409.833040 | 25.659520 | fast-banded-slices | 7.164704 | 57.20x |

The CSV keeps source precision. The tabulated ratios are rounded descriptive values.

**Interpretation:** For this subset, the fast and small/banded paths substantially reduced measured per-call runtime at the stated endpoint error bound. Base and fast accepted/rejected step counts were equal in the 35 matched direct-vs-fast cells, consistent with per-stage implementation costs rather than fewer macro steps. Source-level hypotheses include workspace reuse, reduced diagnostics, LU implementation, and structured band exploitation. Their individual wall-time fractions were **not** separately profiled.

## Hard claim boundaries

- `endpoint_wrms_plus_empirical_spread_bound < 1` is an empirical *endpoint* criterion. It is not a continuous-time certificate or a guarantee for arbitrary dense output.
- Values `wall_median`, `wall_q25`, etc. originate from a shared VM, with no new hardware-admission certificate in this directory. Treat times as **diagnostic**.
- The independent reference and source `rust_*.json` raw files are not republished. Do not treat CSV `source_file` names as resolvable local paths.
- A prior separate dense-output probe found a large accuracy difference between `DenseErrorControl::Off` and `Enforce`. Its raw receipt is not included here, and no new endpoint CSV row certifies that scenario.
- This benchmark is **not** a cross-commit regression comparison. Run 2 contains distinct accuracy testing and earlier conversation summaries of old/new timing. Do not mix the two campaigns' host/timing bases.

## Local inspection

```bash
python3 research/benchmark_results_20261019/verify_csv.py
```

Primary row-level artifact: [endpoint_results.csv](endpoint_results.csv). The SHA-256 and measured commit are pinned in [../README.md](../README.md).
