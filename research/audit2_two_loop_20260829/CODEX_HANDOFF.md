# VigilODE matrix-free W — local continuation

Use the live Draft PR for `research/audit2-matrix-free-w-20260830` and its exact
HEAD/tree, not main or an older audit2 receipt. Parent scientific checkpoint is
PR31 c5fbd6d5703fc396bdf30eb3acfacb6c6bd2b921. Do not execute old K0.

## Already implemented and executed here

The non-default audit2-research module now exposes
`audit2_research::matrix_free::run_audit2_matrix_free_correction`.
It requires an analytic-JVP matrix-free StepContext, supplied GmresConfig,
and a one-shot preconditioner factory. It uses one fixed preconditioner and
one Givens workspace for all eight lower-block rows, then drops both.
FullTargetOracle and ordinary integrator defaults are unchanged.

Read matrix_free_w_design.md, matrix_free_w_results.json,
matrix_free_w_claim_ledger.md and evidence/matrix_free_w_verification.json.
The exact code passed 18 new contracts, 76 total relevant Rust tests, 20 Python
tests, two actual source mutants, feature-off execution, scoped clippy and fmt.
All22 raw output records repeated byte-identically. No fresh independent review
or local full-workspace test is claimed. Remote CI is reported externally.

## Exact local work, not a new packaging project

1. Inspect the current remote PR HEAD/tree and parent; use an isolated worktree.
   Preserve dirty/untracked user files. A moved remote HEAD requires inspecting
   its actual diff, not overwriting it or treating an archive checksum as a
   scientific failure. The optional ZIP is not an execution prerequisite.
2. Run ONE fresh-context HOST_CODEX_ONLY review of the new c5-to-delivery delta.
   Focus on attempted/completed work, original vs projected target, true vs
   preconditioned residual, same-context reuse, failure partials and API scope.
   The implementing agent's checks here are not an independent review.
3. Repair only concrete P0/P1 findings in one bounded pass, with RED/GREEN tests.
   In particular, inherited_work_complete=false and unavailable failed iterates
   must remain explicit unless their underlying observation is actually added.
   Missing output B is not a reason to prevent exploratory measurement.
4. Run the existing readiness entry once; preserve outputs/logs on failure:

```bash
export AUDIT2_OUTPUT_DIR="$(mktemp -d /tmp/vigilode-mfw-host.XXXXXX)"
bash tools/check-audit2-readiness.sh 2>&1 | tee "$AUDIT2_OUTPUT_DIR/host.log"
# Use `set -o pipefail` in the shell invoking this command.
```

The script runs the new feature-gated contracts and the ordinary feature-off
baseline. No historical54/Oregonator/timing/BDF campaign is needed.
5. Report fresh review verdict separately from repaired regression outcomes.
   Push a non-force child branch/Draft PR only if actual changes exist; append
   PM-1 and Confluence9732097, not PM-7/K0. Keep scientific PRs draft/unmerged.
6. The NEXT substantive code node is research-only whole-step transactional
   integration, not a claim already achieved here. First bind it to the existing
   original-target acceptance oracle and rollback/controller/cache semantics.
   Do not invent these from a small residual. Its exact boundaries are in the
   existing handoff.json next_work_unit.transaction_plan.

## Usage pattern for an existing caller-supplied trial K

```rust
use rodas5p_core::{IdentityPreconditioner, Preconditioner, WorkCounters};
use rodas5p_krylov::GmresConfig;
use rodas5p_integrators::build_step_context_matrix_free;
use rodas5p_integrators::audit2_research::matrix_free::run_audit2_matrix_free_correction;
// problem, t, y, h and trial_stages are supplied by the research caller.
let mut context_work = WorkCounters::default();
let context = build_step_context_matrix_free(&problem, t, &y, h, &mut context_work)?;
let cfg = GmresConfig { restart: 32, max_arnoldi: 256, rtol: 1e-11, atol: 1e-13 };
let diagnostic = run_audit2_matrix_free_correction(&context, &trial_stages, &cfg,
    |ctx, _setup_work| Ok(Box::new(IdentityPreconditioner::new(ctx.problem.dimension))
        as Box<dyn Preconditioner>));
// diagnostic.completed means computation completed, NOT an accepted step.
// Retain context_work separately; inspect failure, work and available partials.
```

No default budget, no nonlinear/output certificate, no scaling/timing/ranking
claim, no holdout/freeze, tag/release, PM-7 closure or production activation.
