# Preregistration: a static stage-coordinate candidate adapter (integrated DAG node INT-04)

## Question

External review task M1 (findings TF-06, TF-07): RVJ ideas should enter as a coordinate change of the *stage
residual*, not of the physical ODE. Solve `R(Psi(Z)) = 0` for a regular chart `K = Psi(Z)` and restore `K`. The
original target's root is unchanged. The Jacobian is `D_K R D_Z Psi` (static chain rule, no push-forward term).
The restored `K` is only a candidate: the original `StageTarget` certificate decides. Does a native adapter keep
that contract: root correspondence, typed rejection of singular charts and domain exits, complete work counts, and no
acceptance authority?

## Changes (research API; nothing else changes)

`rodas5p-integrators/src/stage_chart_candidate.rs`:

- `trait StageChart`: `forward(Z) -> K`, `inverse(K) -> Z`, `jvp(Z, dZ) = D Psi(Z) dZ`, `contains(Z)` (the chart
  domain), and a name. Stage vectors are flattened, stage-major, of length `s n`.
- `stage_chart_candidate(target, problem, chart, z0, max_iterations, tolerance) -> CoreResult<ChartCandidate>` for the
  quadratic test family of `outward_certificate.rs`. It runs Newton on `F(Z) = R(Psi(Z))`, with `R` the declared
  stage residual in binary64. The coupling is the midpoint of each `L*` interval. The Jacobian is formed column by
  column from `D_K R (D Psi e_k)` and solved by dense LU.
- `ChartCandidate { stages, status, iterations, chart_residual_inf, work }` with
  `status in {Converged, NotConverged, SingularChart, DomainExit, NonFinite}`. It reports `chart_residual_inf =
  ||R(Psi(Z))||_inf`. There is no acceptance field. A caller admits `stages` only through `certify_stage_target` (or
  the native q2 path) on the restored `K`.
- Work counted in `ChartWork`: residual evaluations, chart forward and JVP calls, `D_K R` applications, LU
  factorizations and their order.
- Typed errors (`Err`) for a start `z0` outside the domain, a shape mismatch (including an `n`-vector endpoint
  offered as `s n` stages), non-finite input, or a non-positive tolerance.

## Charts (fixed now)

1. **Polynomial triangular:** `K_0 = Z_0`, `K_i = Z_i + beta (Z_(i-1) * Z_(i-1))` (componentwise), with
   `beta = 0.5`. It is regular everywhere (`det D Psi = 1`), with an explicit sequential inverse.
2. **Scaling:** `K = sigma * Z` componentwise, with `sigma_(i,a) = 2^-(i mod 4)` times `(1 + a/n)`.
3. **Singular:** `K = Z^3` componentwise, started at `Z = 0` (where `D Psi = 0`).
4. **Positive square:** `K = Z^2` on the domain `Z > 0`, started at `Z = 1` for a family whose root has negative
   stage components, so Newton must leave the domain.

## Families (fixed now)

- R4 diagonal quadratic (`A = diag(-1 - i)`, `q_i = -0.05 (1 + i mod 3)`, `y_i = 1 + 0.1 i`), n in {1, 4, 16},
  `J = A + 2 diag(q y)`, h in {0.05, 0.5}.
- A non-diagonal 2x2: `J = [[-2, 1], [0.5, -3]]`, `q = (-0.1, 0.05)`, `y = (1, -0.5)`, h in {0.05, 0.5}.

Target: `StageTarget::sequential`. Reference root: the direct sequential solve of the same binary64 residual (stage
by stage, a dense LU of `W = I - h gamma J`). Start: `z0 = Psi^-1(0)` (charts 1 and 2).

## Commands

`INT04_OUTPUT=research/int04_stage_chart_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test int04_stage_chart -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-integrators --locked --test int04_stage_chart_contracts`

## Gate

**PASS** if all hold:

1. **Root correspondence.** For charts 1 and 2 on every family and h, Newton reaches `Converged` within 20
   iterations (tolerance `1e-13 (1 + ||F(Z0)||_inf)`). The restored `K` agrees with the reference root to `1e-12`,
   relative to `max |K_ref|`. `certify_stage_target` on the restored `K` (diagonal or small witness) closes, with an
   output WRMS bound at most `1e-8` (atol = rtol = 1e-6).
2. **Chart identities.** For both regular charts, `inverse(forward(Z)) = Z` to `1e-14` relative on 20 seeded random
   `Z`, and `jvp` agrees with a central difference to `1e-6` relative.
3. **Typed rejections.** Chart 3 gives `SingularChart`, and chart 4 gives `DomainExit`. A start outside the domain,
   an `n`-vector "endpoint" in place of `s n` stages, a non-finite start and a non-positive tolerance are `Err`.
4. **No authority.** On every family, a candidate built with an approximate `W` (stages solved sequentially with
   `gamma (1 + 1e-3)`, an RVJ-style approximate-W predictor) is passed to the original certificate. Its certified
   stage bound is at least the measured distance from the reference root, componentwise, so the certificate exposes
   the predictor's error.
5. **Accounting.** `ChartWork` counts match the operations the test can recount from the iteration count:
   residual evaluations = iterations + 1, forward calls = iterations + 1, `s n` JVPs and `s n` `D_K R` applications
   per Jacobian, one LU per iteration.

Otherwise **FAIL**. Reported, not gated: iterations, work against the direct sequential solve (`s` solves of order
`n` against one LU of order `s n` per iteration), and final residuals. No timing; this is a candidate provider for
research, not a solver path.

## Stop condition

Any acceptance derived from the chart status or the chart residual, or a lost root, stops the adapter.

## Prior information

The external review's section 8.4 (static chain rule, symbolic check of `det D_K F = det(W)^s`), L-0040 (RVJ
negative controls) and L-0051 (RVJ5 not uniform). No code of this node exists before this commit.
