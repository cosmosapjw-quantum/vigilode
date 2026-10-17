# Preregistration: Route-wise Independent Implementation & Promotion Decision (LP10)

Node LP10 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP10`). Registered on branch `pr/lp10-independent-decision` (base `a7e146d`). It depends on `LP02` (merged in `L-0119`) and `LP09` (merged in `L-0122`).

**Claim boundary.**
- Independent engineering assessment and promotion decision on Track A (Loop12 packet pipeline).
- Scope of finished routes: Track A recommended first path (`LP01` collective model, `LP02` single-window packet, `LP03` damped moments, `LP04` adaptive multi-window trajectory, `LP09` fair benchmark).
- Verifies mathematical regularity, strict Argon zero-allocation compliance in hot loops, outward `Interval` defect bounding, and complete online elimination of Jacobian evaluations, LU factorizations, and linear solves.
- Records explicit dispositions for remaining DAG nodes:
  - `LP05`, `LP07`, `LP08`: Deferred optional branches.
  - `LP06`: `EXISTING_NONLINEAR_REAL_CLIENT` remains `NOT_ESTABLISHED` (scientific hygiene strictly preserved).
  - `LP11`: `SOURCE_VERIFIED_ALREADY_PRESENT` (`shared_shift_jet.rs:363-375`).
  - `LP12`: `BLOCKED_EXTERNAL_SOURCE`.
- Decision rule:
  - Promote Track A collective packet pipeline as a domain-specialized certified solver (`PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT`).
  - Production default ODE solver for general black-box initial value problems remains `rodas5p_fast` / `rodas5p_mf_fast`.
  - Timing authority remains `"diagnostic_only"` under `R2-STAT-01` and `R4-STAT-DEV-01`.

## Question

Can the independent decision review for LP10:
1. Verify the end-to-end mathematical and numerical compliance of Track A nodes (`LP01`, `LP02`, `LP03`, `LP04`, `LP09`);
2. Formalize clear, immutable dispositions for all remaining Track A nodes without inventing synthetic real clients;
3. Formulate a rigorous promotion verdict (`PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT`) that protects general production default stability while enabling certified specialized packet acceleration;
4. Enforce timing authority hold requirements without unwarranted wall-clock promotion?

## Specification

### 1. Artifact `research/lp10_independent_decision_20261011/INDEPENDENT_DECISION.json`
- Schema `vigilode-independent-decision-v1`.
- Node id `LP10`.
- Decision `PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT`.
- Verdict `PASS`.
- Technical assessments for `LP01`, `LP02`, `LP03`, `LP04`, `LP09`.
- Formal remaining node dispositions for `LP05`, `LP06`, `LP07`, `LP08`, `LP11`, `LP12`.
- Timing authority `diagnostic_only`.

### 2. Checker `tools/lp10_stage0_check.py`
- Validates schema, reviewed nodes, decision string, verdict, and file hashes.
- Outputs `research/lp10_independent_decision_20261011/STAGE0.json`.

## Commands

```bash
python3 tools/lp10_stage0_check.py --output research/lp10_independent_decision_20261011/STAGE0.json
```

## Results

Executed 2026-10-18 under commit `43baf0073c5cf01e9821329e7357a8c3b50e0264` (ledger `L-0123`).

### 1. Stage 0 Feasibility Outcome
- **Scope of finished routes:** Track A recommended first path (`LP01` collective model, `LP02` single-window packet, `LP03` damped moments, `LP04` adaptive multi-window trajectory, `LP09` fair benchmark) is 100% verified, tested, ledgered, and audited.
- **Dispositions of remaining DAG nodes:**
  - `LP05` (Rank-r & multiple clusters): `DEFERRED_OPTIONAL_BRANCH`.
  - `LP06` (Real nonlinear client): `EXISTING_NONLINEAR_REAL_CLIENT` strictly retained as `NOT_ESTABLISHED` (scientific hygiene preserved; no fabricated clients).
  - `LP07` (Implicit closure atlas): `DEFERRED_OPTIONAL_BRANCH`.
  - `LP08` (FFT acceleration): `DEFERRED_OPTIONAL_BRANCH`.
  - `LP11` (Shared-shift jet): `SOURCE_VERIFIED_ALREADY_PRESENT` (`shared_shift_jet.rs:363-375`).
  - `LP12` (Missing input recovery): `BLOCKED_EXTERNAL_SOURCE`.
- **Promotion Decision:** `PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT`. The collective packet solver is promoted as a certified domain-specialized high-performance subsystem for oscillatory collective systems, while `rodas5p_fast` / `rodas5p_mf_fast` is retained as the production default for general initial value problems.
- **Timing authority:** Strictly maintained on hold as `"timing_authority": "diagnostic_only"` under R2-STAT-01 and R4-STAT-DEV-01.
- **Decision:** `PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT`.
- **Verdict:** `PASS`.
