# Preregistration: a lean RODAS5P driver against the sequential path and the native codes

## Question

The profile (`research/stiff_rodas5p_profile_20261002`, L-0030 and L-0031) found most of RODAS5P's time per step
outside the method:

- **Small problems:** 37% to 52% of instructions in malloc and free, and 12% to 17% in a heap-allocating
  small-matrix solve.
- **n = 400:** 27% in unvectorized dense products. These are a diagnostic residual after every direct stage solve
  and a Jacobian product for the gamma terms.

Does a driver that removes these costs, and keeps the method and the step-size control, cut the instructions per
attempted step without changing accuracy? Where does it then stand against the native codes of L-0029?

## The change

`rodas5p_integrators::integrate_rodas5p_fast_observed` (driver id `rodas5p-fast-transformed-v1`) is a new driver.
The sequential path and its records are unchanged.

1. **Transformed stages.** The driver works on the stage variables of the coefficient snapshot's own transformed
   coefficients `a`, `C` and `b_code`. With `W = I/(h gamma) - J`:
   - `W u_i = f(t + c_i h, y + sum_j a_ij u_j) + sum_j (C_ij / h) u_j + gamma_i h f_t`;
   - `y_new = y + sum_j b_code_j u_j`;
   - the embedded error is `u_s`, because `btilde` is the last row of Gamma.

   No Jacobian product is formed.
2. **No residual product after a direct solve.** That product never decided acceptance.
3. **One workspace for the whole integration.** A step allocates nothing beyond the Jacobian the problem returns
   (`OdeProblem::eval_rhs_into` is added for this).
4. **LU choice.**
   - An in-place partial-pivoting LU that skips zero multipliers, for n <= 64 or a first Jacobian with at most 10%
     nonzeros.
   - faer's blocked LU otherwise.
5. **Reuse after rejection.** After a rejected attempt from the same state, its Jacobian, `f(t, y)` and `f_t` are
   reused.

The controller, the represented-clock rules and the output collection are the sequential driver's own functions.

## Commands

- `cargo build --release -p rodas5p-cli --locked`
- `RAYON_NUM_THREADS=1 target/release/rodas5p stiff-benchmark --problems robertson,hires,van-der-pol-mu1000,brusselator-1d-50,brusselator-1d-200 --arms rodas5p,rodas5p-fast --repetitions 7 --warmups 1 --output research/stiff_rodas5p_fast_20261002/RUST.json`
- `python3 tools/stiff_benchmark_native.py --rust research/stiff_rodas5p_fast_20261002/RUST.json --driver target/native_stiff/native_stiff --native-output research/stiff_rodas5p_fast_20261002/NATIVE.json --analysis-output research/stiff_rodas5p_fast_20261002/ANALYSIS.json`
  This reruns the four native arms of L-0029 in the same session, with the same references, parity check and
  matched-error rule.
- `python3 tools/stiff_fast_evaluation.py --rodas5p target/release/rodas5p --analysis research/stiff_rodas5p_fast_20261002/ANALYSIS.json --output research/stiff_rodas5p_fast_20261002/EVALUATION.json --scratch <scratch dir>`
  Callgrind measures instructions per attempted step of both arms at rtol 1e-6 on HIRES, van der Pol and the
  400-component Brusselator, as one integration (the difference of 2 and 1 repetitions).
- Inputs:
  - `crates/rodas5p-integrators/src/rodas5p_fast.rs` and `problem.rs`;
  - `crates/rodas5p-cli/src/stiff_benchmark.rs`;
  - the three tools;
  - the native driver of L-0029;
  - `Cargo.lock`.

## Gate

**PASS** if all of the following hold:

1. **Valid benchmark:** the analysis verdict is PASS (parity, reference uncertainty at most 1e-8, all arms
   deterministic, finite errors).
2. **Small problems:** RODAS5P-fast executes at most 0.5 times the instructions per attempted step of RODAS5P on
   HIRES and on van der Pol.
3. **Brusselator n = 400:** at most 0.8 times.
4. **Same accuracy:** at all 35 problems and tolerances, both arms complete and their final-time errors are within a
   factor of 3 of each other.

Otherwise **FAIL**.

The gate is on deterministic instruction counts and on accuracy. The matched-error wall times, against RODAS5P and
against the native arms, are reported descriptively. The statistical authority of timing stays on hold.

## Prior information

- **L-0028..L-0031:** the earlier benchmarks and the profile.
- **Before this commit, only the contract tests ran:**
  - One step equals the sequential step to 1e-12 of the state scale on seven problems, including a non-autonomous
    one, an 80-component tridiagonal one and an 80-component dense one (faer path). The error norms agree within
    1e-8 tolerance units. No residual or Jacobian products are counted.
  - Adaptive runs on Robertson, van der Pol and Prothero-Robinson at rtol 1e-4 and 1e-7 take within 2% of the
    sequential driver's accepted steps, and end within 10 tolerances of its state.
  - A JVP-only problem gives the same step.
  - A mass matrix is refused.
  - A counting allocator measured 1.44 allocations per attempt for the fast driver on Robertson at rtol 1e-6, of
    which about 40 are the fixed workspace, against 220 for the sequential driver.
- No benchmark, profile or wall time of the fast driver exists before this commit.
