# Task 4 report: p0-atol (branch production/p0-atol, commit 997fefe, pushed)

Status: DONE_WITH_CONCERNS (concerns are design notes, below).

## Commits
- 997fefe p0-atol: per-component absolute tolerance on the fast routes (single commit; code, tests and docs).

## Requirements -> file:line
- R-C1, `Workspace.atol_vector`: rodas5p_fast.rs:674 (field), :771 (empty default). Lagged driver (`Workspace::new`) unchanged.
- R-C1, `integrate_fast(.., atol_vector: &[f64])`, last parameter: rodas5p_fast.rs:1743; stored at :1762; validated at :1746.
  All 8 call sites updated (the 5 existing public wrappers plus `integrate_banded`, plus the 2 new entry points): `&[]` for scalar callers.
- Error-norm hunk: rodas5p_fast.rs:1035-1046. Scalar branch is the old code verbatim; vector branch uses `atol_vector[i] + rtol*max(|y_i|,|y_new,i|)` through `scaled_error_component`. `attempt(.., atol: f64, ..)` signature unchanged. `adaptive.atol` is used once in the loop, so there was no other use to redirect.
- New entries: `integrate_rodas5p_fast_observed_with_atol` (:1639), `integrate_rodas5p_fast_banded_observed_with_atol` (:1664); re-exported in rodas5p-integrators lib.rs. Validation (length n, finite, >= 0, else `InvalidInput`): `validate_atol_vector` :1614. The public entries also reject an empty slice (it is the private "scalar" marker).
- R-C2 carrier: vigilode solve.rs `enum AtolSpec` (:53), `validate() -> Result<AtolSpec,_>` (:511). Refusals (Unsupported) under any `dense_output` or mass, on JVP-only (matrix-free), and on dense Jacobian + declared band. Invalid vector entries -> `InvalidInput`.
- Facade routing: undeclared dense (analytic or FD) + vector -> new `Plan::DenseFast` (solve.rs:139, :221), entry point recorded as `integrate_rodas5p_fast_observed_with_atol`. Banded plan + vector -> `..._banded_observed_with_atol`, entry point recorded likewise. `Plan` otherwise untouched (one added variant).
- Docs (B4c): vigilode lib.rs "What is refused" and routing table; options.rs `Atol::Vector`.

## Tests (RED then GREEN)
- New `crates/rodas5p-integrators/tests/fast_vector_atol_contracts.rs`: (a) uniform_vector_equals_scalar_bitwise (Robertson dense; Brusselator 50 cells banded lower=upper=2; bits, counters, attempts, banded work all equal), (b) robertson_vector_atol_accuracy, (c) invalid_vector_rejected, (d) zero_component_follows_zero_scale_rule.
- Facade (`facade_contracts.rs`): `vector_atol_accepted_on_fast_routes`, `vector_atol_refused_on_mf_sequential_and_declared_band`; the old vector-atol line of `unsupported_requests_are_refused_not_substituted` removed.
- RED: I implemented before the tests compiled, so there was no compile-failure RED. Substitute: mutation check, forcing the scalar branch (`if true`). Result: (d) and (b) FAILED, (a) and (c) pass as expected (a uniform vector equals scalar by construction). (b) was first too weak (passed under the mutant) so I made `adaptive.atol` loose (1e-6) and added `accepted_steps > loose-scalar steps`; then it failed under the mutant. Mutation reverted; GREEN afterwards.
- GREEN: fast_vector_atol_contracts 4/4; rodas5p_fast_contracts 4; int03_banded_fast_contracts 4; a1_zero_tolerance_scale 9; rodas5p_fast_allocations 2; b1_lagged_contracts 5 (+1 ignored); rodas5p-integrators --lib 75; `cargo test -p vigilode` all pass (4 + 20 + 17 unit/integration + 1 doctest).
- `check-research-node.py --base a5a48f7...`: PASS. Touched files rustfmt'd. No new clippy warnings from my files (existing ones in other files).

## Deviations / design notes
1. (d) is driven with rtol = 0, so the scale of a component is exactly `atol_i = 0`. With rtol > 0 a "zero scale and nonzero error" component cannot be built through the driver (y and y_new both exactly 0 with a nonzero embedded error). `adaptive.atol` stays positive so `adaptive.validate()` passes.
2. (b) reference: fast driver, scalar atol 1e-14, rtol 1e-11; error measured in the vector weights at the endpoint. It passed (<= 2 units) with no tuning. B4d still applies: this is a global-error check, not a guarantee.
3. `Stats.adaptive.atol` with a vector is the largest entry (a scalar is needed for `AdaptiveStepConfig`, and a uniform vector then reproduces the scalar run's record exactly). The vector itself is in `Stats.options.atol`. An all-zero vector with rtol = 0 is rejected by `adaptive.validate()` (both zero).
4. Vector refusal messages all contain "vector atol". Order: pre-existing sequential/JVP/declared-band refusals fire first for the mass/dense_output + JVP combinations.
5. Added `Plan::DenseFast` (one variant) rather than restructuring `Plan`; p0-fastdense owns the restructure and rebases on `validate() -> AtolSpec` and the last-position `atol_vector`.

## Handoff commands
Not run here (workstation): `cargo test -p rodas5p-integrators --locked` (full crate), `cargo test --workspace --locked`, `cargo fmt --all -- --check` (known failure in production_audit_packet_curve.rs only), `cargo doc -p vigilode -p rodas5p-integrators --no-deps` for the new intra-doc links.

## Open questions
- Should `Stats.adaptive.atol` for a vector be the max entry (current) or something else? T7 may want an explicit field.
- Task 8 / p0-fastdense must extend the sequential/dense_output refusal (solve.rs, validate) when fast dense output lands.
