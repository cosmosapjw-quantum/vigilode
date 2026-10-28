# E1-bench corpus v1: preregistration

Node `e1-bench` (wave 2, task 5; plan §7). Written and pushed before the measured run.
Ledger row `L-0139` is pre-assigned; its verdict is fixed in advance as **INCONCLUSIVE**: the run is on a
shared 2-core host with other builds running, so it exists for provenance and for the harness, not to
decide a speed question.

## What is measured

One run of `vigilode-bench wp` over `corpus_v1.json` (sha256
`b6ed01af1591130b94900964936ff51b9172a6ccb96da8fbcc2724d5b603ef4e`), harness code at commit
`7665296` (crate `crates/vigilode-bench`, `Cargo.lock` sha256
`d52dbdd01959c4f481a2f42657525a906603b9bc895a6f51e6995ff472873abf`).

- Problems: Robertson (n = 3, t in [0, 40]), HIRES (n = 8, [0, 321.8122]), van der Pol mu = 1000
  (unscaled, n = 2, [0, 2000]), Brusselator 1D (n = 100, [0, 10], band 2/2). One Rust RHS and Jacobian per
  problem, shared by every arm.
- Output grid: 101 uniform points on the span, obtained through each solver's own output interface.
- Tolerances: rtol in {1e-4, 1e-6, 1e-8}; atol vector = rtol x `atol_scale` (1e-4 for Robertson and HIRES,
  1 for van der Pol and Brusselator), the same vector in every arm. The protected drivers take a scalar
  atol, so the scale vectors are uniform.
- Arms (13): `vigilode`, `vigilode-banded` (solver under test), `diffsol-bdf-dense`, `diffsol-bdf-sparse`,
  `cvode-bdf-dense`, `cvode-bdf-band`, `dvode-mf21`, `dvode-mf24`, `radau5`, `radau5-band`, `rodas`,
  `rodas-band` (production), `internal-radau-iia3` (reference-implementation-only, never in a speed
  table). Band and sparse arms run on the Brusselator only.
- Repetitions: 2 discarded warm-ups, then 21 measured repetitions per arm and tolerance, in blocks with a
  rotated arm order, fresh solver per repetition, in one process. A development smoke run with one
  repetition took about 2 s in total, so the full run is far below the 40-minute budget and repetitions are
  not reduced.
- Environment: `OPENBLAS_NUM_THREADS=1` (the system `libblas.so.3` resolves to OpenBLAS; DVODE/LINPACK
  call level-1 BLAS). Binary built with `cargo build -j 1 --locked --profile measurement`
  (`opt-level = 3`, `codegen-units = 1`, no LTO); Fortran `-O3 -std=legacy`, C `-O3`.

## Reference

VigilODE (dense route) at rtol 1e-12 with `reference_atol` (1e-16 for Robertson and HIRES, 1e-12
otherwise), attempt budget 2,000,000. Decisive cross-check: Diffsol BDF dense at the tightest successful
rtol of 1e-12, 1e-11, 1e-10, 1e-9. Disagreement is measured with the `error_common` metric below; above
1e-8 the problem is `reference-disputed` and all its matched entries are `DIAGNOSTIC_ONLY`. RADAU5 and
CVODE at rtol 1e-12 are recorded as informational cross-checks only.

## Metrics and decision rules

- `error_own` (plan §7 rule 2): max over the 101 grid points of the WRMS of `y - y_ref` with weights
  `atol_i + rtol |y_ref_i|` of the run itself.
- `error_common`: the same with weights `error_rtol (atol_scale_i + |y_ref_i|)`, `error_rtol = 1`; it
  does not depend on the run's tolerance and is the work–precision axis.
- Work: solver-reported counters (accepted/rejected steps, factorizations, ...) and the shared model's
  RHS/Jacobian/JVP callback counts; allocations: every heap allocation call of the run (C allocator
  interposer, so Fortran and SUNDIALS allocations count) and its Rust subset. Window: setup, integration,
  grid states in final form, solver teardown; statistics formatting excluded.
- Wall time: median over the 21 repetitions, `timing_authority = "diagnostic_only"`.
- Matched error: common levels `error_common` in {1e-2, 3e-3, ..., 1e-9} (half decades). At each level the
  comparator's and the VigilODE arm's (banded comparators: `vigilode-banded`) median time and RHS count are
  interpolated linearly in log–log on their own successful points. A ratio is reported only where both
  arms bracket the level, the reference is undisputed and both arms are deterministic across repetitions;
  every other entry is `DIAGNOSTIC_ONLY`. No ratio is promoted to a speed claim.
- Failed arms are rows with their status and work, never dropped.

## Known limitations, fixed before the run

1. The facade at a5a48f7 refuses `dense_output` and a vector atol and returns only t0/t1. The VigilODE arms
   therefore call the protected drivers the facade routes to (`integrate_rodas5p_routed_observed`,
   `integrate_rodas5p_fast_banded_observed`) with the facade's defaults and a 101-point `OutputSchedule`;
   a test shows the endpoint is bit-identical to `Rodas5p::solve`. The schedule lands a step on every grid
   time, which adds clipped steps (and accuracy) that an endpoint-only run would not have; the matched-error
   comparison charges that cost to VigilODE.
2. The facade's dense Jacobian callback returns a new `DenseMatrix` per evaluation (n^2 allocation); this is
   the production route and is measured as such.
3. Diffsol's builder API takes a JVP: the shared Jacobian enters Diffsol as J v, and Diffsol assembles its
   matrix from n (dense) or coloured (sparse) products, its defaults.
4. CVODE runs in one-step mode with a total cap of 100,000 steps; DVODE uses its defaults (including
   MXSTEP = 500 per output interval); the Hairer codes use their defaults with H = 0. Assimulo's RADAU5
   passes an extra `WERR` argument to SOLOUT; the bridge follows that signature. RODAS's duplicate DECSOL
   symbols are localized in its object file (no source change).
5. In-process repetitions on a loaded shared host: timing is diagnostic only.

## Development runs before this preregistration (disclosed)

Two smoke runs with one repetition were made to check the harness end to end (outputs not committed).
After the first, three design choices were changed before freezing the corpus: (a) atol was fixed per
problem and is now rtol x scale, because with a fixed atol the small HIRES components were atol-limited and
the work–precision curves saturated; (b) matched levels were VigilODE's own achieved errors and are now
common half-decade levels at which both curves must bracket; (c) the measured window now ends before
statistics formatting. The first smoke run also showed the van der Pol cross-check above 1e-8 under the old
metric; under the frozen metric the second smoke run did not. That change in the dispute outcome is a
consequence of (a), stated here so that it is not read as a tuned result. No tolerance, level, threshold or
arm list is changed after this file is committed.

## Outputs

`results_v1.json` (raw, generated) and `REPORT.md` (generated by `vigilode-bench report` from the JSON only),
appended below this preregistration in a results section after the run.

## Results (appended after the run)

Run at commit `1e3ca6db4f54cb0d1858ab8cc4b35c9f1208d016` (crate tree clean: True), `OPENBLAS_NUM_THREADS=1`, 37.0 s, 2 logical CPUs (Intel(R) Xeon(R) Processor @ 2.10GHz), allocation counters verified live: True. Every number below is copied from `results_v1.json`; the full tables are `REPORT.md` (generated). Timing authority: diagnostic only.

References: all four undisputed. Decisive Diffsol cross-check at rtol 1e-12 (first attempt succeeded everywhere):
- robertson: 1.71e-11 (informational: radau5 1.16e-10, cvode-bdf-dense 1.73e-11)
- hires: 4.16e-11 (informational: radau5 2.10e-09, cvode-bdf-dense 2.94e-11)
- van-der-pol-mu1000: 1.01e-09 (informational: radau5 2.16e-10, cvode-bdf-dense 1.46e-09)
- brusselator-1d-n100: 3.39e-10 (informational: radau5 1.30e-10, cvode-bdf-dense 5.36e-10)

Rows: 102 (none dropped); failures: van-der-pol-mu1000 dvode-mf21 rtol 1e-08: DVODE ISTATE -1.
All rows deterministic across 21 repetitions: True. Allocation counts identical across repetitions except internal-radau-iia3 (brusselator-1d-n100) (reference implementation only).

Matched-error entries: 375, of which 80 MATCHED and 295 DIAGNOSTIC_ONLY (levels not bracketed by both arms). Time ratio = comparator / VigilODE median wall time at matched `error_common` (above 1: VigilODE faster); range and geometric mean over the matched levels of each pair:

| problem | comparator | baseline | matched levels | time ratio min | time ratio max | time ratio geo. mean | RHS ratio geo. mean |
| --- | --- | --- | --- | --- | --- | --- | --- |
| robertson | diffsol-bdf-dense | vigilode | 2 | 2.12 | 2.28 | 2.20 | 0.46 |
| robertson | cvode-bdf-dense | vigilode | 1 | 4.79 | 4.79 | 4.79 | 0.53 |
| robertson | dvode-mf21 | vigilode | 1 | 1.50 | 1.50 | 1.50 | 0.53 |
| robertson | radau5 | vigilode | 1 | 0.57 | 0.57 | 0.57 | 0.57 |
| robertson | rodas | vigilode | 3 | 0.72 | 1.12 | 0.90 | 0.97 |
| hires | diffsol-bdf-dense | vigilode | 5 | 1.58 | 2.05 | 1.84 | 0.63 |
| hires | cvode-bdf-dense | vigilode | 4 | 5.62 | 6.08 | 5.87 | 0.68 |
| hires | dvode-mf21 | vigilode | 4 | 1.36 | 1.52 | 1.44 | 0.69 |
| hires | radau5 | vigilode | 2 | 0.81 | 1.01 | 0.91 | 0.89 |
| hires | rodas | vigilode | 7 | 0.83 | 2.80 | 1.61 | 1.53 |
| van-der-pol-mu1000 | diffsol-bdf-dense | vigilode | 2 | 3.99 | 4.12 | 4.06 | 0.97 |
| van-der-pol-mu1000 | cvode-bdf-dense | vigilode | 1 | 7.59 | 7.59 | 7.59 | 0.93 |
| van-der-pol-mu1000 | radau5 | vigilode | 4 | 0.64 | 0.70 | 0.67 | 1.01 |
| van-der-pol-mu1000 | rodas | vigilode | 5 | 0.89 | 1.38 | 1.14 | 1.64 |
| brusselator-1d-n100 | diffsol-bdf-dense | vigilode | 3 | 1.43 | 1.44 | 1.43 | 0.44 |
| brusselator-1d-n100 | diffsol-bdf-sparse | vigilode-banded | 3 | 0.71 | 0.78 | 0.75 | 0.44 |
| brusselator-1d-n100 | cvode-bdf-dense | vigilode | 3 | 2.71 | 3.78 | 3.21 | 0.33 |
| brusselator-1d-n100 | cvode-bdf-band | vigilode-banded | 3 | 1.59 | 2.24 | 1.89 | 0.33 |
| brusselator-1d-n100 | dvode-mf21 | vigilode | 3 | 0.41 | 0.59 | 0.49 | 0.33 |
| brusselator-1d-n100 | dvode-mf24 | vigilode-banded | 3 | 0.40 | 0.55 | 0.47 | 0.33 |
| brusselator-1d-n100 | radau5 | vigilode | 5 | 0.91 | 1.35 | 1.14 | 0.53 |
| brusselator-1d-n100 | radau5-band | vigilode-banded | 5 | 0.48 | 0.69 | 0.58 | 0.53 |
| brusselator-1d-n100 | rodas | vigilode | 5 | 0.77 | 1.17 | 0.98 | 0.75 |
| brusselator-1d-n100 | rodas-band | vigilode-banded | 5 | 0.59 | 0.89 | 0.75 | 0.75 |

Pairs with no matched level: dvode-mf21 on van-der-pol-mu1000.

Reading, within the preregistered limits. RADAU5 is at or below VigilODE's wall time at every matched level
on the three small dense problems (0.57–1.01); RODAS (dense) is mixed (0.72–2.80), below 1 at the loosest
matched levels and above 1 at the tighter ones; the banded Hairer arms are below `vigilode-banded` on the
Brusselator (0.48–0.89). DVODE is faster on the Brusselator (0.40–0.59) and slower on Robertson and HIRES
(1.36–1.52). Diffsol BDF dense (1.43–4.12) and CVODE dense and band (1.59–7.59) are slower than VigilODE at
every matched level; Diffsol's sparse arm is faster on the Brusselator (0.71–0.78). These are single-host,
in-process, loaded-machine numbers: `INCONCLUSIVE` as preregistered. VigilODE's grid landings
(limitation 1) and per-evaluation Jacobian allocation (limitation 2) are part of what is measured. Ledger
row: `L-0139`.
