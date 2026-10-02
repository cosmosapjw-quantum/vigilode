# Vigilode thread-transfer implementation plan

> **For agentic workers:** Execute one bounded task at a time. Every checkbox below is future native work, not a claim of completion.

**Goal:** Transfer verified certificate and coordinate ideas without changing the protected solver or promoting a failed method.

**Architecture:** Keep the existing RODAS5P target and native q2 admission as the default. Add an opt-in certificate-action experiment first; treat a static stage-coordinate adapter and a physically transformed ODE integrator as different interfaces and different claims.

**Tech Stack:** Rust workspace and its pinned toolchain; existing directed arithmetic and Rayon execution; Python/SymPy exact review oracle. The current review runtime did not run Rust.

## Global constraints

All paths are relative to the repository. Existing paths are marked observed; new paths/symbols are explicitly proposed. Do not change existing ledger rows, coefficient bytes, protected R-JF, holdouts or represented clocks. Register any new top-level research campaign before running it according to docs/RESEARCH_LEDGER.md. Timing authority remains HOLD until its independent requirements are met. Do not make a new branch for this user's work; use the existing branch and non-force publication.

Do not label a stage-target certificate as an ODE local/global error bound. Do not label a supplied spectral range or sampled model agreement as a proof. Every benchmark compares matched physical error and records failed runs as well as costs. No test oracle or current serial certificate may select the candidate used by the new path.

---

## Task N1: Action-first directed certificate, no admission change

**Observed files:** `crates/rodas5p-integrators/src/outward_certificate.rs`, `crates/rodas5p-integrators/src/stage_target.rs`, `crates/rodas5p-integrators/src/lib.rs`.

**Proposed files:** `crates/rodas5p-integrators/tests/thread_transfer_action_certificate.rs`; a preregistered research node only after native tests exist.

**Consumes:** `StageTarget`, `QuadraticStageProblem`, final candidate stages, `InverseWitness`, output/embedded projections, tolerances, caller-owned `ParallelExecution`.

**Produces, proposed:** `action_doubling_certificate_with_execution` with the same argument and result shape as `doubling_certificate_with_execution`. Default q2 routing is unchanged. A private `upper_power_sum_action` applies the finite polynomial to one RHS rather than materializing the sum matrix.

- [ ] Add tests on strict-lower matrices of sizes 1,2,3,8; compare against independent rational/interval fixture bounds, not merely another reordered floating implementation.
- [ ] Observe the new-test failure, then implement `e=a; P=H; e+=P*e; P=P*P` using existing upward operations and structural zeros. For 8 stages perform three vector updates and two squares. Reuse buffers where lifetimes permit.
- [ ] Reject a non-strict-lower target, negative majorant, nonfinite data, overflow or mismatched witness. Keep exact structural zero entries zero; do not manufacture tiny positive entries above the diagonal.
- [ ] Verify every returned bound contains the independently solved target output. Use same-component diagonal blocks first; keep unsupported structure rejection explicit. Do not assert bit identity with the old evaluation order.
- [ ] Count multiplies/adds, allocated slots, peak live slots and pool creations separately. Cross-worker results must be deterministic for workers 1,2,4,8 with fixed per-entry reduction order.
- [ ] Run `cargo test -p rodas5p-integrators --test thread_transfer_action_certificate`, then `cargo test --workspace --locked`. Require nonzero executed test count and all assertions passing. Commit only this independently verified deliverable.

**Stop condition:** Any loss of outward containment, invalid-structure admission or unresolved error inflation prevents admission. A smaller arithmetic count alone is not a timing success.

## Task N2: Residual-seeded radius as an explicit policy

**Observed files:** `crates/rodas5p-integrators/src/outward_certificate.rs`, `crates/rodas5p-cli/src/r4_studies.rs`.

**Proposed file:** `crates/rodas5p-integrators/tests/thread_transfer_radius_policy.rs`.

**Consumes:** H0, H1, a and alpha bounds belonging to the same final candidate. No reference root and no current-step serial bound.

**Produces, proposed:** an opt-in `RadiusPolicy::ResidualSeeded` result retaining the existing `RadiusAttempt` ledger plus `proposal_source`, `candidate_sha256`, `target_id` and actual proposal-work counters. Keep the old PastStepData policy available and unchanged.

- [ ] Write a failing test for the synthetic finite feasible interval `p(D)=29/100+(4/5)D^2`, which the old six-point schedule misses.
- [ ] Add the five R4 diagonal cases. Keep the existing n=8,16 failure as a regression of the old policy; do not rewrite its frozen result.
- [ ] Compute `E0=sum H0^k a` with N1 and `B=max |alpha|E0`. Propose the representable outward-rounded D=2B. Recompute the full native closure inequality at that D. Only this final check decides validity; the seed is not a proof.
- [ ] Handle B=0, representability failure, overflow, empty/tangential feasible intervals and a proposal above the upper feasible boundary. A failed proposal returns an explicit failure/fallback, not an infeasibility verdict. CAS/Sturm isolation remains an offline oracle, not an online dependency.
- [ ] Verify native directed closure on the two formerly capped cases and measure bound width versus the existing serial certificate. Keep target-distance admission separate from radius closure.
- [ ] Run `cargo test -p rodas5p-integrators --test thread_transfer_radius_policy` and the full locked workspace suite. Publish a new research-node result only under the repository's preregistration/ledger policy.

**Dependency:** N1 for the intended action implementation. **Stop condition:** A new policy closes a radius but exceeds the actual target-distance budget or loses on total work; do not activate it by default.

## Task N3: Structural witness pipeline and prepared admission context

**Observed files:** `crates/rodas5p-integrators/src/outward_certificate.rs`, `crates/rodas5p-integrators/src/transactional_q1_q2.rs` (`Q2CertificateSource::capability`, `certify_q2_candidate`), `crates/rodas5p-integrators/src/problem.rs`, `crates/rodas5p-integrators/src/stage_target.rs`.

**Proposed file:** `crates/rodas5p-integrators/tests/thread_transfer_structural_witness.rs`.

**Consumes:** A verified structural model, frozen step identity and witness constructor. **Produces, proposed:** internal diagonal bound action plus an immutable `PreparedQ2Certificate` containing the exact declared problem, witness, target identity and model binding for one attempt.

- [ ] Add allocation/counter tests showing the existing dense J/U floor; distinguish nonzeros from physically allocated f64 slots.
- [ ] Add a typed diagonal representation through the complete residual/witness path, not just H. Preserve general dense fallback. Access bounds through `entry` or `apply_upper`; only materialize legacy wire matrices outside the hot path when explicitly requested.
- [ ] Replace duplicate capability/certificate witness construction only in the new prepared-snapshot mode. General callback sources retain existing behavior unless they opt into immutable snapshot semantics. Keep all model, candidate, step, output and tolerance binding checks.
- [ ] Re-verify deserialized witnesses through the existing mathematical constructors. A matching hash, source name or old h is never sufficient. Reject changed h/J/gamma, changed dimensions, edited bounds and stale candidate data.
- [ ] Test memory and work accounting, unchanged semantics on accepted/rejected candidates, and loss of a witness constructor. Run `cargo test -p rodas5p-integrators --test thread_transfer_structural_witness`, then the locked workspace suite.
- [ ] Admit a new certificate path only after native bound tests and measured certificate cost on preregistered non-holdout data. A matrix-free stage solver with dense proof inputs is not an end-to-end matrix-free pipeline.

**Independent work:** representation design can proceed beside N1/N2; their optional operational bridge requires all three. **Stop condition:** semantic/serialization ambiguity or merely shifting dense storage into an uncounted buffer.

## Task P1: Fast-v3 small kernels and true banded cost

**Observed files:** `crates/rodas5p-integrators/src/rodas5p_fast.rs`, `crates/rodas5p-integrators/src/problem.rs`, `crates/rodas5p-cli/src/stiff_benchmark.rs`, committed fast-v2 evaluation.

**Proposed file:** `crates/rodas5p-integrators/tests/thread_transfer_fast_kernel.rs`.

**Consumes:** Original transformed coefficients and represented-clock/controller functions. **Produces:** a separate driver identifier and diagnostic operation attribution; v2 remains the comparison arm.

- [ ] Preregister dimension 2/3/8 fixed-kernel and general-kernel arms, cold/warm distinctions, compiler/flags/thread settings and matched-error comparisons. Keep L-0033 FAIL unchanged.
- [ ] Instrument coarse costs inside the inlined `run_arm` region: stage accumulation, callback dispatch, validation, copies, factor/solve, norm/controller and clock work. Avoid a per-operation timer that dominates the measured work.
- [ ] Implement stack/fixed-size buffers and static callbacks only as a separate specialization. Preserve coefficient summation order, rejection rollback, Jacobian reuse identity, finite checks, represented endpoint/cap semantics and partial output.
- [ ] For banded scaling, require an explicit structural provider and bound the pivot-search region under row swaps/fill. Dense J/W scans must be removed too before claiming O(n b^2) total complexity. Keep a dense fallback when structure is unavailable or invalid.
- [ ] Run `cargo test -p rodas5p-integrators --test thread_transfer_fast_kernel` and `cargo test --workspace --locked`; compare step decisions and trajectory parity before timing. Do not change tolerance or clocks to improve a number.
- [ ] Only then run the existing native comparison machinery under a newly registered node. Preserve all failed points; timing remains diagnostic while authority is HOLD.

**Independent of N1-N3:** certificate changes do not fix fast-v2, which is a separate driver. **Stop condition:** no small-problem gain, lost physical-error parity, a new correctness corner case, or a speed claim dependent on removing validation semantics.

## Task M1: Stage-coordinate candidate adapter before a new ODE method

**Observed files:** `crates/rodas5p-integrators/src/homotopy.rs`, `crates/rodas5p-integrators/src/block.rs`, `crates/rodas5p-integrators/src/stage_target.rs`, `crates/rodas5p-integrators/src/transactional_q1_q2.rs`.

**Proposed files:** `crates/rodas5p-integrators/src/stage_chart_candidate.rs`, `crates/rodas5p-integrators/tests/thread_transfer_stage_chart.rs`.

**Consumes:** An explicit regular bijection K=Psi(Z), its inverse/domain and JVP, and the original target R(K). **Produces:** candidate K, original-coordinate residual, chart status and complete extra-work counters. This adapter has no authority to accept a step.

- [ ] Write exact-root correspondence tests for the polynomial triangular chart in this review. Add invalid-domain and singular-chart rejections.
- [ ] Evaluate R(Psi(Z)) with the chain-rule action DR(DPsi v). Do not confuse static residual reparameterization with pushing forward the physical ODE. Keep the native coefficient convention and target leakage allowance explicit.
- [ ] Treat all chart/homotopy predictions as untrusted candidates until the original StageTarget certificate/output budget has been evaluated on the restored K.
- [ ] Include the RVJ approximate-W and embedded-blindness fixtures as safety tests, not as assertions against RODAS5P. Never relabel an endpoint predictor as eight native stages without a documented adapter.
- [ ] Run `cargo test -p rodas5p-integrators --test thread_transfer_stage_chart`, then the full locked workspace suite.

**Separate future method:** An ODE Darboux chart such as w=y/x^2-1/kappa changes the discrete map. It requires a new driver id, a generated-model proof, physical error transport, dense output and nonzero fast-initial-data tests. It is not enabled by M1.

## Task V1: Nonnormal/output-error contract and certified polynomial usefulness

**Observed files:** `crates/rodas5p-core/src/polynomial_action.rs`, `crates/rodas5p-core/src/transform_bound.rs`, and the exponential/Krylov reference and contract tests selected from the current tree before editing.

**Proposed file:** `crates/rodas5p-core/tests/thread_transfer_output_bound.rs`.

- [ ] Keep symmetric nonpositive, verified versus declared spectrum and EstimateOnly versus Certified classifications unchanged.
- [ ] Build generated dissipative-metric examples with physical norm equivalence explicitly bounded; add the existing nonnormal Arnoldi residual counterexample and the thread's internal-amplification example under separate method ids.
- [ ] Any new recurrence Green-kernel/metric bound must include coefficient, recurrence, summation and norm-transport errors. A scalar residual history alone cannot become a total output certificate.
- [ ] Derive measured usefulness and cost gates before testing degree/scale choices. Do not tune on holdouts, and do not replace an unusably large certified bound by an estimate while preserving its label.
- [ ] Run `cargo test -p rodas5p-core --test thread_transfer_output_bound`, then the locked workspace suite. Keep unsupported cases typed and explicit.

## Decisions intentionally not made

No automatic backend routing, new default radius policy, deprecation of v2, general RVJ activation, timing-authority promotion or holdout release is authorized by this review. Such changes require the native evidence above and their own recorded decision. Continue on the user's existing branch; do not create a branch or PR solely to execute this plan.
