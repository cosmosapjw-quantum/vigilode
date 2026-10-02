# Preregistration: method-labelled negative controls (thread-transfer DAG node P1-NEGATIVE-CONTROLS)

## Question

The prior thread (`docs/reviews/thread_transfer_20261002`, `inputs/thread_loop3/proofs`) found two counterexamples
for its rational-phi method RVJ5 (order-5 W-type Rosenbrock with exact Jacobian):

1. **Semilinear two-step no-go.** `x' = x^2, y' = (-kappa + 2x) y + x^2`, `x(0) = 1`, `y(0) = 1/kappa`, with
   `kappa = h^-6`: after two steps of size h, the `y` error over `h^3` tends to 4/3 (the exact solution stays on the
   slow manifold `y = x^2 / kappa`).
2. **Embedded blindness on the quintic Prothero-Robinson problem** `y' = lambda (y - t^5) + 5 t^4`, `y(0) = 0`,
   `h = 0.1`, `z = h lambda`: the embedded estimate's effectivity (estimate / true local error) falls below 1e-5 at
   `z = -1e6`.

These are facts about RVJ5. Do the repository's own methods (RODAS5P, and the exponential Rosenbrock methods EXPRB43
and PEXPRB54S4) show them? The DAG requires keeping the method identities apart and comparing physical error, not a
stage certificate or an embedded proxy.

## Commands

1. The prior thread's scripts, vendored unchanged into `research/thread_transfer_negative_controls_20261002/thread_loop3/proofs/`
   (`reference_rvj.py`, `verify_exact_loop3.py`, `verify_numeric_loop3.py`; SHA-256 recorded in the ledger):
   `cd research/thread_transfer_negative_controls_20261002/thread_loop3/proofs && python3 verify_exact_loop3.py && python3 verify_numeric_loop3.py`
   (they write `../results/exact_loop3.json` and `../results/numeric_loop3.json`).
2. Native methods, `crates/rodas5p-integrators/tests/thread_transfer_negative_controls.rs`:
   `THREAD_TRANSFER_NEGATIVE_CONTROLS_OUTPUT=research/thread_transfer_negative_controls_20261002/NATIVE.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_negative_controls -- --nocapture`

Native protocol (binary64; fixed steps, no controller; `h = 1/N` with N a power of two, so every represented time is
exact; the same error norm for every method):

- **Two-step semilinear**, N in {16, 32, 64, 128, 256}, `kappa = h^-6 = N^6`: two steps of RODAS5P
  (`sequential_step`, direct solve, exact Jacobian), EXPRB43 and PEXPRB54S4 (Krylov phi actions at their default
  tolerances); record `(y_2 - y(2h)) / h^3`, `(x_2 - x(2h)) / h^5` and the slow-manifold defect
  `(y_2 - x_2^2 / kappa) kappa`.
- **Fixed kappa = 8** control: N in {8, 16, 32, 64} steps of `h = 1/(4N)`, observed order of the final error.
- **Quintic PR**, `h = 0.1`, `z` in {-10, -100, -1e4, -1e6}, one step from `(0, 0)`: true local error `|y_1 - h^5|`
  (the exact solution is `t^5`), the method's own embedded estimate (RODAS5P: `|error_vector|`; EXPRB43 and
  PEXPRB54S4: `|y_new - y_embedded|`), effectivity = estimate / true error.

## Gate

**PASS** if all hold:

1. **Reproduction.** Both vendored scripts pass all their own checks (the prior thread reported 40/40 exact and
   89/89 numeric), including the RVJ5 4/3 limit (within 0.08 at N = 256) and effectivity below 1e-5 at `z = -1e6`.
2. **Native runs complete.** Every native case above returns finite results for every method, or a typed error
   that is recorded as such (no panic).
3. **Labels kept apart.** Every native row carries its method identifier; the report states for each method,
   separately, whether its two-step `y` ratio at N = 128 and 256 lies within 0.1 of 4/3 ("shares the RVJ5 no-go")
   and whether its effectivity at `z = -1e6` is below 1e-5 ("shares the blindness"), and nothing is inferred from
   one method to another.

The native outcomes themselves are measurements, not gate items: either answer is recorded. Claim ceiling:
specific-method, specific-problem regression results only; no stage certificate or embedded proxy is called an
ODE error bound.

## Prior information

- The prior thread's results (above). Before this commit the vendored scripts were dry-run once in a scratch
  directory (40/40 and 89/89 checks, about 7 s); no native run of these problems exists.
