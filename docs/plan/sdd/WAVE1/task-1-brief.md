## Task 1: A1-defects — close the verified correctness defects (node id `a1-defects`)

Plan §6 "Defects to close inside A1" and §2 "Sibling branches". Scope, all in `crates/`:

1. Port the band-solve hardening of PR #73. Source: branch `origin/research/banded-exact-reuse-20261020`
   (fetch it with `git fetch --depth=40 origin research/banded-exact-reuse-20261020`). Take only the
   `crates/` changes of commits `e27c703`, `8d2b2b7`, `b51c051` (diff `b56bb19..b51c051 -- crates/`):
   nonfinite input/output rejection and checked size arithmetic in `rodas5p_fast_banded_solve`,
   `BandedKernel::FixedWidth2`, `FixedWidth2ExactCache` (`exact_band_cache.rs`), `banded_exact_tests.rs`,
   and the test `tests/banded_recovery_contract.rs` if it is in that range (otherwise from `f8e89b7`).
   Resolve conflicts against the current tree; keep the default kernel selection unchanged. Do not port
   research directories or ledger rows (PR #73's L-0137 collides with another branch).
2. Zero tolerance scale: `atol = 0` with a component that is exactly zero at both ends of a step currently
   gives a NaN norm (perpetual rejection) in `rodas5p_fast.rs` (~770–781), `rodas5p_fast_small.rs`
   (~261, ~370), `rodas5p_fast_small_batch.rs` (~424); an `InvalidInput` abort in
   `rodas5p_matrix_free_fast.rs` (~2990–2998); a typed error in `rodas5p-core/src/norms.rs` (55–58).
   Make all of them follow one rule: a component whose scale is zero and whose error is exactly zero
   contributes 0; a component whose scale is zero and whose error is nonzero makes the norm +∞ (reject,
   step shrinks). Implement the rule once (a helper in `rodas5p-core::norms`) and call it from every
   driver. Add a regression test per driver family with `atol = 0`, `rtol = 1e-6` and a problem with
   one identically-zero component.
3. `rodas5p-fair-ab/src/budget.rs:59` slices `&operator_id[..16]`; make it char-boundary safe for short
   and non-ASCII ids; add a test.
4. Accepted/rejected counter drift: `transactional_q1_q2.rs` (~1908–1912) counts a step accepted when
   `error_norm <= 1` while the driver in `integrate.rs` (~1210–1251) rejects when
   `error_norm + contraction_tail > 1` without calling `reconcile_outer_rejection` (`adaptive.rs` ~115–124).
   Call the reconciliation in that driver and in the dense twin (`dense_output_v2.rs` ~2403–2420). Add a
   test that forces the outer rejection and checks `accepted_steps + rejected_steps == attempts` and the
   reconciled counts.
5. `vigilode-coupled/src/solver.rs` (~159–171 vs ~198–207): the eager path computes m stage-0 JVPs per
   Newton iteration whose derivative is never used and charges them. This crate is outside the root
   workspace (own `Cargo.toml`/`Cargo.lock`); build and test it from its own directory. Remove the unused
   evaluation only if output bits are unchanged (the crate has bit-identity tests between eager and
   residual-first paths); update the JVP count expectations in its tests and document the old/new counts.

Acceptance: every new regression test fails before the fix and passes after (show both in the report);
the touched crates' existing tests pass.

