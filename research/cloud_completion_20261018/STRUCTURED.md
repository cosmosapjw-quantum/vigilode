# A06: all-stage native structured RODAS5P

## Result and contract

`CollectiveRodas5pWorkspace` and `integrate_collective_rodas5p_observed` now execute all eight native K-coordinate RODAS5P stages for an owned immutable `CollectiveModel`. The adaptive wrapper uses the existing controller, represented-clock policy and `OutputCollector`; it lands on every requested output. The public API does not accept an arbitrary RHS paired with declared structure. Its admitted model has carrier metadata zero, positive weights whose **exact represented** sum is one, finite parameters/state and forward time. A failed factor refresh invalidates both solve access and the Schur witness accessor. Successful refresh retains storage; all stage RHS/output/combination buffers are preallocated.

The baseline source is `c513f47` on the cloud-completion branch. Prior Woodbury WB1/WB2 derivations and first-stage binding are reused. This unit closes the missing all-stage/native adaptive integration. It does not change the default generic solver.

## All-stage equivalence theorem

Fix the existing represented RODAS5P tableau and an autonomous admitted collective model. Let `J0 = D + U C V` at the attempt's initial state, `V x = sum_j w_j x_j`, `U v = (v,...,v)`, and

\[
D_j=\begin{pmatrix}-\kappa&-\omega_j\\\omega_j&-\kappa\end{pmatrix},\quad
C=\chi\begin{pmatrix}-2uv&-(u^2+3v^2)\\3u^2+v^2&2uv\end{pmatrix},\quad
(u,v)=\sum_jw_jy_j,\quad \omega_j=\mathrm{width}\,\xi_j.
\]

Put `s=h gamma`, `B=I-sD`, `P=B^{-1}`, `H=V P U` and `S2=I-s H C`. Suppose `S2` is invertible. For each RHS `b`, compute

\[
z=Pb,\quad r=Vz,\quad q=S_2^{-1}r,\quad x=z+sPUCq.
\]

Then `Vx=r+sHCq=q`, so `(I-sJ0)x=Bz+sUCq-sUCVx=b`. This proves the frozen solve, including the order `H C` of the Schur product.

For every stage `i=0,...,7`, the new workspace forms exactly the native recurrence

\[
y_i=y+\sum_{j<i}\alpha_{ij}K_j,\qquad
r_i=h f(y_i)+h J_0\sum_{j<i}\Gamma_{ij}K_j,\qquad
(I-h\gamma J_0)K_i=r_i.
\]

No `f_t` term occurs because this target is autonomous. In particular the first RHS is `h f(y)`, with **no extra `h gamma` scaling**. The native endpoint and embedded vector are

\[
y_{n+1}=y+\sum_i b_iK_i,\qquad e=\sum_i\widetilde b_iK_i.
\]

Induction on `i` proves equality with the native dense sequential method in exact arithmetic: equal preceding stages imply equal stage states, RHS, unique solves, endpoint and embedded estimator. The implementation uses the identical cached coefficient snapshot. Floating-point operation order inside the factorization differs from dense LU, so equivalence is checked numerically rather than asserted bitwise.

The existing sufficient admission condition

\[
3s|\chi||m|^2<1+s\kappa
\]

provides WB2's weighted inverse bound. The code checks it outward and also excludes zero from the interval Schur determinant. A dynamic refusal becomes a linear-solve rejection in the adaptive wrapper; step reduction can recover without committing an output. This restriction is explicitly part of the supported structured solver domain.

## Complexity and storage

With eight stages fixed, setup, each analytic JVP, each Woodbury solve and each stage combination cost O(N); therefore an attempted complete step costs O(N), with O(N) retained storage. Compared with unstructured dense LU's O(N^3) setup and O(N^2) stage solves, this is a proved reduction of arithmetic/storage complexity for the admitted structure. The interval Schur/inverse checks are included: they also use a fixed number of interval operations per mode. This claim is independent of wall-clock speed.

`refactor` clears validity before validation and recomputation. Its weights, block inverses and source identity vectors reserve sufficient capacity before reuse. A successful fixed-dimension refactor contains no capacity growth after its first construction. A successful complete attempt contains no explicit heap allocation in its code path; diagnostic strings on failures may allocate. Allocation instrumentation must pass a cross-crate positive control before the empirical zero-allocation result can be accepted.

## Verification

`collective_rodas5p_contracts` checks all eight K stages, endpoints and estimators against a separately built dense Jacobian/direct sequential solver for 60 combinations: N=1,2,4,8,16; chi=-0.7,0,0.9; kappa=0,0.6; h=0.001,0.07. It also checks exact nonlinear one-mode flow and matching landed outputs against the existing transformed fast driver, invalid model/step refusal, zero-span configuration, transactional output and stale-stage invalidation. Existing six `production_audit_collective_direct` cases preserve frozen-matrix, identity, normalization and residual witness coverage.

The first new reference test failed because an in-place Jacobian callback alone is not the legacy sequential driver's explicit-Jacobian API. The independent test adapter was corrected to provide its explicit matrix callback; production code was unchanged. The log is retained.

Independent review found that the old scalar Schur accessor could return the last successful determinant after a newly introduced failed refresh. The accessor now returns `CoreResult<[f64;2]>` and rejects invalid factors; the regression exercises this exact path. Existing caller tests use `.unwrap()` only after successful factorization.

## Runnable local comparison

```sh
cargo test --locked --offline -p rodas5p-integrators --test collective_rodas5p_contracts --test production_audit_collective_direct
# Separate allocation authority: must pass the library-clone positive control.
cargo test --locked --offline -p rodas5p-integrators --test collective_rodas5p_allocation_contract
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example collective_structured_compare -- 32 5 > structured-32.jsonl
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example collective_structured_compare -- 128 5 > structured-128.jsonl
```

The runner uses a heterogeneous nonlinear collective model, 11 common requested outputs, identical atol/rtol and solver limits, setup-inclusive timing, alternating execution order, and both coarse/fine classical RK4 reference trajectories. Reference refinement is empirical evidence; no enclosure is inferred. Each arm/failure is retained as JSONL. The debug smoke run N=8 completed all six arms. Reference refinement max absolute difference was 2.1649348980190553e-15; achieved max errors decreased from approximately 2.16e-9 to 2.97e-12 over the three tolerance settings for both methods. Local optimized repetitions must compare **achieved error**, retain all failures and preserve environment identity before a speed decision.

## R/C loop record

R1–R3 reused the exact normalized rotating model, native tableau, WB1/WB2 premises and native first-stage convention. R4 selected K coordinates to preserve the existing recurrence directly. R5 proved all-stage equality by induction and O(N) cost. R6 used algebraic substitution and independent code review; R7 used native dense references and exact one-mode flow; R8–R9 accept all-stage implementation and conditional complexity; R10 hands off the executable comparison with local measured speed open.

C1–C4 mapped primitive-only baseline to complete stage/driver integration and shared output/controller. C5 implemented refresh, eight stages, adaptive wrapper and JSONL runner. C6–C8 ran targeted reference/refusal tests and debug smoke; an allocator calibration issue is separately tracked. C9 addressed stale Schur witness access. C10 retains exact commands, logs and scoped use decisions. No new no-go theorem is submitted.

## Final execution and allocator disposition

`evidence/structured_scoped_numerics.log` records **19 passing native tests**: five new structured tests (including the60-case all-stage grid), six existing frozen-direct regressions, one independent PR101 signed-flow test and seven final PR101 provider/driver tests. The allocation gate is a separate test target, `collective_rodas5p_allocation_contract`; it is not ignored or weakened.

The restored Rust runtime links dynamic `libstd`. Its global allocator instrumentation counted **zero** for an externally retained library `CollectiveModel::clone()` with three nonempty vectors. A fresh isolated target reproduced the failed positive control. `evidence/structured_allocation_gate.log` records the explicit failing gate. This prevents accepting empirical zero-allocation claims in this environment. The code-level capacity-reuse argument remains valid. The runner records `allocation_calibration_valid=false`, authoritative `allocation_calls=null` and `allocated_bytes=null`, and separately labels raw counter values. Local execution must resolve/calibrate its own runtime before accepting allocation evidence.

`evidence/structured_compare_smoke_calibrated.jsonl` is the final smoke receipt. All six completed arms passed their **empirical** max-grid WRMS accuracy gate, at the original scales `atol+rtol*abs(reference)`: the structured maxima were approximately0.02622,0.30651 and0.38037. The two-resolution refinement term is reported separately and included in the empirical gate upper value. This is a sampled reference comparison, not a global certificate. The earlier `structured_compare_smoke.jsonl` is retained as the pre-calibration diagnostic: its allocation fields are superseded by the final calibrated-format receipt.
