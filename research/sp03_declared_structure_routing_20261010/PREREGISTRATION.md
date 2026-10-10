# Preregistration: routing by declared, validated problem structure (SP03)

Node SP03 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit,
executed on branch `audit/rvj-accuracy-speed-wave1-20261010`. It depends on AS03 (fail-closed evidence validator).

**Claim boundary.**
- Same-binary callgrind instructions and counted work only; no wall-time claim.
- The router is opt-in: a new entry point, with no default change to any existing entry point.
- The existing banded path is not a general sparse LU, and no sparse claim is made.

## Question

The pilot and the speed cycles found two things:

- Wherever a band is declared, the banded direct driver beats every matrix-free arm by 15-270x in flops (pilot B1),
  and runs at 0.255x the dense driver's instructions at n = 400 (SPD03, L-0083).
- For n <= 8 the dense or small direct drivers beat every Krylov arm.

Yet `OdeProblem` carries no structure declaration, and the choice of driver is left to the caller. Can a router that
trusts only declared and validated structure do three things?

- Reject malformed or inconsistent declarations.
- Reproduce the direct drivers' results exactly.
- Beat the strongest applicable arm at matched accuracy, including assembly cost and without hidden O(n^2) work.

## Change

1. **`ProblemStructure` metadata in `problem.rs`:**
   - `Unstructured`;
   - `Dense`, for small n with an explicit Jacobian;
   - `Banded { lower, upper }`, attached through an explicit, validated constructor or builder. The validation checks
     bounds (`lower, upper < n`), that an explicit band Jacobian callback is present, and dimension consistency.
2. **Band verification at the first attempt.**
   - The router compares the band callback's J v with the problem's own JVP, or its dense Jacobian when only that is
     available.
   - It does so on k = 2 seeded random vectors at (t0, y0).
   - It requires a relative mismatch of at most 1e-12 times the norm.
   - The verification work is charged.
   - A mismatch rejects the declaration: the run is refused with a typed error. It never silently falls back.
3. **Router entry point** (`integrate_rodas5p_routed_observed`, in `rodas5p_fast.rs` or a new module). In order:
   1. A declared band goes to the banded fast driver.
   2. Otherwise, `Dense` with n <= 8 goes to the dense fast driver.
   3. Otherwise, a problem with an explicit Jacobian goes to the dense fast driver.
   4. Otherwise, `Unstructured` with a JVP goes to the U-form matrix-free driver (Legacy).

   The result records the routing reason and any fallback.
4. **CLI.** `rodas5p-cli` `stiff_benchmark.rs` gets a `rodas5p-routed` arm for profiling. It declares the band of the
   Brusselator problems, and declares `Dense` for robertson, hires and van-der-pol-mu1000.

## Cells

- **Brusselator-1D** with N = 50, 160, 200 and 500 (n = 100, 320, 400, 1000), at rtol 1e-6 and 1e-8. The band is
  declared.
- **Robertson, HIRES and van-der-pol-mu1000** at 1e-6 and 1e-8. `Dense` is declared.
- **Brusselator-1D-160 at 1e-6** with no declaration: the matrix-free route.
- **Malformed declarations** (validation tests):
  - a band outside the matrix;
  - a declared band narrower than the true Jacobian, which band verification must reject;
  - a missing band callback;
  - a dimension mismatch.

**Arms per cell:**
- `routed`;
- `dense` (the dense fast driver; v2);
- `dense-colext64` (SPD09);
- `banded` (direct call);
- for the Brusselators, also the U-form matrix-free `Legacy` arm.

Error is measured against the references used by `stiff_benchmark.rs`: `NATIVE.json` for N = 50 and 200; for the
others, a dense run at rtol 1e-13.

## Commands

    SP03_RUNS=research/sp03_declared_structure_routing_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test declared_structure_routing -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --locked -p rodas5p-integrators --test declared_structure_routing
    python3 tools/sp03_routing_profile.py --output research/sp03_declared_structure_routing_20261010/PROFILE.json
    python3 tools/sp03_routing_check.py --runs research/sp03_declared_structure_routing_20261010/RUNS.json --profile research/sp03_declared_structure_routing_20261010/PROFILE.json --output research/sp03_declared_structure_routing_20261010/RESULTS.json

**Profiling.** The profile tool runs callgrind on the release CLI with `stiff-profile-run` (2-minus-1 repetitions,
plus a determinism repeat). It profiles `routed`, `banded`, `dense` and `dense-colext64` on every Brusselator cell,
and `routed` and `dense` on the small problems.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first.

## Gate

**PASS** if all of the following hold.

1. **Validation.** Every malformed declaration is rejected with a typed error before any integration step. A correct
   declaration is accepted.
2. **Parity.** `routed` is bitwise identical to its target driver called directly (states, attempts, counters except
   the charged verification JVPs), in every cell. On the Brusselators, `banded` (hence `routed`) is bitwise identical
   to `dense`, as SPD03 found.
3. **Strongest applicable arm.** On the Brusselators, `routed` Ir per trajectory is:
   - <= 0.60x min(`dense`, `dense-colext64`) at n = 400;
   - <= 1.00x min(`dense`, `dense-colext64`) at n = 100 and n = 320;
   - <= 0.05x matrix-free `Legacy`.
4. **No hidden O(n^2).** The `routed` Ir per accepted step grows with a log-log slope of at most 1.15 from n = 400 to
   n = 1000.
5. **Routing overhead.** `routed` Ir per trajectory is <= 1.02x its direct target's on every cell, verification
   included.

Everything else is **FAIL**, with every number preserved.

**Reported:** the routing reasons; the verification cost; the small-problem cells, where `routed` equals `dense`
apart from the verification.

**Predictions.**
- Gate 3: about 0.26x at n = 400 (SPD03: 0.255x of dense v2), about 0.62x at n = 100, and about 0.003-0.03x of
  matrix-free.
- Gate 4: slope about 1.0 (SPD03: 1.003-1.011).
- Gate 5: verification costs 2 J v products plus 2 banded products, which is negligible.

## Prior information

- SPD03 (L-0083) banded arm; SPD09 (L-0086) colext64; pilot B1/B2 structural rivals.
- No code of this node exists before this commit.
