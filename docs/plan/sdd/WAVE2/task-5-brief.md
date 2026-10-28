## Task 5: E1-bench — matched-accuracy work–precision harness (node id `e1-bench`)

Plan §7. New standalone crate `crates/vigilode-bench` with its own `[workspace]` and `Cargo.lock`
(lock entries for `diffsol` and its dependencies copied from `crates/vigilode-hybrid/Cargo.lock`; path
dependencies on `vigilode` (facade) and `rodas5p-integrators`).

- Binary `vigilode-bench wp --corpus <file> --out <json>`: for each problem × solver × tolerance in a
  declarative corpus file, run fresh in-process repetitions, record achieved error on a common output grid
  (dense output through each solver's own interface: Diffsol `solve_dense`, VigilODE facade), all work
  counters, failure status and allocator-measured allocations (a counting global allocator in the bench
  binary), and wall time labelled `timing_authority = "diagnostic_only"`.
- Solvers: VigilODE facade (Protected), Diffsol 0.16.1 BDF with FaerLU (dense) and, where Diffsol offers it,
  its sparse/banded backend; SUNDIALS CVODE 6.4.1 (BDF, dense and band linear solvers, C FFI); Netlib DVODE (MF21
  dense, MF24 banded) and Hairer RADAU5 and RODAS (dense output via CONTR5/CONTRO), all through a C/Fortran
  bridge built by the crate's build.rs with gfortran, sharing the one Rust RHS/Jacobian; the internal BDF/Radau reference implementations only with
  `comparator_fidelity = "reference-implementation-only"` and excluded from any speed table.
- Corpus v1: Robertson (n = 3), HIRES (n = 8), van der Pol μ = 1000, Brusselator 1D n = 100 — same
  equations and atol vectors in every arm (one Rust RHS and Jacobian shared by all arms), output grid 101
  points, reference from VigilODE fast at rtol 1e-12 cross-checked against Diffsol at its tightest
  successful tolerance (agreement recorded; disagreement > 1e-8 marks the row `reference-disputed`).
- `vigilode-bench report --in <json> --out <md>` generates the Markdown tables from the JSON only; a test
  checks that every number in the generated Markdown appears in the JSON.
- Matched-error comparison: interpolate each arm's log(time) vs log(error) curve at common error levels;
  speed ratios only where both arms bracket the level; otherwise `DIAGNOSTIC_ONLY`.
- Failed arms are rows with status and work, never dropped.
- Run corpus v1 once at rtol {1e-4, 1e-6, 1e-8} and commit the JSON and generated Markdown under
  `research/bench_wave2_20261021/` with a PREREGISTRATION.md committed before the run and one ledger row with id `L-0139`
  (verdict INCONCLUSIVE: shared 2-core host; the row exists for provenance).

