**PROBE A2 report: scientific corpus v2.1 transcribed to Python**

This is an exploratory pilot. It supports future preregistered nodes and has no ledger authority. I only read the worktree `/home/user/wt-speed` at a49f7e4: I built no Rust, and git shows no change there except a `tools/__pycache__/` that dates from 2026-10-05/07, before this probe.

## Question
Can the scientific-corpus-v2.1 families be transcribed into Python faithfully enough for later probes to use them as the problem set? The source is `crates/rodas5p-integrators/src/scientific_corpus_v2.rs`. Faithful here means three things:
- the RHS, analytic JVP, J and f_t match the Rust code;
- the stored references are reproduced;
- the repository's error rule and the 18 n = 96 rows (12 of them out of tolerance per F-033) are reproduced end to end.

## Method
**Source check.** v2 is self-contained. `g4_s5b0_regime_atlas.rs` is not used: the v2 families differ from the atlas by the radical-inverse diversity multiplier and the 2-D semilinear grid. The `research/generic_*` folders hold only legacy-atlas results and no v2 reference states, so they were not used for comparison.

**Module: `corpus_v2.py`.** Importing it creates no files and runs no computation; references are read lazily.
- `build(family, n, grid=None, allow_holdout=False)` returns a `CorpusProblem`. Fields work as attributes and as dict keys, so the existing hyp/ctrl replicas that read `p['f']`, `p['J']`, `p['ft']`, `p['y0']`, `p['span']`, `p['ascale']`, `p['ref']` can use it directly.
- Problem fields:
  - `f`, `jvp` (analytic), `J` (dense), `J_sparse`, `ft` (analytic ∂f/∂t from the Rust `partial_t`);
  - `exact` (only rotating-nonnormal and semilinear have one), `y0`, `span`;
  - `ascale = 0.01` (atol = 0.01·rtol), `rtols = (1e-4, 1e-6, 1e-8)`;
  - `output_times` (101 points plus any breakpoints), `segments`, `autonomous`, `case_id(rtol)`, `campaign_settings(rtol)`.
- Dimensions: the contract n ∈ {96, 384, 1536}. Any other n follows the Rust padding path. Semilinear also accepts an explicit `grid=(nx, ny)`. Both are flagged `on_contract = False`.
- Error rule, as in `global_error.rs:306-380` and `output_accuracy.rs:44-78`:
  - `wrms_rows` uses weights 1e-10 + 1e-8·|ref|, the mean over components, and the max over the grid.
  - `global_error_metrics` reports endpoint, max-grid, rms-grid and conservative error, in the tight basis and in case units (case = tight × 1e-8 / rtol).
  - `reference_admissible` (uncertainty ≤ 0.1·error) and `assess_error_budget`.
- References and recorded rows:
  - `load_stored_reference` reads `selected_raw` for n = 96.
  - `refs/*.npz` hold my regenerated n = 384 and 1536 references.
  - `exact_reference()` gives the exact solution where one exists.
  - `radau_reference()` runs SciPy Radau on the output grid.
  - `rust_recorded_rows()` returns all 54 recorded campaign rows: errors in both bases plus every work counter.
- Other contents:
  - `operator_stats` (ρ, α, μ₂, Henrici departure, sector angles of the eigenvalues and of the field of values);
  - `JVP_COST_MODEL`, a hand count of flops per state component per JVP: Robertson 8.3, HIRES 6.0, vdP 5.5, rotating 14 plus 1 trig, forcing 8 plus 1 trig, semilinear 22;
  - `CAMPAIGN_CONFIG`;
  - `EXTENDED_RTOLS` from 1e-3 to 1e-10, which is outside the Rust contract.
- Holdouts (oregonator, pollution, medical-akzo, brusselator-2d) are transcribed as definitions only and need `allow_holdout=True`. Under `docs/HOLDOUT_HYGIENE.md` I never integrated them and never read their references; I only checked their JVP against finite differences. The only 2-D calibration family is semilinear.
- `python3 corpus_v2.py` runs the self-test (about 1.4 s) and prints `SELF-TEST PASS`.

**Validation arms:**
- (a) the repository's mpmath oracles, plus FD checks at n = 96, 384, 1536 and off-contract n;
- (b1) a Radau ladder at n = 96 compared with the stored references;
- (b2) the stored Rust rtol-1e-8 trajectories re-scored with my error rule;
- (b3) the E-03 SciPy arm A rerun on all 18 rows;
- (b4) regenerated n = 384/1536 references, whose uncertainty is compared with the recorded campaign values;
- (b5) a dense-LU RODAS5P replica of the campaign dense arm (`replica_check.py`) compared with the recorded Rust counters and errors;
- (c) operator diagnostics and the row map;
- (d) the linearized error propagator along the reference.

## Results

**(a) Definitions**

| check | result |
|---|---|
| mpmath calibration oracle, worst scaled error (repo threshold 1e-13) | Robertson 7.8e-14, HIRES 3.7e-15, vdP 2.1e-14, rotating 1.5e-14, forcing 2.0e-14 |
| semilinear oracle: φ, f(φ), f(y), Jv, f_t (threshold 5e-14) | 4.2e-16 / 1.4e-15 / 8.3e-15 at n = 96 / 384 / 1536; φ is bit-identical at the sampled points; half-bandwidth = nx |
| JVP vs central FD | 6e-12 to 2.2e-10 relative, all families and n |
| dense J·v vs JVP (n ≤ 600) | ≤ 2.5e-16 |
| sparse J vs dense J | exactly equal |
| f_t vs FD | ≤ 1.5e-7 |
| manufactured residual f(t, φ) − φ' | ≤ 8.9e-11, limited by the FD |

**(b1) References at n = 96**

My Radau L2 (1e-12/1e-14) against the stored reference, as max-grid WRMS in the tight basis:

| family | mine vs stored | stored uncertainty | nfev/nlu (mine vs stored) |
|---|---|---|---|
| Robertson | 7.19e-8 | 2.56e-5 | 5580/88, identical |
| HIRES | 1.55e-7 | 4.56e-5 | 12611/48, identical |
| vdP | 2.66e-7 | 5.72e-5 | 20386/108, identical |
| rotating | 1.15e-6 | 2.68e-5 | 11986/66, identical |
| forcing | 2.07e-5 | 4.12e-5 | 88987/242 vs 88974/238 |
| semilinear | 1.52e-6 | 3.83e-3 | 2272/8, identical |

- The regenerated D1 matches the stored D1 to 4 digits: 1.579e-3, 1.798e-3, 3.858e-3, 1.088e-3, 1.680e-3, and 1.449e-3 vs 1.451e-3.
- Stored reference vs exact solution: rotating 1.56e-5, semilinear 1.53e-4. The semilinear stored uncertainty of 3.8e-3 is therefore about 25 times too conservative.
- The values are not bitwise equal (1–2 of 101 points); the vectorized RHS rounds differently.

**(b2) Stored Rust trajectories re-scored.** All 12 arms of the 6 rtol-1e-8 records were re-scored with my error rule. The relative difference from the recorded max_grid_wrms is ≤ 3.9e-16. Against my own Radau reference the dense values are the same, for example forcing 2.281292 vs 2.281304 and semilinear 207.482.

**(b3) E-03 arm A, 18/18 rows.** The case-unit errors are identical to 4 digits. Step counts and nfev are identical in 17 of 18 rows; forcing at 1e-8 differs by 1252 vs 1251 steps and 8952 vs 8948 nfev.

**(b4) References at n = 384 and 1536.** The `tools/reference_v2` ladder with LSODA reproduces the recorded campaign uncertainties.
- Ratio of my uncertainty to the recorded one: 0.9993 to 1.0002 for every family except vdP-1536 (1.0035) and forcing-1536 (0.9855).
- The 18 `.npz` files are in `refs/`. Wall time is ≤ 11 s per family.

**(b5) Dense-arm replica vs the recorded Rust counters**

The replica solves stages exactly (dense LU) and uses the integral controller 0.9·err^(-1/5), clamped to [0.2, 5] after an accepted step and [0.2, 0.9] after a rejected one, with h0 = span/100, max step = span, and the H interpolant.

| n | attempts equal | rejections equal | max-grid error within 1% / 10% | exceed set (error > 1) agrees |
|---|---|---|---|---|
| 96 | 15/18 | 16/18 | 13/18 / 16/18 | 18/18 (Rust 12, replica 12) |
| 384 | 14/18 | 14/18 | 12/18 / 14/18 | 18/18 |
| 1536 | 14/18 | 14/18 | 12/18 / 14/18 | 18/18 |

- At rtol ≤ 1e-6, attempts agree in 11/12 rows at n = 96 and 12/12 at n = 384 and 1536. The largest error deviation there is 4% at n = 96 and 10.8% at n = 384/1536.
- The deviations at rtol 1e-4 come from the inexact Rust stage solves. Semilinear 1e-4 at n = 96 gives replica 21.37 vs Rust 17.02, which matches the v3 addendum's tight-inner arm (17.0 / 0.80 = 21.3). The worst case is Robertson 1e-4: Rust 0.272, replica 0.467, both below 1.

**(c) Operator diagnostics, n = 96**

Values are evaluated along the stored reference: at y0 / at mid-span / max over the 101 grid points. "Sector" is the largest |arg(−λ)| over stable eigenvalues; the field-of-values (FOV) angle exists only where μ₂ < 0.

| family | ρ(J) | μ₂ | α (max) | Henrici, relative | sector, eigen / FOV at mid | notes |
|---|---|---|---|---|---|---|
| robertson-ramped | 0.0438 / 2093 / 2392 at t = 0.087 | 0.009 / 427 / 483 | about 0 (one conserved quantity per block) | 0.71 / 0.70 | 0° / none | stiffness ratio at mid 6.5e3; G₂ = 1.41 |
| hires-ramped | 11.3 / 17.8 / 63.0 at t = 1 | 1.40 / 4.82 / 21.6 | about 0 (conserved y7 + y8) | 0.66 / 0.70 | 3.8° max / none | stiffness ratio 1.05e3; G₂ = 2.24 |
| van-der-pol-ramped | 32.8 / 814 / 1595 at t = 0.88 | 0 / 0.0028 / 0.067 | 0.062 (48 slightly unstable eigenvalues at mid) | 0.067 / 0.001 | 0° / none | G₂ = 1.02 |
| rotating-nonnormal | 21.9 / 285 / 590 | −6.25 / −61 / −6.25 | −6.32 | 0.09 / 0.43 (max 0.65) | 0° / 25.3° (4.8° at y0) | dissipative in the 2-norm; G₂ = 0.94 |
| nonautonomous-stiff-forcing | 32.9 / 452 / 549 | −27 / −373 / −27 | −27 | 0 (diagonal) | 0° / 0° | normal, real spectrum; G₂ = 0.76 |
| semilinear-adv-diff (8×12) | 15.7 / 52.7 / 93.6 | 1.66 / 22.8 / 37.5 at t = 0.62 | −5.06 (stable at every frozen t) | 0.55 / 0.67 | 2.0° (max 4.2°) / none | **G₂ = 210 over [0.44, 0.73]**, G_w = 596 |

- All six calibration families are non-autonomous.
- At n = 384 and 1536 the block families barely change: ρ_mid is 2102/2104, 18.6/18.8, 818/819, 286/287 and 454/454.
- Semilinear scales with the grid: ρ0 = 37.0 and 94.8, ρ_mid = 126 and 311, ρ_end = 219 and 540; μ₂ at mid is 24.8 and 26.3.
- G₂ = max over i < j of ‖Φ(t_j, t_i)‖₂ for the linearized propagator. The exponential-midpoint product agrees with a variational LSODA solve to 5 digits (210.27, 2.2363, 1.4124).
- G_w is the same quantity in case weights. Its values of 10 and 12 for rotating and forcing come from the weights changing near zero crossings, not from the dynamics.

**Mapping to the 18 recorded n = 96 rows** (Rust dense max-grid error in case units; the 12 that exceed tolerance are bold)

| family | rtol 1e-4 | rtol 1e-6 | rtol 1e-8 | SciPy Radau at rtol 1e-4 / 1e-6 / 1e-8 |
|---|---|---|---|---|
| robertson-ramped | 0.272 | **3.70** | **2.28** | 0.166 / 0.825 / 0.407 |
| hires-ramped | 0.392 | 0.918 | 0.409 | 0.114 / 0.190 / 0.177 |
| van-der-pol-ramped | 0.797 | **2.55** | 0.510 | 0.291 / 0.415 / 0.566 |
| rotating-nonnormal | **1.39** | **3.65** | **1.62** | 0.294 / 0.283 / 0.141 |
| nonautonomous-stiff-forcing | **4.22** | **2.33** | **2.28** | 0.848 / 0.367 / 0.261 |
| semilinear (F-033) | **17.0** | **53.2** | **207** | 0.241 / 1.57 / 0.473 |

- Dense-arm work at n = 96, as attempts / JVPs / orthogonalization dot products, ranges from 17 / 4565 / 1.34e5 (HIRES, rtol 1e-4) to 617 / 176462 / 5.21e6 (forcing, rtol 1e-8). All 54 rows are in `rust_recorded_rows()` and `results.json["rust_rows"]`.
- The compact file shows the same 12-of-18 exceedance pattern at n = 384 and 1536. Semilinear is 22.8 / 71.0 / 260.8 at n = 384 and 38.3 / 92.8 / 169.1 at n = 1536.
- **Side finding for the F-033 probes.** The v3 addendum says the amplification factor "was not measured". It is now measured: G₂ = 210 and G_w = 596 on the reference. Accepted local errors of at most 1.36 units with sums of 1.37–6.83 units can therefore propagate into global errors of 17–207. Only semilinear has G above 2.3; it is the only transiently expansive family (μ₂ > 0 while α < 0, Henrici 0.67). The other F-033 rows are within 1.4–4.2 units.

## Mandatory discipline
Items (1)–(5) apply to method comparisons; this probe compares no methods, so they do not apply. Item (6), fidelity against recorded Rust numbers, is covered by (b1)–(b5). The work counters and the JVP cost model are exposed so later probes can apply item (2).

## Decision: PROMOTE (as shared probe tooling; this is not a method claim)
Recommended use:
1. Use `corpus_v2.build(family, n)` for all corpus-v2 probes, with case tolerances atol = 0.01·rtol.
2. Score errors with `p.error_metrics(Y, rtol)`: max-grid WRMS in case units over the 101-point grid.
3. Choose references this way:
   - for rotating-nonnormal and semilinear, use `prefer_exact=True` (zero uncertainty), which is required for tight-rtol cells;
   - otherwise use the stored references at n = 96 and `refs/` at n = 384/1536.
4. At rtol ≤ 1e-9, check `reference_admissible`. The stored uncertainty in case units is unc × 1e-8 / rtol: 2.6e-3 to 5.7e-3 at 1e-10 for the non-semilinear families, and 0.38 for semilinear.
5. The strongest cheap rival is SciPy Radau at case tolerance, reproduced here on 18/18 rows. Rust counters come from `rust_recorded_rows()`.
6. Use G₂ from `validate_corpus.py d` as the expansivity indicator. Semilinear is the only family with G ≫ 1.

## Threats to validity
- **Not bitwise.** Vectorized rounding and NumPy's sin/tanh against libm mean results are equal only within uncertainty. SciPy is 1.17.1 here vs 1.17.0 in the references.
- **Oracle coverage.** The mpmath oracle covers t = 0.37 and sampled indices only. f_t is checked only against FD, and the 5 non-semilinear families have no f_t oracle.
- **Holdouts.** They are deliberately not validated against references, and their f_t is 0 on each segment, as in the Rust code.
- **Off-contract n.** Other dimensions and grids are FD-checked only. The Rust rotating padding for odd n uses `div(i)` rather than `div(block)`; I copied that quirk.
- **Regenerated references.** The n = 384/1536 references are my own. Their only cross-check is the recorded uncertainty (within 1.5%), not their states.
- **Replica.** It uses exact stage solves, so the rtol-1e-4 rows differ from Rust by up to 2.3 times in error (still on the same side of 1). Only the dense arm is replicated; the clipping rule of the clipped arm is not.
- **Amplification measure.** G is measured along the reference trajectory only. G_w mixes the change of weights with the dynamics.
- **Cost model.** `JVP_COST_MODEL` is a hand count, not a measurement.

## Files
All in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/corpus/`:
- `corpus_v2.py`: the module, with the self-test
- `validate_corpus.py`: runs sections a, b, c, c2 and d
- `gen_refs.py`: regenerates the n = 384/1536 references
- `replica_check.py`: the dense-arm replica
- `amp_check.py`: checks the propagator estimate
- `results.json`: all numbers
- `replica_dense_arm_n{96,384,1536}.json`
- `unc_compare.json`
- `refs/`: 12 `.npz` references
- `logs/`