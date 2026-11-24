export const meta = {
  name: 'vigilode-loop-l1-wave3-group9',
  description: 'Loop L1 wave 3 group 9: V28 (rule B of the matrix-free stage solve P1 tested alone; switch to P1 if K, V and T hold) and V29 (multi-colouring first-use check by default in the assembly client), each followed by an independent read-and-run review',
  phases: [
    { title: 'Build', detail: 'V28 and V29 builders, one worktree each' },
    { title: 'Review', detail: 'independent review of each builder branch' },
  ],
}

const BASE = args.base
const WTP = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-'
const TGP = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-'
const EV = 'research/production_loop_20261116'
const OTHERS = 'taskset -c 0-5,12-17'

const PRE = `You are one agent of VigilODE research loop L1 (started 2026-11-16), wave 3, group 9. VigilODE is a Rust workspace of stiff ODE integrators (crates rodas5p-core, rodas5p-krylov, rodas5p-integrators) with the facade crate crates/vigilode; RODAS5P (8-stage Rosenbrock-Wanner) is the protected default method.
HARD RULES
- Work only in your own builder worktree and only in the paths your task allows; every other path is read-only for you. Never modify /home/cosmosapjw/vigilode (main checkout; its untracked files belong to the owner), the loop worktree /home/cosmosapjw/worktrees/vigilode-prod-loop-20261116, other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, git stash, git rebase, cargo clean, or any force operation. Stage files with explicit paths.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you. Do not run tools/agent_gate.py start, finish or abort.
- A "Fact-Forcing Gate" hook blocks the first Bash command, each new-file Write and the first Edit of a file. When it does, state the requested facts in one short paragraph and retry the identical call (for a new file: who reads or calls it, that no file with the same purpose exists, its format, and the instruction "execute loop L1 node <your node id> as assigned by the orchestrator"; for an Edit: the files that import or call it, the public items affected, the data formats touched, and the same instruction).
- The shell working directory resets after every Bash call: use absolute paths or "cd <dir> && <command>" in one call.
- Rust: toolchain 1.94.1 ("cargo +1.94.1"), --locked, "-j 4", the CARGO_TARGET_DIR given in your task. Never run an unfiltered "cargo test -p rodas5p-integrators" (about 250 test targets): always name the test targets. One cargo process at a time; prefix builds and runs with "${OTHERS}". No wall-time measurement is part of this group (counters only). The host is heavily loaded by other projects (load average up to 190): runs are slow; commit often.
- Memory: about 8 GB available, other agents build at the same time: one heavy process at a time.
- Protected, never edit unless your task names the file as an exception: fixtures/rodas5p_coefficients_snapshot.json; crates/rodas5p-core/src/{coefficients.rs,work.rs}; crates/rodas5p-integrators/src/{lib.rs,problem.rs,ppsw1.rs,sequential.rs,parallel.rs,stage_batch.rs,ptsw2b.rs,candidates.rs,source_owned_graph_schur.rs,rodas5p_fast.rs,rodas5p_matrix_free_fast.rs,rodas5p_fast_dense.rs,rodas5p_lagged.rs,client_stage.rs,adaptive.rs,routing.rs,stage_clients/mod.rs,stage_clients/dense_lu.rs,stage_clients/timed.rs}; crates/rodas5p-integrators/tests/{ppsw1_6b_contracts.rs,ptsw2b_contracts.rs,stage_clients_dense_identity.rs,stage_clients_alloc.rs}; crates/rodas5p-krylov/**; crates/vigilode/** ; every Cargo.toml and Cargo.lock of the repository workspace; rust-toolchain.toml. Tolerances, controller constants, default routing and the research gate G0 are immutable unless your task's frozen design says otherwise. Do not append to research/LEDGER.jsonl and do not edit docs/agent/**, ${EV}/PREREGISTRATION.md, ${EV}/PROGRESS.md or ${EV}/CHECKPOINT.json.
- Evidence rules: thresholds are frozen and may not be moved or re-interpreted after seeing data; every protocol choice the frozen file leaves open is written to PROTOCOL.json and committed BEFORE the first run; numbers in result files are parsed by a script from raw output of executed runs; failed and negative runs are kept; strict JSON. If a frozen criterion cannot be evaluated as written, report the exact obstacle (status partial or blocked).
- Commit on your branch in small steps (NOTES first, then code, then logs and results); commit messages end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". Do not push. A guard refuses .md files that you create with the Write tool: render REPORT_KO.md with a script.
- Loop checker, from your worktree root: "python3 -I ${EV}/check_checkpoint.py" (a complaint that only names a pinned implementation file your task changes is expected; report it verbatim).
- Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.
All worktrees of this group start at commit ${BASE}.
`

const RESULT_SCHEMA = { type: 'object', properties: {
  status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
  summary: { type: 'string' },
  branch: { type: 'string' }, head_commit: { type: 'string' },
  protocol_commit: { type: 'string' },
  files_written: { type: 'array', items: { type: 'string' } },
  checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['name', 'result'] } },
  discriminators: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, frozen_threshold: { type: 'string' }, measured: { type: 'string' }, verdict: { type: 'string', enum: ['PASS', 'FAIL', 'KILL', 'NOT_DECIDABLE', 'NOT_RUN'] } }, required: ['name', 'measured', 'verdict'] } },
  verdict: { type: 'string' },
  key_numbers: { type: 'array', items: { type: 'string' } },
  rulings: { type: 'array', items: { type: 'string' } },
  open_issues: { type: 'array', items: { type: 'string' } },
}, required: ['status', 'summary', 'checks', 'discriminators', 'verdict', 'open_issues'] }

const REVIEW_SCHEMA = { type: 'object', properties: {
  node: { type: 'string' },
  verdict: { type: 'string', enum: ['accept', 'accept_with_corrections', 'reject'] },
  summary: { type: 'string' },
  scope_ok: { type: 'boolean' }, protected_ok: { type: 'boolean' }, predeclared_before_results: { type: 'boolean' },
  reran: { type: 'array', items: { type: 'object', properties: { command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['command', 'result'] } },
  discriminator_checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, builder_value: { type: 'string' }, reviewer_value: { type: 'string' }, reviewer_verdict: { type: 'string', enum: ['PASS', 'FAIL', 'KILL', 'NOT_DECIDABLE', 'NOT_RUN'] }, agrees: { type: 'boolean' } }, required: ['name', 'reviewer_verdict', 'agrees'] } },
  findings: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] }, claim: { type: 'string' }, evidence: { type: 'string' }, correction: { type: 'string' } }, required: ['id', 'severity', 'claim', 'evidence'] } },
}, required: ['node', 'verdict', 'summary', 'scope_ok', 'protected_ok', 'discriminator_checks', 'findings'] }

const V28_SCOPE = `Allowed: new test targets crates/rodas5p-integrators/tests/mf_budget_*.rs (they may include the generators of the V25 and V24 targets by #[path] modules); ${EV}/mf_p1_budget/** except PREDECLARED.json. ONLY after a SWITCH verdict (K, V and T hold), by owner decisions OD-L1-11, OD-L1-15 and OD-L1-23, the protected edits listed under protected_edits_on_SWITCH of ${EV}/mf_p1_budget/PREDECLARED.json: crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs (default policy and identifier only), src/routing.rs (default of the matrix-free route), one contiguous re-export block in src/lib.rs after the last "// Loop L1" block if a new item must be exported, the sentinel pins of tests/ppsw1_6b_contracts.rs (g3) and tests/ptsw2b_contracts.rs (h2) with ${EV}/mf_p1_budget/SENTINEL_REPIN.json, the four lines of V24 tests listed in ${EV}/mf_p1_confirm/OUT_OF_LIST_EDITS.json, crates/vigilode/src/{solve.rs,solution.rs} where the matrix-free identifier is named, crates/vigilode/tests/default_path_golden.rs and crates/vigilode/tests/golden/default_path/** as G requires, and lines of facade tests that pin the matrix-free identifier or bits (each edited line listed with its reason in RESULTS.json).`

const V28 = `${PRE}
TASK: node V28_MF_P1_BUDGET_FOLLOWUP (owner decision OD-L1-23; addendum A9 of ${EV}/PREREGISTRATION.md and ${EV}/mf_p1_budget/PREDECLARED.json, frozen and on origin: read them first and do not edit them) - rule B of the adaptive stage solve P1 tested alone: replay of identical stage systems (K, V) and perturbed trajectory ensembles (T) on the 40 deciding cells of V25 and V24; switch the default of the matrix-free route to P1 only if K, V and T hold.
Worktree ${WTP}v28, branch research/production-loop-20261116-w3-v28. export CARGO_TARGET_DIR=${TGP}v28.
${V28_SCOPE}
READ FIRST: ${EV}/mf_p1_budget/PREDECLARED.json; node V25 (${EV}/mf_p1_confirm/: PREDECLARED.json with verdicts SWITCH and discriminators D0, G, R, PROTOCOL.json with the cells and amendment P-1 (the SHADOW replay), RESULTS.json, REVIEW.json, OUT_OF_LIST_EDITS.json, run_measurements.py, parse_results.py, the measurement target crates/rodas5p-integrators/tests/mf_confirm_measure.rs with mode SHADOW and crates/rodas5p-integrators/tests/mf_confirm_problems.rs); node V24 (${EV}/mf_adaptive/: PROTOCOL.json deciding_grid, SENTINEL_REPIN_MERGE.json, the measurement target crates/rodas5p-integrators/tests/mf_adaptive_measure.rs and its replay); MfStageSolvePolicy in crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs. Reuse them; do not re-derive the counting of budget-exhausting solves.
ORDER OF WORK (the order is part of the evidence):
0. ${EV}/mf_p1_budget/NOTES.txt, first commit within 15 minutes: the plan, where the budget exhaustion is counted (file:line), how the V24 families enter the target, how the perturbation is generated.
1. The measurement target crates/rodas5p-integrators/tests/mf_budget_measure.rs (modes RUN with the perturbation of the pre-registration, splitmix64 seeded by (cell index, j, i) mapped linearly to [-1, 1]; and SHADOW for the replay of both cell sets) and ${EV}/mf_p1_budget/PROTOCOL.json (the 40 cells with index, family, size, span, tolerance; the commands; the generator code; the binary hash) committed BEFORE any run of this node. A plumbing check on a toy problem outside the 40 cells is allowed before that commit.
2. K and V: the replay of the stage systems of the unperturbed runs of S and of P1 in every cell; raw logs under ${EV}/mf_p1_budget/logs/replay/; commit.
3. T: the 320 pairs of perturbed runs (S and P1 from the same perturbed state, cells x j = 1..8); raw logs under ${EV}/mf_p1_budget/logs/ensemble/; commit as you go.
4. ${EV}/mf_p1_budget/parse_results.py writes RESULTS.json (K with the list of every system where the two kernels disagree, V, T with W, n and the exact one-sided binomial probability, the totals per family and arm, the attempts and error ratios per pair as reports without rule) and renders REPORT_KO.md (Korean, for the owner); commit.
5. Only on SWITCH: step 6 of node V25 exactly as its PREDECLARED.json and PROTOCOL.json step_6_plan_if_SWITCH say (default policy P1 in the driver and in routing.rs with identifier rodas5p-mf-fast-transformed-v3, S keeps -v1, P2 keeps -v2; sentinels re-pinned with SENTINEL_REPIN.json; D0 re-checked on two cells per family; golden records of the matrix-free route re-pinned with GOLDEN_DIFF.json within the bounds, every other record byte-identical; suites R: the integrator targets mf_*, stage_clients_*, both sentinels, rodas5p-krylov, the facade golden and the whole facade suite "cargo +1.94.1 test --locked -j 4 --release -p vigilode --no-fail-fast"). If G or R does not hold, take the switch back and report HOLD. On KEEP or KILL change no default.
6. Checks: clippy -D warnings on the library of rodas5p-integrators and your test targets (and on vigilode all targets after a switch); "cargo +1.94.1 fmt -p rodas5p-integrators -p vigilode -- --check"; the loop checker; strict JSON of every JSON file you wrote; RESULTS.json regenerates identically.
Report head_commit, protocol_commit, every discriminator and the verdict.`

const V29 = `${PRE}
TASK: node V29_ASSEMBLY_DEFAULT_MC (owner decision OD-L1-24, verbatim in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json; contract in docs/agent/production_solver_program_20261112/TASK_CONTRACTS.json) - the default first-use pattern check of the opt-in AssemblyStageClient becomes the multi-colouring check MC on the routes where it applies; the dense route keeps Sample(8) because MC does not apply there; FA, FA_P and MC_P stay selectable.
Worktree ${WTP}v29, branch research/production-loop-20261116-w3-v29. export CARGO_TARGET_DIR=${TGP}v29.
Allowed paths: crates/rodas5p-integrators/src/stage_clients/assembly.rs and crates/rodas5p-integrators/src/stage_clients/assembly/**; crates/rodas5p-integrators/tests/stage_clients_assembly_*.rs (existing tests: only lines that pin the default check or the counts of the default first-use check, each listed with its reason in ${EV}/assembly_default/RESULTS.json); ${EV}/assembly_default/**.
READ FIRST: node V26 (${EV}/assembly_check/: PREDECLARED.json, RESULTS.json, REPORT_KO.md with its correction notice, REVIEW.json, review_probe/ with the counterexamples of ledger row L-0338, NOTES.txt); node V08 (${EV}/jacobian_assembly/: the identity cells); the client (assembly.rs, assembly/engine.rs, assembly/multi.rs, assembly/pattern.rs).
ORDER OF WORK:
0. ${EV}/assembly_default/NOTES.txt, first commit within 15 minutes: where the default is set (file:line), how a route is known to be coloured or dense at construction, which tests depend on the default.
1. Commit A, before any edit: a recording of the V08 identity cells (Jacobian bits, trajectory bits, every counter) and of the corpus decisions of the default check with today's code (Sample(8)).
2. The change: the default first-use check is MC wherever MC applies and Sample(8) on the dense route; the rustdoc of the default and of CheckColumns states what MC detects (the frozen corpus of V26) and what it misses (pairwise cancelling undeclared couplings and entry collapse, ledger L-0338, reproducible with ${EV}/assembly_check/review_probe), and that FA or FA_P is the exact choice up to rounding.
3. Tests: the default per route (coloured, banded, dense); on the V26 corpus the default now detects every case MC detects; the Jacobian and trajectory bits of the V08 identity cells equal commit A (only the counters of the first-use check change, by the documented amount); every existing test passes, with edits only where a test pins the default or its counts.
4. Checks: "${OTHERS} cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test 'stage_clients_*'" and the sentinels "--test ppsw1_6b_contracts --test ptsw2b_contracts"; clippy -D warnings on the library and the stage_clients_assembly_* targets; fmt; the loop checker; ${EV}/assembly_default/RESULTS.json (strict JSON, parsed from the logs: bits equal, counters of the first-use check per cell before and after, edited test lines with reasons) and REPORT_KO.md (Korean, for the owner, rendered by a script).
Report head_commit and the checks; discriminators: "bits unchanged", "default per route", "corpus detection of the default", "suites".`

const AUDIT = {
  v28: 'Was PROTOCOL.json with the 40 cells and the perturbation generator committed before the first run of the node (compare commit times with the log headers)? Are the 40 cells exactly those of the V25 and V24 protocols (sizes, spans, tolerances)? Recount K (subset property) and V from the replay logs and recompute T (W, n, the exact one-sided binomial probability) from the ensemble logs with a script of your own. Is the perturbation the declared one (check two seeds by hand)? If the default was switched: are S and P1 selected explicitly unchanged (D0); does GOLDEN_DIFF.json cover every changed record within the bounds; do the records of every other route keep their bytes; are both sentinels re-pinned with recorded old and new pins; is the identifier rodas5p-mf-fast-transformed-v3 used; were facade tests edited only in lines that pin the matrix-free identifier or bits?',
  v29: 'Is the default MC exactly where MC applies and Sample(8) on the dense route? Are the Jacobian and trajectory bits of the V08 identity cells equal to commit A? Were existing tests edited only where they pin the default or its counts, each with a reason? Does the rustdoc state the limits of MC correctly (L-0338)? Re-run the stage_clients suite, the sentinels and two corpus groups with the default; try one case of the review probe against the new default (it should still pass undetected, as documented).',
}

const reviewPrompt = (n, build) => `${PRE}
TASK: independent review of node ${n.id}, built by another agent. You did not write this work; your job is to find what is wrong with it, not to defend it. Read-and-run only: do not edit, create or commit anything in any worktree (scratch files only under ${TGP}${n.key}/review-scratch/; builds with CARGO_TARGET_DIR=${TGP}${n.key}/review-target). Worktree ${WTP}${n.key}, branch research/production-loop-20261116-w3-${n.key}, base ${BASE}.
The builder's allowed paths and task were:
<<<
${n.scope}
>>>
The builder's final report (JSON):
<<<
${JSON.stringify(build).slice(0, 9000)}
>>>
CHECK in this order and give evidence for each:
1. Scope: "git -C <worktree> diff --name-only ${BASE}..HEAD" against the allowed paths; protected files untouched (or touched only as the task's exceptions allow); "git status --short"; commit order (NOTES and PROTOCOL before results).
2. Frozen rules: compare the thresholds and definitions in the builder's parser and report with the frozen files; flag any moved, loosened or re-interpreted rule, any excluded cell without a declared rule, any typed-in number (trace at least five reported numbers to raw log lines).
3. Re-run the node's tests and a sample of the deciding runs; deterministic counters must match exactly.
4. Method audit: ${AUDIT[n.key]}
5. Recompute each discriminator from the raw logs with a short script of your own.
Verdict "accept" only if the evidence supports every claim; "accept_with_corrections" if specific claims or labels must be corrected; "reject" if a verdict is unsupported or a scope or protection rule was broken. Keep "summary" under 1200 characters.`

const NODES = [
  { id: 'V28_MF_P1_BUDGET_FOLLOWUP', key: 'v28', prompt: V28, scope: V28_SCOPE },
  { id: 'V29_ASSEMBLY_DEFAULT_MC', key: 'v29', prompt: V29, scope: 'Allowed: crates/rodas5p-integrators/src/stage_clients/assembly.rs and assembly/**; crates/rodas5p-integrators/tests/stage_clients_assembly_*.rs (existing tests only where they pin the default check or its counts, each listed with a reason); ' + EV + '/assembly_default/**. Task: OD-L1-24, contract V29_ASSEMBLY_DEFAULT_MC.' },
]
const results = await pipeline(
  NODES,
  n => agent(n.prompt, { label: 'build:' + n.key, phase: 'Build', schema: RESULT_SCHEMA }),
  (build, n) => {
    if (!build) { log('builder ' + n.key + ' returned nothing; review skipped'); return { node: n.id, build: null, review: null } }
    log('builder ' + n.key + ' finished: ' + build.status + ' / ' + build.verdict)
    return agent(reviewPrompt(n, build), { label: 'review:' + n.key, phase: 'Review', schema: REVIEW_SCHEMA }).then(review => ({ node: n.id, build, review }))
  },
)
return { results }
