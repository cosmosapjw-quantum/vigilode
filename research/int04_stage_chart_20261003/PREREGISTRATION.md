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

---

## Results (appended after the run at `feb4375`)

Output: `RESULTS.json`. Ledger row L-0055. Contract tests `int04_stage_chart_contracts` 3/3 (chart identities,
typed singular-chart and domain-exit statuses on all 8 family/h cases, invalid inputs).

**Gate: FAIL** (items 2 and 3 hold; items 1, 4 and 5 fail as recorded).

| Gate item | Outcome |
|---|---|
| 1. Root correspondence | **fails**. The triangular chart does not converge from `Psi^-1(0)` on 3 of 8 cases: R4 n = 16 at h = 0.05 (`singular-chart` after 8 iterations, residual 8e10), n = 4 at h = 0.5 (`not-converged` after 20) and n = 16 at h = 0.5 (`singular-chart`, residual 9e176). Where it converges, the restored `K` matches the sequential root to 1.4e-15 relative (5e-14 on the 2x2 at h = 0.5). The scaling chart converges on all 8 within 4 iterations (difference at most 3.7e-13). But 6 of the 13 converged runs have a certified output bound above the gate's 1e-8 WRMS (largest 2.3e-7). With atol = rtol = 1e-6, 1e-8 WRMS is about 2e-14 absolute, at the level of the binary64 residual the Newton tolerance allows. The threshold was set below what a converged binary64 candidate can certify |
| 2. Chart identities | **holds** (contract tests) |
| 3. Typed rejections | **holds** (contract tests) |
| 4. No authority | **fails as written**. On 7 of 8 cases, the certified stage bound of the approximate-W candidate is at least its distance from the reference root (ratio 1.0000000000004 to 1.09). On R4 n = 16 at h = 0.5 the smallest ratio is 0.999999999999914: the bound is 1.2e-14 below a distance of 0.13 measured from the *binary64* reference root. The certificate bounds the distance to the exact root. The binary64 reference differs from it by its own rounding, and the gate did not allow for that. So the item compares against the wrong quantity; it is not evidence of a non-enclosing certificate. The certificate reports output bounds of 1.7 to 2.1e5 WRMS for these predictors, so they are not admitted |
| 5. Accounting | **fails as recorded, because of the test**. The test checked the counts only for converged runs and counted any other run as a failure. Recounted post hoc from `RESULTS.json` with the preregistered rule (residual evaluations and forward calls = iterations + 1; `s n` JVPs and `s n` `D_K R` actions per formed Jacobian; one LU per formed Jacobian), all 16 runs match, the non-converged ones included |

What the run shows:

- **The adapter keeps its contract.** Every failure is a typed status, and a non-converged candidate's certificate
  reports what it is (output bounds 3.5e5 and 5e5 WRMS, or no certificate at all for a non-finite candidate). The
  approximate-W predictors are not admitted either.
- **A chart is not free.** The polynomial triangular chart, regular everywhere with `det D Psi = 1`, still turns a
  problem that the direct sequential solve handles in one pass into a Newton iteration that diverges from the zero
  start at n = 16 or h = 0.5. The scaling chart does not hurt.
- **Cost.** One adapter iteration factors a matrix of order `s n` (up to 128 here). The direct sequential root needs
  one LU of order `n` and 8 solves. The target is block lower triangular with `det = det(W)^s` (TF-07), so this
  family offers the adapter no cost case. Its use would be as a candidate provider where the native target is not
  solved stage by stage.

Claim ceiling: the adapter's contract on two small families; no solver path, and no timing.
