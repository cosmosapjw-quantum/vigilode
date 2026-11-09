# VigilODE adaptive portfolio router / switcher specification
## Selective activation, evidence-driven choice, and transactional switching

**Status:** design contract for the next research stage. This is not an implementation or a performance claim.

## 1. Core objective

The solver is not a pipeline that runs every available mathematical feature. It is a portfolio of mutually compatible, selectively activated methods and computational capabilities. The controller chooses only the smallest useful subset for the current IVP segment, observes its actual cost and accuracy, and changes the active choice only when admissibility and net-benefit conditions are met.

The optimization problem is lexicographic:

1. Satisfy the original ODE/source identity, requested outputs, physical/domain constraints, derivative permissions, method stability/history conditions, resource limits, and the applicable error contract.
2. Among admissible choices, minimize predicted **complete remaining IVP cost**, including setup, probes, startup, history transfer, monitoring, certification, failed trials, rejected steps, fallback, output and memory costs.
3. Keep the current method when the evidence for switching is weak, ties occur, or the expected gain does not exceed the switching reserve.

“Optimal” means best within the currently admitted candidate set, declared assumptions, observed evidence, and available budget. It does not mean clairvoyant global optimality over unknown future trajectories. No regret bound or global optimizer guarantee is claimed.

## 2. The active configuration is sparse, not the full Cartesian product

Represent the currently active route as

`ActiveChoice = (IVP method, stage/linear backend, optional structural transform,
execution lane, estimator/certificate policy)`.

The set of possible tuples is **not** their unconstrained Cartesian product. A compatibility registry declares which combinations are mathematically and operationally valid. At any time:

- exactly one committed IVP method owns the physical accepted state;
- that method uses one admissible stage/linear backend;
- zero or a small number of explicitly declared structural transforms are activated only when their source guards pass;
- execution choices (SIMD, independent ensemble, spatial rows, peer-stage tasks) are enabled only when this method's actual dependency graph permits them;
- unused observers, probes, charts, spectral analyses, Fourier/Volterra kernels and preconditioners are not run.

A component may be prepared lazily and cached, but setup/cache cost and epoch invalidation are charged. A method-specific feature does not become universally available merely because it exists in the codebase.

## 3. Five nested decision levels

### S0. Problem and permission admission

Read immutable declarations without calling user callbacks:

- source/code/model identity and epoch;
- autonomous/nonautonomous, mass/DAE and state-space contract;
- initial data, time span, output/events and requested norm;
- callback purity/replay-safety and side-effect policy;
- derivative policy: `ANALYTIC_J`, `EXACT_JVP`, `AD_JVP`, `FD_JVP`, `OPERATOR_ONLY`, `RHS_ONLY`;
- dimension, memory, thread and work ceilings.

Reject or defer a candidate before callback execution if its assumptions are not met. A derivative permission makes a candidate eligible; it never forces derivative use. A fatal callback error is not a signal to retry the same physical request using a different method.

### S1. Whole-IVP method selector

Select among distinct integrator identities, such as protected RODAS5P, DOPRI5/Lawson, BDF, Radau, EXPRB/phi-Krylov and qualified Peer-W/SEPP variants.

Each method declares its own tableau/equations, order, stability domain, history, startup, step-ratio restrictions, estimator/controller, dense output, event and restart semantics. Switching RODAS5P to BDF/Peer-W/EXPRB is a **whole method change**, not a backend substitution.

### S2. In-method stage/linear backend selector

Only after the IVP method is selected, choose among backends that preserve its method equations, e.g. dense/banded/sparse direct, GMRES/GCRO-DR, preconditioner, or a source-certified graph-Schur action on the same original W.

A graph transform may be used only after source-identity, algebraic guard, graph topology and original-W residual conditions pass. It cannot silently change the tableau or original stage target. RODAS5P's eight lower-triangular stages remain causally ordered.

### S3. Optional structural worker selector

Choose none, one, or a compatible set of source-specific workers:

- RHS-only homotopy / inverse-Picard corrector;
- polynomial/resolvent/Leja/Laguerre/Chebyshev phi-action;
- quotient/Lawson exact semigroup;
- Fourier or Volterra causal transform;
- invariant-manifold / weighted-observable / degree or perturbation-response certificate;
- graph path/core extraction or affine scan.

Each worker has a cheap eligibility guard, a source-bound certificate, a direct original-equation residual/back-map, an expected amortization horizon, and a fallback. If any condition fails, the worker is not run or its result is discarded. These workers are not all simultaneously applied.

### S4. Execution-lane selector

Choose only the concurrency that the current calculation exposes:

- operator/RHS/JVP/Krylov rows;
- independent IVP ensemble;
- spatial MOL cells/rows/modes;
- true independent time-stage tasks of the selected Peer method;
- SIMD/vectorized kernels.

Do not infer time-stage concurrency from SIMD, batched IVPs, common factorization reuse, or a parallel linear solve. The original RODAS5P stage DAG is still serial.

## 4. Adaptive observation hierarchy

The controller must avoid spending more to diagnose a regime than it can save by adapting.

**Tier 0: already-available telemetry.** Reuse accepted/rejected attempts, step proposals, embedded error ratios, solver work counters, true residuals, memory/allocations, elapsed time, previous candidate outcomes, and source/model epoch. No new derivative calls.

**Tier 1: cheap derived indicators.** Compute work per advanced physical time, rejection trend, step-size trend, observed damping/oscillation scale, stage/linear residual trend, graph descriptors already supplied by the source, and uncertainty in the current method's cost model.

**Tier 2: conditional probes.** Only when the current model is uncertain, has deteriorated, or a new regime is suspected, run a bounded observer or a short candidate pilot. Examples include numerical-abscissa/logarithmic-norm estimates, a small Krylov transient-gain probe, a sparse fill estimate, or same-checkpoint alternative-method replay. Never form a dense Jacobian or call JVP merely to diagnose if policy forbids it.

**Tier 3: expensive structure analysis.** CAD, chart construction, full pseudospectral work, Fourier decomposition, invariant-manifold discovery or large graph analysis are offline/rare setup operations. They are enabled only if the source family is declared and expected repeated use can amortize them.

An observer is itself a candidate computation with cost and error. Absence of an authorized indicator is recorded as `NOT_AVAILABLE`, not interpreted as evidence for or against stiffness.

## 5. Candidate cost and accuracy gate

For candidate j over a forecast horizon H, use a conservative total-cost model such as

`C_j(H) = C_setup,j + C_probe,j + C_transition,j
         + N_j(H) C_step,j + C_certificate,j
         + C_rejections,j + C_failed,j + C_fallback,j
         + C_output,j + C_memory,j.`

The terms can be deterministic work units, measured CPU time, or both, but the units must never be conflated. Maintain uncertainty intervals or calibrated prediction error for each term. Do not rank candidates by RHS count, accepted steps, ideal parallel span, or one kernel benchmark alone.

A candidate is selectable only if:
- its declared stability/step-ratio/source/domain conditions hold;
- required derivative and callback permissions are available;
- its original-target residual and error-control route is qualified for the task;
- its predicted error fits the remaining error budget (or it is explicitly in a non-promotable exploratory probe);
- its resource and remaining work ceilings are respected;
- its cost comparison includes the full probe/setup/transition/fallback overhead.

For a switch from current route c to candidate j, require a conservative lower confidence bound on the predicted saving to exceed a switching reserve:

`LCB(C_c(H) - C_j(H)) > C_switch_reserve.`

If uncertainty overlaps zero, the cost tie is not strict, or the candidate's expected residence horizon is too short to amortize setup, retain c. A configurable safety reserve should increase when evidence is sparse or candidate costs are highly variable.

A same-checkpoint probe must start from the same accepted `(t_n,y_n)`, target the same endpoint/output semantics, charge every callback/solve/attempt, and never overwrite the committed trajectory. A pilot that only compares local tolerances is not a same-global-error comparison. Global error needs a valid certificate or a separately validated source/method-specific calibration; equal `rtol` does not imply equal global error.

## 6. Finite-state supervisor and switching semantics

A proposed controller state is

`Q_n = (active_choice, recent_regime_word, residence_count,
        negative_evidence_age, source_epoch, method_history_epoch,
        workspace_epoch, error_budget_state, work_budget_state).`

Regime alphabet may include:

- `P`: admitted positive/path graph;
- `N`: nonnormal/transient-growth warning;
- `D`: strongly dissipative/stiff;
- `O`: weakly damped/high-frequency oscillatory;
- `S`: sparse irregular/high-fill risk;
- `R`: reentrant or regime-change indication;
- `U`: uncertainty/insufficient evidence;
- `X`: source/model epoch changed or a contract was invalidated.

The alphabet is a routing abstraction, not a physical ODE Markov partition. The accepted transition language is constrained by hysteresis, minimum residence/dwell, bounded probe frequency and source/history epochs. Regime symbols are observations, not proof of stability.

**Normal switch protocol**

1. Continue the committed method to an accepted checkpoint; never switch halfway through an uncommitted step.
2. Verify source identity and the target candidate's full capability contract.
3. Check history compatibility. If there is no exact history map, start the new method with its proper startup from the accepted physical state and charge the lost history/startup work.
4. Transfer or reconstruct the global-error budget, controller state, output/event state, invariant state and work ledger. Quantify any state/history transfer defect.
5. Run the new method under a bounded trial budget. Commit only an accepted, original-target-admissible result.
6. On recoverable candidate failure, discard its uncommitted physical state, retain the prior accepted state, charge all attempted work, and use the protected fallback.
7. On fatal callback/source errors, stop. Do not treat fatal errors as opportunities to try another callback or route.
8. Enter a dwell/backoff interval. Re-probe only on expiry, measured deterioration/relaxation, or a new accepted regime word.

Cache reuse is allowed only when method, step size/ratio, provider, source/model epoch, norm and workspace identities remain valid. A changed epoch invalidates affected caches and historical evidence.

**Emergency path** is separate from routine cost optimization: original-residual failure, nonfinite state, certificate failure, domain exit or method stability violation immediately rejects the candidate and invokes the registered safe fallback or stops. Hysteresis cannot delay safety handling.

## 7. Evidence-memory policy

Maintain per-candidate, per-regime evidence:
- source/model/epoch identity and problem feature envelope;
- exact method/backend/transform tuple and configuration hash;
- same-checkpoint probe interval and output/error contract;
- complete attempted work, time, allocation/peak RSS;
- accepted/rejected steps and actual committed states;
- original equation/stage residuals and error/reference provenance;
- reason for admission, selection, rejection, rollback or expiry;
- validity envelope, expiry/backoff, and adverse/reentrant counterexamples.

A losing result may be reused while its regime/source envelope remains valid; it must expire after a bounded age or when work/step/rejection indicators show deterioration or relaxation. Never generalize a losing probe outside the regime that produced it. Preserve adverse evidence permanently.

## 8. What already exists and what remains

The pinned C5 source already contains an opt-in foundation:

- `crates/vigilode/src/automatic.rs`: `AutomaticOde` / `solve_adaptive`; the current automatic comparison is primarily DOPRI5 versus an explicitly authorized RODAS route. Default derivative permission is forbidden; the code comments explicitly state it does not infer a chart, certify stiffness, or choose thread count.
- `crates/vigilode/src/switching.rs` and `crates/vigilode/SWITCHING.md`: bounded RHS-first switching at accepted windows; persistent RHS-only DOPRI5; optional Jacobian/JVP/FD-authorized implicit challenger; observed-work triggers, rechecks, hysteresis/backoff, common work budget and charged discarded probes.
- `crates/rodas5p-integrators/src/routing.rs`: declared-structure routing among RODAS backends; structure is caller-declared/verified, not inferred as a universal method portfolio.
- `LawsonDopri5`: a declared exact damped-rotation structure selected explicitly by the caller; automatic competition with black-box whole-RHS methods is not yet implemented.
- C8 R2/R4 defines a finite-state regime alphabet, nonnormal observer and source-graph candidate selector. C8 numerical validation remains PENDING.
- PR98 validates a guarded symbolic controller-state/ownership transfer in a scoped task. It is not physical ODE symbolic coding or a general numerical stability proof.

Therefore the **full multi-method, multi-backend, structure-aware adaptive portfolio is not yet implemented**. T01A integrates the design; T02/T03/T06 provide current authority and candidate qualification; T07 should extend the existing selector/switcher only after those gates. Do not rebuild a second switching framework, thread pool, WorkLedger, or benchmark system.

## 9. Historical evidence that governs the policy

The 2026-10-23 single-thread campaign measured 864 configurations. At the same endpoint error cap, rapid oscillation's JVP switching path was about 32.5 times slower than whole DOPRI5; switching-FD was also slower in that case. A smooth nonstiff-to-stiff transition showed a limited switching-FD improvement over whole DOPRI5, but remained slower than whole RODAS-FD. This rules out a pressure-only universal switch.

The later switching study retained positive and negative examples. It showed a useful fast-rotation case where an expensive JVP challenger was rejected with a bounded pilot, but a stiff-transition counterexample where early negative evidence failed to predict the later regime and the probe policy became about 3.05 times slower. This rules out permanently trusting one initial pilot. Regime-aware expiry/re-probing and complete accounting are mandatory.

These data are problem- and implementation-scoped, not evidence of full portfolio optimality. Same `rtol` and local embedded errors are not global-error equivalence.

## 10. Scientific claims and stopping rule

The following are separate claims and require separate evidence:
- **adaptive choice:** selector picks a valid candidate from a known portfolio;
- **economic benefit:** complete remaining-IVP cost improves at matched achieved global error;
- **stability under switching:** the actual accepted-step/history error-transfer maps satisfy a common/multiple norm/Lyapunov or admissible dwell-time bound;
- **global optimality:** requires a problem/horizon/candidate model and proof beyond the current plan; not claimed.

Stop expanding the portfolio if probes, router features, certificate work and transitions erase the full-IVP gain; if source/history transfer cannot be bounded; if a candidate's regime is not distinguishable with permitted observables; or if an accepted route cannot preserve the original requested output/error contract. In that case keep the simplest protected valid method and record the rejected feature combination.

## 11. Provenance and implementation anchors

Code source lock: `research/vigilode-c5-native-graph-schur-20261102` at `569ce02629498fec133db9ce368eb303ba943b69`.

Key evidence:
- `crates/vigilode/src/automatic.rs`, `crates/vigilode/src/switching.rs`, `crates/vigilode/SWITCHING.md`;
- `crates/rodas5p-integrators/src/routing.rs`;
- `research/adaptive_causal_20261023/analysis/SWITCHING_BENCHMARK_KO.md`;
- `research/production_frontier_20261023/analysis/SWITCHING.md`;
- branch `research/vigilode-dyn-symbolic-c8-20261102`: `R2_LITERATURE_AND_REUSE.md`, `R4_CANDIDATES_AND_ROUTER.md`, `RESEARCH_DAG.json`;
- PR98 branch `validation/dynamics-symbolic-20261028-f9615d`, scoped symbolic execution evidence.

No source behavior, default, tolerance, tableau, benchmark threshold or production status is changed by this design.
