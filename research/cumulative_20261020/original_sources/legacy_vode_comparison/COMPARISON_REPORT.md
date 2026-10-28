# VigilODE vs VODE/DVODE vs Diffsol (2026-10-19)

**Evidence legend**: `ARCHIVED_MEASURED` = original VigilODE 154-condition CSV published at GitHub; `PUBLISHED_A07` = remote Diffsol 0.16.1 paired benchmark validated receipts; `NEW_NUMERICALLY_CHECKED` = VODE BDF runs in this sandbox; `NOT_COMPARABLE` = cross-host timing or missing achieved-accuracy-matched comparator.

## Key result

A three-way **accuracy-matched speed ranking is UNRESOLVED**. However, the common Robertson 101-output comparison shows serious accuracy differences at the exact prescribed `rtol=1e-8, atol=1e-11`. On the observed maximum-over-grid WRMS scale:

| Solver, source | R (WRMS, plus empirical reference radius if available) | Gate R<=1 | Within-campaign timing |
|---|---:|---|---|
| VigilODE native fast (`PUBLISHED_A07`) | 0.337530 (A07 interval upper 0.338608) | PASS | A07 paired baseline |
| Diffsol 0.16.1 BDF (`PUBLISHED_A07`) | 3.302699 (A07 interval upper 3.303777) | FAIL | A07 within-host warm paired Diffsol/native = 1.4194, **diagnostic only** |
| SciPy 1.17.0 VODE BDF (`NEW_NUMERICALLY_CHECKED`) | 128.500351 + independent Radau/LSODA discrepancy 0.001978 = **128.502329** | FAIL | approximately 699.33 ms on a **different host**; no cross-host speed ratio |

VODE repeated runs yielded bit-identical output states; reported NST=113,428, RHS callbacks=264,617, Jacobian callbacks=32,157. VODE returned a success code, nevertheless the sampled error failed the original requested tolerance scale. These figures characterize **this SciPy VODE wrapper/configuration**, not an inherent VODE convergence theorem or a complete solver ranking. The absolute timing across the two hosts is not comparable. At input rtol=1e-9 and 1e-10, VODE's error **relative to the frozen 1e-8 requested target** remained 44.90 and 17.72, respectively, with far more work; these probes were observed but are not archived as full rows here.

A07 primary source: https://github.com/cosmosapjw-quantum/vigilode/blob/claude/nice-dijkstra-208xq0/research/production_audit_20261018/evidence/continuation_a07/REPORT.md

## Same five benchmark IVPs: endpoint requested-tolerance-normalized error

All five equations, initial states and integration spans were transcribed from pinned VigilODE Rust sources (see source links below). SciPy VODE used analytic Jacobians; for Brusselator it used *packed banded* Jacobian with `lband=uband=2`, and for the other problems full Jacobians. `first_step=1e-6`, `min_step=1e-14`, one-interval maximum step, up to 100,000 internal steps per `integrate()` call. Reference: fresh tight SciPy Radau (`rtol=2e-12`, `atol=1e-14×problem atol_scale`); no rigorous enclosure is claimed.

### `rtol=1e-05`, `atol=rtol × problem-specific scale`, endpoint
| Case | VigilODE fast WRMS+spread | VODE BDF WRMS (without spread) | VODE status/steps |
|---|---:|---:|---|
| Robertson (3) | 0.253 | 0.4833 | ok, NST=87,294 |
| HIRES (8) | 0.3127 | 1827 | ok, NST=60,039 |
| van der Pol μ=1000 (2) | 0.4507 | not evaluated | **solver_failure**, NST=100,000 |
| Brusselator (100) | 0.426 | 0.3464 | ok, NST=146 |
| Brusselator (400) | 0.4068 | 1.499 | ok, NST=139 |

### `rtol=1e-06`, `atol=rtol × problem-specific scale`, endpoint
| Case | VigilODE fast WRMS+spread | VODE BDF WRMS (without spread) | VODE status/steps |
|---|---:|---:|---|
| Robertson (3) | 0.5773 | 1.744 | ok, NST=94,077 |
| HIRES (8) | 0.1316 | 2409 | ok, NST=64,688 |
| van der Pol μ=1000 (2) | 0.3386 | not evaluated | **solver_failure**, NST=100,000 |
| Brusselator (100) | 0.8747 | 1.53 | ok, NST=196 |
| Brusselator (400) | 0.8651 | 4.016 | ok, NST=193 |


VODE also ran 100-variable Brusselator with a **dense** Jacobian. At `rtol=1e-5`, dense VODE took 9.126 ms and banded VODE 3.815 ms on the *same* sandbox host. Thus banded saved approximately 2.39× runtime in this VODE-only comparison. Do not divide either time by an archived Rust time from another host.

## The required distinction

- **VigilODE Run 1**: its original 154 rows completed; maximum endpoint WRMS including an empirical reference spread was 0.991780 (<1). This applies to *endpoints only* and is not a certification of all intermediate states. The original CSV supports timing estimates but those are from a previous shared VM, not this sandbox.
- **VODE new**: at `rtol=1e-5`, 2/5 selected paths had endpoint WRMS<1; HIRES had ≈1827, 400-variable Brusselator ≈1.499, and van der Pol hit NST=100000 before `tf`. The poor output accuracy and high NST deserve an independent native Fortran DVODE/alternative wrapper check before diagnosing an algorithmic defect.
- **Diffsol A07**: only Robertson has an exact, documented standalone A07 comparison against VigilODE. It used a fixed 101-point output grid, not the Run-1 endpoint-only cost. Diffsol passed process/execution integrity, but *failed accuracy gate*; the A07 warm paired time ratio alone cannot establish accuracy-matched speedup. It used **Diffsol 0.16.1**, not the newer 0.17.x branch.
- At equal input tolerance, error scales may vary across methods. **Same `rtol` is not same achieved error.** A fair performance claim would tune each solver to a *common achieved error*, with identical output schedule/initial data and a shared, admitted host.

## Exact provenance

- VigilODE Run 1 measured: `ccaeaa50beb4580505a36d25ccb919ec3ca38c7f`, original CSV: https://github.com/cosmosapjw-quantum/vigilode/blob/research/benchmark-results-20261019-dual/research/benchmark_results_20261019/run1/endpoint_results.csv
- VigilODE Run 2 measured Rust: `8a8bd409b1d96c7b7421f496d229e119fd3d284e`; original CT 2,700 rows are **not** a standalone cross-solver timing comparison: https://github.com/cosmosapjw-quantum/vigilode/blob/research/benchmark-results-20261019-dual/research/benchmark_results_20261019/run2/ct_results_2700.csv
- A07 recorded measured source `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`, time measurement protocol and raw retention in the GitHub A07 directory.
- Rust source equations: https://github.com/cosmosapjw-quantum/vigilode/blob/8a8bd409b1d96c7b7421f496d229e119fd3d284e/crates/rodas5p-cli/src/stiff_benchmark.rs and https://github.com/cosmosapjw-quantum/vigilode/blob/8a8bd409b1d96c7b7421f496d229e119fd3d284e/crates/rodas5p-integrators/src/problems.rs
- Official VODE source: https://www.netlib.org/ode/ ; SciPy wrapper: https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.ode.html
- Diffsol historical version: https://docs.rs/crate/diffsol/0.16.1

## Scripts and regeneration

From the directory containing this report:

```bash
python -m pip install numpy scipy
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python vode_targeted.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python a07_vode_exact.py
```

The scripts use independent reference SciPy Radau, the latter additionally SciPy LSODA. If any actual solver failure occurs, its status/return code and reached time are retained. Results: `vode_bdf_targeted.csv`, `reference_endpoints.json`, `a07_vode_exact.json`, `environment.json`, `vode_targeted_run.log`. Another host must generate **new** receipts and not overwrite historical raw benchmark data.
