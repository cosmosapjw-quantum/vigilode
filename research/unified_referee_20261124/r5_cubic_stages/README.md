# R5 native cooperative-cubic stage systems

The frozen source has $f_i(y)=-k_i y_i-g_i y_i^3+\sum_{j\ne i} c_{ij}(y_j-y_i)$ with $k_i>0$, $g_i,c_{ij}\ge0$. The first four historical grid/source cohorts are specified by `crates/vigilode-coupled/examples/coupled_achieved_wp.rs`. The sparse output variant reuses the asymmetric source. A separate high-exchange stress case exercises fallback. This executable is opt-in.

## Stage equation

In the native fast driver's unscaled convention, let $a=1/(h\gamma)>0$. Upon every fresh linearization freeze the accepted state. Define $D_i=a+k_i+3g_i y_i^2+\sum_j c_{ij}$. For each signed stage right hand side supplied by the driver, solve $(D-C)u=b$. Each intermediate continuation station solves $(D-\lambda C)u=b$ by parallelizable Jacobi updates with frozen previous iterates. In infinity norm the contraction bound is $\lambda\max_i \sum_j c_{ij}/D_i<1$. Since rows are strictly diagonally dominant, $\|u-u_*\|_\infty\le \|(D-C)u-b\|_\infty/(a+\min_i k_i)$. This inequality applies to every represented signed stage RHS. Historical BE cubic root continuation and the new linear RODAS5P stage solve have different residuals.

The implementation evaluates the signed original residual using outward-enclosing binary64 operations and bounds the inverse margin from below. The IEEE round-to-nearest and gradual-underflow check runs before integration. The fixed model supplies an algebraic diagonal slope, so accepted structured stages need neither a Jacobian callback nor Newton iterations. The implementation evaluates an analytically known source derivative. Generic black-box derivative freedom would require different evidence. The certificate bounds a stage with the represented frozen inputs; it does not bound the time-discretization or the nonlinear propagation of errors from previous stages.

The direct arm solves at `lambda=1`. The continued arm uses `0, 0.5, 1`. Both use a 24-sweep limit per station and check the terminal true residual before committing the solution. All rejected work is charged. If the screening estimate of the contraction is at least 0.75, the iteration exhausts its budget, or a residual enclosure is unrepresentable, the original RHS is still intact and the protected small dense LU solves it. Jacobian and LU are prepared lazily and shared among fallback stages of the same frozen state and attempted step. The one-sweep arm deliberately induces fallback. The source is bounded to the explicitly declared two- and three-dimensional cohorts; no public Auto routing changes.

## Validation

Each arm runs the native adaptive eight-stage tableau on the complete interval `[0,0.1]`. The source and resolution controls use 64 or 128 requested subintervals. These output stops can force tiny steps at both tested rtol values. After the initial SciPy comparison reached its numerical resolution on several fine grids, a four-subinterval follow-up and two independent high-precision Taylor resolutions were added. The Taylor recurrence expands the complete cubic source from its polynomial coefficients, with 22/32 terms, 60/85 decimal digits and rate-based substepping; the fine discretization uses a twofold tighter substep selection in the rate-limited regime. The symmetric linear case additionally has a closed-form reference. SciPy DOP853 and Radau provide checks with implementations independent of the power series. These are numerical reference comparisons. The directed residual certificates have separate exact-real hypotheses.

`WORK_PRECISION.json` reports achieved peak componentwise absolute error over common output times, maximum trajectory difference, source and candidate work, retries and fallback counts. The shared 1% criterion applies to relative differences between achieved errors. Near machine precision it can distinguish trajectories differing by only a few ulps. The `Direct` and `Continued` arms satisfy it for 7 of 12 source/grid/tolerance combinations; their remaining five rows are retained. The underlying driver charges `jacobian_builds` on every linearize callback; `fallback_jacobian_calls` counts dense derivative callbacks actually invoked by this client. The scheme does not remove dependencies between the eight nonlinear evaluations of the native tableau.

## Reproduction

```sh
cargo run --locked --offline -p rodas5p-integrators --example r5_cubic_stages | gzip -n > TRAJECTORIES.jsonl.gz
python3 analyze.py TRAJECTORIES.jsonl.gz WORK_PRECISION.json
cargo build --locked --offline --release -p rodas5p-integrators --example r5_cubic_stages
python3 paired_timing.py "${CARGO_TARGET_DIR:-target}/release/examples/r5_cubic_stages" TRAJECTORIES.jsonl.gz WALL.json
```

Use the repository root for Cargo. Supply the directory paths to the Python scripts as needed. The timing script reads all native outputs back and checks every trajectory against the recorded run. Eight measured blocks alternate the forward and reverse order after two unmeasured warmups. The other processes and CPU frequency of the host were uncontrolled. A result on these small structured sources gives no evidence of a throughput gain on large or undeclared systems.
