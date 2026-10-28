# VigilODE Production Solver Development Plan

\-Oct 20, 2026 · @Jiwon Park

A development plan for turning VigilODE (branch `research/cumulative-evidence-20261020`, HEAD `ebb5b81`) into a production stiff/oscillatory ODE solver while keeping its own ideas — causal budgeted candidates, certificate-first error control, structure-exploiting linear algebra — as the organizing principle rather than an afterthought.

## 1. Verdict and the roadmap on one page

VigilODE today is a research platform with one production-grade core inside it: the protected RODAS5P drivers (`rodas5p_fast`, banded, small-N) with the official 5(4) tableau, the degree-4 continuous extension, one preallocated workspace per integration (per-attempt allocations 0.02–0.1x of the sequential path in SPD04, not yet allocator-certified as zero) and a disciplined adaptive controller. Everything else in the 220k-line workspace — matrix-free Krylov stages, exponential-Rosenbrock candidates, Laguerre/Leja/Taylor phi actions, packets, homotopy, coupled all-at-once stages, hybrid structured AD, certificates — is research, and the repository's own 2026-10-18 audit says so (`research/production_audit_20261018/DECISION.json`: scoped use of the generic path, HOLD on every headline claim).

The plan therefore has one organizing decision: **build a single production crate around the protected path, and make the research ideas enter it only through the causal-budget mechanism the project already designed** — a candidate step is speculative work with a counted budget, a certificate or a matched-accuracy receipt, and a guaranteed fallback. That mechanism, not any individual candidate, is what makes VigilODE different from CVODE, Radau5, DVODE or Diffsol, and it is the thing to finish first.

Five numbers fix the starting point. In the strict three-solver comparison at matched achieved error (PR #71, `research/strict_solver_comparison_20261019/FINAL_REVIEW_KO.md`), the protected `fast` path beat Diffsol BDF (dense) and Netlib DVODE by 1.4x–3.5x on Robertson, HIRES and van der Pol, but on Brusselator N = 400 DVODE's banded BDF was 2.1x–2.2x faster because VigilODE formed 100 Jacobians and 108 factorizations for 100 accepted steps while DVODE formed 5 and 31. On the band-plus-rank-one hybrid model (N = 64, `research/hybrid_20261020/RECOVERY_REVIEW_KO.md` §9) the dense `fast` path took 0.96 ms against 0.54 ms for Diffsol and 0.46 ms for DVODE at comparable error, and the earlier 1.6x–7.3x headline claims against Diffsol were retracted by the audit. Interior (dense-output) error on Prothero–Robinson, λ = −10⁵, rtol 1e-6, is 312 tolerance units with the default dense control (`docs/DENSE_OUTPUT_ERROR.md`). The coupled parallel stage solver reaches a real 2.08x speedup from 1 to 4 workers at n = 12288 but is not faster than the sequential matrix-free driver on most cells (`research/coupled_native_20261020/CRITICAL_REVIEW_KO.md`). The hybrid structured local-AD driver is 11.4x faster than the eager coupled driver inside the certified trajectory loop, yet still 7–8x slower than external BDF on the same model because certificate, curve and proof-materialization costs were never separated from the endpoint solver. The first number says the method and the implementation are competitive; the second says the missing production feature is Jacobian and factorization reuse, which section 5 shows can be added without losing order.

### What to build, in order

| Phase | Weeks (one developer plus agents) | Deliverable | Gate that ends the phase |
| --- | --- | --- | --- |
| A. Consolidate | 0–6 | One crate `vigilode` with a frozen public API; protected RODAS5P (dense, banded, sparse, small-N); W-filtered interior error control; vector tolerances; mass matrix; events; research crates moved behind `research` features or into a second workspace | G-A: standard stiff corpus passes at rtol 1e-3…1e-10 with interior error ≤ 2 tolerance units; zero hot-loop allocation measured by allocator; `cargo semver-checks` clean |
| B. Performance core | 4–16 | Stage-root step with lagged W factorization (Jacobian and LU reuse without order loss, result T1); lagged-LU-preconditioned matrix-free stages with GCRO-DR (T2); structured routing (band, sparse, band + low rank); batched small-N lanes | G-B: matched-accuracy wall time within 1.5x of the best of CVODE/Radau5/Diffsol on the corpus median and better on at least half the problems; matrix-free path converges on the stiff PDE corpus with a lagged preconditioner |
| C. Polyalgorithm | 12–26 | Causal transaction with two candidates: exponential-Rosenbrock (Krylov phi) for the oscillatory regime and structured/packet for declared exact-closure models; cost-aware admission (T8); protected fallback; regime telemetry as the only admission input | G-C: never more than 10% slower than the protected path on the corpus median; at least 2x faster on the oscillatory holdout at the same achieved error; failed candidates fully charged |
| D. Certified mode | 8–30 (parallel to B, C) | Opt-in enclosure for generated polynomial/rational models: continuous defect + two-state dissipativity (T4), inherited radii, Bernstein bounds, directed rounding; the audit's A01–A05 items | G-D: every returned center enclosed by its radius against 90-digit references on the model class, `roundoff_enclosed = true` |
| E. Release | 26–34 | Public benchmark report generated from one validated JSON; API docs; 1.0 | G-E: three independent external comparators, all arms including failures preserved, allocator-measured work, holdout corpus never used for tuning |

The research–exploration loop (section 8) runs alongside from week 0 with a work-in-progress limit of three open research nodes and a four-gate promotion path from exact-rational toy to holdout; it replaces the 130-node sprawl with a cadence that measures before it writes.

Two things the plan deliberately does not do. It does not promote the polynomial phi actions (Laguerre, Leja, Chebyshev, Taylor) beyond a structured backend for operators with a verified spectral enclosure, because the project's own evidence shows their certificates are not yet closed in binary64 and their gains are degree reductions, not measured wall time. And it does not treat stage parallelism as a goal in itself: the measured crossover (n ≈ 10^4 with cheap callbacks) says the shared-subspace coupled solver is a candidate for expensive-callback or large problems, admitted by cost, not a default.

## 2. Where the repository stands on 2026-10-20

`origin/main` stopped at the PR #19 merge on 2026-08-26 (`8d0c791`); every result since lives on research branches. The latest line is `research/cumulative-evidence-20261020` (PR #78, 655 commits ahead of main, HEAD `ebb5b81` at 21:54 KST today), which contains PR #74 (native coupled step), #75 (residual-first economy) and #77 (hybrid structured AD) as ancestors, while PR #71 (strict VODE/Diffsol/VigilODE comparison), #73 (banded exact-LU cache) and #76 (same-host midcheck) sit on sibling branches. All of #71 and #73–#78 are Draft and unmerged. The workspace holds seven crates and 220k lines of Rust (integrators 128k, core 37k, fair-ab 19k, cli 16k, krylov 14k, coupled 3k, hybrid 2k), 131 research-node directories, a 136-row append-only ledger (87 PASS, 44 FAIL, 5 INCONCLUSIVE, 13 superseded) and 54 public `integrate_*` entry points.

### What is production-grade now

| Component | Evidence | Status |
| --- | --- | --- |
| RODAS5P tableau and continuous extension | 17/17 order conditions verified (audit E-07); official degree-4 dense coefficients in `fixtures/rodas5p_coefficients_snapshot.json`; coefficients parsed once per process | Correct |
| `rodas5p_fast` dense driver (transformed stages, in-place LU, one workspace) | Instructions per attempt 0.12x–0.26x of the sequential path with errors within 1.001x (L-0032); malloc/free had been 37–52% of the sequential path (L-0030/31) | Production candidate, opt-in |
| Banded driver (`rodas5p_fast_banded`, slices kernel) | 0.255x instructions at n = 400, cost slope ≈ 1.0 in n up to 4096 (L-0054, L-0083); ME01 marks it a gain in every unit | Production candidate for declared bands |
| Small-N static driver (`rodas5p_fast_small`) | 2,348 instructions per attempt on van der Pol against Hairer RODAS 2,947 (cross-binary, L-0081/L-0085) | Production candidate; warm allocations still a loss (ME01 E5) |
| Adaptive controller (`ControllerKind::Predictive`, clipped variants) | CT01: van der Pol frontier 0.80–0.86 of the legacy controller's cost, rejections 0.283x (L-0102/103); ME01 E2 counts it an instruction loss | Usable; calibration open |
| Dense-output error control (`DenseErrorControl::Report/Enforce`) | Enforce brings Prothero–Robinson interior error from 312 to 1.05 tolerance units at rtol 1e-6, at one Jacobian and LU per sampled step | Correct but too expensive to be the default |
| Fair-A/B harness, ledger, holdout hygiene, comparator fidelity tags | 598–1,074 tests passing at successive audits; timing authority never admitted (all registry entries `Hold`) | Mature process, not product |

### Measured performance against external codes

| Setting | Result | Source |
| --- | --- | --- |
| Matched achieved error, isolated protocol, 12 paired blocks | `fast` 2.86x faster than Diffsol BDF and 2.65x faster than DVODE on Robertson endpoint; 3.48x faster than Diffsol on van der Pol dense grid; 1.43x faster than DVODE on HIRES | PR #71 `FINAL_REVIEW_KO.md` §4 |
| Same, Brusselator N = 400 | DVODE banded 2.19x (endpoint) and 2.08x (dense grid) faster than VigilODE banded; VigilODE 100 Jacobians, 108 LU, 864 solves, 856 RHS against DVODE 5, 31, —, 313 | PR #71 §5 |
| Native Hairer codes against the sequential path (before `fast`) | RADAU5 0.04–0.25x and RODAS 0.04–0.21x of RODAS5P wall time; per step 8–11 µs vs 0.5–1.2 µs at n ≤ 8; CVODE 0.27–1.24x | L-0029 |
| Same, against `fast` v1 and v2 | Hairer 0.27–1.09x of `fast` v1; on Brusselator Hairer takes 1.03–4.0x of v2's time, on small problems 0.41–1.9x | L-0032, L-0033 |
| A07 Robertson, rtol 1e-8, atol 1e-11, 101 outputs | Native max-grid WRMS 0.34 (pass) vs Diffsol 3.30 (fail) at the same settings; allocations 443 vs 1,039 | `production_audit_20261018_followup` |
| Hybrid reflecting-grid model, N = 64, same achieved error | Diffsol dense 0.54 ms, DVODE 0.46 ms, VigilODE `fast` dense 0.96 ms, certified structured auto 3.93 ms | `hybrid_20261020/RECOVERY_REVIEW_KO.md` §9 |

The picture is consistent: at small n the 8-stage linearly implicit step with a fresh LU wins on step count and on linear algebra; at n ≥ 100 with a banded or sparse Jacobian it loses to BDF codes that reuse one factorization for tens of steps. The earlier SciPy-wrapped VODE numbers (113,428 steps on Robertson) were an upstream SciPy 1.17 dense-Jacobian layout bug and are withdrawn (PR #71 §6).

### Accuracy findings that constrain the design

- Interior error: with dense control Off, Prothero–Robinson λ = −10⁵ reaches 3.2, 312 and 209 tolerance units at rtol 1e-4, 1e-6, 1e-8; the nonlinear witness at rtol 1e-8 reaches 1,801 (`docs/DENSE_OUTPUT_ERROR.md`, `DEFAULT_PATH_KO.md` DP-01). The embedded estimate controls the endpoint only.
- Global error: on semilinear advection–diffusion families the global error exceeds the case tolerance by 17, 53 and 207 units while the sums of true local errors are 12–30x smaller, so the excess is propagation by the flow; the VigilODE/SciPy-Radau error ratio at matched tolerance has median 5.28 (F-033). The repository records this as an owner decision not yet taken.
- Inexact stage solves: audit experiment E-04 found that only direct LU and an inner tolerance of 1e-12 keep the observed order slope ≥ 4.5; stiff order on Prothero–Robinson-type problems is 3–4 (E-09). This is the empirical face of result T1: residuals must be tied to the tolerance through the stage sensitivities, not fixed.
- Krylov recycling: GCRO-DR with the broken `M⁻¹AU = C` identity failed 21/336 solves; the refresh removes the failures and the saving (1.08–1.32x cold); zero starts were cheapest in 14/14 warm-start cases (SPD07) and zero-start recycling was 1.7–2.2x worse on non-normal cells (SP04). Recycling across steps is not a free win here.

### Research outcomes in one table

| Direction | Best recorded result | Verdict in the ledger |
| --- | --- | --- |
| Polynomial phi actions | Warm Chebyshev 3.17x \[1.17, 13.4\] over Arnoldi fused phi on five symmetric operators (L-0009); Laguerre cheaper in 0/79 and 0/32 cases; Leja `EstimateOnly`, infinite certified cost at hρ ≥ 40 | Chebyshev only, frozen symmetric operators, under statistical hold |
| Homotopy / stage-parallel | HOM-06 speed 0.17x (P = 1) to 0.04x (P = 8) with 5x RHS; serial certificate always cheapest; HT01 found no expensive client (callback share 0.011–0.108) | FAIL / ABSTAIN |
| Coupled all-at-once stages (PR #74/#75) | 2.08x worker scaling at n = 12288; residual-first 1.41x bit-identical; sequential matrix-free still equal or faster; 20/80 cells pass the embedded time test | COMPLETE\_BOUNDED; no trajectory |
| Packets LP01–LP10 | Online JVP/LU = 0 by structure; certificate not roundoff-enclosed; LP09 benchmark returned unconditional PASS with hard-coded zero allocations | LP10 promotion superseded; HOLD |
| Hybrid structured AD (PR #77) | 11.4x over eager full-AD under the same certificate; structured 23.6x over sequential; local AD alone 0.9986 | COMPLETED\_BOUNDED; one generated model |
| Algorithmic directions ALG01–06 | Coupled stage target geometric mean 0.74 but 4.18x error on `vig1b-k20`; budget guard 0.418x JVPs but not robust (median 2.39x under h₀ perturbation) | FAIL except ALG06-by-rule |
| zeta34 switching policy v3.1–v3.7 | 0 unsafe recommendations; shadow overhead 0.98% of JVPs; one positive event only | Not discriminating; holdouts N = 320/384 consumed |
| Derivative-free generative models | 1,600-variable stencil reduced to a scalar nonlinear state with 0 JVPs; exact scalar certified run radius 8.9e-7 vs actual 1.5e-7 | Python only |

### Process state

Twenty nodes have no ledger row, including every node of 2026-10-11 and 2026-10-19/20; the top-level `DAG.json` and `CONTINUATION_DAG.json` disagree on A02, A03, A05 and A10; the component-triage preregistration names 142 components while its manifest has 152 and its 56k-cell main campaign has not run; single-case timing designs cover 0.78–0.93 against a 0.94 threshold, so no timing decision is admissible until five-case designs (coverage ≥ 0.99) are used; the N = 320 and N = 384 holdouts are consumed and Oregonator is still sealed. The process is rigorous about claims and weak about throughput — the opposite of most research codes, and the reason the plan spends its first phase on consolidation rather than on a new mechanism.

### Sibling branches that the latest branch does not contain

The cumulative branch indexes PR #71, #73 and #76 by SHA but does not carry their evidence directories, and it never mentions `research/all-dag-completion-20261018`. What those branches hold matters for the plan:

| Branch (PR) | What it holds | Consequence |
| --- | --- | --- |
| `research/benchmark-results-20261019-dual` (#71) | The strict matched-error VODE/Diffsol/VigilODE comparison (§2 above) and the archived October campaigns; also the comparator source pin `9c6a6d9` that the hybrid CI on the cumulative branch fetches | Merge is conflict-free; the pin must live on the production line or the hybrid node is not reproducible from it |
| `research/banded-exact-reuse-20261020` (#73) | Four verified band-solve defects (NaN/Inf returned as success, shape-arithmetic panic) with fixes; `BandedKernel::FixedWidth2` (4.7–5.3% faster); an exact-bits LU cache that cut factorizations 134 → 8 on a linear diffusion case (10–11% time) and hit nothing on nonlinear cases | The defects are still present on the cumulative branch and belong in phase A; the bit-exact cache is not Jacobian reuse and does not address the Brusselator gap |
| `research/midcheck-three-solver-20261020` (#76) | Same-host re-measurement: the ranking of §2 reproduces (banded/DVODE 2.28, 2.10, 2.16 across three replicate runs); on a 96-dimensional few-mode case the coupled residual-first step is 3.6x slower than `fast` | Confirms that the coupled path is a large-n or expensive-callback candidate only |
| `research/all-dag-completion-20261018` (no PR) | A second, divergent implementation of the audit's A01–A12 (nine new modules: `nonlinear_homotopy.rs`, `packet_verified_trajectory.rs`, `damped_packet.rs`, `actual_k_homotopy.rs`, `generic_verified.rs`, `collective_rodas5p.rs`, `gray_scott_verified.rs`, `affine_transport.rs`, `polynomial_router.rs`), its own `bound_pr101.rs`, a PR101 certified run with 492 accepted / 292 rejected steps and certified radius 4.5e-12, and a 246-component benchmark campaign that never ran | Thirteen conflict hunks against the cumulative branch, a forked ledger (two different L-0136/L-0137), opposite DAG statuses (all twelve nodes `IMPLEMENTED_SCOPED_VALIDATION_COMPLETE` there, HOLD on the cumulative branch) and two competing component-triage campaigns; phase A must choose one line and tombstone the other |
| `audit/rvj-reaudit-remaining-20261011` | A parallel execution of the same preregistered nodes with different verdicts (AS04 FAIL there, PASS on the cumulative branch) and ledger ids L-0104–L-0111 reused | The conflicting AS04 verdict must be adjudicated before AS04's debit accounting is reused in the transaction type |

The planning document of 2026-09-04 (`planning/vigilode-research-coding-spiral-20260904`) already ordered the work as contamination certificate → order preservation → approximate-Jacobian/Rosenbrock-W/Krylov order → non-normal GMRES → transactional dense output → clients → performance last; the October campaigns ran while that order's A01–A12 were still HOLD. The plan below keeps the spirit (order preservation before speed) but ties each step to a measured gap.

## 3. The production contract

A production solver is a promise a user can rely on without reading the research ledger. VigilODE should make three promises of increasing strength, each a separate tier with its own tests, so that no research feature can weaken a weaker tier's promise.

| Tier | What the user gets | What is promised | What is only measured |
| --- | --- | --- | --- |
| P0 Protected | `Rodas5p` 5(4), stiffly accurate, L-stable, degree-4 continuous extension; dense, banded, sparse (faer sparse LU with FD colouring), small-N const-generic and JVP-only linear algebra; constant mass matrix (singular allowed, index-1 DAE); nonautonomous RHS; vector `atol`; events with root finding; max/min step; full work counters | Endpoint and interior error controlled against the same WRMS tolerance (result T3); no NaN propagates silently; failure returns the partial solution and a typed reason; zero heap allocation per step after setup; bit-reproducible runs for a fixed thread count | Wall time, step counts, Jacobian/LU reuse rate |
| P1 Performance | Lagged W factorization with stage-root correction (T1, T2); structure routing (band, sparse, band + low rank); batched small-N lanes; the polyalgorithm: exponential-Rosenbrock and structured/packet candidates under a causal budget with protected fallback (T5, T7, T8) | Same error contract as P0 on every accepted step; a candidate can never change an accepted state without passing the P0 acceptance test; all speculative work charged and reported | Speedup over P0, admission rate, fallback rate |
| P2 Certified | Opt-in enclosure mode for generated polynomial/rational models: every returned value carries an outward radius containing the true solution of the stated model under IEEE round-to-nearest (T4) | Inclusion of the true solution of the declared model, not of the user's intent; refusal when the model binding, the tube or the rounding budget cannot be established | Enclosure width, certificate cost |

### The public API

The current workspace exposes 54 public `integrate_*` entry points across `rodas5p-integrators` and requires a Jacobian or JVP at construction (`OdeProblem::new` refuses RHS-only problems). A production surface is one problem type, one solver type, one options type and one result type:

```rust
let problem = Problem::new(n, rhs)            // f(t, y, dy)
    .jacobian(Jacobian::Sparse { pattern, fill })   // or Dense, Banded{lo,hi}, Jvp(f), FiniteDifference
    .mass(mass)                                // constant, may be singular
    .structure(Structure::BandPlusLowRank{..}) // optional declaration, verified at t0
    .event(|t, y| y[0] - 1.0, Direction::Down);
let opts = Options { rtol: 1e-6, atol: Atol::Vector(v), dense: true, tier: Tier::Performance, ..Default::default() };
let sol = Rodas5p::solve(&problem, (t0, t1), &y0, &opts)?;   // sol.at(t), sol.stats, sol.transactions
```

Three rules fix the surface. A problem without a Jacobian provider gets finite differences (dense, banded or coloured sparse) and the result reports it, because a solver that refuses RHS-only problems is not a production solver. Every option that changes a numerical result is in `Options` and serialized into `sol.stats`, so a run is reproducible from its record alone — the fair-A/B discipline moved into the product. And `Tier` is a monotone switch: `Protected` never calls a candidate, `Performance` may, `Certified` requires a model binding and refuses otherwise.

### Feature boundary

The five `rodas5p-*` crates become internal modules of `vigilode`; `vigilode-coupled` and `vigilode-hybrid` become the P1 candidate backends after their drivers adopt the common `Transaction` type (section 6); every module whose name carries a research-node tag (`g4_*`, `audit2_*`, `thread_transfer_*`, `v38d_*`, `scientific_corpus_v2`, `policy_redesign_v25`) moves to a `vigilode-research` workspace that depends on `vigilode`, never the other way round. The ledger, the fair-A/B harness and the comparators (BDF1/2, RadauIIA1/3, labelled `reference-implementation-only`) stay in the research workspace.

## 4. Design thesis: what VigilODE is for

VigilODE's own idea is not Rosenbrock, Krylov or exponential integration; it is that a stiff solver should be a **causal, budgeted decision process** whose every speculative act is paid for, certified or receipted, and reversible. Five threads in the repository carry that idea, and the plan organizes the production solver around them rather than around any single numerical method.

**1. The stage-root view.** The eight RODAS5P stage equations with the frozen Jacobian J₀ form one block-lower-triangular root problem F(U) = 0 with every diagonal block equal to W = I − hγJ₀ (`research/cumulative_20261020/integration/REPORT_KO.md`, section 2; `raw_stage_target.rs`). Seen this way, a direct LU solve, a Krylov solve, a lagged-factorization iteration, the coupled all-at-once solve and the homotopy corrector are all *inexact solvers of the same target*, differing only in how their residual is bounded. Result T1 makes this precise: the accepted step is the exact RODAS5P step perturbed by a quantity bounded by the stage residual through ‖W⁻¹‖ ≤ 1 on dissipative problems, so order is preserved whenever the residual is held below the local error scale. This single fact is the production enabler the project has not yet used: it allows reusing an old LU across steps (something standard Rosenbrock codes cannot do without losing order), it gives the matrix-free path its preconditioner, and it gives every candidate the same acceptance test.

**2. Causal budget and transaction.** `transactional_q1_q2.rs`, `certified_budget.rs`, `packet_work.rs` and the coupled crate's `coupled_or_sequential_budgeted` already implement the pattern: a trial consumes counted work, commits only on acceptance, rolls back state, controller and history on rejection, and the fallback is charged in the same ledger (`research/coupled_economy_20261020/CRITICAL_REVIEW_KO.md`, sections 1 and 6). The project's negative result that a fixed JVP cap is not an economic policy is the right lesson: the budget must be a *decision rule with an expected-cost model* (T8), not a constant. The production transaction type is the one place where candidates meet the protected path.

**3. Certificate-first error accounting.** Three statuses, never conflated: an *estimate* (embedded error, filtered defect), a *measured receipt* (independent reference, allocator counts, same-output comparison), and a *certificate* (outward enclosure of the actual returned curve). `bound_pr101.rs`, `outward_certificate.rs`, `causal_majorant.rs` and the hybrid crate's continuous-defect windows (`crates/vigilode-hybrid/src/{bounds,trajectory}.rs`) implement certificates for narrow model classes; `dense_output_v2.rs` implements estimates; `rodas5p-fair-ab` implements receipts. The audit's finding that the packet certificate was not `roundoff_enclosed` and that the Leja total certificate admitted forged caches shows why the three statuses must be type-distinct in the product (`CertifiedRadius` vs `ErrorEstimate` vs `Receipt`), so a value cannot be upgraded by renaming.

**4. Structure before spectrum.** When a model has an exact algebraic structure — an invariant affine manifold, a band plus low rank, a quotient relation q(X) = 0, a collective mode with exact closure — the project exploits it directly instead of asking a Krylov method to rediscover it (`docs/reviews/20261011_promotion_theory/*.md`; `collective_model.rs`; the hybrid crate's tridiagonal + rank-one Woodbury solve). The 11.4x internal gain of the structured driver came from this, not from parallelism. Production routing therefore screens *declared and verified* structure first (band verified against the JVP at t₀ as `routing.rs` already does), and only then falls back to generic linear algebra.

**5. Accounting as a product feature.** Work counters that distinguish attempted from completed, failed arms kept in the record, allocator-measured allocations, same-output comparison and a timing authority label are not benchmark hygiene; they are what lets a user trust a polyalgorithm whose behaviour changes per step. `sol.stats` carries them in every tier.

What the thesis rules out is as important. A candidate is never admitted because it is elegant, because a theorem says it reduces polynomial degree, or because it parallelizes; it is admitted because its charged expected cost per unit of accepted physical time is lower than the protected path's on the regime the telemetry observes (T8), and because its accepted state passes the same P0 test. The research directions that failed this test in the project's own evidence — unpreconditioned stage-parallel Picard, fixed JVP caps, degree-reduction claims without wall-time receipts, naive threading of memory-bound RHS kernels — are retained as recorded negatives and not revisited without a new mechanism.

## 5. Mathematical foundations

Nine results carry the plan. T1–T3 justify the production core (lagged factorizations, preconditioned matrix-free stages, interior error control); T4 and T5 justify the certified tier and the parallel candidates; T6 and T7 justify the structured and exponential candidates; T8 and T9 justify the budget and the residual allocation. Notation: y' = f(t, y) on ℝⁿ, M = I unless stated, ‖·‖ a fixed norm with inner product ⟨·,·⟩, μ(J) = sup\_{x≠0} ⟨x, Jx⟩/‖x‖² the logarithmic norm, γ = 0.21193756319429014 the RODAS5P diagonal coefficient (`fixtures/rodas5p_coefficients_snapshot.json`), s = 8 stages, A and C the strictly lower implementation matrices and b the output weights of the repository's U-form (`rodas5p_fast.rs`, `raw_stage_target.rs`):

```latex
(I - h\gamma J_0)\,U_i = h\gamma\, f\big(t_n + c_i h,\; y_n + \textstyle\sum_{j<i} A_{ij} U_j\big) + \gamma \textstyle\sum_{j<i} C_{ij} U_j + h^2\gamma\gamma_i f_t,\qquad y_{n+1} = y_n + \textstyle\sum_j b_j U_j .
```

Write W = I − hγJ₀, D = I\_s ⊗ W, and F(U) = 0 for the whole stage system; F is block lower triangular with every diagonal block equal to W.

### T1. Stage-root perturbation: inexact stage solves preserve the step up to a tolerance-proportional term

**Lemma 1 (resolvent bound).** If μ(J₀) ≤ 0 then for every h > 0 the matrix W is nonsingular and ‖W⁻¹‖ ≤ 1; more generally ‖W⁻¹‖ ≤ 1/(1 − hγμ(J₀)) whenever hγμ(J₀) < 1.

*Proof.* For x ≠ 0, ⟨x, Wx⟩ = ‖x‖² − hγ⟨x, J₀x⟩ ≥ (1 − hγμ(J₀))‖x‖², so ‖Wx‖‖x‖ ≥ ⟨x, Wx⟩ gives ‖Wx‖ ≥ (1 − hγμ)‖x‖. ∎

**Theorem 1.** Let W be nonsingular, U\* the exact stage solution, and Ũ any stage vector with residual F(Ũ) = r, r = (r₁,…,r\_s). Let E = Ũ − U\*. Suppose that on the segments between the exact and computed stage arguments the Jacobian satisfies hγ‖W⁻¹(J(ξ) − J₀)‖ ≤ δ\_J. Then with the strictly lower nonnegative matrix K,

```latex
K_{ij} = \gamma |C_{ij}|\,\|W^{-1}\| + |A_{ij}|\big(1 + \|W^{-1}\| + \delta_J\big),\qquad 
\|E_i\| \le \sum_{k} \big[(I-K)^{-1}\big]_{ik}\,\|W^{-1}\|\,\|r_k\|,\qquad 
\|\tilde y_{n+1} - y_{n+1}\| \le \sum_i |b_i|\,\|E_i\| .
```

*Proof.* Subtracting the two stage systems, W E\_i = r\_i + hγ\[f(Ỹ\_i) − f(Y\_i)\] + γΣ\_{j\<i}C\_{ij}E\_j with Ỹ\_i − Y\_i = Σ\_{j\<i}A\_{ij}E\_j. By the mean-value theorem f(Ỹ\_i) − f(Y\_i) = J(ξ\_i)(Ỹ\_i − Y\_i), and hγW⁻¹J(ξ\_i) = hγW⁻¹J₀ + hγW⁻¹(J(ξ\_i) − J₀) = (W⁻¹ − I) + hγW⁻¹(J(ξ\_i) − J₀). Taking norms, ‖E\_i‖ ≤ ‖W⁻¹‖‖r\_i‖ + Σ\_{j\<i}K\_{ij}‖E\_j‖; K is strictly lower so (I − K)⁻¹ = Σ\_{k\<s}K^k ≥ 0 entrywise and the recursion majorizes. The output bound is the triangle inequality on Σ b\_iE\_i. ∎

**Why it matters.** With μ(J₀) ≤ 0 Lemma 1 gives ‖W⁻¹‖ ≤ 1, so K ≤ γ|C| + 2|A| + δ\_J|A| does not grow with h‖J₀‖: the amplification of a stage residual into the step is *stiffness-independent*. The only stiffness-sensitive term is δ\_J, the drift of the Jacobian across the step, which is O(h) for smooth problems and is exactly what a Jacobian-freshness monitor must track. The corollary is the production rule: if every stage residual is held at ‖r\_k‖ ≤ η\_k with Σ\_i |b\_i|\[(I−K)⁻¹\]\_{ik}η\_k ≤ θ·tol for a fixed θ (say 0.1), the computed step differs from the exact RODAS5P step by at most θ tolerance units, so the work–precision curve shifts by a bounded factor and the effective order is kept. In particular the step may be computed with *any* nonsingular preconditioner (an old LU, a banded approximation, a Krylov solve) as long as the residual is controlled — which standard Rosenbrock codes cannot do, because substituting an old Jacobian into the tableau itself leaves only the W-method order conditions the tableau happens to satisfy (Steinebach 2023 reports that Rodas5P with an inexact Jacobian behaves like Rodas4P2, which meets the W-conditions up to order 2). The same argument covers the embedded estimate, since it is another linear combination of the stages.

**Numbers for RODAS5P.** Every block of (I − D⁻¹B)⁻¹D⁻¹ is a polynomial in W⁻¹ (B is nilpotent and its blocks are γC\_{ij}W⁻¹ + A\_{ij}(W⁻¹ − I)), so the sensitivity of the output to a residual on stage j is a vector-valued polynomial m\_j(W⁻¹) of degree ≤ 8 with tableau coefficients. Computed from the repository's coefficient snapshot: for the scalar test equation (and hence any normal J₀ with spectrum in the left half plane, by diagonalization) the exact sensitivities at z = hλ = 0 are |m| = (0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.01, 1.00), sum 23.3; they fall to a sum of 3.8 at z = −10 and 0.13 at z = −100, because a stage residual on a stiff component is damped by W⁻¹ before it reaches the state. For non-normal J₀ the stiffness-independent bound Σ\_k|c\_{jk}| ‖W⁻¹‖ᵏ ≤ Σ\_k|c\_{jk}| on the polynomial coefficients gives (100, 15, 24, 19, 14, 15, 1, 1), sum 189. Three facts follow. Stages 3–6 carry the output sensitivity in the nonstiff regime, stages 1–2 almost none, so the allocation of T9 is not optional. The entrywise triangle-inequality majorant K is about seven times more pessimistic than the exact map at z = 0 (169 versus 23) and should be used only as a certificate; the exact normal-case sensitivities are the targets. And since the U-form residual equals γ times the K-form residual stage by stage (U = ΓK), the choice of implementation coordinates changes nothing in the sensitivity — the project's measured raw-U Picard transient (‖T\_U³‖∞ ≈ 81.9 at z = −100, `integration/REPORT_KO.md` §2) is an iteration property, not a property of the exact recursion, which is why the plan prefers preconditioned GMRES on each stage over Picard on the stage system. The code already computes this object: \`rodas5p\_matrix\_free\_fast.rs:708-745\` forms τ\_y = b\_codeᵀL(z)⁻¹ with L(z) = (1 − γz)I − γ(zA + C) and maximizes it over 1,501 imaginary-axis points (ALG01/ALG04); the numbers above are the same map on the negative real axis, and the polynomial-coefficient sum is its certified, non-normal-safe form, so a proved supremum can replace the grid maximum that the crate map flags as heuristic.

### T2. Preconditioner admissibility: when an old factorization is good enough

**Theorem 2.** Let P be nonsingular and T = I − P⁻¹W. If ‖T‖ ≤ q < 1 then W is nonsingular, ‖W⁻¹‖ ≤ ‖P⁻¹‖/(1 − q), Richardson iteration x\_{k+1} = x\_k + P⁻¹(c − Wx\_k) satisfies ‖x\_k − W⁻¹c‖ ≤ qᵏ‖x₀ − W⁻¹c‖, left-preconditioned GMRES satisfies ‖P⁻¹r\_k‖ ≤ qᵏ‖P⁻¹r₀‖, and right-preconditioned GMRES satisfies ‖r\_k‖ ≤ κ(P)qᵏ‖r₀‖.

*Proof.* P⁻¹W = I − T is invertible by the Neumann series with ‖(I − T)⁻¹‖ ≤ 1/(1 − q); the error of Richardson is multiplied by T each step; GMRES minimizes the (preconditioned) residual over all polynomials p with p(0) = 1, and the Richardson polynomial (1 − ζ)ᵏ is admissible: on the left, P⁻¹r\_k = Tᵏ(P⁻¹r₀) up to the minimization; on the right, r\_k = (I − WP⁻¹)ᵏr₀ = PTᵏP⁻¹r₀, which costs the factor κ(P) = ‖P‖‖P⁻¹‖. The stopping rule of T9 uses the left-preconditioned residual, where no κ(P) appears. ∎

**Corollary 2a (step-size change only).** If P = I − h\_oldγJ₀ (same Jacobian, old step), then T = (1 − h/h\_old)(P⁻¹ − I), so q ≤ |1 − h/h\_old|(1 + ‖P⁻¹‖) ≤ 2|1 − h/h\_old| under μ(J₀) ≤ 0. A step-size change of 25% leaves an old LU with contraction q ≤ 1/2 and no Jacobian or factorization work at all.

**Corollary 2b (lagged Jacobian).** If P = I − hγJ\_old then T = hγP⁻¹(J₀ − J\_old) and q ≤ hγ‖P⁻¹‖‖J₀ − J\_old‖. The certifiable form uses a Lipschitz constant of the Jacobian on the segment between the two states (a model-level witness, the project's parameter-cell idea A1/A2 in `docs/reviews/20261011_promotion_theory/CERTIFICATE_ATLAS_KO.md`); the operational form estimates q from two consecutive preconditioned residuals, which is an estimate and is labelled as one.

**Corollary 2c (structural preconditioner, matrix-free).** If only a JVP is available but a banded or sparse part J\_S of the Jacobian is declared (`SparseLinearPart`, `BandedJacobian` in `problem.rs`), then P = I − hγJ\_S gives T = hγP⁻¹(J₀ − J\_S); the off-structure part need only be small *relative to P*, not in absolute terms, which is why a banded transport operator plus a low-rank or local nonlinear remainder is the right production structure for JVP-only PDE clients. GCRO-DR recycling (`gcrodr.rs`) then removes the few directions P misses, and T2 bounds its residual as well.

The production decision rule follows: run each stage solve with the current P; if the estimated q exceeds q\_max (0.5) or the iteration budget from T9 is exceeded, refresh J and the factorization. The refresh is a counted transaction, so Jacobian reuse becomes a budget decision rather than a heuristic.

### T3. Interior error control with the existing factorization (W-filtered defect)

Let u(θ), θ ∈ \[0, 1\], be the degree-4 continuous extension of an accepted step and d(θ) = u'(θ)/h − f(t\_n + θh, u(θ)) its defect. Define the estimate

```latex
E_\theta = (I - h\gamma J_0)^{-1}\,\tfrac{h}{2}\, d(\theta),
```

one right-hand side and one back-solve with the factorization the step already holds.

**Lemma 3.** For y' = λy with z = hλ ≤ 0 and a constant defect d, the interior error at θ = 1/2 is ε = hd(e^{z/2} − 1)/z and the ratio R(z) = E\_{1/2}/ε = (z/2)/((1 − γz)(e^{z/2} − 1)) is monotone increasing in −z with 1 ≤ R(z) ≤ 1/(2γ) = 2.36 for all z ≤ 0 (R → 1 as z → 0, R → 1/(2γ) as z → −∞; monotonicity holds for γ < 1/4 and was checked on a 2000-point grid).

*Proof.* With w = −z/2 > 0, R = \[w/(1 − e^{−w})\]·\[1/(1 + 2γw)\]; the limits are immediate, and d(log R)/dw = 1/w − e^{−w}/(1 − e^{−w}) − 2γ/(1 + 2γw) is positive near 0 (it equals 1/2 − 2γ + O(w)) and tends to 0⁺ at infinity. ∎

For normal J₀ with spectrum in the left half plane the same bounds hold componentwise in the eigenbasis; for general dissipative J₀ one still has ‖E\_θ‖ ≤ (h/2)‖d(θ)‖ by Lemma 1, and in the stiff limit E\_θ → −(1/(2γ))J₀⁻¹d(θ), the quasi-static interior error inflated by at most 2.36. The existing `DenseErrorControl::Enforce` uses the shift h/2 instead of hγ, is exact in the stiff limit, and costs a new Jacobian and LU per sampled step (`docs/DENSE_OUTPUT_ERROR.md`; on the nonlinear Prothero–Robinson witness at rtol 1e-8, DEFAULT\_PATH\_KO.md DP-01 records 17 accepted and 24 rejected steps with 79 Jacobians); the W-filtered estimate loses at most the factor 2.36 and costs one back-solve. Production use: sample θ ∈ {1/2} (optionally {1/3, 2/3}), take err = max(err\_embedded, ‖E\_θ‖\_WRMS) in the controller so the step shrinks before an interior failure rather than after. In the matrix-free tier the back-solve is one preconditioned Krylov solve under T2.

### T4. Enclosure of the returned curve from a continuous defect and a two-state dissipativity inequality

**Theorem 4.** Let ŷ be absolutely continuous on \[t\_n, t\_n + h\] with defect d(t) = ŷ'(t) − f(t, ŷ(t)) and ‖d(t)‖\_M ≤ δ(t), and let y be a solution of y' = f(t, y) with ‖y(t\_n) − ŷ(t\_n)‖\_M ≤ r\_n. Suppose a tube 𝒯 contains both curves and ⟨x − w, f(t, x) − f(t, w)⟩\_M ≤ μ(t)‖x − w‖²\_M for all x, w ∈ 𝒯. Then

```latex
\|y(t) - \hat y(t)\|_M \;\le\; e^{\int_{t_n}^t \mu}\, r_n + \int_{t_n}^{t} e^{\int_s^t \mu}\,\delta(s)\,ds .
```

*Proof.* Let r(t) = ‖y − ŷ‖\_M. Where r > 0, r r' = ⟨y − ŷ, f(t, y) − f(t, ŷ) − d⟩\_M ≤ μr² + δr, so D⁺r ≤ μr + δ, and the Gronwall comparison with the linear equation gives the bound; where r = 0 the Dini derivative is bounded by δ directly. If 𝒯 is an invariant box of the true flow and contains the Bernstein hull of ŷ, the tube hypothesis holds a priori; otherwise it is established a posteriori by the first-exit argument (the bound would have to fail before the curve leaves 𝒯, contradiction). ∎

Three consequences shape the certified tier. (i) Nothing is double counted: incomplete stage roots, time truncation and the polynomial fit all appear in the one defect of the *actual returned* curve, so no embedded estimate is added on top (`research/cumulative_20261020/derivative_free/REPORT_KO.md` §9). (ii) For polynomial or rational f the defect of a degree-p Bernstein curve is a polynomial of degree ≤ 2p, and sup|d| ≤ max|Bernstein coefficient|, with dyadic subdivision converging to the true sup (the audit's T2 item: (1−τ)¹² has coefficient bound 4096 but Bernstein bound 1). (iii) The one-sided constant μ comes from an *algebraic identity in two states*, not from a runtime Jacobian norm: for graph-Laplacian diffusion eᵀM\[F(x) − F(w)\] = −(Bᵀe)ᵀK(Bᵀe) ≤ 0, and for the hybrid model Σ\_j J\_{ij} ≤ α − λ gives μ\_∞ ≤ α − λ ≤ 0 on the box (`RECOVERY_REVIEW_KO.md` §6). The inherited radius r\_{n+1} is the bound at t\_n + h plus the exact rational displacement of the binary64 center, and the returned radius at a query time is the smaller of the whole-window bound and the pointwise bound — both valid.

### T5. Causal majorant for stage-parallel and homotopy corrections

**Theorem 5.** Let a stage iteration satisfy e\_{m+1} ≤ B e\_m + ρ\_m entrywise with B ≥ 0 strictly lower triangular (s × s) and ρ\_m ≥ 0. Then Bˢ = 0, e\_m ≤ Bᵐe₀ + Σ\_{j\<m}B^{m−1−j}ρ\_j, and in exact arithmetic the error from the initial guess vanishes after s rounds. If the actual map has majorant B + E with ‖(I − B)⁻¹E‖ ≤ q < 1, then (I − B − E)⁻¹ = (I − T)⁻¹(I − B)⁻¹ with T = (I − B)⁻¹E is nonnegative and the error of any approximate root with residual d satisfies ‖e‖ ≤ ‖(I − B)⁻¹d‖/(1 − q).

*Proof.* Strictly lower nonnegative matrices are nilpotent of index ≤ s, which gives the first two claims; the third is the Neumann series on (I − T). ∎

Two repository facts refine the use. The fixed-W stage homotopy has det(∂H/∂U) = det(W)ˢ ≠ 0, so no fold exists and the λ-path need not be followed in order (`integration/REPORT_KO.md` §2); and B for RODAS5P is the matrix T(z) of T1, so for normal J₀ the per-round contraction is known from the tableau and from the Ritz values, not estimated. The economic lesson recorded in `coupled_native_20261020` (the 8 sequential solves are replaced, but the sequential matrix-free driver is still faster below n ≈ 10⁴) is consistent with T5: parallel rounds reduce span, never work, so admission is by T8.

### T6. Exact structural reduction

**Theorem 6 (invariant affine manifold).** Let U ∈ ℝ^{n×r} have full column rank and suppose f(y\_ref + Uz) = U g(z) and J(y\_ref + Uz)U = U J\_r(z) for all z in a domain. Then every stage of the RODAS5P step from y\_n = y\_ref + Uz\_n equals U times the corresponding stage of the step applied to z' = g(z), the outputs and the embedded estimates lift exactly, and the n-dimensional factorization is replaced by an r-dimensional one.

*Proof.* By induction over stages: if U\_j = U V\_j for j < i then the right-hand side of stage i lies in the range of U, and (I − hγJ₀)U = U(I − hγJ\_r), so the unique solution is U\_i = U V\_i with (I − hγJ\_r)V\_i equal to the reduced right-hand side. ∎

**Theorem 6b (band plus low rank).** If W = T − SVᵀ with T banded nonsingular (half-bandwidth b) and S, V ∈ ℝ^{n×r}, then x = W⁻¹c = x₀ + Z(I\_r − VᵀZ)⁻¹Vᵀx₀ with x₀ = T⁻¹c, Z = T⁻¹S, at cost O(nb²) for the factorization, O(nbr + r³) for setup and O(nb + nr) per right-hand side, provided the r × r capacitance I\_r − VᵀZ is nonsingular; the residual c − Wx is one matvec and certifies the solve.

*Proof.* Woodbury identity. ∎

**Theorem 6c (quotient action).** If q(X) = 0 with deg q = d, then for every polynomial p, p(X) = r(X) where r = p mod q has degree < d; a degree-m Laguerre or Chebyshev phi action therefore costs d − 1 operator products instead of m − 1. For Hermitian X with spectrum in \[−1, 1\], A = cI + sX ≤ 0 and interpolation nodes in \[−1, 1\], the near-annihilator bound ‖φ\_k(hA) − r\_k(X)‖ ≤ ‖q(X)‖(hs)^d/(d + k)! holds (`docs/reviews/20261011_promotion_theory/POLYNOMIAL_KO.md`, P1–P3; proof there, reproduced by the exact-rational degree-64 example with d = 2).

T6a is the collective-mode packet (mean + transverse closure, `RECOVERY_REVIEW_KO.md` §4), T6b is the structured solve that produced the 11.4x internal gain, and T6c is the only polynomial-route result the plan promotes, and only for operators with a verified enclosure.

### T7. The exponential-Rosenbrock candidate and its regime

For y' = J y + g(y), exponential Rosenbrock–Euler y\_{n+1} = y\_n + hφ₁(hJ\_n)f(y\_n) is exact when g is affine, and its local error depends on the nonlinear remainder only, with constants independent of ‖J‖ under a semigroup bound ‖e^{tJ}‖ ≤ C (Hochbruck–Ostermann 2010). A Rosenbrock method instead treats an oscillatory mode e^{iωt} through its stability function R(ihω) with |R(iy)| < 1, which damps what it does not resolve; the mode stays accurate only for hω ≲ 1. The Krylov cost of φ₁(hJ)v for skew-Hermitian-dominated J needs dimension m ≳ 2hρ (ρ the spectral radius) before the Hochbruck–Lubich bound 12e^{−(hρ)²/m}(ehρ/m)ᵐ becomes small, so the exponential step pays about two JVPs per radian of carrier phase plus O(m²n) orthogonalization, against one full Rosenbrock step (one Jacobian, one factorization, eight solves) per fraction of a radian.

**Proposition 7 (regime test).** With cost c\_RB per Rosenbrock step, c\_J per JVP and m(h) ≈ 2hρ, the expected costs per unit time are ≈ ρ c\_RB/κ for Rosenbrock (κ the admissible phase per step, ≈ 1) and ≈ 2ρ c\_J + (4h²ρ²n)/h orthogonalization for the exponential candidate. The candidate wins when c\_RB/κ ≫ 2c\_J + 4hρ²n/ρ, i.e. when a Rosenbrock step is expensive relative to a JVP and the phase per exponential step hρ is moderate (m ≲ 50). Both ρ and the Re/Im ratio are read from the Ritz values of the Arnoldi process the matrix-free path already runs; the carrier of a *known* collective mode is removed exactly by a rotating-frame transformation (the packet's carrier convention), after which no Krylov cost remains.

This is a cost model, not a theorem about wall time; the admission in T8 uses it only as the prior for the candidate's success probability and charges the measured cost.

### T8. Causal economics of a candidate

Let a candidate attempt cost C\_a, its acceptance check C\_v, the protected fallback C\_f, and let it succeed with probability p on the current regime. Then E\[C\] = C\_a + C\_v + (1 − p)C\_f and the attempt lowers expected cost iff p > (C\_a + C\_v)/C\_f (`integration/REPORT_KO.md` §5). With different step sizes the objective is cost per accepted physical time, J = E\[C\_setup + C\_trial + C\_cert + C\_reject + C\_fallback + C\_remap\]/E\[Δt\_accepted\] (`derivative_free/REPORT_KO.md` §11).

**Proposition 8 (bounded overhead).** If at most one candidate attempt is allowed per m accepted protected steps while the empirical success rate is below the threshold, the worst-case overhead over the protected path is (C\_a + C\_v)/(mC\_f); so m ≥ 10(C\_a + C\_v)/C\_f guarantees the 10% overhead gate G-C whatever the candidate does. Above the threshold, estimating p per regime bin with a Beta posterior and admitting when the upper confidence bound exceeds (C\_a + C\_v)/C\_f is a two-armed bandit with bounded costs, whose regret against the better of {always protected, always candidate} per regime is O(√(T log T)) by the standard UCB1 argument (Auer, Cesa-Bianchi, Fischer 2002).

The causality constraint is part of the definition: p̂ is updated only from charged attempts on accepted states, never from reference solutions, holdout outcomes or future steps. The repository's fixed caps (`candidate_jvp_cap` = 64/256) failed because they encode neither p nor C\_f (`coupled_economy_20261020/CRITICAL_REVIEW_KO.md` §6); the rule above replaces them.

### T9. Influence-weighted residual allocation

**Theorem 9.** Let stage i be solved by an iteration with contraction q\_i ∈ (0, 1), cost c\_i per iteration and initial residual R\_i, let a\_i > 0 be the influence of its residual on the output (T1), and let B be the admissible output perturbation. Minimizing total work Σ\_i c\_i k\_i subject to Σ\_i a\_i R\_i q\_i^{k\_i} ≤ B has, after relaxing k\_i to the reals, the solution

```latex
\rho_i^{*} = \min\!\Big(R_i,\; \frac{p_i}{\lambda a_i}\Big),\qquad p_i = \frac{c_i}{-\log q_i},\qquad \sum_i a_i \rho_i^{*} = B,
```

with k\_i = ⌈log(R\_i/ρ\_i\*)/(−log q\_i)⌉ and a final outward check of Σ a\_i ρ\_i.


*Proof.* With ρ\_i = R\_i q\_i^{k\_i}, k\_i = log(R\_i/ρ\_i)/(−log q\_i), so the objective is Σ p\_i log(R\_i/ρ\_i), convex in ρ; the KKT conditions of the Lagrangian Σ p\_i log(R\_i/ρ\_i) + λ(Σ a\_iρ\_i − B) give −p\_i/ρ\_i + λa\_i = 0 on the interior and ρ\_i = R\_i where the bound is inactive. ∎

The repository's exact example (a = (4096, 1, 1, 1), q = 1/2, unit cost, B = 1/16) checks: a common threshold needs 17 iterations per stage, 68 in total (4099·2⁻¹⁶ > 1/16 ≥ 4099·2⁻¹⁷), while (18, 6, 6, 6) gives 4096·2⁻¹⁸ + 3·2⁻⁶ = 1/16 exactly with 36 iterations. For RODAS5P the a\_i are the sensitivities of T1 (0.58 : 0.24 : 6.25 : 5.97 : 3.73 : 4.57 : 1.01 : 1.00 in the nonstiff limit, decaying with hλ), so the allocation is computed once per factorization from the current Ritz estimate of hλ and reused across the stages of the step; with a lagged LU as preconditioner, GMRES can stop on the preconditioned residual ‖P⁻¹r‖ ≈ ‖W⁻¹r‖ directly, because the output sensitivity to that quantity, bᵀ(I − T)⁻¹, has entries at most 6.25 and a sum between 23.3 (hλ = 0) and 1.0 (hλ → −∞) for every stiffness, with the W⁻¹ damping already inside the monitored residual.

## 6. Workstreams and phases

Fourteen workstreams, grouped by the five phases of section 1. Each row names the mechanism, the result that justifies it, the code it starts from, the kill gate that can stop it and the acceptance that closes it. Effort assumes one developer with agent assistance; weeks are elapsed, and D runs alongside B and C.

&#91;embedded content: roadmap · five phases, five gates, weeks 0–34\]

A and B carry the accent because they decide whether VigilODE is a production solver at all; C, D and E add value on top of a protected path that already meets G-B.

| ID | Workstream | Foundation | Starts from | Kill gate | Acceptance |
| --- | --- | --- | --- | --- | --- |
| A1 | One crate, one API: `Problem`/`Options`/`Solution`, FD Jacobian (dense, banded, coloured sparse), constant mass matrix, events, vector atol, serialized stats | §3 | `problem.rs`, `rodas5p_fast.rs`, `output.rs`, `sparse_direct.rs` | — | `cargo semver-checks` clean; RHS-only problems solve; the four band-solve defects of PR #73 fixed and its FixedWidth2 kernel adopted; one A01–A12 line chosen (all-dag or cumulative) with a single ledger; every option in `sol.stats`; docs build |
| A2 | W-filtered interior control in the controller norm; `sol.at(t)` on the degree-4 extension | T3 | `dense_output_v2.rs` (Report/Enforce modes), `adaptive.rs` | Prothero–Robinson λ = −10⁵ interior error > 2 tol units at rtol 1e-6, or > 1.3x the protected step count | Interior error ≤ 2 tol units on the stiff corpus at rtol 1e-3…1e-10 with ≤ 1.2x steps |
| A3 | Controller hardening: PI/predictive with limiter, stiffness-aware first step, rejection cap | — | `adaptive.rs` (`ControllerKind`), `path_controller.rs` | — | van der Pol μ = 1000 rejections ≤ 20% of steps at rtol 1e-4 (today 87 of 155), no accuracy loss |
| A4 | Structure routing by verified declaration: band, sparse, band + low rank, small-N, JVP-only | §4 idea 4 | `routing.rs`, `rodas5p_fast_small*.rs`, `collective_direct.rs` | — | Router never silently falls back; mismatch is a typed error; every route has a work receipt |
| B1 | Lagged-factorization stage GMRES: reuse LU across steps and h changes, residual targets from influence weights, refresh as a charged transaction | T1, T2, T9 | `rodas5p_fast.rs`, `gmres_into.rs`, `certified_budget.rs` | Factorizations per accepted step > 0.4 on HIRES/Brusselator at rtol 1e-6, or accepted-step count up > 5% | Matched-accuracy time ≤ 0.6x protected on Brusselator 1D N = 100 and Gray–Scott N = 10⁴; identical error contract |
| B2 | JVP-only tier: structural or FD-coloured preconditioner, GCRO-DR recycling, forcing terms from T9 | T2c, T5 | `rodas5p_matrix_free_fast.rs`, `gcrodr.rs`, `problem.rs::SparseLinearPart` | GMRES iterations per stage > 6 on Gray–Scott N = 10⁴ with the banded P | Converges on the stiff PDE corpus at all tolerances; no inner-forcing failure |
| B3 | Batched small-N lanes (SoA) for ensembles; GPU lane as a later card | — | `rodas5p_fast_small_batch.rs` | — | ≥ 4x throughput over sequential small-N on 1024 Robertson instances |
| C1 | `Transaction` type and regime telemetry: Ritz spectrum, q estimate, δ\_J, rejection history; all counters attempted/completed | §4 idea 2 | `transactional_q1_q2.rs`, `rhs_telemetry.rs`, `work.rs` | — | Every candidate uses the one type; rollback restores state, controller, history, Krylov state bit for bit |
| C2 | Exponential-Rosenbrock candidate (pexprb54s4) with Krylov φ and a posteriori error, under the P0 acceptance test | T7 | `exponential.rs`, `adaptive_exponential.rs`, `g2_exponential_gate.rs` | < 2x on the sealed oscillatory holdout | Admitted by cost on oscillatory regimes; charged fallbacks; never changes P0 semantics |
| C3 | Structured candidates: band + rank-r Woodbury, invariant-manifold packet with rotating-frame carrier, quotient action on enclosed symmetric operators | T6 | `vigilode-hybrid`, `collective_model.rs`, `polynomial_action.rs` | Residual witness fails or closure test fails on the model | Exact closure verified per step; 11x-class gains reproduced under the common transaction |
| C4 | Admission policy: expected-cost rule, per-regime Beta posteriors, bounded-overhead cap | T8 | `coupled_or_sequential_budgeted`, `me01_full_cost_judgement` | Corpus median > 1.10x the protected path | G-C: ≤ 1.10x median; ≥ 2x on holdout |
| D1 | Certified windows for generated models: immutable binding, Bernstein sup bounds, directed rounding, inherited radii (audit A01, A02, A10) | T4 | `vigilode-hybrid/src/{bounds,trajectory}.rs`, `packet_window.rs`, `bound_pr101.rs` | Enclosure fails against a 90-digit reference on any fixture | `roundoff_enclosed = true` on the reflecting-grid and collective models |
| D2 | Damped packets, Leja arithmetic closure, generic continuous-output residual (audit A03, A04, A05) | T4, T6c | `damped_moments.rs`, `leja_action.rs`, `dense_output_v2.rs` | Cross term or underflow term left open | Audit findings B02–B04 closed with independent high-precision cases |
| E1 | Benchmark harness v2: pinned CVODE, Radau5/RODAS, DVODE, Diffsol, OrdinaryDiffEq `Rodas5P`; one validated JSON; allocator counts; isolated host | §7 | `rodas5p-fair-ab`, `tools/benchmarks/diffsol` | — | Zero report/data discrepancies; failed arms preserved; holdout sealed |
| E2 | Release 1.0: docs, MSRV, crates.io, changelog, external review | — | `README.md`, CI | — | G-E |

Dependencies: A1 precedes everything; A2–A4 are independent; B1 needs A1 and A4; B2 needs A4; C1 needs A1; C2–C4 need C1 and B1 (the candidates must be compared against the improved protected path, not the old one); D1 needs A1 only; D2 needs D1; E1 can start at week 0 for the protected path and reruns at each gate.

### Defects to close inside A1 (found in the static crate review of the latest branch)

- A zero tolerance scale (atol = 0 with a component that is 0 at both ends) is handled three different ways: NaN norm and perpetual rejection in `rodas5p_fast.rs:770-781` (also the small and batch drivers), `InvalidInput` abort in `rodas5p_matrix_free_fast.rs:2990-2998`, a typed error in `rodas5p-core/src/norms.rs:55-58`. One rule, in `error_scale`.
- Accepted/rejected counters drift in the transactional driver (`transactional_q1_q2.rs:1908-1912` vs `integrate.rs:1210-1251`): a step with error in (0.9, 1\] is counted accepted and then rejected by the outer check without `reconcile_outer_rejection`; the same pattern in the dense variant, legacy SABR and homotopy drivers. The `Transaction` type owns the counters so this cannot recur.
- The router refuses a `LinearPart` problem that has a JVP but no explicit Jacobian with a false message (`routing.rs:263-277`, `gray_scott_2d`); `StructureDeclaration::linear_part` can be attached without matrix data; `jvp_only_clone` resets the structure but keeps `linear_part`.
- The fast drivers abort on a user-callback `InvalidInput`/`Dimension` error and lose the partial solution; the protected driver returns it. Production semantics: always return the partial solution and a typed reason.
- Non-autonomous problems without `partial_t` get central finite differences for f\_t at step √ε in the strict matrix-free paths (`problem.rs:693-700`, \~1e-8 relative); the production API makes this explicit (`Options::ft = FiniteDifference | Provided`) and reports it in `sol.stats`.
- `GcrodrRecyclePolicy::Legacy` is the default although it breaks M⁻¹AU = C; the default becomes `RefreshAfterUpdate` and reuse is verified.
- The adaptive exponential driver bypasses the shared controller update (`adaptive_exponential.rs:268-276, 330-357`): no `last_rejected_trial`, no typed failure counters.
- `rodas5p-fair-ab/src/budget.rs:59` slices `&operator_id[..16]`, a panic (abort, under `panic = "abort"`) on short or non-ASCII ids.
- The eager coupled path charges m unused stage-0 JVPs per Newton iteration (`vigilode-coupled/src/solver.rs:159-171`).
- The four band-solve false-success and panic cases verified in PR #73 (NaN/Inf returned as `Ok`, shape-arithmetic overflow).

## 7. Benchmark corpus and comparison protocol

The protocol is the one the project converged on in `coupled_native_20261020` and `coupled_economy_20261020`, with the audit's A07 requirements added: one physical model per problem (same equations, same boundary treatment in every arm), one atol vector, one output grid obtained through each comparator's dense-output interface, one independent reference, failed arms preserved, allocator-measured allocations, fresh-process timing in Williams-ordered paired blocks, median of process medians with bootstrap family intervals, and a single validated JSON from which every table is generated. A speed statement is admissible only at matched achieved error (interpolated on each arm's work–precision curve), and otherwise is `DIAGNOSTIC_ONLY`.

### Corpus

| Group | Problems | Purpose |
| --- | --- | --- |
| Classical stiff (tuning set) | Robertson (n = 3), van der Pol μ = 1000, HIRES (n = 8), E5, OREGO\* , Pollution (n = 20), Ring modulator (n = 15), chemical Akzo-Nobel, Medakzo (1D PDE, n = 400), Brusselator 1D (n = 100…2000), Gray–Scott 1D/2D (n = 10³…10⁵), CUSP, Burgers (ν small) | Work–precision at rtol 1e-3…1e-10, Jacobian reuse rate, routing by structure |
| Interior-error set | Prothero–Robinson λ = −1, −10⁵, −10⁸; stiff-oscillatory linear test y' = (−a + iω)y, a ≪ ω; Prothero–Robinson with nonlinear term (audit example) | T3 gate; dense-output error at 101–201 output points; stiff order reduction |
| Oscillatory (candidate set) | Damped wave equation (n up to 10⁴), nonlinear Schrödinger-type skew-dominant systems, Fermi–Pasta–Ulam stiff springs, collective-mode models with and without damping (κ = 0, κ > 0), rotating-frame carriers with slow envelopes | T7 regime test; exponential candidate admission; packet closure |
| Structured (candidate set) | Reflecting-grid reaction–diffusion of `vigilode-hybrid` (N = 16…4096, ε ∈ {0, 1}), band + rank-one and band + rank-r collective models, few-mode/chain/periodic-diffusion synthetic set (n = 96, 384, 12288), broad-spectrum negative control (n = 64) | T6 structured routes; projection-leakage control; Woodbury baseline |
| Expensive-callback set | RHS with explicit cost (table lookups, nested quadrature), JVP-only PDE clients, Bateman four-state client | Where parallel stages and shared subspaces can win on cost, not span |
| Author's physics set | BBN reaction network (RABBIT), Bianchi anisotropic cosmology (BASS/lowell), radiative-transfer E–θ toy of `derivative_free/REPORT_KO.md` | Real clients; not used for tuning |
| Holdout (sealed) | OREGO\* (already sealed in the repository), plus one problem from each group above chosen by an outcome-independent rule before phase B | Gate 4 of every card; opened once per release |

The star on OREGO marks the sealed Oregonator holdout that the scientific-validity-v2 campaign never opened; it stays sealed until G-B.

### Comparators and their fidelity

| Comparator | Version pin | Interface | Fidelity tag |
| --- | --- | --- | --- |
| SUNDIALS CVODE (BDF, dense/band/sparse/GMRES) | pinned release, `sundials-sys` | `CVodeGetDky` dense output on the common grid | production |
| Hairer RADAU5 and RODAS (Fortran) | reference sources, pinned | `CONTR5`/`CONTRO` dense output | production; RODAS is the same method family with order 4 |
| DVODE (Netlib) | pinned | interpolated output | production |
| Diffsol 0.16.1 (BDF, ESDIRK, TR-BDF2; faer/nalgebra) | `Cargo.lock` pinned | `solve_dense` on the common grid | production |
| OrdinaryDiffEq.jl `Rodas5P` | pinned Julia environment | `saveat` grid | production; the gold standard for the same tableau, used to validate VigilODE's steps, errors and interpolant bit-for-bit where possible |
| Internal BDF1/2, RadauIIA1/3 | — | — | `reference-implementation-only`; never in a speed table |

### Rules that decide a row

1. Reference: two independent codes at rtol 1e-12 (Radau5 and CVODE) that agree to 1e-10 on the output grid; otherwise the row is `reference-disputed`.
2. Error: max over the grid of the WRMS error with the run's own atol vector and rtol; the embedded estimate is never the error.
3. Work: RHS, Jacobian, JVP, factorizations, solves, Krylov products, accepted/rejected steps, allocations after setup; attempted and completed counted separately; failed arms keep their cost.
4. Time: isolated host, pinned CPU frequency, fresh process per arm, warmup discarded, Williams order over arms, ≥ 8 paired blocks, bootstrap family intervals; a shared runner yields `timing_authority = diagnostic_only`.
5. Speed claim: at matched achieved error only, on the tuning set for development and on the holdout for promotion; the tuning set is never re-tuned against the holdout.
6. Comparator options are the vendor defaults plus the same atol/rtol; any deviation is recorded in the row.

The harness for this already exists in `rodas5p-fair-ab` (same-output, holdout hygiene, comparator fidelity tags); what changes is that it binds external production comparators through FFI instead of internal reference implementations, and that the report is a derived view of the JSON rather than a hand-written document. The first run of the new harness re-measures the Hairer codes: the recorded RADAU5/RODAS numbers (0.04–0.25x of VigilODE's wall time, L-0029) predate the \`fast\` drivers and the matched-error protocol and were never repeated under it, so today the strongest production baseline on small problems is unmeasured.

## 8. The research–exploration loop

The repository already has the hard part of a research process: an append-only ledger with four verdicts, preregistration before measurement, holdout hygiene, authority citations that must point at pushed bytes, and a stall rule (`docs/RESEARCH_LEDGER.md`). What it lacks is throughput discipline: 130 research nodes in eight weeks, many with preregistration only, and a drift toward process artifacts (receipts about receipts) over measurements. The loop below keeps the ledger and adds a cadence that makes creative divergence cheap and promotion expensive.

### Hypothesis cards

Every idea enters as a one-page card with six fields: the *mechanism* (what quantity changes and why, in one equation), the *regime* (which telemetry signal identifies the problems where it should help), the *kill gate* (a number that, if not reached at gate 2, ends the node), the *comparator* (the protected path, same output policy), the *cost model* (C\_a, C\_v, C\_f of T8, estimated before any code), and the *theorem obligation* (which of T1–T9 the mechanism must satisfy, or a new one with a proof sketch). A card without a kill gate is not opened. Cards are cheap: twenty a month is fine; three open nodes at a time is the limit.

### Four gates from toy to holdout

1. **Exact-rational toy (days).** The mechanism on a scalar or ≤ 8-dimensional model in Python `Fraction` or SymPy, sealed identities for every closed-form claim, the perturbation or enclosure bound evaluated exactly. Passing means the theorem obligation is discharged for the toy. The repository's own examples at this gate are `adaptive_scalar_certified.py` and the degree-64 quotient-action check.
2. **Native micro-benchmark (one week).** One Rust implementation unit behind a feature flag, bit-reproducible, with work counters and allocator counts, on fixed steps or one step. The kill gate is a *work* number (JVPs, solves, stage tasks), never wall time, because wall time on a shared runner is not a receipt. `coupled_economy_20261020` is the model: RED tests first, then the counted reduction (59→35 JVPs).
3. **Matched-accuracy A/B (two weeks).** Full adaptive trajectories against the protected path under the same output grid, same atol vector, same reference, failed arms kept, paired blocks and Bonferroni-family intervals as in `coupled_native_20261020`. The kill gate is the T8 inequality on measured costs: the candidate's charged cost per accepted physical time must beat the protected path on its claimed regime, including fallbacks.
4. **Holdout (one run).** Problems never used in gates 1–3, chosen by an outcome-independent rule and sealed before gate 3. One attempt; a failure closes the node with a `FAIL` row, and the mechanism may return only under a new card with a new mechanism.

Promotion to the production crate happens only after gate 4, and only into the tier whose contract the node discharged: an estimate into P0, a budgeted candidate into P1, an enclosure into P2.

### Cadence and rules

- **Measure before you write.** A node's first commit after preregistration is a numeric results file; documents come after the number (the ledger's stall rule, made the default rather than the exception).
- **One mechanism per node.** Nodes that bundle two changes cannot attribute a result; the project's own lesson from `hybrid_20261020` (local AD showed no gain once separated from the structured solve) came from separating them.
- **Negative results are assets.** Fixed JVP caps, naive stage threading, Picard on raw-U stages, unverified projected residuals and degree-reduction-as-speed are recorded negatives; a card that reopens one must name the new mechanism.
- **Idea budget.** One divergent session per month with no implementation allowed: the output is cards only, ranked by the cost model, not by elegance. The two 2026-10-20 reports (`integration/REPORT_KO.md`, `derivative_free/REPORT_KO.md`) are good examples of that session type; they produced the structural-solve direction that became the best measured result.
- **Formal obligations last.** Lean/Wolfram/SymPy sealing of a bound is required for P2 promotion and optional elsewhere; a proof assistant is never a substitute for gate 3.
- **Comparators are frozen.** External solvers are run with pinned versions and documented options; the internal reference implementations keep the `reference-implementation-only` tag and are never cited in a speed comparison.

### What the loop should explore first

The cards with the best cost model today are, in order: lagged-factorization stage GMRES on the dense/banded path (gate 2 obligation T1–T2, kill gate: factorizations per accepted step ≤ 0.4 on HIRES and Brusselator at rtol 1e-6 with no change in accepted-step count beyond 5%); W-filtered interior control (T3, kill gate: Prothero–Robinson interior error ≤ 2 tolerance units at rtol 1e-6 with ≤ 1.3x the protected path's steps); structural preconditioning for the JVP-only tier (T2c, kill gate: GMRES iterations per stage ≤ 6 on Gray–Scott N = 10⁴ with a banded P); the exponential candidate under the budget (T7–T8, kill gate: ≥ 2x on a sealed oscillatory holdout); and the rotating-frame packet for known carriers (T6a, kill gate: exact closure verified by the residual witness on the collective model with κ > 0).

Four further cards extend the project's own ideas rather than the literature's, and are worth opening once G-B is passed. **Solver memory**: persist the per-regime Beta posteriors of T8 and the Ritz-spectrum signatures across runs of the same problem family, so that a second integration of a BBN network or a Bianchi model starts with an informed admission prior — the uncertainty-adaptive search idea of QUARTZ applied to method selection, with the same causal rule that the prior is updated only from charged attempts. **Certificate as estimator**: run the Bernstein defect bound of T4 on the degree-4 continuous extension itself, in floating point and without the tube argument, as a cheap *estimate* of the interior error for P0 — the same arithmetic serves both tiers and the estimate inherits the certificate's structure (no double counting, exact at the endpoints). **Stage-root as a plug-in contract**: because T1 only needs a stage residual, any external linear solver — batched GPU LU for ensembles, cuSPARSE, a user's own multigrid — can be admitted behind the same acceptance test; this is how the RTX 3090 enters the picture without a GPU port of the integrator. **Generated flux models**: the `GeneratedFluxModel` + `CertifiedWindow` pair proposed in `derivative_free/REPORT_KO.md` §12 (flux, reaction, invariant and domain declared algebraically; local reaction ideals solved in closed form; no runtime derivative oracle) is the right home for the E–θ radiative-transfer client, and its first native unit is a fixed nonuniform grid with conservation, positivity and continuous output closed before any remap.

## 9. Module disposition

Four verbs. **Keep** = enters the production crate `vigilode` as is or after the phase-A API change. **Merge** = its mechanism enters the production crate but the module is absorbed into another. **Freeze** = moves to the `vigilode-research` workspace unchanged, with its ledger rows, and depends on `vigilode`. **Retire** = kept in history only; its purpose is served by something else or by a recorded negative result.

### rodas5p-core → `vigilode::core`

| Modules | Verb | Note |
| --- | --- | --- |
| `coefficients`, `work`, `operator`, `solver_types`, `norms`, `error`, `matrix`, `matrix_functions`, `sparse_direct`, `hash` | Keep | `norms::error_scale` must handle a zero scale consistently (risk 1 of the crate map); `sparse_direct` gains symbolic-pattern reuse for B1 |
| `directed`, `binary_scaling`, `transform_bound` | Keep (P2 tier) | Directed-rounding primitives are the certified tier's arithmetic |
| `nonnormal_certificate`, `residual_gain`, `damped_moments` | Keep (P2, feature `certified`) | Crouzeix–Palencia and log-norm certificates feed T4; `residual_gain` is n ≤ 8 only and says so |
| `polynomial_action`, `polynomial_action/router`, `laguerre_adjoint`, `leja_action`, `leja_setup_work`, `taylor_phi`, `taylor_phi_total` | Freeze; Chebyshev part Merge later | Only the Chebyshev action with a verified symmetric enclosure has a measured win (3.17x, L-0009); Laguerre was never cheaper and Leja's total is `EstimateOnly` — they return through a C3 card, not by default |
| `shared_shift_jet`, `shared_shift_policy`, `complex_shift_jet` | Freeze | No client with shifted solves exists (PP07, HT01) |

### rodas5p-krylov → `vigilode::krylov`

| Modules | Verb | Note |
| --- | --- | --- |
| `common`, `kernels`, `small`, `workspace`, `accounting`, `gmres`, `gmres_into`, `lgmres`, `lgmres_into` | Keep | `gmres_into` with right preconditioning is the B1/B2 stage solver; stopping on the left-preconditioned residual per T2/T9 |
| `gcrodr` | Keep, default `RefreshAfterUpdate` | The `Legacy` policy breaks M⁻¹AU = C (L-0059); recycling is admitted per regime by T8, not by default |
| `gmres_givens`, `gmres_staged` | Retire | Research-only and unused; the ν-guard idea is recorded |
| `block_gmres` | Freeze | Only the coupled candidate needs it |

### rodas5p-integrators → `vigilode::solver`

| Modules | Verb | Note |
| --- | --- | --- |
| `problem`, `adaptive`, `output`, `dense_output_v2`, `integrate` | Keep, rewritten behind the new API | One `Problem`/`Options`/`Solution`; `dense_output_v2` keeps the quartic extension and gains the W-filtered estimate (T3); the routing refusal for `LinearPart` + JVP and the `jvp_only_clone` structure reset are fixed here |
| `rodas5p_fast`, `rodas5p_fast_small`, `rodas5p_fast_small_batch`, `routing` | Keep | The protected drivers; `fast` becomes the default dense path, with partial-output-on-error semantics taken from the protected driver (risk 4) and the PR #73 band fixes |
| `sequential`, `stage_target`, `raw_stage_target`, `rodas5p_matrix_free_fast` | Merge | The K-form stepper and the U-form JVP driver become one stage-root stepper parameterized by coordinates and linear backend (T1); inner forcing from `g4_s5b0_inner_tolerance` is replaced by T9 targets |
| `block`, `parallel` | Keep | Block operator and local pool serve B2/C2 |
| `certified_budget`, `certification`, `transactional_q1_q2`, `controller_telemetry`, `rhs_telemetry` | Merge → `Transaction` and `Telemetry` types (C1) | The commit/rollback, budget reservation and attempted/completed accounting are the product; the q1/q2 candidate itself is frozen after HOM-06 |
| `exponential`, `adaptive_exponential` | Keep (feature `candidates`) | C2 candidate; its controller must go through the shared update (risk 5) |
| `collective_model`, `collective_direct`, `bound_collective`, `bound_pr101`, `packet_window`, `packet_curve`, `packet_curve_output`, `packet_trajectory`, `packet_work`, `outward_certificate`, `causal_majorant` | Keep (feature `certified`), after the all-dag/cumulative reconciliation | D1 material; one `bound_pr101.rs` survives |
| `homotopy`, `homotopy_policy`, `homotopy_order_policy`, `homotopy_experiments`, `path_controller`, `stage_chart_candidate`, `chart_transport`, `fourier_path_certificate`, `fourier_path_candidate`, `sabr`, `candidates` | Freeze | Negative or abstained results (HOM-06, HT01/02, R-NEXT-08); retained as recorded negatives with their ledger rows |
| `g1_transactional_gate`, `g2_exponential_gate`, `g3_fused_adaptive_gate`, `g4_prefix_kernel_gate`, `g4_s5b0_*`, `g4_s5b3_*`, `policy_redesign_v25`, `native_gates`, `unified_gates`, `unified_screen`, `common_w_gate`, `stage_batch`, `v38d_performance_tournament`, `a1_two_arm_receipt`, `scientific_corpus_v2`, `scientific_validity_v2_gate` | Freeze | Campaign and gate code of the v3.x and v2 programs; the research workspace keeps them runnable against the production crate |
| `audit2_*` (feature-gated) | Retire | Exploratory substrates superseded by the coupled crate and the transaction type |
| `bdf`, `radau`, `nonlinear`, `comparator_fidelity`, `problems`, `gray_scott` | Freeze (research workspace) | Reference implementations and corpus problems; the fidelity tag is enforced by the harness |

### rodas5p-fair-ab and rodas5p-cli

| Modules | Verb | Note |
| --- | --- | --- |
| `contracts`, `adapters`, `runner`, `session`, `budget`, `global_error`, `adaptive_global_error`, `output_admissibility`, `output_accuracy`, `numerical_reference`, `paired_timing`, `paired_receipt`, `timing_design`, `timing_authority`, `session_median*`, `external_comparators` | Freeze → `vigilode-bench` | The harness of §7; `budget.rs:59` slicing panic fixed; external comparators bound through FFI (CVODE, RADAU5/RODAS, DVODE, Diffsol) |
| `scientific_validity_v2_campaign`, `two_arm_campaign_v3`, `packet_benchmark` | Freeze | Campaign code; `packet_benchmark` only after A07's measured-allocation fix |
| `rodas5p-cli` (52 subcommands) | Retire → one `vigilode-bench` binary with `solve`, `wp` (work–precision), `compare`, `triage`, `ledger` | The research subcommands stay in the research workspace |

### Companion crates

| Crate | Verb | Note |
| --- | --- | --- |
| `vigilode-coupled` | Keep (feature `candidates`) after joining the workspace | Its `Work` ledger with budget reservation before scheduling is the template for `Transaction`; eager stage-0 JVPs removed (risk 11); becomes the large-n/expensive-callback candidate under T8 |
| `vigilode-hybrid` | Keep (feature `certified`) after joining the workspace | Its `bounds`/`trajectory` windows are the D1 starting point; the `external` feature (Fortran VODE, Diffsol) moves to `vigilode-bench` |

Two mechanical facts drive the first week of phase A: the two companion crates are not workspace members, so `cargo test --workspace` never runs their 52 tests, and five `coupled-*.yml` plus one hybrid workflow exist only to compensate; and the research surface (`integrate_*` × 54, CLI × 52) is where every later audit found its process defects. Making one crate and one binary is not cosmetic — it is what makes the four-gate loop of §8 cheap.

## 10. Risks and anti-patterns in the history, with mitigations

| Risk | Where it showed | Mitigation in this plan |
| --- | --- | --- |
| Claim inflation from unmatched comparisons | Diffsol report's 1.6x–7.3x (49 of 52 timings disagreed with its JSON; failed arms dropped; allocations estimated) — retracted by the audit | Reports generated from one validated JSON; same-output protocol; failed arms kept; allocator-measured counts; a comparison without matched error is labelled `DIAGNOSTIC_ONLY` by the harness, not by a reviewer |
| Research sprawl | 130 nodes, 54 public `integrate_*` entry points, 72k lines of integrator source of which the protected path is under 5k | Two workspaces; WIP limit of three nodes; promotion only through the four gates; one public solver type |
| Process over measurement | Nodes with preregistration only; receipts, contracts and closeouts outnumbering numeric results | Stall rule as default; first commit after preregistration is a numeric file; hypothesis cards without a kill gate are refused |
| Certificate status drift | Packet certificate returned without roundoff enclosure; Leja cache accepted forged coefficients; `fairness_verified = true` from tolerance settings alone | Type-distinct `Estimate` / `Receipt` / `CertifiedRadius`; certificate constructors private to the module that proves them; no `Certified` status without an arithmetic-domain witness (audit A04) |
| Coordinate conditioning of inexact stage solves | Raw-U Picard transient (‖T³‖ ≈ 82); crude majorant 2.7·10⁵; influence weights spanning 1–57 | Residual targets from T1/T9 influence weights; GMRES with lagged LU rather than Picard; exact normal-operator recursion for bounds |
| Parallelism as a goal | Naive threading of memory-bound RHS kernels slower at every size; coupled solver slower than sequential below n ≈ 10⁴ | Candidates admitted by T8 cost, never by span; worker count a budget variable |
| Fixed caps as policy | `candidate_jvp_cap` 64/256 rejected cheap successes and did not stop expensive failures | Expected-cost admission (T8) with per-regime posteriors and the bounded-overhead rule |
| Interior error unguarded | 312 tolerance units on Prothero–Robinson at rtol 1e-6 with dense control Off | W-filtered defect in the controller norm (T3) as the default; `Off` only as an explicit opt-out |
| Comparator fidelity | Internal BDF/Radau reference implementations usable as comparators by mistake | Tag kept and enforced by the harness; external comparators pinned by version and options; the gold standard for the method itself is OrdinaryDiffEq.jl `Rodas5P` |
| Single-host, single-author verification | Shared 4-vCPU runners; independence meaning a second computation path by the same author | Final benchmarks on an isolated host with pinned CPU frequency; external review requested per release, not per node; the ledger records who measured |
| Model scope of certificates mistaken for solver scope | Certified scalar and reflecting-grid examples read as general guarantees | P2 tier refuses any problem without an immutable model binding; the result type names the model identity it encloses |
| Jacobian freshness assumed | Every accepted dense step recomputes J and LU (26 LU setups for 25 Robertson steps) | Lagged factorization under T1–T2 with q-monitoring; the refresh is a charged transaction |

## 11. Sources

All repository facts come from a full clone of [cosmosapjw-quantum/vigilode](https://github.com/cosmosapjw-quantum/vigilode) taken on 2026-10-20 at 22:00 KST with `research/cumulative-evidence-20261020` (`ebb5b81`) checked out; nothing was built or executed except the coefficient computations of §5, which read `fixtures/rodas5p_coefficients_snapshot.json` with Python (`numpy`, `sympy`). Branch and PR facts were read with read-only git plumbing (`log`, `diff`, `show`, `merge-base`, `merge-tree`, `ls-remote refs/pull/*`) and the public PR pages; the GitHub API is not reachable from this environment.

**Documents read in full on the latest branch.** `README.md`; `docs/RESEARCH_LEDGER.md`; `docs/DENSE_OUTPUT_ERROR.md`; `docs/DIFFSOL_BENCHMARK_REPORT.md`; `docs/reviews/20261018_production_audit/{REPORT_KO,PACKET_AUDIT_KO,SOLVER_AUDIT_KO,PERFORMANCE_AUDIT_KO,DEFAULT_PATH_KO,THEORY_KO,INDEPENDENT_REVIEW_KO,CONTINUATION_A01_KO,CONTINUATION_POLICY_KO}.md`; `docs/reviews/20261011_promotion_theory/REPORT_KO.md`; `research/production_audit_20261018/{DECISION,NEXT_STEPS,CONTINUATION_DAG,FINDINGS}.json`; `research/cumulative_20261020/{README.md,integration/REPORT_KO.md,derivative_free/REPORT_KO.md}`; `research/coupled_native_20261020/CRITICAL_REVIEW_KO.md`; `research/coupled_economy_20261020/{CRITICAL_REVIEW_KO.md,THEORY.md}`; `research/hybrid_20261020/RECOVERY_REVIEW_KO.md`; `research/LEDGER.jsonl` (136 rows); the `DECISION.json`/`RESULTS.json`/`PREREGISTRATION.md` of the research nodes cited by name in §2 and §8.

**Code read.** `crates/rodas5p-core/src/{coefficients,work,operator,solver_types,norms,directed,polynomial_action,leja_action,residual_gain,nonnormal_certificate,damped_moments}.rs`; `crates/rodas5p-krylov/src/{gmres,gmres_into,gmres_staged,gmres_givens,lgmres,gcrodr,accounting,common}.rs`; `crates/rodas5p-integrators/src/{problem,sequential,adaptive,integrate,output,dense_output_v2,rodas5p_fast,rodas5p_fast_small,rodas5p_matrix_free_fast,routing,exponential,adaptive_exponential,transactional_q1_q2,outward_certificate,causal_majorant,homotopy,collective_model,packet_window,packet_trajectory,g4_s5b0_regime_atlas,g4_s5b0_inner_tolerance,rhs_telemetry,policy_redesign_v25,block,parallel,comparator_fidelity,bdf,radau,nonlinear}.rs`; `crates/rodas5p-fair-ab/src/{global_error,output_admissibility,timing_authority,budget,runner,contracts,adapters,packet_benchmark}.rs`; `crates/vigilode-coupled/src/{lib,solver,algebra,fallback}.rs`; `crates/vigilode-hybrid/src/{model,bounds,native,trajectory}.rs`; `tools/check-research-node.py`.

**Branches and PRs.** PR #71 (`research/benchmark-results-20261019-dual`: `research/strict_solver_comparison_20261019/FINAL_REVIEW_KO.md`, `CONCLUSION.json`), PR #73 (`research/banded-exact-reuse-20261020/{README,RESULTS,THEORY}.md`), PR #76 (`research/midcheck_20261020/REPORT_KO.md`, `ANALYSIS.json`), `research/all-dag-completion-20261018` (`nodes/A12/DECISION.json`, `PROGRESS_KO.md`), `audit/rvj-reaudit-remaining-20261011`, `planning/vigilode-research-coding-spiral-20260904`, the `pr/*` stack, the `claude/jolly-wozniak-7wl15h-*` series and the `audit/*` series; PR bodies of #18–#21, #29, #39, #41, #42, #64–#78.

**External references.**

- G. Steinebach, *Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package*, BIT Numerical Mathematics 63, 27 (2023) — [link.springer.com/article/10.1007/s10543-023-00967-x](https://link.springer.com/article/10.1007/s10543-023-00967-x) (opened; 8 stages, order 5(4), stiffly accurate, A-stable embedding, degree-4 continuous extension, inexact-Jacobian behaviour like Rodas4P2).
- M. Hochbruck and A. Ostermann, *Exponential integrators*, Acta Numerica 19 (2010) 209–286 — exponential Rosenbrock–Euler error structure (cited from memory; approximate).
- M. Hochbruck and C. Lubich, *On Krylov subspace approximations to the matrix exponential operator*, SIAM J. Numer. Anal. 34 (1997) 1911–1925 — Krylov error bounds used in T7 (cited from memory; approximate).
- M. Crouzeix and C. Palencia, *The numerical range is a (1+√2)-spectral set*, SIAM J. Matrix Anal. Appl. 38 (2017) 649–655 — the constant used by `nonnormal_certificate.rs`.
- P. Auer, N. Cesa-Bianchi and P. Fischer, *Finite-time analysis of the multiarmed bandit problem*, Machine Learning 47 (2002) 235–256 — the UCB1 regret argument of T8 (cited from memory; approximate).
- P. Tranquilli and A. Sandu, *Rosenbrock–Krylov methods for large systems of differential equations*, SIAM J. Sci. Comput. 36 (2014) — the K-method alternative named in the 2026-09-04 planning document; not used for any number here.
- P. N. Brown, G. D. Byrne and A. C. Hindmarsh, *VODE: a variable-coefficient ODE solver*, SIAM J. Sci. Stat. Comput. 10 (1989) 1038–1051, and the Netlib source [netlib.org/ode/vode.f](https://www.netlib.org/ode/vode.f), as pinned by PR #71.
- SciPy pull request [#24934](https://github.com/scipy/scipy/pull/24934), the VODE dense-Jacobian layout fix that invalidates the earlier SciPy-wrapped VODE comparison.
