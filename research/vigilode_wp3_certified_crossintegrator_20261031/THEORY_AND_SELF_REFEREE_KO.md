# VigilODE WP3 | 원본 cubic IVP 오차상계 개선, proof-search, 적분기 비교

2026-10-31 KST. **소유 프로젝트** `cosmosapjw-quantum/vigilode`, immutable PR160 HEAD `cf690ad5ec723052e9eb6f71335f7cb3b43607a6`, PARENT PR158 `72bd52d6936b923bd110222f36df79172b9ced49`. Original source Git blob `d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482`. **공식 VigilODE DAG 7/23, admitted successor 0/12, distinct-author C9 0/1 NOT_RUN, production HOLD, no merge/default/tolerance relaxation.**

## R1-R3 | 물리 출처와 승인 범위

Four exactly frozen WP1 source cases from original native Rust logs: `asymmetric_2` (2 DOF, N=64), `asymmetric_2_refined` (same source, N=128), `coupled_3_cubic` (3 DOF, N=128), `linear_exchange_control` (2 DOF, N=128), all on original [0,0.1] and caller 5-alpha sweep frozen in WP1. No new owner RHS, no new physical assumptions. Metric is `l∞`, binary64 coefficients interpreted as exact dyadic real numbers.

Original source for i=1,...,d:

\[
\dot y_i=f_i(y)=-\kappa_i y_i-g_i y_i^3+\sum_{j\ne i}c_{ij}(y_j-y_i),\quad \kappa_i>0,\quad g_i,c_{ij}\ge0,\quad y_i(0)\in[0,M].
\]

Positivity and inward boundary derivatives make `[0,M]^d` invariant. If `z=u-v` and `|z_i|=||z||∞`, the monotone real cubic and nonnegative off-diagonal couplings imply `sgn(z_i)(f_i(u)-f_i(v)) ≤ -κmin||z||∞`. Thus any differentiable interpolant X with defect r=X'-f(X) satisfies `D+||X-y||∞ ≤ -κmin||X-y||∞ + ||r||∞`.

## R4-R5 | exact-rational de Casteljau + integrating-factor theorem

For each original fixed fine WP2 dyadic panel, retain the *same represented endpoint states* and exact-source Hermite cubic `P(s)`. Its original cubic RHS defect `P'/h-f(P)` is polynomial degree ≤9. Exact power-to-Bernstein conversion from existing WP2 yields rational Bernstein coefficients `b_{ik}`. Bernstein basis nonnegativity gives `R0=max_{i,k}|b_ik| ≥ sup_{s∈[0,1]}||r(s)||∞`.

Apply de Casteljau subdivision exactly over rationals. Each child Bernstein coefficient is a convex combination of its parent coefficients, and on the child `s↦(s+j)/2^d` the represented function is IDENTICAL to the original polynomial. Therefore each level d gives the certified `R_d ≤ R_{d-1}`, while still bounding the same full continuous-time residual over the original panel. This is not evaluating at a finite collection of times.

For `x=κmin h≥0`, define `P_m(x)=sum_{j=0}^m x^j/j!`. The positive exponential Taylor series ensures `P_m(x)≤exp(x)`, hence `exp(-x)≤1/P_m(x)`. For any odd q=2k+1, the Taylor remainder of `exp(-x)` has nonnegative sign, so `L_q(x)=sum_{j=0}^q (-x)^j/j!≤exp(-x)` and

\[
 0≤\frac{1-e^{-x}}x ≤ \min\{1,\ B_q(x)\},\qquad
 B_q(x)=\sum_{j=1}^{q}\frac{(-x)^{j-1}}{j!}.
\]

Define `B_0=1`, and for `q=0,3,5,7` and `m=1,2,3,4` use the mathematically upper-bounding rational coefficients. The fully conservative recurrence is

\[
\delta_{n+1}=\mathrm{up}_{f64}\!\left(\delta_n\,\mathrm{up}_{f64}(1/P_m(x)) + hR_{d,n} \min\{1,B_q(x)\}\right),\quad\delta_0=0.
\]

Exact `Fraction` intermediates with final smallest upward binary64 conversion; both factors are nonnegative. The recurrence encloses the exact ODE error on each fine panel. At each original coarse grid checkpoint take max over certified δ, and include the **unchanged WP2 original label clock displacement bound**. A false candidate reference only enlarges its original defect. At `(depth,m,q)=(0,1,0)` the new code reproduces the WP2 prior δ **bit-for-bit** in all four cohorts.

**Actually executed 14336 fine panels micro32**. Results are exact-rational certifier outcomes conditioned on original WP2 source and bit-locked reference states. Values below include only reference envelope, not an independently reestimated clock displacement (clock values are separately preserved):

| Cohort | baseline δ | improved δ | relative tightening | chosen lemma |
|--|--:|--:|--:|--|
| asymmetric_2 | 7.800560893826198e-13 | 5.518739215995957e-13 | 29.2520% | D2_Q4_B5 |
| asymmetric_2_refined | 1.1570014393561093e-13 | 8.58449642095019e-14 | 25.8039% | D2_Q3_B5 |
| coupled_3_cubic | 4.3589692674853883e-13 | 3.0814954953385333e-13 | 29.3068% | D2_Q3_B5 |
| linear_exchange_control | 2.5505122601086332e-14 | 2.1282366606085117e-14 | 16.5565% | D2_Q3_B5 |

Additionally, `asymmetric_2` micro64 exactly replays WP2 and tightens its bound to `8.58449642095019e-14`; this micro64 source/h aligns with the refined case micro32, as expected. Tightening of a certified upper bound is **not** reduced solver error, speedup, new RODAS order, or novel universal convergence theorem.

## R4/C5 | actual OpenEvolve ↔ MCTS alternating search

Library-archived OpenEvolve official v0.4.0 source installed isolated, with a minimal *annotation-only* OpenAI namespace shim because external OpenAI Python SDK is not installed; the shim explicitly refuses API calls. Actual `openevolve.database.ProgramDatabase` performs two-island MAP-Elites archiving/scored fitness. The assistant-authored candidate mutations are injected as new `Program` objects; UCT tree search allocates between split depth, exponential upper order and defect integral upper order. Four evolution periods and four MCTS periods, **48** predetermined exact candidates, **29 unique visited**, **32 actual UCT root traversals**, actual DB 23 stored / provenance in `OPENEVOLE_MCTS_...json`, zero external LLM calls. Tree visits do not prove theorems; exact rational certifier is the admission criterion. Research design adopted from user-owned Dropbox `tree_search_research_preimplementation_final_v1_0.zip` extracted text (completed N versus pending U; serial U=0; no false Gibbs semantics).

## R7/C7-C8 | actual original RHS scientific data with five independent integrators

On **exact original four cooperative cubic source cases**, run SciPy `solve_ivp` RK45, DOP853, Radau, BDF and LSODA ×4 user-independent exploratory tolerance settings `(1e-4,1e-6,1e-8,1e-10)` ×3 timed repeats = **80 science rows, 240 actual integrations** (same current sandbox). All trajectories evaluated at the original Rust output clock labels. Their achieved peak-global error is enclosed by exact dyadic discrepancy from **frozen WP2 micro64 reference state bits**, plus the verified WP2 upper bound and clock displacement. **63/80** have certified achieved peak-global error upper ≤1e-6. All source coefficients and y0 bits were independently asserted; no actual original native RODAS5P/RODAS8 run is included. Times are not comparable directly to old WP1 remote GitHub runner.

Independent analytic linear limit: for `linear_exchange_control`, `y+(t)=((y1(0)+y2(0))/2)e^{-4t}`, `y-(t)=((y1(0)-y2(0))/2)e^{-16t}`, `y=(y++y-,y+-y-)`. Using 95-digit mpmath and **exact dyadic initial coefficients**, the true WP2 reference peak discrepancy is `2.059450431927236878...e-16` (micro32); it lies below new certified `2.1282366606085117e-14`. The original micro64 3.00594e-16 lies within previous micro64 3.79755e-14. This is a true physical analytic consistency check, not formal interval proof for mpmath roundoff.

## Same-author hostile referee, actual findings and disposition

1. **IMPLEMENTATION DEFECT FIXED:** An early odd-order source-integral Taylor implementation applied alternating signs twice, producing a safe but non-improving B factor. Discovered in self-referee and corrected `sum((-x)^(j-1)/j!)`. Recomputed all four science cohorts and tests. Discard earlier B-variant selection.
2. **SEMANTIC IDENTITY DEFECT FIXED:** 8/80 LSODA returned `t=0` sampled states 1 ULP different from the *exact caller input*. This is a representation/interpolation difference, not source replacement. Separate caller-initial bit identity (all 80 true) and sampled t0 bit parity (72 true, 8 LSODA false), count true t0 output error in achieved error. Do not suppress negative observations.
3. **HOST TOOL BLOCKER:** Neither Rust 1.94.1 nor Cargo are installed in this ephemeral sandbox; no native RODAS5P executable was run. A source-frozen GitHub hosted CI workflow and original-call example are written but must be separately verified before claiming native measurements or changes to official DAG.
4. **FORMAL PROOF BLOCKER:** Library offline archive includes Zarith/Camlp5/OpenEvolve source and historic OCaml `.byte` but not usable ocamlrun, opam, OCaml compiler or HOL Light kernel in the current sandbox; archived Camlp5 port fails `Unbound module Parsetree`, no HOL certificate. Independent exact Fraction polynomial identities plus Wolfram symbolic Taylor verification do NOT constitute HOL Light kernel verification.
5. **SEARCH BIAS CONTROL:** Invalid purported upper decay `q=1/3 at x=1` refuted because `e<3` gives `e^-1>1/3`. Candidate UCT fitness is heuristic resource allocation, never a proof admission rule.
6. **VALIDATION:** New Python focused hostile `pytest 8/8 PASS`. Original library archives ZIP CRC and SHA verified; prior WP2 reference provenance preserved. Distinct-author blind C9 `0/1 NOT_RUN` even though this same-author adversarial audit ran.

## C5 native future work, independent of current mathematical result

`native_cubic_rodas5p.rs` encodes actual original four WP1 cubic IVPs and original native `rodas5p-integrators::integrate_sequential_matrix_free_adaptive_dense_observed`, with analytic JVP, original eight-stage fifth-order tableau, source-state bits, original output clock grid, rtol×4 and three measured full solves. It does **not** change the original solver or default. A new research-only workflow downloads WP1/WP2 original Actions artifacts and independently checks Rust output against exact binary64 source and WP2 achieved-error certification. At this handoff: Rust source authored; compile and native scientific execution **NOT_RUN locally**. GitHub Actions CI on the isolated research branch will be the authoritative native evidence if actually triggered and completed.

External primary literature: G. Steinebach, *Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package*, BIT Numer Math (2023), DOI `10.1007/s10543-023-00967-x`. Relevant exact one-sided dissipative a posteriori literature and WP2's pinned prior theory remain background; this scoped calculation is not asserted to be novel.

## Decision

- R1–R10 are bounded theory deliverables including explicit scientific uncertainty. C1–C10 artifacts are recorded; native part of C5–C8 **NOT_RUN** until CI evidence. No R/C stage status may be used to promote the 23-node official DAG.
- Official **VigilODE 7/23; successor 0/12; distinct-author C9 0/1**. Production HOLD; NO_MERGE, NO_DEFAULT_CHANGE, NO_TOLERANCE_RELAXATION, AUTO_NEXT_NODE=false.
- Save all stage JSON separately, archive entire current actual source/data/plots/test/handoff with SHA manifest; remote provider acknowledgement and actual restore must be reported separately from local ZIP.
