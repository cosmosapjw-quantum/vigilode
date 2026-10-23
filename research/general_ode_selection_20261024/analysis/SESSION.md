# Persistent explicit integration: operational contract and verification

## Selected change

The public DOPRI5 interface now exposes a scoped persistent session. A caller can request observations or perform a bounded amount of integration work, inspect the last accepted state, and resume without reconstructing the native workspace. The full output schedule is registered once. The adaptive controller, FSAL derivative, work accounting and total attempt budget remain attached to the computation.

The change adds `crates/vigilode/src/session.rs`, its contract tests and a serial benchmark. It reuses the native `Dopri5Session` added before this work and improves registered-target lookup in the native explicit and implicit sessions. The numerical tableau and controller are unchanged. The source baseline is `057fecf0698fa262d881f101625e6b0bb93791cf`; prior native numerical-transition tests are reused, with one new lookup regression test.

## Public interface

```rust
Dopri5::with_session(problem, span, initial, options, times, epochs, |session| {
    session.advance_to(target, attempt_quota, epochs)?;
    let accepted = session.checkpoint();
    // Inspect accepted.t, accepted.y, proposed_h and cumulative work.
    Ok::<_, ExplicitSolveError>(())
}) // Result<(driver_return_value, ExplicitSolution), ExplicitSolveError>
```

`ExplicitSession` is borrowed inside the driver closure. This prevents a self-referential owner of a callback and a native session borrowing that callback, and requires neither unsafe code nor a callback object allocated solely to resolve that ownership problem. The session cannot escape the closure. Returning early ends the computation and returns its accepted partial solution; it does not create a serializable/restartable session.

`SessionAdvance` has four results: `Yield` after using the per-call trial quota, `Checkpoint` at a requested observation, `Complete` at the final time, and `Stopped` after an integration failure or exhaustion of the shared attempt budget. The last two are terminal. The quota counts attempted intervals, including rejected attempts. A rejected attempt exposes the previous accepted state and the additional work it actually used.

`SessionEpochs` records caller-declared versions of the model, provider, norm and tolerances. Every call checks them against the original versions before further numerical work. Declaring a change refuses continuation, and the caller can construct a new session from its accepted state. Epochs cannot detect concealed changes to an interior-mutable model, time-dependent external data, or nondeterministic callbacks.

This API supports forward identity-mass nonstiff ODEs with scalar or componentwise state-WRMS tolerances and scheduled step landing. It refuses mass matrices, requested dense output and declared events before invoking either the driver or RHS. It never invokes Jacobian or explicit time-derivative providers. The schedule and tolerance configuration are immutable during the scoped computation.

## Numerical argument and cost

### Pausing preserves the trial sequence

Assume a deterministic RHS, unchanged floating-point environment, fixed model/tolerance/norm bindings, the same initial data and full landing schedule, and the native controller/state transition implementation used by the one-shot driver. Let the native state after trial `j` contain the accepted `(t,y)`, FSAL cache, proposed step, controller memory, output cursor, work counters and global attempt count. A quota boundary performs no numerical transition: it returns this state without changing any component. A resumed call computes the next trial from the same state as uninterrupted execution. Induction on the trial count therefore gives the same floating-point trial inputs, accept/reject decisions, observation states and cumulative work until termination. A target is restricted to registered observations so that requesting it cannot introduce a new step-clipping point.

The façade adds a finite-output diagnostic scan, callback counters and a read-only accepted checkpoint. None changes the arithmetic used by a successful native stage. Consequently, under the assumptions above, facade session yields preserve the one-shot native path. This is an operational equivalence argument, not a new convergence theorem. Existing DOPRI5 error/stability assumptions remain necessary.

### Construction and FSAL savings

For `A > 0` fully evaluated DOPRI5 trials with FSAL enabled, no callback failures or arithmetic overflow, and no binding change, the implementation evaluates the initial derivative once and six further stages per trial:

\[
N_f^{\mathrm{persistent}} = 6A_p + 1.
\]

The derivative at the base remains valid after a local-error rejection, so ordinary embedded-error rejections are included in `A_p`. If the caller instead constructs `K` nonempty independent runs at the same observations, with `A_r` total fully evaluated trials, then

\[
N_f^{\mathrm{restart}} = 6A_r + K,\qquad
N_f^{\mathrm{restart}}-N_f^{\mathrm{persistent}}
=6(A_r-A_p)+(K-1).
\]

For equal trial counts this removes exactly `K-1` RHS evaluations and `K-1` repeated solver constructions. If persistent controller memory also avoids initial-step regrowth, its reduced `A_p` provides the larger saving. The formula deliberately retains both trial counts: restarting can alter rejection behavior, so a universal wall-time speedup does not follow merely from persistence.

The active step workspace remains linear in state dimension. Retained observation storage remains proportional to the number of stored states times dimension; the public checkpoint itself borrows state and counters and allocates nothing. Neither output collection nor the overall solve is advertised as allocation-free.

### Logarithmic validation of registered observations

The existing native sessions checked target membership by scanning the full sorted observation array. Using one `advance_to` call per successive observation in an `M`-point schedule therefore required a total of `O(M^2)` membership comparisons, independently of the numerical work. Both native explicit and implicit sessions now use binary search in the already validated finite, strictly increasing schedule. Each comparison removes at least half of the remaining candidate range; membership requires `O(log M)` comparisons, and `M` deliveries require `O(M log M)` membership work. More generally, `C` active calls including quota-induced resumes reduce membership cost from `O(CM)` to `O(C log M)`. The change allocates no additional storage and does not change step selection or observation values.

IEEE numeric equality is preserved by `partial_cmp` after the finite-target guard: a target `-0.0` still matches a scheduled `+0.0` and conversely. `total_cmp` would distinguish their bit patterns and would change this contract. The new native regression test checks both signs, both solver sessions, a 1025-observation array, nonfinite/missing/out-of-span targets and the absence of numerical work on zero-quota/refused calls. This complexity improvement concerns target validation; integration and storing the actual `M` states retain their own costs.

### Accepted-state and failure invariant

The native driver writes trial endpoints into a separate buffer. It copies that endpoint into the public physical state only after embedded-error and output acceptance succeed. Rejected, failed and malformed trials therefore leave the accepted `(t,y)` unchanged. Their attempts and callbacks stay counted. A fatal or malformed RHS records one diagnostic and sets the native terminal stop; subsequent calls do not retry it. On early driver return or failure between scheduled observations, the façade appends the last accepted state to the public output if it is not already present.

The partial solution sets `success=false` unless the final time was reached. A deliberate early pause has no numerical failure reason; `ensure_success()` still rejects it and retains the entire partial solution. This separates a complete trajectory from a usable but incomplete checkpoint.

## Verification and benchmark package

The new `production_session_contracts` tests cover the following public boundaries while reusing prior native-controller tests:

1. Exact trajectory/work agreement with one-shot observation delivery across quota-one pauses, rejected steps, nonautonomous RHS and vector tolerances; derivative callbacks deliberately panic if entered.
2. Zero-work quotas and invalid target/epoch refusal without state, controller-proposal or work changes.
3. Partial output at an unscheduled accepted checkpoint and preservation through `ensure_success()` failure.
4. Sticky fatal callback and malformed-output stops after already accepted progress.
5. Recoverable-error accounting and recovery across a pause.
6. Unsupported features refused before both driver and RHS.
7. A global attempt budget shared across calls and zero-span completion without RHS work.

`frontier_session_contracts::sorted_target_lookup_preserves_signed_zero_and_refuses_nonfinite_or_missing_targets` separately covers the shared native target-lookup optimization.

The executable `production_session_bench` compares native one-shot, facade one-shot, persistent per-observation delivery and restarting the facade for each observation. Each method uses the same 65 step-landing times, tolerances, initial conditions and exact reference. The cases are scalar decay, a 32-component diagonal decay system and a two-component rotation of frequency 3. Every measured run is serial. Native, facade and persistent variants must return identical observation values and work; every variant must satisfy maximum absolute observation error below `1e-7`. Each mode is calibrated for at least 3 ms and then grouped to target 5 ms per timing sample. Mode order rotates cyclically between samples. JSON contains raw grouped timings, normalized per-solve timings, median/minimum/maximum, callback counts, attempts, rejected steps and actual reference error.

Commands, from the repository root in the prepared offline build environment:

```sh
cargo test -p vigilode --locked --offline --test production_session_contracts
cargo run -p vigilode --locked --offline --release --example production_session_bench -- 31
cargo run -p vigilode --locked --offline --release --example production_session_bench -- 31 1024
```

The focused tests were executed with the recovered Rust 1.94.1 offline environment: all 9 session tests and all 6 production-selection tests passed. The exact command and output are represented by `../evidence/tests/session-rerun.stdout.log`, `session-rerun.stderr.log` and `session-rerun.exit` (exit 0). The previous combined compilation failure is retained in `session-initial.*`; it exposed five uses of the incorrect `.autonomous(true)` builder spelling in the independent selector tests, which were corrected to `.autonomous()`. The scoped callback trait-object coercion was also corrected before this successful run. These were implementation fixes, not numerical failures.

The benchmark accepts a second argument for observation intervals, defaulting to 64; 1024 intervals exercise a larger delivery schedule over the same `[0,4]` span. It was prepared for the parent's serialized release run; its eventual execution receipt determines performance status. The subsequent native binary-search change requires the parent's final native and facade regression run. No benchmark speedup is claimed here before those measurements. The three cases isolate observation-restart overhead and do not substitute for the broad stiff/nonstiff work-precision campaign.

## Remaining integration work

The accepted-checkpoint API lets a caller examine state after each attempt and decide to stop. Continuous event localization still requires access to a validated within-step interpolant and a terminal event policy inside the integration loop. The existing implicit event code processes a completed trajectory; its later RHS work is not eliminated by this explicit session change. Those interfaces must not be conflated.

Native RODAS-fast persistence remains available at the native layer. A production implicit facade must preserve its Jacobian/provider guards, finite-difference work, typed failure details, requested solver options and method-selection policy across every advance before being exposed with the same ergonomic interface. It is retained as a concrete next integration task rather than silently routing a stiff problem through DOPRI5.

A subsequent switching implementation can use these checkpoints to change method only after accepted states. Switching must construct a fresh method session, invalidate the old method's derivative/cache history and charge setup work. A persistent session should not be reused across an unannounced model, dimension or norm change.

## Research-loop trace

R1–R3 and C1–C2 identified repeated façade construction as the integration gap and reused the unchanged native session/test basis. R4/C3 selected the scoped wrapper over unsafe/self-referential ownership, artificial output restarts and premature event support. R5 derived trial-path equivalence and the exact callback-count difference under explicit assumptions. R6/C9 were independently reviewed by the `session_review` agent using the native implementation, public wrapper and tests; it found the scoped lifetime/state/failure/budget argument and cost formula sound under the stated assumptions. Its timing-order criticism was addressed with cyclic interleaving and grouped samples. Its paused-result documentation correction was passed to the parent for `ExplicitSolution::stop_reason`. A follow-up review confirmed finite/signed-zero semantics of the new binary search and clarified its complexity for `C` quota-limited calls. R7/C6–C8 include the successful focused test run and prepared equal-observation benchmark, with final performance status supplied by the benchmark receipt. R8–R10/C10 adopt the scoped explicit interface and retain explicit remaining obligations for implicit sessions and continuous events. This work makes no new impossibility or no-go claim.
