# VigilODE WP1 | source-owned coupled nonlinear BE: matched achieved physical error

**2026-10-31** | bounded derivative-free/VigilODE solver algorithm experiment | parent `cosmosapjw-quantum/vigilode` PR #156 HEAD `bde853af2cd5fc1ac670a15cca6227b84ac20b8f` | no production change.

## R1–R3 source / literature

Freeze the original positive-cooperative cubic source, first-order backward-Euler discretization, `coupled_inverse_be::Source::run` and source-q/original-residual/whole-IVP certified upper bound. Model:

    dy_i/dt = -k_i*y_i-g_i*y_i^3+sum_{j!=i}c_ij*(y_j-y_i)
    k_i>0,g_i>=0,c_ij>=0,y_i(0)>=0.

Preconditioned stage map G_i(z)=psi_i(v_i+h sum_j c_ij z_j), where psi_i is the nonnegative inverse of `(1+h*(k_i+C_i))*w+h*g_i*w^3`. Proven source-scoped q<=max_i(h*C_i/(1+h*(k_i+C_i)))<1 on invariant box. Each run's directed original BE residual and physical-error upper bound remains an independent claim. We test **one scientific source and grid at a time** with two iteration orders (original baseline-first vs fast-first). No alteration of Rust library, default, source coefficients, tolerances at any *individual* run, or physics.

Background: Hundsdorfer (1994), *On the error of general linear methods for stiff dissipative differential equations*, IMA J Numer Anal, DOI 10.1093/imanum/14.3.363, discusses stiffness-robust global-error bounds. Walker–Ni (2011), *Anderson Acceleration for Fixed-Point Iterations*, DOI 10.1137/10078356X, supports the method lineage. Neither paper proves the below *specific numerical work-precision ratios*.

## R4/R5 derived theorem 1: reference-free error-gap bound

For time-grid trajectories A=(a_i), B=(b_i) and the exact ODE trajectory Y=(y_i), let

    e_A = max_i ||a_i-y_i||_infty,
    e_B = max_i ||b_i-y_i||_infty,
    Delta = max_i ||a_i-b_i||_infty.

Then **|e_A-e_B| <= Delta**, directly from the reverse triangle inequality and 1-Lipschitzness of the maximum. This holds independently of discretization, local root iterations, and any reference calculation. Delta is computable solely from frozen represented output-bit vectors using exact binary rational comparisons. This *does not* imply e_A/e_B ~ 1 without a nonzero lower bound on either achieved error.

## R5 derived theorem 2: conditional reference-matching interval

Let R=(r_i) be a numerical high-precision reference and assume (as a separate obligation) a rigorous bound max_i ||r_i-y_i||_infty <= delta. Let `eHat_A=max_i ||a_i-r_i||_infty`, similarly B, and `d=max_i ||a_i-b_i||_infty`. Then

    max(0,eHat_A-delta) <= e_A <= eHat_A+delta,
    max(0,eHat_B-delta) <= e_B <= eHat_B+delta,
    |e_A-e_B| <= min(d, |eHat_A-eHat_B|+2*delta).

A sufficient **rigorous** relative parity screen of tolerance eta (with both nonzero) is

    min(d, |eHat_A-eHat_B|+2*delta)
        <= eta * (min(eHat_A,eHat_B)-delta),
    min(eHat_A,eHat_B)>delta.

**Critical scope:** here `delta` is NOT rigorously established. We compute two independent Decimal70 RK4 grids (microsteps 32 and 64), report their discrepancy `delta_est` as a diagnostic, never interpret it as a certified upper bound. The exact-rational cross-arm Delta is rigorous relative to represented state bits. Consequently the `1% matched achieved` result is `NUMERICALLY_CHECKED_NONRIGOROUS`, not interval proof. The original source BE physical-error upper bound remains **rigorous source-scoped** and must not be relabeled by empirical matching.

## R5 constructive work–precision definition

For fixed source, horizon T, and fixed N grid, define a *predeclared* finite sweep over full original physical tolerance inputs

    eps(alpha) = truncation_upper + alpha*(eps_parent-truncation_upper),
    alpha in {1/16, 1/4, 1, 4, 16}.

The values are distinct calls to the **existing** solver, not a change to its internal stopping conditions. The original eps_parent at alpha=1 stays included for exact replay. For each order and alpha record:

    achieved_peak_global=max_{n<=N}||y_n^num-r_n^ref||_infty,
    achieved_terminal=||y_N^num-r_N^ref||_infty,
    original directed E_upper(n), map_attempted, actual interval charges,
    all-in wall_ns/solve across paired timing blocks, source and output bits.

Let `W_order(t)=min{wall_time(alpha): achieved_peak_global(alpha)<=t}` for explicitly supplied *achieved error caps* t in {5e-5,1e-4,2e-4,4e-4,8e-4}. An empty feasible set is **UNRESOLVED** not interpolated. A Pareto cap comparison does NOT assert identical achieved errors. Record additionally a deterministic best-parity pair among the finite full 5x5 cross-arm tolerance sweep: minimize achieved relative gap, then total mapped calls, then alpha index. A pair is **matched achieved** only if its **peak global** relative gap <=1%. Report terminal parity separately; it can differ dramatically. Independently check reference-free Delta inequality with exact Fractions from all represented state bits.

No continuous-tolerance interpolation as a new measurement. No extrapolation outside observed finite sweep. The timing envelope is bounded to one runner and one source set, with the entire source-inverse+directed residual work charged. Calibration/search runs and high-precision oracle cost are **not included** in the time of a single selected production solve; report candidate count and do not claim search overhead is free if adaptive calibration is proposed as production behavior.

## Scientific/implementation gates and anti-overclaim

- Preprocess: match the exact GitHub PR156 HEAD/tree/source-bits, and verify the source-specific truncation bound at alpha=1.
- Independently parse executed Rust output for every run, use exact Fraction arithmetic for original BE residual, per-step q and physical-error recurrence, all actual clock labels. Preserve failed points/failures and exact counts.
- Decimal70 RK4 32/64 microsteps is independent numerical **reference**, not rigorous exact ODE truth; cannot promote 1% relative parity without a bound for its error.
- `h` and N remain fixed within each cohort and pair; no swapping source, integrator order, or physical grid under the same label.
- Do not retune coefficient precision, source-specific tolerances, time step, or work budgets *within a solver run*.
- Report raw call counts separately from timing, including linear/exchange control and any speedup <=1.
- Preserve R6 same-author adversarial failure analysis without manufacturing independent C9, original RODAS8 comparison or official DAG node admission.

**Authority:** OFFICIAL_VIGILODE_DAG=7/23; REI=28/88 INFO_ONLY; successor admitted=0/12; C9 distinct author=0/1 NOT_RUN; C10=NOT_STARTED; NO_MERGE; PRODUCTION HOLD; AUTO_NEXT_NODE=false.
