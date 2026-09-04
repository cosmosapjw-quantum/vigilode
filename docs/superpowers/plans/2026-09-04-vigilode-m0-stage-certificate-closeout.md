# VigilODE M0 Stage-Certificate Closeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:using-git-worktrees first, then superpowers:executing-plans or superpowers:subagent-driven-development task-by-task. For M0, the repository contract further restricts execution to `LOCAL_CODEX_JOB_ONLY`; no local LLM, candidate execution, or holdout access is permitted.

**Goal:** Close exactly the PR #41 observed formal-scope and software-contract defects under Draft PR #42, produce one integrated fail-closed closeout, and reach an M1 Bateman real-client preflight without executing the real client during M0.

**Architecture:** M0 resumes the immutable predecessor run at its latest valid durable checkpoint. It first restores the unpublished dirty source/proof state into a clean detached worktree at exact PR #42 head, then alternates `R01 formal closure -> C01 software closure -> J01 integrated adjudication -> D01 durable publication`. The formal theorem authority remains Lean/mathlib + Rocq; CAS tools are role-limited cross-checks. Runtime and Git identities are typed and non-substitutable.

**Tech Stack:** Rust/Cargo; serde/serde_json; Python 3 validators; Lean 4 + mathlib; Rocq/coqchk; Wolfram Language, SageMath, Singular as restricted cross-checks; Git/GitHub; SHA-256 manifests.

**Spec:** `docs/superpowers/specs/2026-09-04-vigilode-research-coding-spiral-design.md`

## Global Constraints

- Repository: `cosmosapjw-quantum/vigilode`.
- Scientific successor branch: `research/audit2-stage-certificate-repair-handoff-20260831`.
- Required Draft PR: `#42`, `OPEN / DRAFT / UNMERGED`.
- Inspected PR #42 head at plan compilation: `b3e8165c8dc3b5016702821d280daea1a3f1feb7`.
- Predecessor PR #41 head/tree: `9fbdd84c64e99620805ebf634dcaf57aaad05cbc` / `12a5bb79f94f2cb47d4f808f7254e01af0446cdb`.
- Immutable predecessor run: `/home/cosmosapjw/.local/state/vigilode/stage-certificate-telemetry/20260831T141627Z`.
- Executor: `LOCAL_CODEX_JOB_ONLY`; `local_llm_allowed=false`.
- Candidate executions: exactly `0` throughout M0.
- Holdout: `NOT_OPENED_OR_EXECUTED` throughout M0.
- Claim ceiling: `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`.
- One major objective and one verification bundle per phase.
- One diagnostic retry total across the successor closeout.
- Exactly one final integrated review; no repair after that review.
- Two consecutive process-only cycles terminate `STALL_PROCESS_ACCRETION`.
- `BYTE`, `CONTENT`, `NUMERICAL`, and `SEMANTIC` identity are separate comparators and may not substitute for one another.
- Never `reset`, `clean`, `stash`, `rebase`, overwrite the predecessor run, or checkout across the preserved dirty predecessor worktree.
- Historical checks cannot validate repaired bytes.
- Wolfram failure is `BLOCKED_EXTERNAL`/backend availability evidence, not a universal-theorem failure; Lean/Rocq remain universal authorities.
- No performance, speedup, scalability, M09/M11, production, merge, tag, or release work belongs to M0.
- All shell snippets below are executed inside `bash -lc '...'` or a dedicated script; do not enable fail-fast options in the user's interactive shell.

## Source Authority Note

The latest attempted stage-certificate implementation/proofs are **not published in GitHub**. At PR #42 head, `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs` does not exist. The PR #41 executor explicitly created those bytes in the preserved local dirty worktree before terminating `STOP_INVALID`. Therefore exact line numbers and current function/type declarations cannot be truthfully compiled from the remote repository.

This is not a planning placeholder. It is the primary V0 provenance gate: Task 1 must recover and freeze those exact bytes before any repair. Later tasks use contract-fixed file paths, theorem IDs, and semantic anchors; if recovered source does not contain the expected paths, stop as `SOURCE_OR_PUBLICATION_IDENTITY_UNRESOLVED` instead of inventing replacements.

## Expected recovered implementation/proof paths

These paths were fixed by the predecessor executor and are the only stage-certificate scientific paths this plan expects to recover:

- `crates/rodas5p-integrators/Cargo.toml`
- `crates/rodas5p-integrators/src/lib.rs`
- `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs`
- `crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs`
- `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean`
- `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v`
- `research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl`
- `research/audit2_stage_certificate_telemetry_20260831/formal/sage/check_stage_majorant.sage`
- `research/audit2_stage_certificate_telemetry_20260831/formal/singular/check_jacobi_numerator.sing`

M0 compact result files are restricted to the PR #42 publication contract, including:

- `research/audit2_stage_certificate_repair_20260831/evidence/repair_closeout.json`
- `research/audit2_stage_certificate_repair_20260831/evidence/formal_receipt.json`
- `research/audit2_stage_certificate_repair_20260831/evidence/formal_obligation_abi.json`
- `research/audit2_stage_certificate_repair_20260831/evidence/integrated_closeout_review.json`
- `research/audit2_stage_certificate_repair_20260831/evidence/ANALYSIS.md`
- `research/audit2_stage_certificate_repair_20260831/evidence/SHA256SUMS`

---

### Task 1: V0 bind, checkpoint, and recover the unpublished predecessor bytes

**Files:**
- Read only: `research/audit2_stage_certificate_repair_20260831/CODEX_START_HERE.md`
- Read only: `research/audit2_stage_certificate_repair_20260831/EXECUTION_CONTRACT.json`
- Read only: `research/audit2_stage_certificate_repair_20260831/FORMAL_SCOPE.md`
- Read only: `research/audit2_stage_certificate_repair_20260831/HANDOFF_INPUT_LOCK.json`
- Read only: predecessor run `/home/cosmosapjw/.local/state/vigilode/stage-certificate-telemetry/20260831T141627Z/**`
- Recover: only checkpointed stage-certificate implementation/test/formal/evidence paths.

**Interfaces:**
- Consumes: exact PR #42 head and immutable predecessor run.
- Produces: a fresh successor external run directory, a clean detached worktree at exact PR #42 head, `RECOVERY_INVENTORY.json`, and byte/semantic replay evidence for all recovered paths.

- [ ] **Step 1: Verify the published control plane before touching the dirty worktree**

Run from a normal clone:

```bash
bash -lc '
set -euo pipefail
python3 tools/validate_audit2_stage_certificate_repair_handoff.py
python3 tools/test_audit2_stage_certificate_repair_handoff.py -v
git fetch --no-tags origin research/audit2-stage-certificate-repair-handoff-20260831
HEAD=$(git rev-parse origin/research/audit2-stage-certificate-repair-handoff-20260831)
printf "successor_head=%s\n" "$HEAD"
test "$HEAD" = "b3e8165c8dc3b5016702821d280daea1a3f1feb7"
'
```

Expected: both Python commands PASS and successor head equals the plan-bound SHA. If the remote head moved, do **not** silently update this plan; bind the new head, compare it against this head, and stop `PROCESS_DRIFT_DETECTED` unless the change is an authorized successor commit covered by the PR #42 contract.

- [ ] **Step 2: Verify predecessor run hashes and locate the preserved dirty worktree without mutating it**

```bash
bash -lc '
set -euo pipefail
RUN=/home/cosmosapjw/.local/state/vigilode/stage-certificate-telemetry/20260831T141627Z
test -d "$RUN"
sha256sum "$RUN/manifest.json" "$RUN/SHA256SUMS" || true
python3 - "$RUN/terminal/command_readback.json" <<'"'"'PY'"'"'
import json, pathlib, sys
p = pathlib.Path(sys.argv[1])
data = json.loads(p.read_text())

def walk(x):
    if isinstance(x, dict):
        for k, v in x.items():
            if isinstance(v, str) and ("worktree" in k.lower() or "worktree" in v.lower()):
                print(f"{k}={v}")
            walk(v)
    elif isinstance(x, list):
        for v in x:
            walk(v)
walk(data)
PY
'
```

Expected external hashes are frozen by PR #42: manifest `3812075cc7457f1797d512fde55b49b3202053d257e8a350a6cf48c2cea5029f`; `SHA256SUMS` file `057a1f5ccafc5889684caa72af0cd716edc3e04ec0f58b2b771a3971baeeb3d8`. The local executor must compare the exact stored objects according to the predecessor manifest rather than assuming filename hashing semantics.

- [ ] **Step 3: Create a fresh successor run and a clean detached worktree**

```bash
bash -lc '
set -euo pipefail
ROOT=${XDG_STATE_HOME:-$HOME/.local/state}/vigilode/stage-certificate-repair
RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)
NEW_RUN="$ROOT/$RUN_ID"
test ! -e "$NEW_RUN"
mkdir -p "$NEW_RUN"
WT=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-stage-cert-repair.XXXXXX")
git worktree add --detach "$WT" b3e8165c8dc3b5016702821d280daea1a3f1feb7
printf "%s\n" "$NEW_RUN" > "$NEW_RUN/RUN_PATH"
printf "%s\n" "$WT" > "$NEW_RUN/WORKTREE_PATH"
git -C "$WT" status --porcelain=v2 -z > "$NEW_RUN/clean_start_status.z"
test ! -s "$NEW_RUN/clean_start_status.z"
'
```

Expected: clean detached worktree at exact PR #42 head. If the user's local environment already supplies a native isolated-worktree mechanism, use it instead and record the equivalent exact HEAD/tree.

- [ ] **Step 4: Snapshot the preserved dirty worktree before any replay**

Record, externally only:

```bash
bash -lc '
set -euo pipefail
DIRTY_WORKTREE=$(cat /path/from/command_readback)
NEW_RUN=$(cat /path/to/current/RUN_PATH)
git -C "$DIRTY_WORKTREE" rev-parse HEAD > "$NEW_RUN/predecessor_dirty_HEAD"
git -C "$DIRTY_WORKTREE" rev-parse HEAD^{tree} > "$NEW_RUN/predecessor_dirty_tree"
git -C "$DIRTY_WORKTREE" status --porcelain=v2 -z > "$NEW_RUN/predecessor_dirty_status.z"
git -C "$DIRTY_WORKTREE" diff --binary > "$NEW_RUN/predecessor_tracked.diff"
git -C "$DIRTY_WORKTREE" diff --cached --binary > "$NEW_RUN/predecessor_staged.diff"
'
```

`/path/from/command_readback` and `/path/to/current/RUN_PATH` above denote shell variables resolved from Step 2 and Step 3 outputs, not literal paths to create. Do not add a new repository file to store them.

For each allowed untracked path reported by porcelain-v2, copy content into the new external run and record SHA-256 + byte size. If any unrelated/unexplained dirty path appears, terminate `SOURCE_OR_PUBLICATION_IDENTITY_UNRESOLVED`.

- [ ] **Step 5: Replay only stage-certificate paths into the clean worktree**

Rules:

```text
non-overlap path: exact byte copy + SHA256_EXACT
path changed both in PR #42 and dirty predecessor: explicit three-way merge using
  base = PR #41 head version
  before = preserved dirty predecessor bytes
  after = PR #42 control-head bytes
then record base/before/after/result hashes and a semantic merge rationale
```

Do not copy harness archives, raw logs, compiler products, caches, datasets, or dependencies into Git.

- [ ] **Step 6: Assert the expected recovered scientific paths exist**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
for p in \
  crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs \
  crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs \
  research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean \
  research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v \
  research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl \
  research/audit2_stage_certificate_telemetry_20260831/formal/sage/check_stage_majorant.sage \
  research/audit2_stage_certificate_telemetry_20260831/formal/singular/check_jacobi_numerator.sing
do
  test -f "$WT/$p"
done
'
```

If any expected path is absent, stop. Do not regenerate it from memory or from this plan.

- [ ] **Step 7: Commit nothing yet**

Task 1 ends with an external durable checkpoint only. `progress_delta = RECOVERED_UNPUBLISHED_SCIENTIFIC_BYTES`; no Git commit is made until RED evidence exists.

---

### Task 2: R01 RED — prove the current formal source is still too narrow

**Files:**
- Modify later: `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean`
- Modify later: `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v`
- External probes only: `$NEW_RUN/formal_probes/**`

**Interfaces:**
- Consumes: recovered formal source bytes from Task 1.
- Produces: failing arbitrary-`n` external consumers for F01/F03/F04 plus assumption-audit evidence.

- [ ] **Step 1: Freeze recovered proof-source hashes**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
sha256sum \
 "$WT/research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean" \
 "$WT/research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v" \
 > "$NEW_RUN/formal_pre_repair.sha256"
'
```

- [ ] **Step 2: Inspect theorem declarations and require the contract-fixed theorem IDs**

The recovered source must export or be repaired to export exactly these universal theorem IDs:

```text
f01_strict_lower_nilpotent
f01_finite_neumann_left_inverse
f01_finite_neumann_right_inverse
f03_residual_solution_norm_bound
f04_stage_majorant_nonnegative
f04_stage_majorant_dominates
f04_weighted_contamination_bound
f05_safe_accept
f05_safe_reject
```

F02 remains regression-only and must not be promoted to a universal theorem.

If recovered source uses different public theorem names, do not silently rename the contract. Record the mismatch as `FORMAL_CHECK_FAILED` unless the old names are private helpers and the exact required ABI can be added without changing theorem meaning.

- [ ] **Step 3: Create external Lean and Rocq generic consumers that quantify over dimension**

The probes must import the recovered project module but live outside Git. They must keep `n` symbolic rather than replacing the theorem with a `Fin 3` fixture. At minimum they instantiate the exported ABI in an environment where `n` is an arbitrary positive natural and separately compile a concrete `n != 3` witness to kill hidden fixed-dimension assumptions.

Required RED outcomes before repair:

```text
F01: current fixed-Fin3 or insufficiently quantified theorem cannot satisfy generic consumer
F03: current theorem cannot discharge arbitrary b,x,rho=b-Wx with pointwise inverse bound
F04: current fixed-stage theorem cannot discharge arbitrary-stage nonnegative recurrence + weighted bounds
```

Record compiler argv, cwd, exit, stdout/stderr hashes, and source hashes externally. A zero exit without generic quantifier/assumption evidence is itself a failing probe.

- [ ] **Step 4: Run assumption audits on the pre-repair declarations**

Lean must use `#print axioms` for every exported theorem. Rocq must use `Print Assumptions` plus `coqchk`. Any `sorry`, `Admitted`, project-local axiom, or unexplained assumption is RED.

- [ ] **Step 5: Preserve the RED checkpoint**

Write an external `R01_RED.json` with each of F01/F03/F04 classified as `EXPECTED_FAIL` only when the failure is the intended quantifier/scope defect. Environment/tool failure is `FORMAL_BACKEND_UNAVAILABLE`, not RED success.

---

### Task 3: R01 GREEN — repair F01 arbitrary-dimension strict-lower nilpotency and two-sided Neumann inverse

**Files:**
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean`
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v`
- Cross-check only: `research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl`

**Interfaces:**
- Consumes: recovered matrix representation and strict-lower predicate.
- Produces: generic theorem ABI for `f01_strict_lower_nilpotent`, `f01_finite_neumann_left_inverse`, `f01_finite_neumann_right_inverse`.

- [ ] **Step 1: Implement F01 with `n >= 1` as a quantified theorem, not a generated fixture**

The mathematical content is fixed:

```text
For arbitrary strict-lower T on Fin n:
  T^n = 0.
Let S = sum_{k=0}^{n-1} T^k.
Then (I-T)S = I and S(I-T) = I.
```

Preferred proof shape in both provers:

1. establish that a product contributing to `(T^k)_{ij}` requires a strictly descending index chain of length `k`;
2. no chain of length `n` exists in `Fin n`, hence `T^n=0`;
3. use finite geometric telescoping
   `(I-T) S = I - T^n` and `S (I-T) = I - T^n`;
4. substitute nilpotency.

Do not replace the general proof with symbolic determinant inversion, a `3x3` expansion, or numeric examples.

- [ ] **Step 2: Keep the Wolfram role limited**

If Wolfram is available, `check_stage_certificate.wl` may check exact finite geometric identities and declared finite fixtures. It must not emit or be recorded as universal proof authority. If Wolfram is unavailable, preserve the backend failure; do not weaken Lean/Rocq requirements.

- [ ] **Step 3: Re-run the generic external consumers and assumption audits**

Expected: arbitrary-`n` consumer PASS, an `n != 3` witness PASS, Lean `#print axioms` clean under allowed foundational axioms, Rocq `Print Assumptions`/`coqchk` clean, no `sorry`/`Admitted`.

- [ ] **Step 4: Commit F01 only after its isolated bundle is green**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git add \
  research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean \
  research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v \
  research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl
git commit -m "proof: generalize stage Neumann closure"
'
```

If Wolfram source was byte-identical and unchanged, omit it from `git add`.

---

### Task 4: R01 GREEN — repair F03 residual-to-solution bound without conflating RHS and residual

**Files:**
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean`
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v`
- Optional role-limited cross-check: `research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl`

**Interfaces:**
- Consumes: arbitrary two-sided inverse `V=W^{-1}`, vectors `b,x`, `rho=b-Wx`, `kappa>=0`, and pointwise bound `||Vv|| <= kappa ||v||`.
- Produces: `f03_residual_solution_norm_bound` with all three contract conclusions.

- [ ] **Step 1: Encode the algebraic identity first**

The proof must explicitly use

```text
rho = b - W x
V b = x + V rho
```

because `V W x = x` by the two-sided inverse assumption. `b` and `rho` remain distinct variables throughout the theorem statement and proof.

- [ ] **Step 2: Derive both norm inequalities from the pointwise inverse bound**

Required conclusions:

```text
||V b - x|| <= kappa ||rho||
||V b|| <= ||x|| + kappa ||rho||
```

Use the algebraic identity, the hypothesis `forall v, ||V v|| <= kappa ||v||`, and the triangle inequality. Do not replace the pointwise hypothesis with a fixture-specific condition number.

- [ ] **Step 3: Add a negative external probe that swaps `b` and `rho`**

The mutant theorem/application must fail to typecheck or fail its proof. Preserve this as the explicit guard against the predecessor F03 semantic mistake.

- [ ] **Step 4: Run Lean/Rocq generic consumers and assumption audits**

Expected: generic PASS, swap mutant FAIL, no forbidden assumptions.

- [ ] **Step 5: Commit F03 independently**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git add \
  research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean \
  research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v \
  research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl
git commit -m "proof: generalize residual solution bound"
'
```

Again omit unchanged cross-check source.

---

### Task 5: R01 GREEN — repair F04 arbitrary-stage nonnegative majorant and weighted contamination bounds

**Files:**
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean`
- Modify: `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v`
- Modify if needed for exact generated-dimension checks: `research/audit2_stage_certificate_telemetry_20260831/formal/sage/check_stage_majorant.sage`

**Interfaces:**
- Consumes: arbitrary finite `n>=1`, nonnegative strict-lower `A`, `q>=0`, admissible componentwise error `d`, and nonnegative weight vectors `alpha,beta`.
- Produces: `f04_stage_majorant_nonnegative`, `f04_stage_majorant_dominates`, `f04_weighted_contamination_bound`.

- [ ] **Step 1: Define the recurrence generically**

Contract recurrence:

```text
z_i = q_i + sum_{j<i} A_ij z_j
```

The proof representation may use the recovered project's vector/matrix conventions, but the theorem domain must remain arbitrary finite dimension and strict-lower support.

- [ ] **Step 2: Prove nonnegativity and finite-Neumann representation**

Required facts:

```text
z >= 0
z = (sum_{k=0}^{n-1} A^k) q
```

Use F01's finite-Neumann closure rather than a new fixed-stage inversion proof.

- [ ] **Step 3: Prove componentwise domination and weighted bounds**

For every admissible nonnegative error `d` satisfying the declared one-step/componentwise inequalities, prove

```text
d_i <= z_i
alpha dot d <= alpha dot z
beta dot d <= beta dot z
```

for arbitrary nonnegative `alpha,beta`. The weight theorem must quantify over the weights; hard-coded endpoint/estimator coefficients are insufficient.

- [ ] **Step 4: Use Sage only as generated exact cross-check**

Run exact rational fixtures for multiple dimensions including at least one `n != 3`. Sage evidence is supplemental and cannot replace the Lean/Rocq generic theorem ABI.

- [ ] **Step 5: Run the fixed-dimension mutants**

`LEAN_F01_FIXED_FIN3`, `ROCQ_F04_FIXED_DIM3`, and `SAGE_F04_ONLY_DIM3` must be killed by the verifier/probes.

- [ ] **Step 6: Commit F04 independently**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git add \
  research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean \
  research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v \
  research/audit2_stage_certificate_telemetry_20260831/formal/sage/check_stage_majorant.sage
git commit -m "proof: generalize stage contamination majorant"
'
```

---

### Task 6: R01 formal receipt — bind exactly 13 backend-role records and the real-to-binary64 bridge

**Files:**
- Create at publication time: `research/audit2_stage_certificate_repair_20260831/evidence/formal_receipt.json`
- Create at publication time: `research/audit2_stage_certificate_repair_20260831/evidence/formal_obligation_abi.json`
- Formal source remains in recovered telemetry paths.

**Interfaces:**
- Consumes: Tasks 3–5 green proof source and existing F02/F05 regression source.
- Produces: exactly 13 unique backend-role records plus theorem ABI/signature/source/probe/assumption hashes.

- [ ] **Step 1: Execute exactly the frozen backend-role matrix**

```text
F01: Lean, Rocq
F02: Wolfram, Sage, Singular(numerator-only)
F03: Lean, Rocq, Wolfram(scalar cross-check)
F04: Lean, Rocq, Sage(generated-dimension cross-check)
F05: Lean, Rocq
```

Total: exactly 13 records. Missing, duplicate, unknown, or role-escalated record is failure.

- [ ] **Step 2: Execute the directed-rounding bridge check**

The formal receipt must distinguish exact real inequalities from the binary64 implementation that computes conservative nonnegative upper bounds. The bridge does not prove all floating-point code correct by theorem; it records the mathematical inequality required of the implementation and delegates bit-level boundary behavior to C01 tests.

- [ ] **Step 3: Bind all formal evidence**

For each role record include at least source SHA-256, compiler/tool version, exact argv/cwd, exit status, stdout/stderr SHA-256 and byte count, theorem signature/ABI hash where applicable, external probe hash, and assumption-audit result.

- [ ] **Step 4: Run formal fail-closed mutants**

The required mutation classes from the execution contract must all be rejected, including fixed-Fin3/fixed-dim3, sorry/Admitted, numeric-only CAS promotion, role escalation, missing/duplicate record, hash tamper, and exit-zero-without-scope evidence.

- [ ] **Step 5: Do not commit the receipt yet**

R01 evidence remains external until C01 and integrated verification are green. This prevents a stale formal PASS receipt from being reused after subsequent byte changes.

---

### Task 7: C01 RED — restore and extend the focused Rust contract tests

**Files:**
- Modify: `crates/rodas5p-integrators/Cargo.toml`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Modify: `crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs`
- Later modify: `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs`

**Interfaces:**
- Consumes: recovered source/test state.
- Produces: focused failing tests for exactly the observed C01 defects.

- [ ] **Step 1: Verify the recovered feature remains opt-in and isolated**

The recovered Cargo delta must define `audit2-stage-certificate` as depending on `audit2-research`, not `audit2-bateman-authority`. `src/lib.rs` must gate the stage-certificate module/export under that feature. If recovery instead altered default features or candidate/holdout imports, stop `SOFTWARE_CONTRACT_FAILED` before repair.

- [ ] **Step 2: Add/confirm RED tests for total Arnoldi cap semantics**

Required cases:

```text
MAX_ARNOLDI_EQUALITY: iterations == max_arnoldi -> legal
MAX_ARNOLDI_EXCEEDED: iterations == max_arnoldi + 1 -> reject
MULTI_ROW_ONE_EXCEEDS: one completed row exceeds -> whole admission rejects
INDEPENDENT_FROM_ITERATION_LIMIT: max_arnoldi violation rejects even when below iteration_limit
RESTART_NOT_TOTAL_CAP: restart length is per-cycle and cannot substitute for total Arnoldi count
RESIDUAL_HISTORY_NOT_COUNTER: residual-history length cannot satisfy the cap check
```

The authoritative counter is `LinearSolveReport.iterations` unless the recovered trace field demonstrably means something else, in which case add an explicit `arnoldi_vectors` field and bind its semantics. Do not infer the count from residual history.

- [ ] **Step 3: Add/confirm receipt JSON round-trip RED test**

The test must serialize the concrete receipt type with `serde_json`, deserialize back into the same type, assert structural equality, canonical reserialize, and verify all plan/trace/provenance hashes, coefficient digest, norm/scale bits, operator/preconditioner identities, RHS digests, residual completeness/history, restart/max-Arnoldi/iterations, work counters, authority, decision, and partial failure state survive exactly.

- [ ] **Step 4: Add/confirm directed-rounding boundary RED tests**

Required cases:

```text
ROUNDING_EXACT_ZERO
ROUNDING_EXACT_REPRESENTABLE
ROUNDING_INEXACT_UPWARD
PRODUCT_OVERFLOW_REJECTED
SUM_OVERFLOW_REJECTED
```

Exact zero and exact representable operations must not be bumped by one ULP. Inexact nonnegative operations must be rounded upward. Overflow becomes `+infinity`, followed by typed nonfinite rejection with no decision.

- [ ] **Step 5: Run only the focused test target and preserve RED**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
set +e
cargo test --locked -p rodas5p-integrators --features audit2-stage-certificate --test audit2_stage_certificate_contracts > "$NEW_RUN/c01_red.stdout" 2> "$NEW_RUN/c01_red.stderr"
rc=$?
set -e
printf "%s\n" "$rc" > "$NEW_RUN/c01_red.exit"
test "$rc" -ne 0
'
```

Expected: failures correspond to the observed defect classes only. Compile/environment failure is not accepted as RED evidence.

---

### Task 8: C01 GREEN — enforce `max_arnoldi` independently and per completed row

**Files:**
- Modify: `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs`
- Test: `crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs`

**Interfaces:**
- Consumes: frozen plan/trace and completed linear-solve reports.
- Produces: fail-closed admission logic implementing `TOTAL_ARNOLDI_VECTOR_CAP_PER_COMPLETED_GMRES_LINEAR_SOLVE_TRACE_ROW`.

- [ ] **Step 1: Use the actual completed-row Arnoldi count**

Implement the smallest change so every completed decision-bearing row checks

```text
observed_arnoldi_vectors <= frozen_plan.max_arnoldi
```

Equality is legal. Any completed row with `max+1` rejects before synthetic accept/reject disposition is emitted.

- [ ] **Step 2: Keep the check independent of `iteration_limit` and restart cycle length**

Do not combine caps as `min(max_arnoldi, iteration_limit)` and do not treat restart length as total work. Existing iteration-limit semantics remain unchanged.

- [ ] **Step 3: Do not use residual-history length**

Residual-history completeness remains a separate evidence/admission requirement. It is not the Arnoldi counter.

- [ ] **Step 4: Run focused tests**

Expected: every Arnoldi-cap case from Task 7 passes; unrelated stage-certificate tests remain green.

- [ ] **Step 5: Commit only the Arnoldi repair + its tests**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git add crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs \
        crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs
git commit -m "fix: enforce stage certificate Arnoldi cap"
'
```

---

### Task 9: C01 GREEN — make receipt JSON round-trip a concrete contract

**Files:**
- Modify: `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs` only if derive/field serialization changes are required.
- Test: `crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs`

**Interfaces:**
- Consumes: concrete receipt type.
- Produces: lossless same-type serde_json round-trip and canonical content equality.

- [ ] **Step 1: Add only the serde traits required for the concrete receipt graph**

Do not introduce a parallel DTO unless the existing type graph cannot serialize without changing public scientific semantics. If a parallel DTO becomes necessary, stop and classify as plan drift rather than silently widening scope.

- [ ] **Step 2: Round-trip the same concrete type**

Test sequence:

```text
receipt
 -> serde_json::to_value / to_vec
 -> serde_json::from_slice::<SameReceiptType>
 -> assert structural equality
 -> canonical reserialization
 -> assert canonical CONTENT identity
```

- [ ] **Step 3: Assert all provenance and partial-failure fields**

Do not settle for `decision` equality alone. The full retained-field list from the execution contract must survive.

- [ ] **Step 4: Run focused tests and commit**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
cargo test --locked -p rodas5p-integrators --features audit2-stage-certificate --test audit2_stage_certificate_contracts
git add crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs \
        crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs
git commit -m "test: bind stage certificate receipt roundtrip"
'
```

---

### Task 10: C01 GREEN — correctly upward-round nonnegative binary64 add/multiply

**Files:**
- Modify: `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs`
- Test: `crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs`

**Interfaces:**
- Consumes: nonnegative finite binary64 operands used in certificate upper-bound arithmetic.
- Produces: correctly conservative add/multiply helpers and typed nonfinite rejection.

- [ ] **Step 1: Preserve exact results exactly**

For `0+x`, `x+0`, multiplication by zero, and every exactly representable positive sum/product covered by the tests, return the ordinary binary64 result unchanged. Do not unconditional-`next_up`.

- [ ] **Step 2: Detect whether the rounded-to-nearest result is below the exact real result**

Use a method that is demonstrably exact for binary64 operands, such as an error-free transformation / exact decomposition suitable for nonnegative add and multiply, then bump to the next representable value **only** when nearest rounding is downward relative to the exact real result. The implementation must not depend on ambient hardware rounding-mode mutation.

- [ ] **Step 3: Handle overflow fail closed**

If exact sum/product exceeds finite binary64 range, produce `+infinity`; the existing certificate validation must then return the typed nonfinite failure and no accept/reject decision.

- [ ] **Step 4: Add one adversarial mutant**

Temporarily replace conditional upward correction by unconditional `next_up`; `ROUNDING_EXACT_REPRESENTABLE` must fail. Restore source bytes and record both mutant and restoration hashes.

- [ ] **Step 5: Run focused tests and commit**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
cargo test --locked -p rodas5p-integrators --features audit2-stage-certificate --test audit2_stage_certificate_contracts
git add crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs \
        crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs
git commit -m "fix: round certificate bounds upward"
'
```

---

### Task 11: J01 integrated verification on one frozen source snapshot

**Files:**
- Read/execute: repaired Rust + formal source.
- Create only after successful run: compact evidence files under `research/audit2_stage_certificate_repair_20260831/evidence/`.

**Interfaces:**
- Consumes: all R01/C01 commits.
- Produces: one source snapshot hash, one integrated command/evidence bundle, no stale receipt reuse.

- [ ] **Step 1: Freeze exact source identity before integrated execution**

Record HEAD/tree, `git status --porcelain=v2 -z`, all repaired source hashes, Cargo.lock hash, formal source hashes, and plan/trace authority hashes. Worktree must contain only allowed evidence outputs after execution.

- [ ] **Step 2: Run all 13 formal roles from repaired bytes**

Rebuild `formal_receipt.json` from scratch. Old predecessor formal receipts remain historical and may not be copied as PASS.

- [ ] **Step 3: Run the focused Rust suite**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
cargo test --locked -p rodas5p-integrators --features audit2-stage-certificate --test audit2_stage_certificate_contracts
'
```

- [ ] **Step 4: Run candidate-free readiness and feature/default isolation**

Use the repository's current `tools/check-audit2-readiness.sh` plus explicit stage-certificate feature checks. No command may run the Bateman candidate example; `cargo check` of an example is allowed only where the frozen readiness contract already permits compile-only checking.

- [ ] **Step 5: Run static quality gates**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
cargo check --locked -p rodas5p-integrators --no-default-features
cargo check --locked -p rodas5p-integrators --features audit2-stage-certificate
cargo clippy --locked -p rodas5p-integrators --all-targets --features audit2-stage-certificate -- -D warnings
cargo fmt --all -- --check
git diff --check
'
```

- [ ] **Step 6: Compile compact evidence and validate publication policy**

`repair_closeout.json` must distinguish formal, software, provenance, and packaging validity. `formal_obligation_abi.json` binds the generic theorem signatures/probes. `SHA256SUMS` covers checked-in compact evidence. Raw logs and compiler products remain external.

- [ ] **Step 7: No retry unless this is the one permitted diagnostic retry**

A diagnostic retry may diagnose an execution-environment issue but may not change scientific thresholds or silently regenerate a different plan. Record `retry_used` explicitly.

---

### Task 12: J01 one-shot integrated review

**Files:**
- Create: `research/audit2_stage_certificate_repair_20260831/evidence/integrated_closeout_review.json`
- Review snapshot: exact frozen J01 HEAD/tree and source hashes.

**Interfaces:**
- Consumes: J01 integrated evidence.
- Produces: exactly one review event with `formal_scope` and `software_contract` lenses.

- [ ] **Step 1: Freeze the review snapshot**

No source/test/proof mutation after this point in the node.

- [ ] **Step 2: Review formal scope adversarially**

Required checks include arbitrary dimension, F01 both inverse directions, F03 `b` versus `rho`, F04 arbitrary nonnegative weights, assumption audits, exact 13 role records, no CAS role escalation, and no exit-zero-as-scope inference.

- [ ] **Step 3: Review software contract adversarially**

Required checks include total Arnoldi count semantics, per-row enforcement, independence from iteration limit/restart, receipt structural round-trip, exact-zero/exact-representable rounding, inexact upward rounding, overflow typed rejection, failure preservation, and feature/candidate isolation.

- [ ] **Step 4: Apply terminal rule without post-review repair**

```text
zero P0/P1 -> continue to D01
any P0/P1 -> STOP_INVALID
```

No second review and no repair in this node.

---

### Task 13: D01 compact publication and remote readback

**Files:**
- Commit only allowed repaired source/tests/proofs and compact evidence.
- Do not commit raw logs, build products, archives, datasets, caches, or forbidden suffixes.

**Interfaces:**
- Consumes: zero-P0/P1 review and integrated green bundle.
- Produces: successor commits pushed only to PR #42 branch and exact GitHub readback.

- [ ] **Step 1: Validate size/suffix/path policy**

Single research file <= 262144 bytes; total repair research directory <= 2000000 bytes; forbidden suffix list remains empty.

- [ ] **Step 2: Commit compact evidence separately from implementation/proofs**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git add research/audit2_stage_certificate_repair_20260831/evidence/repair_closeout.json \
        research/audit2_stage_certificate_repair_20260831/evidence/formal_receipt.json \
        research/audit2_stage_certificate_repair_20260831/evidence/formal_obligation_abi.json \
        research/audit2_stage_certificate_repair_20260831/evidence/integrated_closeout_review.json \
        research/audit2_stage_certificate_repair_20260831/evidence/ANALYSIS.md \
        research/audit2_stage_certificate_repair_20260831/evidence/SHA256SUMS
git commit -m "research: bind stage certificate repair evidence"
'
```

- [ ] **Step 3: Push non-force only to the existing successor branch**

```bash
bash -lc '
set -euo pipefail
WT=$(cat "$NEW_RUN/WORKTREE_PATH")
cd "$WT"
git push origin HEAD:research/audit2-stage-certificate-repair-handoff-20260831
'
```

No merge, tag, release, other PR, Jira, or Confluence mutation.

- [ ] **Step 4: Read back GitHub authority**

Verify remote head/tree/ancestry, PR #42 remains OPEN/DRAFT/UNMERGED, and all required checks correspond to the final repaired head. Do not infer overall GREEN from local tests alone.

- [ ] **Step 5: Emit D01 disposition**

Success is exactly `REPAIR_CLOSEOUT_VERIFIED`. Otherwise preserve the appropriate PR #42 terminal disposition. `candidate_executions=0` and holdout state must be restated.

---

### Task 14: M1 Bateman preflight only — open the next substantive node after verified M0

**Files:**
- Read only: existing Bateman authority/runner/adjudicator under `research/audit2_real_client_authority_construction_20260830/` and subsequent local-validation control directories.
- Do not execute candidate in this task.

**Interfaces:**
- Consumes: D01 `REPAIR_CLOSEOUT_VERIFIED` and exact final #42 readback.
- Produces: one M1 execution contract or plan whose first scientific action is the already-authorized Bateman real-client run; no new governance stack.

- [ ] **Step 1: Gate on D01**

If M0 is not `REPAIR_CLOSEOUT_VERIFIED`, M1 remains closed.

- [ ] **Step 2: Rebind the Bateman exact implementation/authority bytes**

Use the existing Bateman authority construction and local-validation contracts; do not rewrite the client or fit thresholds.

- [ ] **Step 3: Identify the runtime telemetry needed for the first research return loop**

At minimum preserve per-stage residual/solve information needed to assess a bound of the form

```text
q_i >= ||W^{-1} r_i||
```

under the declared norm/certificate mechanism, and propagate stage contamination to endpoint/embedded-estimator enclosures.

- [ ] **Step 4: Keep the decision semantics tri-valued**

The M1 plan must support certified accept, certified reject, and typed inconclusive. No observed run may be used to widen a predeclared threshold.

- [ ] **Step 5: Stop the planning loop and hand off execution**

M1 preflight is the last planning action here. The next cycle must execute or become honestly blocked; creating another process-only planning/audit layer would violate the two-process-only-cycle rule.

---

## Plan Self-Review Result

### Spec coverage

- V0 exact authority + durable checkpoint: Task 1.
- R01 F01/F03/F04 quantified closure: Tasks 2–6.
- C01 RED/minimal repair: Tasks 7–10.
- One integrated verification bundle: Task 11.
- Exactly one review/no post-review repair: Task 12.
- Compact publication/readback: Task 13.
- Direct transition toward real-client execution: Task 14.
- Typed identity, failure preservation, anti-meta-loop, claim ceiling, candidate/holdout prohibition: Global Constraints + relevant tasks.

### Placeholder scan

No scientific theorem, tolerance, role, path, or acceptance criterion is left as `TBD`/`TODO`. The only runtime-resolved values are paths already required to be read from authoritative predecessor/current-run receipts. Exact source line numbers are intentionally not fabricated because the current scientific bytes are unpublished; semantic theorem IDs and file paths are fixed instead.

### Type/semantic consistency

- F03 keeps `b` and `rho` distinct throughout.
- F01/F04 dimension domain is arbitrary finite `n>=1`.
- `max_arnoldi` remains total Arnoldi vectors per completed solve row, independent of restart and iteration limit.
- Formal and binary64 evidence remain separate layers.
- M0 never executes the Bateman candidate.

## Execution Handoff

For ordinary Superpowers work there are two modes: subagent-driven or inline execution. For this M0, the repository contract overrides that choice: the scientific repair must run as the existing `LOCAL_CODEX_JOB_ONLY` successor using the preserved local predecessor worktree. This ChatGPT session can continue to provide literature analysis, theorem derivations, review of returned receipts/diffs, and GitHub readback, but it must not pretend to have executed the local proof/Rust toolchains or recovered the dirty worktree.
