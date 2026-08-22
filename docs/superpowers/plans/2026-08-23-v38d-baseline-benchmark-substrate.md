# v3.8-D Common Exploratory Benchmark Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build one reproducible, machine-readable benchmark substrate for ranking v3.8-D performance candidates without creating timing-authority evidence.

**Architecture:** A new integrator research module defines deterministic matrix-free synthetic cases and produces stable probe reports using the current full-MGS fused-phi authority. A dedicated CLI binary owns allocation instrumentation so the production allocator remains unchanged. Python analysis retains every repetition, computes work/defect distributions, and generates common plots for every candidate worktree.

**Tech Stack:** Rust 1.94.1, existing workspace crates and dependencies, Serde/JSON, Clap, Python 3, CSV, matplotlib.

**Spec:** `docs/superpowers/specs/2026-08-23-v38d-high-entropy-performance-tournament-design.md`

## Global Constraints

- Timing Authority Validator must be merged before this branch is proposed.
- Every report status is `EXPLORATORY_NOT_TIMING_AUTHORITY`.
- Retain one warm-up and seven measured repetitions; never remove failures or slow samples.
- Sequential execution is default; batching is a separate explicit case.
- Full-MGS `fused_phi_action` is the authority candidate.
- No selector, continuation-cap, R-JF, v3.6/v3.7 schema, active-switching, or N=2048 change.
- No speedup claim or new production dependency.

---

### Task 1: Stable probe schema and candidate/case identities

**Files:**
- Create `crates/rodas5p-integrators/src/v38d_performance_tournament.rs`.
- Modify `crates/rodas5p-integrators/src/lib.rs`.
- Create `crates/rodas5p-integrators/tests/v38d_performance_probe_contracts.rs`.

**Interfaces:**

```rust
pub enum V38dProbeCaseId {
    StiffDiagonal96,
    NonnormalJordan96,
    OscillatoryBlocks96,
    DiffusionLike192,
    MixedForcing192,
}

pub enum V38dCandidateId { FullMgsAuthority }

pub struct V38dProbeSample {
    pub repetition: usize,
    pub wall_seconds: f64,
    pub allocations: u64,
    pub allocated_bytes: u64,
    pub work: WorkCounters,
    pub output_checksum: String,
    pub authority_wrms_defect: f64,
    pub residual_estimate: f64,
    pub converged: bool,
}
```

- [ ] **Step 1: Write failing schema/cardinality test**

```rust
#[test]
fn authority_probe_is_explicitly_non_authority_and_retains_samples() {
    let report = run_v38d_probe(
        V38dProbeCaseId::StiffDiagonal96,
        V38dCandidateId::FullMgsAuthority,
        1,
        7,
    ).unwrap();
    assert_eq!(report.schema, "vigilode-v38d-exploratory-probe-v1");
    assert_eq!(report.status, "EXPLORATORY_NOT_TIMING_AUTHORITY");
    assert_eq!(report.measured.len(), 7);
}
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
```

- [ ] **Step 3: Add serializable types and explicit not-implemented runner**

Return `CoreError::InvalidInput("v3.8-D probe case not implemented")`; rerun and confirm behavior-level RED.

- [ ] **Step 4: Commit schema boundary**

```bash
git add crates/rodas5p-integrators
git commit -m "test: define v3.8-D exploratory probe contract"
```

### Task 2: Deterministic matrix-free synthetic operators

**Files:** Modify tournament module/tests.

**Interfaces:** Produces `build_v38d_case(id) -> CoreResult<V38dProbeCase>`.

- [ ] **Step 1: Write failing operator-action tests**

Required cases:

```text
StiffDiagonal96: lambda_i = -(1+i)
NonnormalJordan96: diagonal -8, superdiagonal 40
OscillatoryBlocks96: 2x2 blocks [[-0.1,-omega],[omega,-0.1]], omega=1+i/48
DiffusionLike192: tridiagonal [-2,1,1] scaled by 400
MixedForcing192: diffusion operator plus deterministic sin/cos/polynomial fused vectors
```

Small-dimension test matrices may be explicit; production probes remain closure-based matrix-free operators.

- [ ] **Step 2: Verify RED, implement closures, verify GREEN**

Assert fixed-vector actions and deterministic case checksums twice.

- [ ] **Step 3: Commit**

```bash
git add crates/rodas5p-integrators
git commit -m "feat: add deterministic v3.8-D probe cases"
```

### Task 3: Current full-MGS authority probe

**Files:** Modify tournament module/tests.

**Interfaces:** Produces `run_v38d_probe_once(case, candidate) -> CoreResult<(FusedPhiActionReport, WorkCounters)>`.

- [ ] **Step 1: Write failing deterministic-output test**

Run a nonnormal case twice; require bitwise-identical output and counters, convergence, and zero Jacobian builds/direct factorizations/nonlinear iterations.

- [ ] **Step 2: Implement with existing `fused_phi_action`**

Use the case's current full-MGS config. Hash output IEEE-754 bytes in index order. Authority defect is exactly zero.

- [ ] **Step 3: Verify and commit**

```bash
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-core --test fused_phi_contracts --offline --locked
git add crates/rodas5p-integrators
git commit -m "feat: run full-MGS v3.8-D authority probes"
```

### Task 4: Dedicated allocation-counting probe binary

**Files:**
- Create `crates/rodas5p-cli/src/bin/v38d-performance-probe.rs`.
- Create `crates/rodas5p-cli/tests/v38d_performance_probe_cli_contracts.rs`.

**CLI:**

```text
v38d-performance-probe --case CASE --candidate full-mgs-authority --warmups 1 --repetitions 7 --output REPORT.json
```

- [ ] **Step 1: Write failing CLI test**

Execute the binary and assert schema/status, one warm-up, seven measured samples, source/config identity, and finite positive wall values.

- [ ] **Step 2: Implement binary-local allocator**

```rust
struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size.saturating_sub(old.size()) as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, old, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
```

Reset counters immediately before each probe; never alter a library/production allocator.

- [ ] **Step 3: Verify and commit**

```bash
cargo test -p rodas5p-cli --test v38d_performance_probe_cli_contracts --offline --locked
git add crates/rodas5p-cli
git commit -m "feat: add v3.8-D exploratory probe binary"
```

### Task 5: Normal and known-tail event adapters

**Files:**
- Modify tournament module.
- Modify `g4_s5b0_regime_atlas.rs` only if a read-only filtered adapter is unavailable.
- Modify probe tests.

**Required events:** N96/N192 normal completions, N256/N320 intermediate completions, N192 semilinear attempt 12 exhaustion, N384 semilinear attempt 23 exhaustion.

- [ ] **Step 1: Write failing event-identity tests**

Tail events must be recommended/resumed, charge exactly 80 continuation JVP, and emit no endpoint/admissibility label. Normal events complete.

- [ ] **Step 2: Implement by filtering existing v3.7 reports**

Do not duplicate the solver or recompute retained prefixes outside the existing runner.

- [ ] **Step 3: Verify compatibility and commit**

```bash
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-integrators --test v37_continuation_transaction_contracts --offline --locked
git add crates/rodas5p-integrators
git commit -m "feat: add v3.8-D event probe adapters"
```

### Task 6: Analyzer, exact sample retention, and common plots

**Files:**
- Create `research/generic_v38d_high_entropy_performance_tournament/scripts/analyze_probes.py`.
- Create `scripts/test_analyze_probes.py` and `scripts/plot_probes.py`.
- Create `contracts/V38D_EXPLORATORY_PROBE_CONTRACT.json`.

**Outputs:** summary JSON, event CSV, `WALL_RATIO_BY_CASE.png`, `WORK_COUNTER_DECOMPOSITION.png`, `DEFECT_VS_WORK.png`, `ALLOCATION_RATIO.png`, `TAIL_EVENT_PROGRESS.png`.

- [ ] **Step 1: Write failing analyzer tests**

Reject missing/extra reports, missing repetition, nonfinite wall, changed identity/config, deleted failed samples, nonzero authority defect, and wrong status.

- [ ] **Step 2: Implement exact file/sample validation**

Use retained samples only; no trimming or winsorization. Output sets `timing_authority=false` and `speedup_claim_authorized=false`.

- [ ] **Step 3: Plot from one-row-per-repetition CSV**

Tests compare source row counts/labels rather than image pixels.

- [ ] **Step 4: Verify and commit**

```bash
python -m unittest research/generic_v38d_high_entropy_performance_tournament/scripts/test_analyze_probes.py -v
git add research/generic_v38d_high_entropy_performance_tournament
git commit -m "feat: analyze and plot v3.8-D exploratory probes"
```

### Task 7: Run baseline, read plots, review diff, checkpoint

**Files:** Create `reports/BASELINE_RESULT.md`, `reports/BASELINE_PHYS_MATH_CODE_AUDIT.md`, `results/BASELINE_SUMMARY.json`, `results/ARTIFACT_MANIFEST.json`.

- [ ] **Step 1: Build and run all probes**

```bash
cargo build -p rodas5p-cli --bin v38d-performance-probe --profile measurement --offline --locked
```

Use one immutable run directory; never overwrite reports.

- [ ] **Step 2: Analyze plots as evidence**

Record whether projected dense work, allocations, orthogonalization, or restart/substep waste best explains normal and tail cases. State whether N=384 is qualitatively distinct or only larger.

- [ ] **Step 3: Focused verification**

```bash
cargo fmt --all -- --check
cargo clippy -p rodas5p-integrators -p rodas5p-cli --all-targets --offline --locked -- -D warnings
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-cli --test v38d_performance_probe_cli_contracts --offline --locked
python -m unittest research/generic_v38d_high_entropy_performance_tournament/scripts/test_analyze_probes.py -v
```

- [ ] **Step 4: One independent diff review and checkpoint**

Confirm no candidate implementation, selector change, timing-authority claim, or R-JF mutation entered the substrate branch.

- [ ] **Step 5: Open one substrate PR**

Claim ceiling: reproducible exploratory substrate only.
