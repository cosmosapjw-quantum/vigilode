# Wave-3 plan summary — docs/plan/IMPLEMENTATION_WAVE3.md
Base: production/wave2-integration after production/w2-final-fix (+ h-checker) merges (Task 0, controller placeholder).
Stage 1 (parallel): T1 e1b-dense (cloud harness 1a), T2 b1c-staleness, T3 p0-fastdense, T4 p0-atol, T5 p0-events, T6 ci-docs. Stage 2: T7 p0-api. T1b (E1 run) and the full suites run on the workstation via LOCAL_*_W3.md handoffs.
Ledger ids: L-0141 = b1c-staleness (PASS/FAIL), L-0142 = e1b-dense (INCONCLUSIVE, supersedes L-0140, the head of the L-0139 chain).
T1: 4 facade dense-output arms (WFilteredEnforce/Off × dense/banded) via Rodas5p::solve + at(t); output_clipped_steps must be 0; fixes stale arms.rs:151 note (minor 6); measured after T3/T4/T5/T7 merge.
T2: B1c staleness bound: q_pred = (|1-h_P/h| + h_P γ ||J-J_P||_S)/(1 - h_P γ mu_S(J_P)) in the weighted max-norm; refresh iff q_pred > 0.5; stop when ||P^-1 r||_S <= (1-q_pred) rho_i, which gives WRMS(W^-1 r) <= rho_i.
T2 kill gate unchanged from B1/B1b (fact/acc <= 0.4, steps <= 1.05x, error <= 1.5x; HIRES + Brusselator n=100, rtol 1e-6). No Tier::Performance routing in wave 3 under any verdict.
A2 decision: retune parked (gate is set against Off at 321 tol units; a kappa=2 safety factor would be tuned on the failure case); WFilteredEnforce stays the facade dense-output default as the P0 contract; E1b collects the corpus evidence.
T3: dense output moves to the fast dense/banded drivers (W-filtered estimate = W^-1((h/2)d)/(h γ) with the step's own LU); facade dense Jacobian switched to with_jacobian_into; stats schema v3. Mass/Enforce stay sequential.
T4: vector atol on the fast dense/banded routes via new *_with_atol entry points; MF/sequential/declared-band routes stay Unsupported (fast dense output joins in integration).
T5: events (Direction, event/terminal_event, EventRecord), Illinois root finding over accepted steps, post-processing; requires dense_output.
T6: .github/workflows/production.yml (fmt, clippy, tests, docs, examples, informational semver, bench) on pinned 1.94.1; four examples.
T7: #[non_exhaustive] on facade enums and output structs (not Options/Atol); typed SolveError::Unsupported(Unsupported).
Tier/time: T2,T3 most capable (8-10 h, 12-16 h); T1,T4,T5,T7 standard (3-6 h); T6 cheap (2-3 h).
Deferred to wave 4: A3 controller, A4 small-N/sparse routes, B2 JVP tier, vector atol on sequential, in-loop event stop, facade-owned Stats types.
