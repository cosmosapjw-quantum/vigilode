# N03/N04 implementation handoff: code is present

## Shipped code

`crates/rodas5p-integrators/src/global_budget.rs` implements N03's model-bound whole-horizon replay and N04's bounded algebraic allocator. `generic_verified.rs` now keeps private immutable scientific binding metadata: model identity, unique trajectory identity, declared initial centre/radius, original absolute L2 budget, fixed span and sealed completion. Mutable public diagnostics never mint authority. No new external dependency is required.

The integration entry point is:

```rust
use rodas5p_integrators::global_budget::integrate_verified_residual_global;
let checked = integrate_verified_residual_global(
    &owner, span, &initial_center, initial_radius, &output_times, &config,
)?;
let native_run = checked.run();
match checked.global() {
    Ok(certificate) => {
        assert!(certificate.binds_run(native_run));
        assert_eq!(certificate.absolute_l2_budget(), config.atol);
    }
    Err(refusal) => {
        // The entire native run, rejected solver work and replay prefix/work
        // remain available. A conservative extra replay can refuse a run
        // whose original model-specific certificate remains valid.
        eprintln!("{}", refusal.error);
    }
}
```

The owner factories are the already implemented `VerifiedContinuousProblem::{from_pr101,from_collective}` and `gray_scott_2d_verified`. The replay accepts no arbitrary RHS, public rho/mu constructor, or copied diagnostic certificate. `VerifiedGlobalBudget` borrows a single sealed run, internally selects its next receipt and checks contiguous time. `finish` requires the original sealed full-horizon completion; changing `run.success` does not bypass it. The returned certificate records both model and unique run identity. Wrapper fields are private, so a different run cannot be silently substituted. Both public replay constructors refuse even a one-ULP increase above the source run’s original `config.atol`; a smaller budget is permitted.

## Exact implemented mathematical contract

For each already verified continuous tube receipt, let `h` be an outward physical duration, `rho` its continuous residual upper bound, `mu` its one-sided logarithmic norm bound, and `J` the outward Euclidean gap between represented polynomial endpoints. The implemented recurrence is

`G = exp_upper(max(mu, 0) * h)` and `R_next = mul_up(G, add_up(add_up(R, J), mul_up(h, rho)))`.

This bounds the entire current segment by T4's comparison argument. The original sealed provider already proves that the actual solution and represented curve lie in its tube. A looser replay radius does not change that proven tube premise. `exp_upper` reuses the repository's interval Cody–Waite reduction, degree-20 Taylor expansion and explicit remainder; it is not a one-ULP libm guess. Overflow refuses.

The initial declared L2 ball is charged at construction. Every segment jump is charged before propagation. Every stored output centre is compared with an outward evaluation of its actual represented polynomial and the displacement is added to its segment radius. The scientific prefix commits only after all checks succeed. Failed work remains counted. Four `n`-component interval scratch buffers are allocated once and reused; successful per-segment/query replay allocates no scratch vectors. Counts report calls to directed primitives, interval exponentials and existing curve work, not a fictional floating-point instruction count.

This is a **post-run whole-horizon replay in one fixed Euclidean L2 norm**, with the same `config.atol`. It does not change candidate `rtol`, output times or step controller, and it does not promote local WRMS estimates to global WRMS certificates. New arbitrary-time binary64 evaluations require their own centre-rounding check; the exact continuous polynomial is covered throughout and original stored outputs are checked explicitly.

## N04 algebraic work allocation

`AllocationBlock` supplies the algebraic coefficients `A_j` and positive cost ceilings `c_j`; this API does not certify that a caller's `A_j` bounds an ODE. The returned `ValidatedWorkAllocation` stores its original blocks, order, budget and integer counts immutably.

`allocate_global_work` uses G2's positive-real optimum as a logarithmic floating seed, takes integer ceilings, then independently verifies `sum A_j/n_j^p <= epsilon` with outward repeated division/addition. Equal integer denominators also use the algebraically equivalent `sum(A_j)/n^p`; the final bound is the minimum of the pointwise and grouped outward bounds. Both enclose the same nonnegative real sum, so this minimum is sound and never widens the original bound. If grouping overflows before division, the valid pointwise calculation remains available. Sorting uses O(m log m) comparison work and O(m) scratch for m bounded blocks; per-block cost accounting is unchanged. The cost ceiling is outward `sum c_j*n_j`. A failed seed receives bounded refinement; resource/overflow/underflow uncertainty produces a typed refusal. There are at most 4096 blocks, order 1–32, 64 refinements and exactly representable per-block counts at most 2^53, with caller-configured smaller caps. `validate_work_allocation` validates supplied integer counts without a heuristic seed.

No exact optimum or `W*+sum c_j` guarantee is attached to an approximate floating seed. The G2 theorem remains available for mathematically exact seeds. The native rational witness uses `A=[1,9]`, `p=1`, `epsilon=1/2`: supplied counts `[8,24]` give error exactly 1/2 and cost 32, while common counts `[20,20]` cost 40 at the same exact algebraic budget. The independently rounded sum `1/20 + 9/20` originally had upper bound `0.5000000000000001`; the retained native RED log demonstrates that limitation. Equal-denominator grouping now uses `(1+9)/20 = 1/2` exactly, certifying the same `[20,20]` count at cost 40 without changing the budget. A separate adversarial test preserves refusal for genuinely unresolved nonexact/subnormal bounds and verifies the safe pointwise fallback when an unscaled grouped numerator overflows. Actual ODE speed still needs model-bound constants and complete-cost measurement.

## Verification-only commands

From repository root with the pinned Rust 1.94.1 and existing offline vendor configured:

```bash
cargo test --locked --offline -j 1 -p rodas5p-integrators --test global_budget_contracts
cargo test --locked --offline -j 1 -p rodas5p-integrators --test all_dag_a05_verified --test all_dag_a11_verified
python3 research/production_reaudit_20261021/verify_global_budget.py
```

`global_budget_contracts` covers nine tests: stationary full horizon with a nonzero initial ball; empty span and model/trajectory identity; atomic refused-prefix/work retention; inability to promote an incomplete run by editing public status; nonlinear positive gain and unchanged native solver work; exact and seeded cost allocations; subnormal/overflow/resource refusal; a Gray–Scott immutable factory; and equal-denominator nonexact, subnormal and numerator-overflow boundary cases.

Native validation passed all 8 new tests plus 7 existing A05 and 7 existing A11 tests (22/22). The subsequently added original-budget guard, grouped-denominator improvement and ninth boundary test await the root final matrix. Initial test-authoring failures are retained in `evidence/n03`; no test budget was loosened. Independent mathematical/source review passed after the run-identity fix. No further code task is required to exercise this implementation. Execution status and exact source/evidence identities are in `NEXT_N03_IMPLEMENTATION.json`. NCP should benchmark the additional replay overhead separately from the unchanged solver run and retain both successful and refused rows. It must report fixed-L2 absolute accuracy separately from the facade's WRMS benchmark families, and cannot infer production speed from algebraic cost ceilings.
