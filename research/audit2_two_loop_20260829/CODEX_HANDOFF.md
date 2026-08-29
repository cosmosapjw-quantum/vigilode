# Codex continuation: CI repaired, baseline runnable, acceleration still research-only

## A. Authority and scope

Repository: `cosmosapjw-quantum/vigilode`. PR #29 remains OPEN/DRAFT/UNMERGED on
`research/audit2-output-policy-20260829`, based on
`codex/vigilode-scientific-validity-v2@93fe348ce36859dd5f78b31267d771ea9c054677`.
This maintenance starts from the inspected Codex delivery
`c8942d7840eb8a165bd1b02c0ec8051757cf1919`, tree
`baedef527245eb557e503db51b799a4e047cb0e5`. The live publication comment supplies
this maintenance's exact commit/tree. No commit embeds its own future identity.

The c894 research module, projection, uncertainty policy and 49 numerical tests
are unchanged. One earlier fresh review and its repairs remain recorded in
`evidence/fresh_review_disposition.md`; no second fresh review is claimed here.
Old K0, PM-7, historical 54-case data, sealed holdouts, production dispatch and
Cargo manifests are not part of this change.

## B. Reproduced defects and delivered repair

The A1 receipt workflow compared every triggered PR with a historical A1
starting head, even when no A1 evidence changed. Scope now comes from the actual
PR diff and A1 branch: A1 evidence additions, modifications, deletions and renames
still activate the original frozen checks. An unrelated PR reports
`A1_RECEIPT_NOT_APPLICABLE`, not A1 scientific validation. Ordinary regression CI
remains enabled.

A non-default feature must actually be enabled to exercise its tests. The new
`audit2-research.yml` invokes the same `check-audit2-readiness.sh` used locally,
including explicit research-on tests and a separate research-off solver example.

`crates/rodas5p-integrators/examples/solve_stiff.rs` is a working client of the
existing solver: supplied analytic JVP, no assembled Jacobian, dense output,
JSON states/work, and nonzero exit with retained partial output on exhaustion.
It does not use the common-W research correction.

## C. Executed verification and its limits

This revision executed 54 Rust tests (8 global error, 9 accuracy, 11 correction,
15 dense output, 6 homotopy, 5 usage) and 20 Python tests (12 CI scope, 8 existing
research contracts). Default-feature-off build, affected clippy and formatting
passed. The normal and deliberately exhausted example were both executed; the
latter's nonzero exit was accepted only after parsing the partial-result JSON.
See `evidence/readiness_verification.json` and `evidence/readiness_verification.log`.

No local full-workspace, 54-case campaign, new holdout, second fresh review,
wall-time or speedup result is claimed. CI state is read from checks for the
published head, not inferred from these local tests. An A1-not-applicable child
job is skipped, not an executed scientific pass.

## D. Immediate use and the next local work unit

No ZIP, archive pin, old bootstrap or package merge is required. Fetch PR #29's
branch normally. Preserve dirty/unpushed work; use a new isolated worktree rather
than reset/stash/rebase. If the branch has moved, inspect the actual delta and
rebind provenance; do not treat a new commit SHA as scientific refutation.

For a direct baseline run in that checkout:

```bash
cargo run --locked -p rodas5p-integrators --no-default-features --example solve_stiff > solution.json
```

Do not repeat completed process work. The next acceleration work unit is
`ORIGINAL_TARGET_BRIDGE` in `handoff.json`. Keep it research-only and start by
reusing the existing 12 trial-state cases, not a new calibration campaign.
Evaluate both unprojected and projected residual/Jacobian actions at identical
trial K. With `r_o=R_o(K)`, `r_p=R_p(K)`, `A_o=DR_o(K)`, `A_p=DR_p(K)` and a
computed correction z, record

```text
rho_p = A_p z - r_p
DeltaA = A_o - A_p; Deltar = r_o - r_p
rho_o = A_o z - r_o = rho_p + DeltaA z - Deltar
Newton update: K <- K - z
```

Use the actual original residual implementation; do not silently project it.
Count original-target diagnostic work separately. Preserve both arm failures and
partial work. Compare original-target backward error, condition-aware correction
agreement and output/embedded-error projections; do not compare tiny secondary
error estimates to each other with an arbitrary relative tolerance.

The existing thresholds `4096*eps` (backward error) and
`8192*eps*condition_f` (state agreement) may be reused only on their existing
small linear-system domain, with explicit same-target validation. They are not a
nonlinear/output acceptance rule. New regimes require justified criteria before
outcomes, not tolerance widening. No external output budget means diagnostic-only,
not PASS, and does not prevent the probe from running. Estimate-only reference
uncertainty cannot produce categorical accuracy admission.

## E. Review boundary

The c894 fresh-review dispositions are not reopened. Review this maintenance
only for scope selection, feature coverage, output/failure semantics and missing
current-task regressions. For a new original-target implementation, use targeted
checks and one read-only fresh review of that new delta, then one bounded repair.
Do not create review-of-review or a new authority-schema family.

## F. Next transition after the bridge

Only after measured original-target compatibility should the common-W backend
be connected to a research whole-step driver. Its current implementation requires
explicit W; implement matrix-free W solves and reusable preconditioning if the
intended use is assembled-Jacobian-free. Otherwise label the explicit-W path
honestly. Preserve the legacy/default oracle and transactional fallback; do not
replace nonlinear acceptance by a correction norm or a residual norm alone.

## G. Remaining-stage estimate, not an admission contract

The baseline is runnable now in a narrow demonstrated domain. For a particular
real application, one problem-specific reference/refinement/failure validation
work package remains before relying on it; new defects may require another.

The accelerated solver has roughly four substantive transitions: (1) original-
target bridge, (2) scalable W backend/setup reuse, (3) transactional whole-step
integration, (4) actual-client error/work/end-to-end validation. A competitive
BDF/RODAS polyalgorithm adds (5) a working production BDF comparator and fair
cost measurement, (6) history-aware switching, hysteresis and held-family/long-
run validation. These are capabilities, not PR counts or time estimates; allow
an extra repair transition if a new scientific defect is found. No speed benefit
is guaranteed. See the machine handoff for what is and is not authorized now.

## H. Handback and synchronization

Return exact source/head/tree, changed paths, executed commands/exits, evidence
paths, result/provenance/packaging dispositions and remaining unknowns. Append
actual results to PR #29 and existing PM-1 / Confluence page 9732097; do not close
PM-7 or overwrite the older project DAG. Report ATLAS_SYNC_PENDING only after a
real failed sync attempt. Keep PRs draft/unmerged; no activation, holdout opening,
freeze, timing/ranking/speedup claim, tag or release. The ZIP is a convenience
mirror; its byte identity is not a numerical acceptance condition.
