## Task 5: A1-api — production facade crate (node id `a1-api`)

Plan §3 "The public API". New workspace crate `crates/vigilode` (added to the root `[workspace] members`;
`Cargo.lock` updated with `--offline` only for the new path crate; no external deps beyond those already in
`[workspace.dependencies]`).

- `Problem::new(n, rhs)` with builders `.jacobian(Jacobian::{Dense(fn), Banded{lower, upper, fill}, Jvp(fn), FiniteDifference})`,
  `.mass(matrix)`, `.time_derivative(fn)`; `Options { rtol, atol: Atol::{Scalar, Vector}, h0, max_steps,
  dense_output: bool, tier: Tier::{Protected, Performance} }`; `Rodas5p::solve(&problem, (t0, t1), &y0,
  &opts) -> Result<Solution, SolveError>`; `Solution { t, y, stats, status }` where a failed run returns
  its partial solution and a typed reason.
- Routing: `Tier::Protected` calls only existing protected drivers through `integrate_rodas5p_routed_observed`
  or the fast/banded/small/matrix-free drivers directly; `Tier::Performance` is accepted and currently
  behaves as `Protected` (documented; wave 2 attaches the lagged driver).
- `Jacobian::FiniteDifference`: dense forward differences with the standard scaled increment
  √ε·max(|y_j|, 1), counted as RHS calls in `stats`; a banded variant using column grouping (Curtis–Powell–Reid)
  when `.banded(lower, upper)` structure is declared.
- `Atol::Vector`: supported if the underlying drivers support a vector atol; if they do not, return
  `SolveError::Unsupported` rather than silently using a scalar.
- `stats` serializes every option and every work counter (serde), so a run is reproducible from its record.
- Tests: Robertson and the stiff example of `examples/solve_stiff.rs` solved through the facade equal the
  direct driver call bit for bit; FD Jacobian solves Robertson to rtol 1e-6 with error within 2x of the
  analytic-Jacobian run; a failing RHS returns the partial solution.
- Doc comments on every public item; a crate-level example that compiles as a doctest.

