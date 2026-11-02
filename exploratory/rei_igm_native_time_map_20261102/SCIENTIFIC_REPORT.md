# REI F05 original source-clock native Rust H/He/OTS/thermal | Scientific checkpoint 2026-11-02

## Source authority

- VigilODE original PR #183 HEAD before this loop: `dee7812c2c4c40d23f6c0970f56bca1c51b8c89d`. Scope isolated to `rei_original46k_native_time_map.rs`, additive wrappers in `rei_original46k_native_nonphoto.rs`, read-only owner clock exporter, and tests.
- Frozen REI owner original HEAD `fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`; source archive `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`; original 26-event kernel `579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644`; accepted t160 state `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`.
- Original accepted tick 160/131072, N=46,080, 4 photon groups, 3 shared absorber moments, 5 material populations and log temperature. Original source clock JSON SHA256 `0bd888ce706ee110cea4ba23a1e871817cf1f054b85ebe5837934fd19db0492e`.
- Source clock times (s): full [779495799866.9248, 780922731328.641, 784367648616.093]; half1 [779495799866.9248, 780209265597.7828, 781931724241.5089]; half2 [781931724241.5089, 782645189972.367, 784367648616.093].
- At these three maps, original z=5.95 and H=2.253429258545391e-17 s^-1 are constant, but original J_g and external sinks E_g vary; group2 J relative drift is +3.663532164959703e-4. Frozen t160 J in the entire IVP would violate original source time history.
- The Python `export_original_clock.py` only reads immutable original source using `forcing_bounds(t,t)`, checks source bytes and kernel locks and exports 9 forcing sites. All step integration is native Rust, not Python.

## Model and method

The native point-shadow reuses the original stage0 Rust 26-event rate equations, with per-stage shared 4-group photon denominators:
`D_g = E_g(t) + Σ_s α_gs(t) Σ_i n_is`;
`P_is=n_is Σ_g J_g(t) α_gs(t)/D_g`.
It recomputes full global moments and `P_is` at every required stage, not just at tick160.

Original MPRK22(1) predictor/corrector solves `(I+dt L(F,den)) y = y_in` in independent positive 2x2 H and 3x3 He blocks; corrector uses average initial/predictor flux and predictor denominator. Temperature uses ideal gas `U=(3/2) k_B T (Σ_s n_s+n_e)` and Alexander SDIRK2 `gamma=1-1/sqrt(2)` with stage/final fixed-energy root and a bounded thermal outer iteration. Branch `v(T)` uses the original strict Hummer-Seaton table corners (incl. knot degeneracy); the exploratory point chooses `f=0.55` inside original `[0.1,1]` uncertainty. The original interval domain has **not** been ported to native Rust here.

## Evidence (actually executed)

- Rust unit tests **21/21 PASS** (includes recovered original source tests, mathematical H/He positivity/conservation, clock reorder negative check, thermal zero-duration and Hummer-Seaton source v knot checks).
- Stage0 native source against immutable owner E1 oracle: 25 source flux interval columns × 46,080 = **1,152,000** strict inclusions PASS; 7 energy columns × 46,080 = **322,560** strict inclusions PASS. 0 failures.
- Full, half1, half2 point-valued native 46,080-cell maps were run: thermal outer iterations 3,2,2; source table knot crossing cells 0/0/0; point-valued conservation/OTS/charge and global group source closures all pass.
- Native full vs two-half point coordinate max: `x_HII=1.71107600768403e-10`, `x_HeII=1.58174271858184e-10`, `x_HeIII=6.694550267485281e-13`, `log_T=2.8773428084605257e-11`.
- Native full point electron increment `-1.5598037718295086e60` electrons, point thermal U increment `-3.0484967705620344e49 erg`. Frozen first-order electron `-1.5599597593179075e60`, frozen first-order RHS thermal estimate `-3.0458315957000276e49 erg`; differences are NOT authority changes.
- Independent stdlib-only raw-state audit: PASS, nuclei H max relative `7.884587572182362e-16`, He `9.089204099304681e-16`. Ordinary f64 U_out-U_in cancellation requires explicit rounding allowance, independent of any original scientific acceptance gate.
- Separate original-owner read-only 26-event M-matrix interval replay: all `full`/`half1`/`half2` maps PASS; original 14 interval set ledgers **14/14 PASS**; original no-table-event PASS; original 4-channel correlated increment candidate PASS. Owner result schema/status `SCOPED_SOURCE_REAL_F05_M_MATRIX_RESIDUAL_GATE_CANDIDATE_PASS`.
- Independent cross-implementation strict point-versus-owner interval inclusion of original full and two-half increments: **368,640/368,640** (4 channels × 2 maps × 46,080 cells), no missing points. Owner CSV SHA256 `11df439390209bd48742ca9f4303c8743c9e14900289805e13d1ed489450a5dd`. This is strong consistency, NOT proof of original full uncertain operator in Rust.
- `REPRODUCE.sh` cold rerun with original archives + owner CSV: PASS, resulting raw output deterministic SHA256 `f7488ac29a51e28a2f12092947fe656eeefd4900ddd9ae0e5cf7c77e977a8f05`, 6,635,536 bytes.
- Hostile negative controls: corrupted clock, parent SHA and raw state SHA each correctly rejected (3/3). No native subagent execution; adversarial same-author analysis only.

## Authority and unresolved

Original V1 failed logT `2.1245050576368385e-4 > 2e-4`; keep V1 as failed evidence, do not overwrite with point accuracy. Original versioned V2 remains separate owner-side candidate; do not adopt tick161. HeII physically positive but f64 local source underflow remains UNRESOLVED; original source rank3 log magnitude preserved. Full uncertainty interval Rust IVP and its own 14 interval ledger enclosures NOT ESTABLISHED. Full-IVP apples-to-apples speedup NOT ESTABLISHED. Official tick161 NOT_ACCEPTED; production HOLD; canonical delta 0. Do not mutate original REI, accepted state/REIADP1, mainline, tolerance or scientific gate.

## Drift watchdog / DAG

Physical science result is **new actual native 46,080-cell 3-map physical state data**, cross-validated against original source intervals. Next task is rigorous native interval *enclosure* and correct positive HeII channel; no photon-only atlas/generic speedup diversion. No official DAG increments without source owner gates: VigilODE 7/23; VigilODE successor 0/12; REI central 28/88; REI working 33/88; REI fast 4/7.

Reproducer:
`bash exploratory/rei_igm_native_time_map_20261102/REPRODUCE.sh /path/to/isolated/output /path/to/original/REI_ORIGINAL26_COMMON_PARENT_INCREMENT_46080.csv.gz`.
Archive receipts/results and original source hashes must accompany publications.
