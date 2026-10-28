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

---

## Results (appended after the run at `7d25d19`)

Outputs:

- `RUST.json`: both RODAS5P arms on 5 problems x 7 tolerances, 7 timed repetitions each;
- `NATIVE.json`: the four native arms rerun in the same session, with references and parity;
- `ANALYSIS.json`: errors and matched-error costs;
- `EVALUATION.json`: the callgrind instruction counts, the accuracy pairs and the gate.

**Gate: PASS.**

1. **Valid benchmark.** Right-hand sides and Jacobians agree bit for bit, the reference uncertainty is at most
   3.8e-13, every arm is deterministic, and all 245 runs completed.
2. **Instructions per attempted step** (callgrind, rtol 1e-6, the same number of attempts in both arms):

| Problem | RODAS5P | RODAS5P-fast | Ratio | Hairer RODAS (L-0030) |
|---|---|---|---|---|
| HIRES | 115,000 | 20,500 | **0.178** | 9,160 |
| van der Pol | 79,800 | 9,480 | **0.119** | 2,950 |
| Brusselator n = 400 | 36.4 M | 9.41 M | **0.259** | 8.83 M |

3. Both limits hold: at most 0.5 on HIRES and van der Pol, and at most 0.8 on the n = 400 Brusselator.
4. **Same accuracy.** At all 35 problems and tolerances, the two arms' final-time errors agree within a factor of
   1.001, far inside the factor of 3 the gate allows. The attempts are identical at rtol 1e-6.
   - The fast arm builds fewer Jacobians: 204 against 210, 351 against 469 and 82 against 92. These are the reuses
     after rejections.

**Matched-error wall time** (descriptive; median ms of the fast arm, and the other arms as a multiple of it):

| Problem | E | rodas5p-fast (ms) | rodas5p | cvode-bdf | cvode-bdf-lapack | hairer-radau5 | hairer-rodas |
|---|---|---|---|---|---|---|---|
| Robertson | 1e-3 | 0.035 | 6.0 | 6.1 | 4.6 | 0.47 | 0.41 |
| Robertson | 1e-5 | 0.035 | 9.0 | 11.2 | 15.9 | 0.72 | 0.53 |
| Robertson | 1e-7 | 0.18 | 5.7 | 4.1 | 4.3 | 0.27 | 0.42 |
| HIRES | 1e-3 | 0.14 | 6.8 | 4.8 | 5.8 | 0.89 | 0.48 |
| HIRES | 1e-5 | 0.29 | 6.8 | 5.4 | 8.0 | 1.09 | 0.70 |
| HIRES | 1e-7 | 0.97 | 6.8 | 3.4 | 4.0 | 0.53 | 1.09 |
| van der Pol | 1e-3 | 0.21 | 10.5 | 3.5 | 3.7 | 0.59 | 0.43 |
| van der Pol | 1e-5 | 0.42 | 10.3 | 10.6 | 8.4 | 0.56 | 0.57 |
| van der Pol | 1e-7 | 0.85 | 10.6 | - | - | 0.46 | 0.91 |
| Brusselator n=100 | 1e-3 | 5.8 | 3.0 | 1.30 | 0.98 | 0.71 | 0.38 |
| Brusselator n=100 | 1e-5 | 13 | 3.1 | 0.97 | 0.75 | 0.48 | 0.39 |
| Brusselator n=100 | 1e-7 | 28 | 2.8 | 1.18 | 0.80 | 0.44 | 0.48 |
| Brusselator n=400 | 1e-3 | 97 | 3.2 | 1.07 | 0.74 | 0.69 | 0.40 |
| Brusselator n=400 | 1e-5 | 218 | 3.1 | 0.82 | 0.53 | 0.48 | 0.41 |
| Brusselator n=400 | 1e-7 | 471 | 3.0 | 0.88 | 0.55 | 0.42 | 0.52 |

**Reading.** The wall readings are diagnostics: one host, single thread, final-time error only.

- **Against the sequential path.** At matched error the fast driver took 3 to 11 times less wall time. Over all 35
  tolerances its own time was 0.08 to 0.36 of the sequential path's, with the same steps and the same errors.
- **Against CVODE.**
  - On the small problems, CVODE took 3.4 to 16 times the fast driver's time, against 0.3 to 1.3 times RODAS5P's
    before.
  - On the Brusselator, CVODE with OpenBLAS took 0.53 to 0.98 times. CVODE is still faster there, since it reuses
    one factorization across many steps.
- **Against Hairer.** RADAU5 and RODAS took 0.27 to 1.09 times the fast driver's time, against 0.04 to 0.25 before.
  The remaining gap is 1 to 3.7 times.
  - At n = 400, the fast driver executes 1.07 times RODAS's instructions per attempt. RODAS5P has 8 stages to RODAS's
    6.
  - On the small problems it executes 2.2 to 3.2 times RODAS's instructions per attempt.
- **What remains in the fast driver.**
  - Nearly all instructions sit in the inlined driver itself (73% to 87% under `run_arm`).
  - The output and represented-clock helpers (`land_capped`, `step_to`) take 4% to 9% on the small problems.
  - Zeroing the Jacobian the problem allocates each step takes 12% at n = 400.
  - The method has more stages than RODAS. On the small problems the represented-clock and output machinery is a
    visible fixed cost per step.
