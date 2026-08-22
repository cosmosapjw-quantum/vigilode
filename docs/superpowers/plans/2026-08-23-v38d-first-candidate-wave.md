# v3.8-D First Candidate Wave Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evaluate K1, K2/K3, C1/C2, and C4 as isolated performance candidates against the common exploratory substrate, then nominate at most one or two survivors.

**Architecture:** Every candidate forks from the same merged substrate commit into an isolated Git worktree. TDD and correctness checks precede measurement. Candidate reports use the common substrate schema and are compared only after isolated dispositions are final. Failed or held candidates remain local or in durable bundles; only a coherent survivor receives a remote PR.

**Tech Stack:** Rust 1.94.1, current Padé-13 projected oracle, fused-phi Krylov implementation, Git worktrees, common v3.8-D probe binary/analyzer, Python plots.

**Spec:** `docs/superpowers/specs/2026-08-23-v38d-high-entropy-performance-tournament-design.md`

## Global Constraints

- Timing validator and benchmark substrate must be merged before worktree fork.
- Preserve R-JF authority and all 64 frozen recommendations.
- K2/K3 target bitwise output/work parity; K1 uses a predeclared strict roundoff envelope.
- C1/C2/C4 may change Krylov/projected work but preserve endpoint WRMS, residual honesty, cap semantics, and normal-event completion.
- Survival requires all hard gates plus a material signal and normal-event p95 exploratory wall regression `<=5%`.
- No frozen-policy retuning, production selector, active switching, N=2048, new dependency, tag, or release.
- One initial attempt plus one diagnostic correction; second equivalent failure is `KILL`.

---

### Task 1: Worktrees and candidate registry

**Files:**
- Create `research/generic_v38d_high_entropy_performance_tournament/results/CANDIDATE_REGISTRY.json`.
- Create `reports/CANDIDATE_DECISION_LEDGER.md`.
- Create `scripts/test_candidate_registry.py`.

**Stable IDs:** `K1`, `K2`, `K3`, `C1`, `C2`, `C4`.

- [ ] **Step 1: Write failing registry validation test**

Require exact IDs, unique branches, retry limit 1, one allowed-file surface per candidate, explicit hypothesis/signal/kill criterion, and initial `READY` disposition.

- [ ] **Step 2: Create worktrees from one exact substrate SHA**

```bash
BASE_SHA="$(git rev-parse origin/research/v38d-exploratory-benchmark-substrate)"
test -n "$BASE_SHA"
git worktree add ../vigilode-k1 -b perf/v38d-projected-oracle-fusion "$BASE_SHA"
git worktree add ../vigilode-k2 -b perf/v38d-fused-phi-workspace "$BASE_SHA"
git worktree add ../vigilode-k3 -b perf/v38d-contiguous-layout "$BASE_SHA"
git worktree add ../vigilode-c1 -b perf/v38d-checkpoint-schedule "$BASE_SHA"
git worktree add ../vigilode-c2 -b perf/v38d-selective-mgs "$BASE_SHA"
git worktree add ../vigilode-c4 -b perf/v38d-adaptive-krylov-substep "$BASE_SHA"
```

Record `BASE_SHA` and its tree before mutation.

- [ ] **Step 3: Verify and commit registry**

```bash
python research/generic_v38d_high_entropy_performance_tournament/scripts/test_candidate_registry.py
git add research/generic_v38d_high_entropy_performance_tournament
git commit -m "research: register v3.8-D first-wave candidates"
```

### Task 2: K1 projected exp/phi1 oracle fusion

**Files:**
- Modify `crates/rodas5p-core/src/matrix_functions.rs`, `src/lib.rs`.
- Modify `crates/rodas5p-integrators/src/exponential.rs` at `projected_exponential_action_with_residual_estimate`.
- Create core/integrator K1 tests.

**Interface:**

```rust
pub struct DenseExpPhi1Action {
    pub exponential: Vec<f64>,
    pub phi1: Vec<f64>,
}

pub fn dense_exp_phi1_action(
    matrix: &DenseMatrix,
    scale: f64,
    vector: &[f64],
) -> CoreResult<DenseExpPhi1Action>;
```

- [ ] **Step 1: Write failing dense-equivalence tests**

Use zero, diagonal, Jordan, nonnormal, scale-zero, dimension-error, and nonfinite cases. Compare fused output with current `dense_phi_action(...,0,...)` and `dense_phi_action(...,1,...)` at WRMS `<=5e-14`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p rodas5p-core --test dense_exp_phi1_fusion_contracts --offline --locked
```

- [ ] **Step 3: Implement one augmented exponential**

Build `(n+1)x(n+1)` with top-left `scale*A` and top-right `vector`. One `matrix_exp_pade13` call supplies the phi1 column; multiply its top-left block by `vector` for exponential action.

- [ ] **Step 4: Write projected-counter test and replace two calls**

At one checkpoint require one `phi_projected_exponentials` and one `phi_dense_oracle_calls`, unchanged residual formula, equivalent convergence decision, and no JVP change.

- [ ] **Step 5: Verify consumed compatibility**

```bash
cargo test -p rodas5p-core --test dense_exp_phi1_fusion_contracts --offline --locked
cargo test -p rodas5p-integrators --test v38d_k1_projected_oracle_fusion --offline --locked
cargo test -p rodas5p-integrators --test v37_continuation_transaction_contracts --offline --locked
```

- [ ] **Step 6: Benchmark, audit, and disposition**

Survive only if projected calls fall as expected, endpoint/residual remain in envelope, and normal p95 regression stays within 5%.

### Task 3: K2 reusable workspace

**Files:** Modify `exponential.rs`; add K2 unit/integration tests.

**Internal interface:**

```rust
struct FusedPhiWorkspace {
    work: Vec<f64>,
    reduced_input: Vec<f64>,
    previous_projected: Vec<f64>,
    current_projected: Vec<f64>,
}
```

Crate-private runner is accessed publicly only through `run_v38d_probe(..., V38dCandidateId::K2Workspace)`; no public solver API is added.

- [ ] **Step 1: Write failing allocation characterization and bitwise-parity tests**

The direct comparison lives in an `#[cfg(test)]` unit-test module in `exponential.rs`; public integration tests use the research probe candidate.

- [ ] **Step 2: Implement one workspace per top-level fused action**

Reuse work/reduced/difference buffers across Arnoldi columns, checkpoints, and substeps while preserving arithmetic order. K2 does not flatten basis/Hessenberg.

- [ ] **Step 3: Verify bitwise output/report/counter parity and 64/62/2 replay**

Kill on any drift. Primary signal is allocation count/bytes; wall is secondary.

### Task 4: K3 contiguous basis/Hessenberg layout

**Files:** Modify `exponential.rs`; add K3 tests.

**Internal interfaces:**

```rust
struct FlatBasis { data: Vec<f64>, rows: usize, cols: usize }
struct FlatHessenberg { data: Vec<f64>, rows: usize, cols: usize }
```

- [ ] **Step 1: Write failing indexing and bitwise-parity tests**

Check every small index against nested-vector reference and full-MGS output/counters.

- [ ] **Step 2: Implement flat storage only**

Preserve row traversal, two MGS passes, and Hessenberg update order. Do not merge K2 reuse into this branch.

- [ ] **Step 3: Verify, benchmark, and disposition**

Primary signal is allocation/bytes and orthogonalization traversal wall. Kill on drift or asymptotic memory growth.

### Task 5: C1 checkpoint schedule sweep

**Files:** Modify tournament probe first; modify `exponential.rs` only if adaptive spike is opened.

- [ ] **Step 1: Run fixed `dimension_increment` sweep 1/2/4/6 without production change**

Record JVP, projected calls, residual, convergence, completion, and wall for all common cases/events.

- [ ] **Step 2: Apply information-gain stop rule**

If no fixed variant reduces projected calls enough to explain a 10% event signal without completion loss, disposition is `KILL_NO_SIGNAL`; do not add adaptive code.

- [ ] **Step 3: If opened, write pure bounded adaptive schedule tests**

The schedule uses only residual history/current dimension, returns increments `[1,6]`, falls back to 1 on nonfinite contraction, and never reads endpoint labels or future work.

- [ ] **Step 4: Implement candidate-only schedule, verify, benchmark, disposition**

Authority `dimension_increment=2` remains unchanged.

### Task 6: C2 selective reorthogonalization

**Files:** Modify `exponential.rs`; add C2 tests.

**Private experimental interface:**

```rust
enum V38dExperimentalOrthogonalization { SelectiveMgs }
const V38D_SELECTIVE_MGS_NORM_LOSS_TRIGGER: f64 = 0.5;
```

Existing public `FusedOrthogonalization` and serialization remain unchanged.

- [ ] **Step 1: Write failing trigger tests**

One easy orthogonal column must use one pass; one nearly dependent column must trigger two passes when `after_norm < 0.5 * before_norm`.

- [ ] **Step 2: Implement explicit candidate telemetry**

Record first-pass and triggered-second-pass columns in exploratory report only. Preserve standard orthogonalization work counters.

- [ ] **Step 3: Numerical validation**

Compare Gram defect, residual, endpoint WRMS, and convergence against full MGS on diagonal, Jordan, oscillatory, diffusion, and both tail events; include tighter-tolerance adversarial run.

- [ ] **Step 4: Benchmark and disposition**

Wall gain without bounded Gram/residual behavior is `KILL`; stress-only fail-closed result is `HOLD_RESTRICTED`.

### Task 7: C4 adaptive Krylov-dimension/substep controller spike

**Files:** Modify `exponential.rs`; add C4 tests.

**Pure decision interface:**

```rust
fn choose_krylov_or_substep(
    current_dimension: usize,
    maximum_dimension: usize,
    current_substeps: usize,
    maximum_substeps: usize,
    residual_history: &[f64],
    charged_work: WorkCounters,
) -> V38dKrylovSubstepDecision;
```

Decisions: `ExtendDimension(usize)`, `DoubleSubsteps(usize)`, `StopFailClosed`.

- [ ] **Step 1: Write pure decision tests**

Strong contraction extends dimension; flat/nonfinite contraction increases substeps; bounds never exceed config; no future/endpoint label is accepted.

- [ ] **Step 2: Implement candidate-only runner**

Do not claim basis reuse. Charge every discarded Arnoldi attempt. Keep authority 1→2→4 path unchanged.

- [ ] **Step 3: Validate normal/tail events and disposition**

Primary signal is JVP/restart/substep reduction or greater bounded progress. Kill if work is merely shifted among failed attempts.

### Task 8: Survivor ablation and smallest compatible combination

**Files:**
- Create `results/FIRST_WAVE_ABLATION.json`.
- Create `plots/FIRST_WAVE_ABLATION.png`.
- Update registry/decision ledger.

- [ ] **Step 1: Verify each evidence set**

Require raw samples, work/accuracy CSV, common plots, hard-gate result, one diff review, branch/HEAD/tree, and retry count.

- [ ] **Step 2: Generate ablation without imputation**

Missing/failed samples remain explicit; incomplete evidence cannot outrank a complete survivor.

- [ ] **Step 3: Read plots adversarially**

Check gain concentration, p95 normal regression, defect/work tradeoff, allocation explanation, and order reversal.

- [ ] **Step 4: Assign `SURVIVE`, `HOLD`, or `KILL`**

At most two survivors. A valid result may have none.

- [ ] **Step 5: Combine only compatible isolated survivors**

Expected order: K1, then one of K2/K3 until interaction is measured, then C1 or C2. C4 enters only after isolated work reduction.

- [ ] **Step 6: One independent review and at most one survivor PR**

Claim ceiling remains `PROMISING_PERFORMANCE_CANDIDATE_ON_CONSUMED_EXPLORATORY_PROBES`; merge requires explicit user approval.
