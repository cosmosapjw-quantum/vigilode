---
doc_id: VIGILODE-OPT-PLAN-20261104
kind: machine-readable optimization plan for local implementation
base_commit: 4339facc            # repository state that every code reference below was checked against
repo_root_assumed: vigilode/     # paths below are relative to the repository root
inputs:
  S1: "attachment fdb57d30 — 'VigilODE Candidate Propositions: Proofs, Counterexamples, and Numerical Experiments' (P1–P18)"
  S2: "attachment 965219fa — 'Evolutionary Theoretical Candidates for Next-Generation VigilODE' (EVO-01–EVO-16)"
  S3: "attachment e283b8a5 — 'VigilODE Theory-Evolution Run: Candidate Propositions' (P1–P20, KC*, E1–E7)"
  T:  "VigilODE_THEORY_MACHINE_20261103.md / VigilODE_THEORY_AUDIT_20261103.md (anchor ids RK-*, RB-*, QA-*, SW-*, CE-*)"
verification_scripts:
  - opt/opt_verify.py   # VER-01, VER-02, VER-04..VER-09
  - opt/opt_verify2.py  # VER-02b, VER-08b, VER-10
  - opt/opt_verify3.py  # VER-03 (controller f-eval comparison)
verification_summary: "13 checks run in Python; 11 passed on first expectation, 2 failed on a wrong first expectation and were reinterpreted (recorded in VER-02b, VER-06b). No Rust code was executed or modified."
status_vocabulary:
  ADOPT-A: implement now; cheap, local, verified, default-off until gate G5
  ADOPT-B: implement after ADOPT-A; verified mechanism, larger scope
  ADOPT-C: implement only after WP-00 data shows the targeted cost
  RESEARCH: not solver-ready; keep in backlog with the stated missing evidence
  REJECT: not adopted; reason given
  IN-CODE: already present at base_commit; work item reduces to audit/wiring
hard_constraints:
  - never run `git push`, `git reset --hard`, or `agent_gate.py approve`
  - never change a published tableau (DOPRI5, RODAS5P, exprb*) or its constants
  - every new behaviour ships behind an opt-in config field whose default reproduces base_commit bit-for-bit (gate G1)
  - never upgrade a result to `Certified` unless every error component has an outward bound (T: CE-RULE-01)
  - acceptance of any linear/nonlinear solve stays on the true (original-space) residual (T: LA-RULE-01)
---

# 0. How to use this document

Read §1 (adjudication) only when a work package cites an `ADJ-*` id. Implement work packages (§3) in the order of §4. Each work package lists: `files`, `preconditions`, `math`, `algorithm`, `config`, `tests`, `gates`, `non_goals`. Before writing code for a package, run its `locate` commands; do not invent function names that the commands do not return.

Notation: `u = 2^-52` (binary64 unit roundoff ×2), `k` = estimator exponent order used by the controller (`RODAS5P_ESTIMATOR_ORDER = 5`; DOPRI5 also uses 5), `R(z) = 1 + z + z²/2 + z³/6 + z⁴/24 + z⁵/120 + z⁶/600` (DOPRI5 stability function, T: RK-FRM-01), `γ = 0.21193756319429014` (RODAS5P).

---

# 1. Adjudication ledger (claims in S1, S2, S3)

Format: `ADJ-id | source claim | source verdict | this plan's verdict | reason / evidence | work package`.

## 1.1 Control plane

| id | source claim | source verdict | verdict | reason / evidence | WP |
|---|---|---|---|---|---|
| ADJ-01 | S3-P13, S2-EVO-14: transition table not total (finding F4) | PROVED | ADOPT-A | finite 7×14 check; repair is mechanical | WP-01 |
| ADJ-02 | S2-EVO-09(2): FNV-1a epochs (F3) | PROVED | ADOPT-A (modified) | `rodas5p-core/src/hash.rs` already provides SHA-256; use a truncated SHA-256 with a fixed byte layout instead of adding FNV-1a code | WP-02 |
| ADJ-03 | S3-P9, S2-EVO-15: replace Wilson by KL-inverted bound | PROVED | ADOPT-A | VER-04: KL worst miscoverage 0.0492 ≤ δ = 0.05; Wilson(prior 1,1) and Beta 0.95-quantile are not frequentist bounds (worst miss 1.0 at extreme p, small n) | WP-03 |
| ADJ-04 | S1-P14: "throttle bound does not use U, so any U is safe" | PROVED | ACCEPT with correction | `AdmitPosterior` admissions are **not** throttled; the safety bound comes from the overhead guard `net_overhead + C_a + C_v ≤ protected_base / overhead_factor`, which also does not use U. Conclusion (U can be swapped safely) stands | WP-03 |
| ADJ-05 | S1-P14: exact Beta quantile / Thompson is "the correct" choice | PROVED | PARTIAL | a Bayesian quantile under a uniform prior does not give frequentist coverage (VER-04). Offer `BetaQuantile` and `Thompson` as options, default candidate is `KlUcb` | WP-03 |
| ADJ-06 | S2-EVO-15: discount counts with γ_forget = 0.98 for regime drift | DES | ADOPT-B (option) | sound for drift; breaks the integer-count coverage statement; label EST when discount < 1 | WP-03 |
| ADJ-07 | S3-P10: memoryless E^{-1/5} controller rings; PI with Jury-certified gains fixes it (E3: 4× attenuation) | PROVED | PARTIAL | baseline description is wrong: `adaptive.rs` already has `ControllerKind::{Integral, Pi, Predictive, PredictiveCapped, PredictiveCapped2}`; default `Integral`. VER-02: disturbance-to-log E gain at Nyquist is 2.0 (Integral), 2.22 (existing Pi), 1.25 (S3 gains), 1.0 (H211b, H211PI). VER-03: no controller wins f-evals across problems; H211b −16 % on oscillator at rtol 1e-4, Integral cheapest on Van der Pol and non-stiff | WP-05 |
| ADJ-08 | S3-P11: horizon shortest-path route scheduling | PROVED | RESEARCH | needs a predictive per-route work model (finding F10); single-step rule stays | — |
| ADJ-09 | S3-P19: context-tree regime predictor | PROVED | RESEARCH | mechanism check only (KC14) | — |
| ADJ-10 | S3-P20: waterfilling error budget | PARTIAL | RESEARCH | gain only under order reduction (S3 KC15/E5); needs an order-reduction detector first | — |
| ADJ-11 | S2-EVO-16: MDL pruning cuts benign cost 34.1 % | PROVED | REJECT as evidence | number comes from a synthetic cost simulation, not the solver | — |
| ADJ-12 | S2 §2.3, S3 §3: MCTS+GA+BO search speedups | — | not solver evidence | rubric/synthetic scores; offline meta-search is outside the solver | — |

## 1.2 Stiffness / oscillation detection

| id | source claim | source verdict | verdict | reason / evidence | WP |
|---|---|---|---|---|---|
| ADJ-13 | S1-P5, S3-P2 premise: DOPRI5 amplifies for hω > θ* = 0.997189 | PROVED | ACCEPT | exact (T: RK-FRM-02). VER-10: with rtol 1e-3 on ω = 50 the integral controller holds mean hω = 0.966, max 0.9995, 13.7 % of steps above θ*, zero rejections, energy error −2.1 %: the run is stability-limited and the controller does not see it | WP-04 |
| ADJ-14 | S1-P6: use disc(|R(iθ)|²−1) instead of disc(R) | REFINED | ACCEPT concept, not mechanism | the boundary is a constant of the method; precompute the boundary radius along rays once (VER-11 table) and test `|z|/r_b(arg z)` at run time; no run-time discriminant | WP-04 |
| ADJ-15 | S2-EVO-09(1): ordinal-pattern sign-flip trigger | PROVED | REJECT (superseded) | sampling-based, has aliasing blind spots (the source shows one); WP-04 uses the stage data to estimate the spectrum directly | WP-04 |
| ADJ-16 | S1-P11: stiffness detector over-fires on benign stiffness (unexcited fast mode) | HEURISTIC | ACCEPT; resolved by design | the secant-Ritz estimate of WP-04 only sees directions present in the stage displacements, so an unexcited fast mode does not appear | WP-04 |
| ADJ-17 | S2-EVO-13: online Prony frequency inference | PROVED | ADOPT-B (by-product) | WP-04 returns ω̂ = |Im λ̂| at no extra cost; multi-tone Prony stays RESEARCH | WP-04 |

## 1.3 Derivative / Jacobian access

| id | source claim | source verdict | verdict | reason / evidence | WP |
|---|---|---|---|---|---|
| ADJ-18 | S3-P17: complex-step JVP with holomorphy gate | PROVED | ADOPT-B (opt-in trait) | VER-07: error 8.9e-16 vs 1.17e-7 (forward difference); VER-08b: gate flags `abs` (cs = 0 vs fd = −1). Requires a complex-capable RHS; invasive, so opt-in | WP-06 |
| ADJ-19 | S2-EVO-04: 4-node Chebyshev directional derivative | PROVED | ADOPT-B (fallback rung) | VER-07: 1.70e-13 at 4 real f-evals; kink safeguard via cubic coefficient | WP-06 |
| ADJ-20 | S2-EVO-03: γ-rescaling of U/K stopping rule fixes F9 | PROVED | IN-CODE | `rodas5p-integrators/src/raw_stage_target.rs` already states `r_U = γ r_K` and budgets the U equation with `|γ| τ_K` (`raw_absolute_residual_budget`). The source's relation `r_K = hγ r_U` contradicts the code; the code governs. Remaining work: audit call sites | WP-01b |
| ADJ-21 | S2-EVO-03: zero-call secant ("shadow") Jacobian from stage displacements | PROVED | ADOPT-C (preconditioner) | algebra correct; value depends on GMRES iteration counts measured in WP-00 | WP-07 |
| ADJ-22 | S1-P4: randomized low-rank sketch needs a singular-value safeguard | CONDITIONAL | safeguard ACCEPT, sketch RESEARCH | the source counterexample (rank-1 sketch at 1/(hγ)) is flawed for symmetric J (the true operator is then equally near-singular), valid for nonnormal J. The safeguard is applied to WP-07's capacitance matrix | WP-07 |

## 1.4 Stage solves, linear algebra, parallelism

| id | source claim | source verdict | verdict | reason / evidence | WP |
|---|---|---|---|---|---|
| ADJ-23 | S2-EVO-01: 4 superstages give M_inter⁴ = 0, 3 rounds instead of 7 | PROVED | REJECT for speed | VER-05 confirms nilpotency and the norms 46.8 / 424.8 / 428.1. But (i) the existing simultaneous update (T: C4) inverts the full linear C-coupling exactly through L⁻¹, while EVO-01 lags `C_inter` (a regression); (ii) each superstage round contains two serial f-evaluations, so the critical path stays 8 serial evaluations — only the barrier count drops; (iii) the "3 vs 7 rounds" comparison counts unequal work | — |
| ADJ-24 | S3-P7, S1-P16: block stage Krylov serves the 8 RODAS5P stage systems | PROVED | REJECT for RODAS5P | stage i's right-hand side contains `γ Σ_{j<i} c_ij z_j`; inside one simultaneous sweep, L z = rhs is still a block forward substitution with 8 sequential W-solves. S3-E6 solved 8 independent systems, which is not the RODAS5P structure. Possible use: ensembles of independent trajectories | — |
| ADJ-25 | S1-P10 counterexample: H_λ(z) = Wz − g − λhγz² has a fold | PROVED cond. | CORRECT scope | the example lets stage i depend on itself; causal stages have N_i depending only on z_{<i}, so det ∂H = det(W_m)⁸ (T: C7) and no fold exists. Folds arise only in implicit nonlinear fixed points (`nonlinear_be.rs`, `coupled_inverse_be.rs`, `rhs_homotopy.rs` maps) | — |
| ADJ-26 | S3-P5: bordered fold crossing for causal stages | PROVED | RESEARCH (scope moved) | per ADJ-25, applies to implicit nonlinear fixed points only | — |
| ADJ-27 | S2-EVO-02: Chebyshev-cluster q for the quotient resolvent | PROVED | ADOPT-C | algebra correct; `conditioned_polynomial.rs` already takes a shifted/scaled polynomial with a declared bound; the missing piece is automatic interval estimation (finding F11). Skew counterexample (‖D‖ = 2.67) valid: needs the rotation factor | WP-09 |
| ADJ-28 | S2-EVO-05: two-step map T∘T contracts by η² | PROVED | PARTIAL | true per application, but each application costs 2 F-evaluations: no gain per evaluation. The S2-EXP-5 gain comes from the preconditioner P = p(B) (T: C15 with P = p(B)), which needs B = hγJ actions and therefore breaks the derivative-free contract of `rhs_homotopy` unless B is declared | WP-14 |
| ADJ-29 | S1-P2: screened Anderson residual does not contract by q | PARTIALLY DISPROVEN | REJECT the refutation | the source's map has Lipschitz bound 0.5 + 0.4π = 1.757 > 1 (VER-01b), so it is not a contraction. For a q-contraction T: selected residual ≤ Picard-candidate residual = ‖T(Tx) − Tx‖ ≤ q‖Tx − x‖. VER-01: 300 random nonlinear contractions, worst ratio r_{k+1}/(L r_k) = 0.945 ≤ 1. Condition: the screen and the reported residual use the same map T (the λ-map at the current station) | WP-01c |
| ADJ-30 | S2-EVO-07: blocked orthogonalization in Arnoldi | PROVED | ADOPT-B as CGS2 | classical Gram–Schmidt applied twice gives O(u) orthogonality like MGS2 and replaces j dot-product passes by one matrix–vector pass; SIMD WorkModel v3 is RESEARCH (finding F10) | WP-08 |
| ADJ-31 | S2-EVO-06, S3-P8: affine prefix scan / waveform relaxation | PROVED | RESEARCH | associativity exact; benefit needs exposed affine factors and many workers; overflow safeguard (normalized prefix products) required | — |
| ADJ-32 | S3-P6: Cayley–Hamilton on H_m gives a matrix-free solve | PROVED | REJECT as new solver | q = charpoly(H_m) annihilates H_m, not B; the "one Horner application" reproduces the FOM solution on K_m. Spectral split via approximate GCD stays RESEARCH | — |
| ADJ-33 | S1-P1: C18 with Krylov defect δ_i | PROVED | ACCEPT (documentation) | identical to T: C18 with r_i → δ_i; no code change | — |
| ADJ-34 | S2-EVO-08: [2/1] Padé predictor | PROVED | RESEARCH (low) | VER-09 exact on a hyperbolic branch; `rhs_homotopy` has no predictor stage to plug into yet | — |

## 1.5 Exponential, oscillatory and structural routes

| id | source claim | source verdict | verdict | reason / evidence | WP |
|---|---|---|---|---|---|
| ADJ-35 | S3-P1: complex Leja on imaginary/elliptic sets | PROVED | ADOPT-C | `rodas5p-core/src/leja_action.rs` handles symmetric nonpositive operators only; oscillatory φ-actions need the complex set | WP-10 |
| ADJ-36 | S3-P12: C¹ dense-output splice at route switches | PROVED (theory) | ADOPT-C | cheap and local, no evidence yet | WP-11 |
| ADJ-37 | S3-P2 (averaged phase), S3-P3 (WKB), S2-EVO-11/12, S3-P14/P16 | mixed | RESEARCH | declared-structure or asymptotic; no solver-level evidence | — |
| ADJ-38 | S3-P4: radius-of-analyticity step governor | PROVED | RESEARCH | the solver does not compute Taylor coefficients of the solution; the Cauchy-ratio estimate has no data source in the explicit/Rosenbrock routes | — |
| ADJ-39 | S3-P18: adjoint-defect rounding certificate for Krylov φ | PROVED | RESEARCH-B | covers rounding only; truncation stays estimate-only | — |
| ADJ-40 | S1-P17: Lawson/cofactor wrong structure is detected only eventually | PROVED | ACCEPT | documentation of existing contract | — |
| ADJ-41 | S2-EVO-10: Hankel recovery of annihilating polynomials | PROVED | RESEARCH | ill-conditioned beyond d ≈ 6 (source) | — |

---

# 2. Verification results (this session)

All checks are Python/NumPy/SciPy experiments on small models; none executes the Rust code.

```yaml
VER-01: {check: "screened depth-1 Anderson on 300 random nonlinear contractions T(x)=Mx+a·sin(3x)+c, n=4",
         result: "worst r_{k+1}/(L·r_k) = 0.945", status: PASS, supports: ADJ-29}
VER-01b: {check: "Lipschitz bound of S1-P2 map 0.5x+1+0.1 sin(4π(x−2))", result: 1.757, status: PASS, supports: ADJ-29}
VER-02: {check: "PI-form closed loop, k=5, model e_n = k z_n + c_n",
         rows: {Integral:{roots:[0,0], nyquist_gain:2.0},
                existing_Pi(E^-0.7/k·E_prev^0.4/k):{roots:[0.8,0.5], nyquist_gain:2.222},
                S3_E3(kI=0.04,kP=0.02):{roots:[0.822,0.122], nyquist_gain:1.25}},
         status: PASS (Jury holds for all)}
VER-02b: {check: "general 2-step filter log(h+/h)=−b1 log E_n − b2 log E_{n−1} − a2 log(h_n/h_{n−1})",
          rows: {H211b(b1=b2=1/(4k),a2=1/4):{roots:[0.5,0.0], nyquist_gain:1.0},
                 H211PI(b1=b2=1/(6k),a2=0):{roots:[0.5,0.333], nyquist_gain:1.0}},
          note: "first expectation (gain 0) was wrong: these filters put a zero on the step-size response at Nyquist, so log E carries the disturbance with gain 1 (no controller reaction) instead of the factor 2 of Integral"}
VER-03: {check: "DOPRI5 (python), f-evals [rejections], controllers with safety 0.9, clip [0.2,5]",
         oscillator_w50: {rtol1e-4: {Integral: "13620 [471]", Pi: "13530 [0]", H211b: "11490 [0]", H211PI: "13014 [0]"},
                          rtol1e-7: {Integral: "51582 [849]", Pi: "58800 [0]", H211b: "56928 [762]", H211PI: "59226 [318]"}},
         vdp_mu5:        {rtol1e-4: {Integral: "1002 [31]", Pi: "1152 [17]", H211b: "1068 [25]", H211PI: "1140 [23]"},
                          rtol1e-7: {Integral: "3084 [37]", Pi: "3636 [2]", H211b: "3276 [19]", H211PI: "3498 [4]"}},
         nonstiff_mix:   {rtol1e-4: {Integral: 156, Pi: 222, H211b: 180, H211PI: 186},
                          rtol1e-7: {Integral: 378, Pi: 534, H211b: 438, H211PI: 486}},
         conclusion: "no single controller dominates; keep Integral default; H211b only as a ringing-triggered option"}
VER-04: {check: "worst miscoverage P(U < p) over n∈{1,2,3,5,10,21,50}, p∈[0.01,0.99]",
         result: {KlUcb_delta_0.05: 0.0492, Wilson_prior_1_1: 1.0, BetaQuantile_0.95: 1.0}, status: PASS}
VER-05: {check: "RODAS5P fixture, EVO-01 M_inter = (I−γD_intra)^-1 γC_inter",
         result: {M_inter_norms_k1..4: [46.838, 424.760, 428.067, 0.0],
                  gammaC_norms_k1..8: [56.409, 97.539, 143.521, 227.151, 94.202, 103.997, 58.547, 0.0]}, status: PASS}
VER-06: {check: "secant-Ritz spectrum from DOPRI5 stages, oscillator ω=50, h=0.04",
         result: "hλ̂ = ±2.000000 i (exact); |R(hλ̂)| = 1.0318", status: PASS}
VER-06b: {check: "Van der Pol μ=30 at y=(2,−0.6), h=0.01",
          result: "Ritz {−89.91, 7.72} vs exact {−90.78, 0.78}",
          note: "first expectation (both eigenvalues) was wrong; dominant eigenvalue within 1 %, subdominant unreliable. WP-04 uses the dominant eigenvalue only"}
VER-07: {check: "d/dx [sin x e^{−x/2} + x³] at x=1.2",
         result: {forward_difference: 1.17e-7, chebyshev4: 1.70e-13, complex_step: 8.9e-16}, status: PASS}
VER-08: {check: "complex-step on sqrt((x−x_k)²) branch", result: "cs = fd = −1 (branch agrees here)", status: PASS}
VER-08b: {check: "complex-step on |x−x_k| implemented as modulus", result: "cs = 0, fd = −1 → gate rejects", status: PASS}
VER-09: {check: "S2-EVO-08 Padé on z=(1.05−λ)^-1 at λ=0.8", result: 4.0 (exact), status: PASS}
VER-10: {check: "DOPRI5 + Integral on oscillator ω=50, T=20",
         rtol1e-3: {mean_hw: 0.966, max_hw: 0.9995, frac_above_theta_star: 0.137, rejections: 0, energy_rel_err: -0.0207},
         rtol1e-6: {mean_hw: 0.205, frac_above_theta_star: 0.0}, status: PASS}
VER-11: {check: "DOPRI5 stability boundary radius r_b(φ) = first exit of |R(r e^{iφ})| ≤ 1 along the ray",
         table_deg_radius: [[90.0,0.997189],[101.25,2.988136],[112.5,3.334520],[123.75,3.388097],[135.0,3.296645],
                            [146.25,3.295850],[157.5,3.342661],[168.75,3.331763],[180.0,3.306568]]}
VER-12: {check: "SHA-256-truncated epoch golden values (WP-02 layout)",
         values: {"(0, 0.0, 1.0)": "0x5718834c1304ae04", "(3, 0.25, 0.5)": "0x27a1328542cc99d0", "(17, -1.5, 2e-300)": "0x8b1b1407ca5e33aa"}}
```

---

# 3. Work packages

## WP-00 Measurement baseline (prerequisite for every default change)

```yaml
id: WP-00
tier: ADOPT-A
goal: "a reproducible work–precision record for the base commit, so that every later package is judged on measured Ir/f-evals/wall time at matched error"
files:
  harness: crates/vigilode-bench/src/{harness.rs,corpus.rs,metrics.rs,report.rs}
  existing_bench: crates/rodas5p-cli/src/stiff_benchmark.rs, crates/rodas5p-fair-ab/
locate:
  - "grep -n 'pub fn' crates/vigilode-bench/src/corpus.rs crates/vigilode-bench/src/metrics.rs"
battery (minimum):
  B01: {name: nonstiff_mix, rhs: "y1'=-y1+y2², y2'=-0.1 y2+sin(3 y1)", y0: [1,1], T: 20}
  B02: {name: harmonic_w, rhs: "x'=v, v'=-ω² x", omega: [10, 50, 200], T: 20}
  B03: {name: van_der_pol, mu: [5, 100, 1000], y0: [2,0], T: "2μ (one period approx)"}
  B04: {name: robertson, T: 1e5}
  B05: {name: hires, T: 321.8122}
  B06: {name: brusselator_1d, n: [100, 1000], T: 10}
  B07: {name: stiff_forced, rhs: "y'=-1000(y-sin t)+cos t", T: 10}
  B08: {name: pulse_order_reduction, note: "problem with a short stiff transient (S3-E5 regime)"}
tolerances: [1e-3, 1e-5, 1e-7, 1e-9]   # rtol; atol = 1e-3·rtol unless the problem defines it
reference: "tight-tolerance solution (rtol 1e-12) from the most accurate available route; store as fixture with its SHA-256"
metrics_per_run: [f_evals, jac_builds, jvp_calls, lu_factorizations, linear_iterations, accepted, rejected,
                  route_switches, work_model_v2_ir, wall_ns_median_of_5, err_wrms_at_T, err_max_dense_100pts,
                  control_plane_ns]   # control_plane_ns = time outside RHS/Jacobian/linear algebra
outputs: "bench/out/opt_baseline_4339facc.json (one record per problem × tolerance × route policy)"
tests:
  - "rerun gives identical counters (deterministic); wall time within 10 % median-of-5"
gates: [G4]
non_goals: "no solver change"
```

## WP-01 Control-plane correctness repairs

```yaml
id: WP-01
tier: ADOPT-A
goal: "make next_solver_state total on reachable (state, cause) pairs (finding F4)"
files: [crates/vigilode/src/registry.rs]
locate: ["grep -n 'fn next_solver_state' crates/vigilode/src/registry.rs", "grep -rn 'next_solver_state(' crates --include=*.rs"]
math: "total ⇔ every reachable cell of States(7) × Causes(14) maps to Ok(state)"
algorithm: |
  add arms:
    (EntryTrial,     BudgetStop) => Ok(Stopped)
    (ReturnTrial,    FatalStop | BudgetStop) => Ok(Stopped)
    (ReturnSentinel, FatalStop | BudgetStop) => Ok(Stopped)
  keep (Stopped, _) => Err(...) and the illegal-pair error for unreachable cells
  callers: the trial's partial work must already be charged before the Stopped transition (check each call site; do not double-charge)
tests:
  - "exhaustive table test over all 98 cells: expected Ok/Err per the table in §10.2 of the audit plus the 5 new arms"
  - "reachability closure from Initial: every reachable non-Stopped state has Ok for FatalStop and BudgetStop"
  - "a run that hits the budget during EntryTrial/ReturnTrial/ReturnSentinel ends with Stopped, not an error"
gates: [G1 except the error-path behaviour, G4]
note: "check whether crates/vigilode/tests/switching_v2_remediation_contracts.rs (untracked at base) already covers this; merge, do not duplicate"
```

```yaml
id: WP-01b
tier: IN-CODE audit
goal: "confirm every comparison between U-form and K-form residuals uses raw_absolute_residual_budget (|γ|·τ_K) (finding F9)"
files: [crates/rodas5p-integrators/src/raw_stage_target.rs, crates/rodas5p-integrators/src/stage_target.rs, crates/rodas5p-integrators/src/stage_target_policy.rs]
locate: ["grep -rn 'raw_absolute_residual_budget\\|stage_residual_target' crates --include=*.rs"]
math: "r_U = γ r_K for U = ΓK with Γ = (I/γ − C)^-1 (raw_stage_target.rs header). A K-tolerance τ_K maps to the U-tolerance |γ| τ_K."
tests: ["for each call site, a unit test that a K-converged stage passes the U check and vice versa within one ulp-scaled margin"]
```

```yaml
id: WP-01c
tier: IN-CODE audit
goal: "make the rhs_homotopy contraction statement exact"
files: [crates/vigilode-coupled/src/rhs_homotopy.rs]
check: "the residual used by the Anderson screen and the reported residual are computed with the λ-map of the current station (G_λ), not with G at λ=1"
if_true: "keep the module doc sentence; add: 'residual = fixed-point residual of the map iterated at the current λ'"
if_false: "the q-per-update statement does not hold; change the doc to 'non-increasing relative to the Picard candidate'"
tests: ["property test: random contractions (as VER-01), assert r_{k+1} ≤ L·r_k·(1+1e-12)"]
```

## WP-02 Build-stable window epochs

```yaml
id: WP-02
tier: ADOPT-A
goal: "replace DefaultHasher in compute_window_epoch with a fixed-algorithm digest (finding F3)"
files: [crates/vigilode/src/registry.rs, crates/rodas5p-core/src/hash.rs]
locate: ["grep -n 'DefaultHasher' crates/vigilode/src/registry.rs"]
algorithm: |
  bytes = (checkpoint_index as u64).to_le_bytes() ++ start.to_bits().to_le_bytes() ++ target.to_bits().to_le_bytes()   # 24 bytes
  digest = SHA-256(bytes)
  epoch  = u64::from_le_bytes(digest[0..8])
  apply the same layout rule to the other DefaultHasher use near registry.rs:890 (Jacobian kind tag as one byte)
tests:
  golden: {"(0, 0.0, 1.0)": 0x5718834c1304ae04, "(3, 0.25, 0.5)": 0x27a1328542cc99d0, "(17, -1.5, 2e-300)": 0x8b1b1407ca5e33aa}
  - "−0.0 and +0.0 give different epochs (bit layout); document this"
gates: [G4; G1 does not apply to epoch values — list every test/fixture whose stored epoch changes]
non_goals: "no FNV-1a implementation; no new dependency"
```

## WP-03 Admission confidence value

```yaml
id: WP-03
tier: ADOPT-A
goal: "replace the Wilson value used by T8 admission with a selectable rule; default stays Wilson until gate G5"
files: [crates/rodas5p-integrators/src/transaction.rs]
locate: ["grep -n 'beta_upper_confidence_bound\\|prior_alpha\\|prior_beta\\|fn decide\\|fn record_attempt' crates/rodas5p-integrators/src/transaction.rs"]
math:
  kl: "D(p‖q) = p ln(p/q) + (1−p) ln((1−p)/(1−q)), with 0·ln 0 = 0"
  counts: "s = α − α0 (successes), n = (α + β) − (α0 + β0) (attempts); α0 = β0 = 1 at base"
  kl_ucb: "U_KL = max{q ∈ [p̂, 1] : n·D(p̂‖q) ≤ ln(1/δ)}, p̂ = s/n; U_KL = 1 if n = 0"
  coverage: "P(U_KL < p) ≤ δ for every p (Chernoff–Sanov); VER-04 worst 0.0492 at δ=0.05"
  beta_quantile: "U_B = BetaInv(1−δ; α, β) (Bayesian; not a frequentist bound)"
  thompson: "U_T ~ Beta(α, β) with a seeded deterministic RNG stream per bin"
  discount_option: "on each record_attempt: α ← α0 + ρ(α−α0), β ← β0 + ρ(β−β0) before adding the outcome, ρ ∈ (0,1]; with ρ<1 report the rule as EST"
algorithm: |
  enum ConfidenceRule { Wilson { z }, KlUcb { delta }, BetaQuantile { delta }, Thompson { seed } }
  fn kl_upper(s: f64, n: f64, delta: f64) -> f64:
      if n == 0: return 1.0
      p = s / n; if p >= 1: return 1.0
      target = ln(1/delta) / n
      lo = p; hi = 1.0
      repeat 64 times or until hi - lo <= 1e-15:
          mid = 0.5*(lo+hi)
          if D(p, mid) <= target: lo = mid else: hi = mid
      return next_up(hi).min(1.0)          # round outward: an upper bound must not be too small
  decide(): replace `beta_upper_confidence_bound(α, β, z)` by `rule.upper(α, β)`; nothing else changes
config: "AdmissionConfig.confidence_rule: ConfidenceRule = Wilson { z: 1.6448536269514722 } (default, bit-identical)"
tests:
  - "coverage: for n in 1..=50, p on a 99-point grid, Σ_{s: U(s,n)<p} Binom(s;n,p) ≤ δ + 1e-12 (exact pmf in f64)"
  - "monotone non-decreasing in s for fixed n; non-increasing in n for fixed p̂ < 1"
  - "edges: n=0 → 1; s=n → 1; s=0, n=1, δ=0.05 → 0.95 (closed form: D(0‖q) = −ln(1−q) ⇒ q = 1 − δ)"
  - "overhead guard invariant unchanged: simulation of 10^4 decisions with each rule keeps net_overhead ≤ protected_base/10 + one attempt"
gates: [G1, G4, G5 before changing the default]
```

## WP-04 Spectral stability trigger for the explicit route

```yaml
id: WP-04
tier: ADOPT-A (diagnostic) → ADOPT-B (switch input)
goal: "detect, from data DOPRI5 already computes, that the explicit route is stability-limited, and whether the limiting mode is oscillatory or dissipative"
files:
  compute: crates/rodas5p-integrators/src/dopri5.rs
  consume: crates/vigilode/src/switching.rs   # SwitchingEvidence, pressure decision near line 1940
locate:
  - "grep -n 'pub fn step_into\\|DOPRI5_C\\|DOPRI5_A' crates/rodas5p-integrators/src/dopri5.rs"
  - "grep -n 'pub enum SwitchingEvidence\\|rejection_pressure\\|pressure_steps' crates/vigilode/src/switching.rs"
math:
  stability_function: "R(z) = Σ_{j=0}^{5} z^j/j! + z^6/600"
  boundary: "r_b(φ) = sup{r > 0 : |R(ρ e^{iφ})| ≤ 1 for all 0 < ρ ≤ r}, φ ∈ [π/2, π]; VER-11 reference points"
  scalar_estimate: "ρ_s = h·‖K7 − K6‖ / ‖Y7 − Y6‖  (stages 6 and 7 both at c = 1; Hairer's dopri5 stiffness test; STD — confirm against Hairer–Nørsett–Wanner I, §II.10)"
  secant_ritz: |
    S = [Y_1 − Y_0, …, Y_6 − Y_0] (n×6), F = [K_1 − K_0, …, K_6 − K_0]   (F ≈ J S, error O(‖S‖²))
    S P = Q R (QR with column pivoting); keep columns with |R_jj| > 1e-8 |R_11|  → rank r
    H = Qᵀ F_r R_r^{-1}  (r×r)  = Rayleigh–Ritz matrix of J on span(S)
    λ̂ = eigenvalue of H with largest modulus
  indicator: "σ_n = |h λ̂| / r_b(arg(h λ̂)) if Re λ̂ ≤ 0 else 0"
  class: "Oscillatory if |arg(λ̂) − π/2| < π/6, else Dissipative"
  by_product: "ω̂ = |Im λ̂| (feeds chart routes, ADJ-17)"
algorithm: |
  precompute r_b on 256 angles in [π/2, π] by bisection at first use (OnceLock); assert against VER-11 points (1e-6)
  per accepted DOPRI5 step (no extra f-evaluations):
      ρ_s = scalar_estimate                                   # O(n)
      if ρ_s < 0.25 * 0.997189: σ_n = ρ_s / 3.306568; push(σ_n); continue
      compute secant_ritz (O(36 n) flops + 6×6 eig)
      σ_n = indicator; push(σ_n, class)
  window rule: stability_limited = count(σ ≥ σ_lim over last N_w accepted steps) ≥ N_s
  when stability_limited at a batch boundary: record.evidence = ExplicitStabilityLimited { class, sigma_median, omega_hat }
     and treat it as pressure (same path as ExplicitWorkPressure)
config:
  field: "SwitchingConfig.stability_trigger: Option<StabilityTriggerConfig> = None (default off)"
  StabilityTriggerConfig: {sigma_lim: 0.8, n_window: 20, n_required: 15, ritz_rank_tol: 1e-8}
tests:
  - "linear rotation y' = [[0,−ω],[ω,0]] y: λ̂ = ±iω to 1e-8; σ = hω/0.997189"
  - "linear diagonal stiff y' = diag(−1000, −1) y with y0 = (0, 1): the fast mode is unexcited → σ uses λ̂ ≈ −1 (ADJ-16)"
  - "Van der Pol μ=30: dominant λ̂ within 5 % of the Jacobian's dominant eigenvalue (VER-06b)"
  - "B02 at rtol 1e-3 (ω=50): trigger fires with class Oscillatory; at rtol 1e-6 it does not fire (VER-10)"
  - "bit-identity of the integration with the trigger on (pure observer until the switching consumer is enabled)"
gates: [G1, G4; G5 when the evidence is allowed to cause a switch]
non_goals: "no new route; the trigger only feeds the existing PressureAndDue path; route choice for the Oscillatory class stays with the existing trial machinery"
```

## WP-05 Step-size controller options

```yaml
id: WP-05
tier: ADOPT-B
goal: "add a ringing-resistant digital-filter controller as an opt-in kind, plus ringing telemetry; do not change the default"
files: [crates/rodas5p-integrators/src/adaptive.rs, crates/rodas5p-integrators/src/controller_telemetry.rs, crates/vigilode/src/records.rs]
locate: ["grep -n 'enum ControllerKind\\|fn propose_factor_prevalidated\\|previous_accepted_error' crates/rodas5p-integrators/src/adaptive.rs"]
math:
  filter: "log(h_{n+1}/h_n) = −b1 log E_n − b2 log E_{n−1} − a2 log(h_n/h_{n−1})"
  factor: "fac = safety · E_n^{−b1} · E_{n−1}^{−b2} · (h_n/h_{n−1})^{−a2}, clipped to [min_factor, max_factor]"
  H211b: "b1 = b2 = 1/(4k), a2 = 1/4   (Söderlind 2003, b = 4)"
  closed_loop: "χ(λ) = (λ − 1)(λ + a2) + k(b1 λ + b2); stable iff both roots have modulus < 1 (Jury: |c0| < 1, χ(1) > 0, χ(−1) > 0 for monic χ = λ² + c1 λ + c0)"
  existing_Pi_in_this_form: "b1 = 0.7/k, b2 = −0.4/k, a2 = 0"
algorithm: |
  add ControllerKind::H211b (serde name "h211b")
  state needed: previous accepted E (exists) and previous accepted step ratio h_n/h_{n−1} (new field, Option<f64>)
  after a rejection: use the elementary factor (as the Pi arm does) and clear the ratio history
  launch-time check: for every configured (b1, b2, a2), assert Jury conditions; reject config otherwise
  telemetry (observer only): ringing_score over the last 16 accepted steps = fraction of sign changes of log(h_{n+1}/h_n);
     ringing if score ≥ 0.75 and ≥ 2 rejections in the window
optional (ADOPT-C, after WP-00 data): within one tenure, switch Integral → H211b when ringing is observed, reset on route switch
tests:
  - "Jury test table: Integral, existing Pi, H211b, H211PI all stable; b1 = 1/k, a2 = −0.9 rejected"
  - "Integral and Pi arms bit-identical (existing test integral_and_pi_are_unchanged_bit_for_bit must stay green)"
  - "H211b on B02 rtol 1e-4 reduces rejections to 0 and f-evals by ≥ 10 % (VER-03 predicts −16 %)"
  - "H211b on B01/B03 does not exceed Integral by more than 15 % f-evals (VER-03: +6 to +15 %)"
gates: [G1, G4, G5 for any default change]
non_goals: "no online-learned gains (S3-P10b); no change to clip values"
```

## WP-06 JVP accuracy ladder

```yaml
id: WP-06
tier: ADOPT-B
goal: "raise the accuracy of Jacobian–vector products for matrix-free routes when the linear tolerance requires it"
files:
  current_fd: [crates/vigilode/src/fd.rs, crates/rodas5p-integrators/src/problem.rs]
  operator: crates/rodas5p-core/src/operator.rs
locate:
  - "grep -rn 'Jacobian::Jvp\\|fn jvp\\|EPSILON.sqrt()' crates --include=*.rs | grep -v tests"
rungs:
  R0_forward: {cost_f: 1, error: "≈ √u·scale ≈ 1.5e-8·scale", formula: "(f(y+εv) − f(y))/ε, ε = √u (1+‖y‖)/‖v‖"}
  R1_central: {cost_f: 2, error: "≈ u^{2/3}", formula: "(f(y+εv) − f(y−εv))/(2ε), ε = u^{1/3}(1+‖y‖)/‖v‖"}
  R2_chebyshev4: {cost_f: 4, error: "≈ u^{4/5} (VER-07: 1.7e-13)",
                  formula: "(1/δ) Σ_{q=0}^{3} w_q f(y + δ ξ_q v), ξ_q = cos((2q+1)π/8), δ = u^{1/5}(1+‖y‖)/‖v‖",
                  weights: "w solves Vᵀw = e_1 with V_{qj} = ξ_q^j, j=0..3 (derivative of the cubic interpolant at 0)",
                  kink_guard: "fit cubic coefficients c_0..c_3 from the same 4 values; if |c_3| > 1e-2 |c_1| use R1 with ε of R0"}
  R3_complex_step: {cost: "1 complex evaluation (≈ 2–4 real)", error: "≈ u (VER-07: 8.9e-16)",
                    formula: "Im f(y + i ε v)/ε, ε = 1e-20 (1+‖y‖_∞)/‖v‖_∞",
                    requires: "opt-in trait ComplexStepRhs { fn eval_complex(&self, t: f64, y: &[C64], out: &mut [C64]) -> Result }",
                    gate: "once per Jacobian epoch: compare R3 with R1 on the current v; accept if ‖R3 − R1‖ ≤ 1e3 u^{2/3} max(1, ‖R1‖); else disable R3 for the run and record the reason (status EST: the gate checks one direction only)"}
selection: "pick the cheapest rung whose error estimate ≤ 0.1·η_lin·‖b‖, where η_lin is the current Krylov relative tolerance; R0 stays the default"
complex_type: "define `C64 { re: f64, im: f64 }` with the ops needed in rodas5p-core, or add num-complex; adding a dependency needs explicit approval"
tests:
  - "smooth test RHS: R0..R3 errors ordered as VER-07"
  - "RHS with abs(): gate disables R3 (VER-08b)"
  - "B06 brusselator n=1000, rtol 1e-9, matrix-free route: GMRES true residual reaches tolerance with R3 where R0 stalls (measure; WP-00 must first show the stall)"
gates: [G1, G4, G5]
non_goals: "no automatic differentiation; no change to user RHS signature for users who do not opt in"
```

## WP-07 Zero-call secant preconditioner (ADOPT-C)

```yaml
id: WP-07
tier: ADOPT-C   # implement only if WP-00 shows linear iterations dominate matrix-free cost
goal: "precondition matrix-free stage solves with a rank-k secant model built from stage data already computed"
files: [crates/rodas5p-krylov/src/*, the Rodas5pMatrixFree route]
locate: ["grep -rln 'Rodas5pMatrixFree' crates --include=*.rs", "grep -n 'pub fn' crates/rodas5p-krylov/src/gmres*.rs"]
math:
  data: "S = [s_1..s_k] stage displacements, Y = [f(Y_j) − f(y_n)] (k ≤ 6, from the previous step)"
  model: "Ĵ = D + U Vᵀ, D = diag estimate or 0, U = Y − D S, Vᵀ = (SᵀS + λ I)^{-1} Sᵀ, λ = 1e-6 ‖S‖_F²"
  preconditioner: "P = (I/(hγ) − Ĵ)^{-1} = A^{-1} + A^{-1} U (I_k − Vᵀ A^{-1} U)^{-1} Vᵀ A^{-1}, A = I/(hγ) − D  (Woodbury)"
  safeguard: "capacitance C_k = I_k − Vᵀ A^{-1} U; use P only if σ_min(C_k) ≥ 1e-6 σ_max(C_k) (ADJ-22); else no preconditioner"
  column_drop: "QR with pivoting on S; drop columns with |R_jj| < 1e-4 |R_11| before forming Vᵀ"
algorithm: |
  after each accepted step: rebuild (S, Y) from the stage arrays; O(nk) memory
  in GMRES: right preconditioning with P; acceptance still on the true residual of the unpreconditioned operator
tests: ["linear J: P is exact on span(S)", "collinear S (s2 = (1+1e-8) s1): drop path taken, no NaN", "iteration count non-increasing on B06"]
gates: [G1, G4, G5: ≥ 20 % fewer linear iterations on matrix-free battery entries, no accuracy regression]
```

## WP-08 CGS2 orthogonalization in Arnoldi

```yaml
id: WP-08
tier: ADOPT-B
goal: "replace two-pass MGS by classical Gram–Schmidt applied twice (one gemv-like pass per sweep)"
files: [crates/rodas5p-krylov/src/kernels.rs]
locate: ["grep -n 'pub fn two_pass_mgs_into' -A40 crates/rodas5p-krylov/src/kernels.rs", "grep -rn 'two_pass_mgs_into(' crates --include=*.rs"]
algorithm: |
  fn two_pass_cgs_into(V[0..j], w, h_out):
      h1 = Vᵀ w            # one pass over V: for each row i of V, accumulate j dot products
      w  = w − V h1
      h2 = Vᵀ w
      w  = w − V h2
      h_out[0..j] = h1 + h2;  h_out[j] = ‖w‖₂;  w /= h_out[j] (breakdown rule unchanged)
  keep two_pass_mgs_into; select by KrylovConfig.orthogonalization ∈ {Mgs2 (default), Cgs2}
math: "CGS2 gives ‖I − VᵀV‖ = O(u) under the same non-breakdown condition as MGS2 (Giraud, Langou, Rozložník 2005)"
tests:
  - "‖I − VᵀV‖_F ≤ 1e-13·m on random and on Hilbert-like bases, m ≤ 200"
  - "GMRES iteration counts equal to Mgs2 within ±1 on the battery"
  - "work counters: orthogonalization inner products and vector updates unchanged in count (the work model counts flops, not passes)"
gates: [G1, G4, G5: wall-time reduction on n ≥ 1000 entries]
```

## WP-09 Automatic Chebyshev-cluster polynomials (ADOPT-C)

```yaml
id: WP-09
tier: ADOPT-C
goal: "generate q for conditioned_polynomial.rs from an outward spectral interval, instead of requiring a declared q (finding F11)"
files: [crates/rodas5p-integrators/src/conditioned_polynomial.rs, crates/rodas5p-integrators/src/polynomial_resolvent.rs]
math:
  q: "q(z) = Π_j T_{d_j}((z − c_j)/ρ_j), c_j = (a_j+b_j)/2, ρ_j = (b_j−a_j)/2 for σ(B) ⊂ ∪_j [a_j, b_j] ⊂ (−∞, 0)"
  defect: "η ≤ max_j |T_{d_j}((1 − c_j)/ρ_j)|^{-1} for normal B with spectrum in the intervals"
  interval_source: "Gershgorin for explicit B (outward); Lanczos/Arnoldi Ritz values give an estimate only (EST)"
  skew_guard: "if a 2-vector skew test ρ_skew = |⟨v,Bw⟩ − ⟨Bv,w⟩| / ‖Bv‖ > 0.25, include the factor (z + d)² + ω² (damped rotation) instead of a real Chebyshev factor"
gates: ["existing declared-bound contract unchanged; generated q is Certified only with a Gershgorin interval"]
```

## WP-10 Complex Leja φ-actions (ADOPT-C)

```yaml
id: WP-10
tier: ADOPT-C
goal: "φ_k actions for spectra on or near the imaginary axis (exponential route on oscillatory problems)"
files: [crates/rodas5p-core/src/leja_action.rs, crates/rodas5p-core/src/polynomial_router.rs]
math: "Newton interpolation of φ_k on Leja points of a compact set E (segment i[−β, β] or an ellipse); error bound needs normal A or a field-of-values enclosure"
status_rule: "estimate-only unless the operator is normal with a verified spectral enclosure"
evidence_needed: "S3-KC1 shows super-geometric decay on i[−16,16]; no Rust evidence yet"
```

## WP-11 C¹ dense-output splice at route switches (ADOPT-C)

```yaml
id: WP-11
tier: ADOPT-C
goal: "remove the derivative jump of dense output at a route switch"
math: "at switch time t_s both routes have f(t_s, y_s); if the younger window's interpolant derivative at t_s differs from f(t_s,y_s), add a cubic Hermite correction on that window that matches value and derivative at both ends; the correction size is O(local error)"
tests: ["derivative continuity at t_s to 1e-12 relative", "error at interior points not worse than before by more than 2×"]
```

## WP-14 Preconditioned fixed-point map for implicit nonlinear solves (ADOPT-C)

```yaml
id: WP-14
tier: ADOPT-C
goal: "for implicit nonlinear fixed points (nonlinear_be.rs, coupled_inverse_be.rs, rhs_homotopy maps) offer T(x) = x − P F(x) with P = p(B) from the quotient resolvent"
precondition: "B = hγJ action available: declared operator (keeps derivative-free contract) or a JVP rung of WP-06 (breaks it; label the route accordingly)"
math: "C15: if sup ‖I − P ∂F‖ ≤ q < 1 on a ball and ‖P F(x_c)‖ ≤ (1−q) r, then T is a q-contraction with a unique root"
not_adopted: "the two-step map T∘T of S2-EVO-05 (no gain per evaluation, ADJ-28)"
```

## Research backlog (no code before new evidence)

```yaml
RESEARCH:
  - {id: R-01, item: "averaged-phase route (S3-P2)", missing: "detector for an angle variable from a general RHS; resonance guard"}
  - {id: R-02, item: "WKB gauge chart (S3-P3)", missing: "turning-point handling; kernel numerics"}
  - {id: R-03, item: "affine prefix scan / waveform relaxation (S3-P8, S2-EVO-06)", missing: "exposed affine factors; overflow-safe normalized prefixes; worker count where it pays (C5)"}
  - {id: R-04, item: "horizon route scheduling (S3-P11)", missing: "predictive per-route work model with enclosures (S1-P15)"}
  - {id: R-05, item: "CTW regime predictor (S3-P19)", missing: "solver-level benefit"}
  - {id: R-06, item: "waterfilling error budget (S3-P20)", missing: "order-reduction detector"}
  - {id: R-07, item: "bordered fold crossing (S3-P5)", scope: "implicit nonlinear fixed points only (ADJ-25)"}
  - {id: R-08, item: "adjoint-defect rounding certificate for Krylov φ (S3-P18)"}
  - {id: R-09, item: "radius-of-analyticity governor (S3-P4)", missing: "data source for Taylor coefficients"}
  - {id: R-10, item: "randomized sketching (S1-P4)"}
  - {id: R-11, item: "Hankel moment closure discovery (S2-EVO-10)"}
  - {id: R-12, item: "complex-time contour bypass, Sundman reparametrization (S2-EVO-11/12)"}
  - {id: R-13, item: "SIMD-aware WorkModel v3 (S2-EVO-07 second half)", missing: "hardware calibration data (finding F10)"}
REJECTED: [ADJ-11, ADJ-15, ADJ-23, ADJ-24, ADJ-32, "S2-EVO-05 two-step map", "S1-P2 refutation"]
```

---

# 4. Order and dependencies

```yaml
order:
  - WP-00                       # measurement first
  - [WP-01, WP-01b, WP-01c, WP-02, WP-03]   # independent; each a small PR
  - WP-04                       # diagnostic first (observer), then switch input after G5
  - WP-05
  - WP-08
  - [WP-06, WP-07]              # only if WP-00 shows matrix-free linear cost or JVP-noise stalls
  - [WP-09, WP-10, WP-11, WP-14]
dependencies:
  WP-04.switch_input: [WP-00, WP-01]
  WP-05.auto_switch: [WP-00, WP-05.telemetry]
  WP-06.R3: ["approval for num-complex or own C64 type"]
  WP-07: [WP-00]
  WP-10: [WP-04]       # oscillatory class needs a route that uses it
one_pr_per: "work package (or sub-id); no PR mixes a default change with new code"
```

---

# 5. Global gates and anti-drift rules

```yaml
G1_default_bit_identity: "with all new config fields at defaults, every battery run reproduces base_commit trajectories bit-for-bit (compare SHA-256 of the output arrays)"
G2_status_labels: "no new `Certified` label unless every error component is outwardly bounded; new estimators are `EstimateOnly`"
G3_true_residual: "all acceptance decisions use the true residual of the unpreconditioned operator in original coordinates"
G4_build: "cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace"
G5_performance: |
  per problem × tolerance: W = work_model_v2_ir (primary), f_evals, wall_ns (secondary)
  accept a default change iff
    geometric mean over battery of W_base/W_new ≥ 1.00, and
    no single entry with W_new > 1.05·W_base, and
    err_new ≤ 1.5·err_base for every entry (same tolerance), and
    the result holds on a holdout split (B04, B05 not used for tuning)
G6_scope: "a PR touches only the files listed in its work package plus tests; any other file change needs a note in the PR body"
anti_drift:
  - "copy constants from this document or from the code; never retype RODAS5P/DOPRI5 coefficients"
  - "before using an API name, confirm it with the package's `locate` command"
  - "when a test fails, do not relax the tolerance before checking the math block of the package"
  - "if a claim here conflicts with the code at base_commit, the code wins; record the conflict in the PR body"
self_audit_checklist:
  - "Which config field turns this on, and is its default the old behaviour?"
  - "Which battery entries did I run, and where is the JSON?"
  - "Did any `Certified` label move?"
  - "Did I add a dependency?"
```

---

# 6. Reproducing the verification of §2

```
cd <scratchpad>/opt
python3 opt_verify.py  <repo>/fixtures/rodas5p_coefficients_snapshot.json opt_verify.json   # VER-01,02,04..09 (VER-06b prints FAIL: first expectation corrected)
python3 opt_verify2.py opt_verify2.json                                                     # VER-02b (prints FAIL: first expectation corrected), VER-08b, VER-10
python3 opt_verify3.py opt_verify3.json                                                     # VER-03 table
# VER-11 and VER-12 come from the snippets described in WP-04 (bisection of |R| along rays) and WP-02 (SHA-256 layout)
```

---

# 7. References

- E. Hairer, S. P. Nørsett, G. Wanner, *Solving Ordinary Differential Equations I*, 2nd ed., Springer 1993 — DOPRI5, stiffness detection in the explicit code (§II.10) [STD; confirm the exact test against `dopri5.f`].
- G. Söderlind, "Digital filters in adaptive time-stepping", ACM TOMS 29 (2003) 1–26 — H211b, H211PI filters [STD].
- K. Gustafsson, M. Lundh, G. Söderlind, "A PI stepsize control for the numerical solution of ODEs", BIT 28 (1988) [STD].
- W. Squire, G. Trapp, "Using complex variables to estimate derivatives of real functions", SIAM Review 40 (1998) [STD].
- A. Garivier, O. Cappé, "The KL-UCB algorithm for bounded stochastic bandits and beyond", COLT 2011 — KL-inverted upper bound [STD].
- L. Giraud, J. Langou, M. Rozložník, "The loss of orthogonality in the Gram–Schmidt orthogonalization process", Comput. Math. Appl. 50 (2005) — CGS2 [STD].
- G. Moore, A. Spence, "The calculation of turning points of nonlinear equations", SIAM J. Numer. Anal. 17 (1980) — bordered fold systems [STD].
- Bibliographic details above were written from memory and not re-fetched in this session.
