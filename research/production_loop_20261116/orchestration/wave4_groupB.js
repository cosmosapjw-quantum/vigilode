export const meta = {
  name: 'vigilode-wave4-groupB',
  description: 'Wave 4 group B (loop L1): V33 CI liveness with two reviewers, V23 capabilities truth then V34 jvp_calls counter with a reviewer, the independent review of V22, and phase 1 of V30 (V27b: E7 diagnosis and a draft pre-registration)',
  phases: [
    { title: 'Build', detail: 'V33, V23+V34, V30 phase 1; V22 review' },
    { title: 'Review', detail: 'independent reviews' },
  ],
}

const SCR = '/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad'
const LOCKA = SCR + '/locks/cargo_a.lock'
const LOCKB = SCR + '/locks/cargo_b.lock'
const WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w4-'
const TL1 = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-'
const EV = 'research/production_loop_20261116'

const COMMON = (lock) => `COMMON RULES
- Work only in your own worktree and only in the paths your task allows; everything else is read-only. Never modify /home/cosmosapjw/vigilode (main checkout; its untracked files belong to the owner), other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, git stash, git rebase, git commit --amend, cargo clean, or any force operation. Stage files with explicit paths.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you. Do not run tools/agent_gate.py start, finish, abort or approve.
- A "Fact-Forcing Gate" hook blocks the first Bash command, each new-file Write and the first Edit of a file. When it does, state the requested facts in one short paragraph and retry the identical call (who reads or calls the file, that no file with the same purpose exists, its format, and the instruction "owner 2026-11-25 09:21 KST: 남은 노드들을 이어서 마저 진행해. 수정은 전부 승인할게.").
- The shell working directory resets after every Bash call: use absolute paths or "cd <dir> && <command>" in one call.
- Rust: toolchain 1.94.1 ("cargo +1.94.1"), --locked, "-j 4", the CARGO_TARGET_DIR of your task. EVERY cargo command is prefixed with "flock -w 14400 ${lock} taskset -c 0-5,12-17" (a host-wide build lock shared with other agents; hold it only for the command). Never run an unfiltered "cargo test -p rodas5p-integrators": name the test targets. Other projects' batch jobs load the host and memory is tight: one heavy process at a time; commit often.
- No wall-time measurement decides anything in this group (counters, instruction counts and bits only); deciding timing cells are preregistered and run later by the orchestrator.
- Evidence: numbers in result files are parsed by a script from raw output of executed runs; failed and negative runs are kept; strict JSON. Do not report an unrun check as passed. If the same failure repeats twice, stop and report it.
- A guard refuses .md files created with the Write tool: render any new report .md with a script.
- Commit on your branch in small steps; commit messages end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". Do not push.
- Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.`

const PRE = (lock) => `You are one agent of VigilODE research loop L1 (since 2026-11-16), wave 4 group B. VigilODE is a Rust workspace of stiff ODE integrators (crates rodas5p-core, rodas5p-krylov, rodas5p-integrators) with the facade crate crates/vigilode; RODAS5P is the protected default method.
${COMMON(lock)}
- Protected, never edit unless your task names the file: fixtures/rodas5p_coefficients_snapshot.json; crates/rodas5p-core/src/{coefficients.rs,work.rs}; crates/rodas5p-integrators/src/{lib.rs,problem.rs,ppsw1.rs,sequential.rs,parallel.rs,stage_batch.rs,ptsw2b.rs,candidates.rs,source_owned_graph_schur.rs,rodas5p_fast.rs,rodas5p_matrix_free_fast.rs,rodas5p_fast_dense.rs,rodas5p_lagged.rs,client_stage.rs,adaptive.rs,routing.rs,stage_clients/mod.rs,stage_clients/dense_lu.rs,stage_clients/timed.rs}; crates/rodas5p-integrators/tests/{ppsw1_6b_contracts.rs,ptsw2b_contracts.rs,stage_clients_dense_identity.rs,stage_clients_alloc.rs}; crates/rodas5p-krylov/**; every Cargo.toml and Cargo.lock; rust-toolchain.toml. Tolerances, controller constants, default routing and the research gate G0 are immutable unless your task's frozen design says otherwise. Do not append to research/LEDGER.jsonl and do not edit docs/agent/**, ${EV}/PREREGISTRATION.md, ${EV}/PROGRESS.md or ${EV}/CHECKPOINT.json.
- Loop checker from your worktree root: "python3 -I ${EV}/check_checkpoint.py" (a complaint naming only a pinned or protected file your task changes is expected; report it verbatim).
Owner decisions OD-L1-26..30 (2026-11-25) are in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json; node contracts in TASK_CONTRACTS.json.`

const RESULT_SCHEMA = { type: 'object', properties: {
  status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
  summary: { type: 'string' },
  branch: { type: 'string' }, head_commit: { type: 'string' },
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
  scope_ok: { type: 'boolean' }, protected_ok: { type: 'boolean' },
  reran: { type: 'array', items: { type: 'object', properties: { command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['command', 'result'] } },
  discriminator_checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, builder_value: { type: 'string' }, reviewer_value: { type: 'string' }, reviewer_verdict: { type: 'string', enum: ['PASS', 'FAIL', 'KILL', 'NOT_DECIDABLE', 'NOT_RUN'] }, agrees: { type: 'boolean' } }, required: ['name', 'reviewer_verdict', 'agrees'] } },
  findings: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] }, claim: { type: 'string' }, evidence: { type: 'string' }, correction: { type: 'string' } }, required: ['id', 'severity', 'claim', 'evidence'] } },
}, required: ['node', 'verdict', 'summary', 'scope_ok', 'protected_ok', 'discriminator_checks', 'findings'] }

const V33 = `${PRE(LOCKA)}
TASK: node V33_CI_IGNORED_LIVENESS (owner decision OD-L1-29). Worktree ${WT}v33, branch research/production-loop-20261116-w4-v33. Allowed paths: tools/ci/**, .github/workflows/ignored-tests.yml, .github/workflows/production.yml, ${EV}/ci/** (new files; render .md by script or use .txt).
READ FIRST: ${EV}/ci/TRIAGE_KO.md sections 9 to 12 (12.2 and 12.3 state the defect with hosted evidence), ${EV}/ci/V3_NOTES.md, ${EV}/ci/V3_REVIEW.json, tools/ci/select_tests.py (v3; its self-test), both workflows.
DEFECT: a full selection that cannot finish inside the job limit leaves no baseline. The job timeout cancels the run step, the save step ("if: \${{ !cancelled() }}") is skipped, and the baseline step accepts only complete outcomes, so every later run is full again (hosted: ignored runs 45378440571 and 45414722588 both cancelled at 330 minutes). The test job's first v3 run took 276 of 300 minutes.
REQUIRED (owner-approved design): (1) the run step writes the outcome and the input map after every target (atomic replace), complete=false until every selected command ran, with the measured targets and the targets not run listed; (2) a self time budget (an option such as --budget-minutes) after which no new target starts; the workflows pass the job limit minus a stated margin (justify each margin from the hosted step durations in the triage) and the run step ends with the outcome saved; the job must not look green when selected targets were not run (decide and state how: for example the run step fails with a clear message after saving); (3) the save steps run on cancellation too ("if: always()"); (4) a baseline from an incomplete outcome: the targets it measured are mapped with their measured inputs and their failures carry over as today; targets selected but not run are unmapped and selected by the next run; complete outcomes behave exactly as before; (5) nothing else in the selection rules changes.
ORDER: notes file first (commit), then code, self-test cases (a budget-stopped run writes an incomplete outcome; the next selection maps the measured targets and selects the rest; complete baselines select as before; a cancelled run with a partial outcome; corrupted or empty partial files), then the workflow edits.
CHECKS: "python3 -u tools/ci/select_tests.py self-test"; "python3 -I tools/check_ignored_tests_in_ci.py"; YAML syntax of both workflows (python3 -c with yaml if installed, else a careful structural check you describe); a local end-to-end simulation of a budget-stopped run on a tiny selection and of the next selection, with logs under ${EV}/ci/v4_logs/; the loop checker. The hosted evidence comes after merge (the orchestrator records it).`

const V33_REVIEW = (lens) => (b) => `${PRE(LOCKA)}
YOU ARE AN INDEPENDENT REVIEWER of node V33_CI_IGNORED_LIVENESS (read and run; you did not build it). Builder worktree ${WT}v33 (read-only for you). Scratch files only under ${SCR}/review_V33_${lens}/.
LENS: ${lens === 'soundness' ? 'soundness of the selection: can any test whose inputs changed be skipped, can an incomplete baseline hide a failure or mark a target as measured that did not run, are partial files written atomically and read defensively, do complete baselines select exactly as before (run the self-test and your own adversarial cases)' : 'mechanics of the workflows: step conditions under success, failure, cancellation and timeout, the budget arithmetic against timeout-minutes and the hosted step durations, artifact names and retention, concurrency groups, that the job cannot be green with unrun targets, YAML validity, and the interaction with the baseline step of the next run'}.
Return accept, accept_with_corrections or reject with findings. Builder result: ` + JSON.stringify(b).slice(0, 6000)

const V23 = `${PRE(LOCKB)}
TASK: node V23_CAPABILITIES_TRUTH (owner decision OD-L1-10, pre-registered in addendum A6 of ${EV}/PREREGISTRATION.md and ${EV}/capabilities_fix/PREDECLARED.json: read both first, do not edit them), THEN node V34_JVP_CALLS_COUNTER (owner decision OD-L1-30) in the same worktree.
Worktree ${WT}v23, branch research/production-loop-20261116-w4-v23. export CARGO_TARGET_DIR=${TL1}v23.
V23 allowed paths: ${EV}/capabilities_fix/** (not PREDECLARED.json), crates/vigilode/src/capabilities.rs, crates/vigilode/src/solve.rs, crates/vigilode/src/error.rs, crates/vigilode/tests/capabilities_contracts.rs, crates/vigilode/tests/capabilities_truth_contracts.rs, crates/vigilode/tests/golden/capabilities_truth/**, crates/vigilode/tests/unified_frontend_contracts.rs, crates/vigilode/tests/default_path_golden.rs, crates/vigilode/tests/golden/default_path/** (golden edits only as D3 allows, each listed).
V23 goal: Rodas5p::capabilities gives a plan only to requests that the named driver family runs; what cannot run is refused with a typed reason before any callback. Acceptance D1-D4 of the contract (TASK_CONTRACTS.json id V23_CAPABILITIES_TRUTH) and of PREDECLARED.json. Order: notes, the before-recording of the 9216-request grid and the golden files, the fix, the after-recording, results by script (RESULTS.json, REPORT_KO.md rendered by script), checks (the facade tests you touch, "cargo +1.94.1 test --locked -j 4 --release -p vigilode --no-fail-fast" once at the end, clippy -D warnings on vigilode all targets, fmt, loop checker).
V34 (after V23 is committed): Stats::counters.jvp_calls reads 0 on the facade's banded sparse-JVP route although Jacobian-vector products are evaluated; count them truthfully. Allowed: crates/vigilode/src/solve.rs, crates/vigilode/src/capabilities.rs, crates/vigilode/tests/capabilities_*.rs, new crates/vigilode/tests/jvp_counter_*.rs, ${EV}/jvp_counter/**. A failing-first test (fails on the V23 head by assertion, passes after); state bits and every other counter of the route unchanged; default-path golden files unchanged; facade suite, clippy, fmt. Report both nodes in one structured result (discriminators prefixed V23 and V34).`

const V23_REVIEW = (b) => `${PRE(LOCKB)}
YOU ARE THE INDEPENDENT REVIEWER of nodes V23_CAPABILITIES_TRUTH and V34_JVP_CALLS_COUNTER (read and run). Builder worktree ${WT}v23 (read-only for you); your own CARGO_TARGET_DIR=${TL1}v23-review; scratch only under ${SCR}/review_V23/. Check scope and protected files, that the before-recordings precede the fix, recompute D1-D4 from the raw recordings, try requests near the boundary of each driver family (mass matrices, dense output, events, sparse-JVP providers, banded declarations, n = 1), confirm refusals happen before any callback, and for V34 that the failing-first test failed on the base and that no other counter or bit changed. Return accept, accept_with_corrections or reject with findings. Builder result: ` + JSON.stringify(b).slice(0, 6000)

const V22_REVIEW = `${PRE(LOCKB)}
YOU ARE THE INDEPENDENT REVIEWER of node V22_FACADE_BANDED_KERNEL (completed 2026-11-24 by the orchestrator session; frozen verdict PASS, ledger L-0334: the facade's banded route uses the Slices kernel from half band width W = 6 on, BANDED_SLICES_MIN_HALF_WIDTH). Its review never ran (the owner paused the loop on 2026-11-23); owner decision OD-L1-30 resumes it. Detached worktree ${WT}v22review at d4e2772f (read-only for you); your own CARGO_TARGET_DIR=${TL1}v22-review; scratch only under ${SCR}/review_V22/.
READ: addendum A5 and amendment A5.1 of ${EV}/PREREGISTRATION.md, ${EV}/banded_kernel_route/ (PREDECLARED.json, AMENDMENT_1.json, RESULTS.json, REPORT_KO.md, logs/, build_results.py, finish_scan.py, d2_compare.py, measure.py, SENTINEL_REPIN.json), the contract V22_FACADE_BANDED_KERNEL in docs/agent/production_solver_program_20261112/TASK_CONTRACTS.json, PROGRESS.md entries about V22 (rulings about the last allowed measurements and the parser correction), and the code: crates/vigilode/src/solve.rs (the banded kernel rule), crates/rodas5p-integrators/src/rodas5p_fast.rs and rodas5p_fast_dense.rs (the kernel entry points), tests banded_kernel_entry_identity.rs and banded_kernel_route_contracts.rs.
CHECK: the pre-registration preceded the deciding measurements; recompute every discriminator (validity, the width scan and W, the small-n confirmation, D1-D4) from the raw logs with your own script; re-run the identity and contract tests (cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test banded_kernel_entry_identity; -p vigilode --test banded_kernel_route_contracts --test default_path_golden); judge the rulings recorded in PROGRESS.md (were thresholds moved after seeing data?); do not re-measure wall time. Return accept, accept_with_corrections or reject with findings.`

const V30 = `${PRE(LOCKA)}
TASK: node V30_ROUTER_BREAKEVEN_V27B, PHASE 1 ONLY (owner decision OD-L1-26): diagnose E7 and draft the corrected pre-registration. Do NOT run the deciding set of the new evaluation: the orchestrator reviews your draft, commits and pushes PREDECLARED.json, and only then a phase 2 runs it.
Worktree ${WT}v30, branch research/production-loop-20261116-w4-v30 (the V27 FBP implementation of branch w3-v27 at 60d67729 merged with the loop head d4e2772f; commit 91e9e582). export CARGO_TARGET_DIR=${TL1}v30.
Allowed paths: ${EV}/router_breakeven_v27b/**; crates/vigilode/src/session.rs, crates/vigilode/src/unified/**, crates/vigilode/tests/unified_breakeven_*.rs, crates/vigilode/tests/unified_router_contracts.rs only if the E7 diagnosis shows a defect of the route (then a minimal correction with a failing-first test).
READ FIRST: ${EV}/router_hyperparameter_free/ (DESIGN_RECOMMENDED.md, PREDECLARED.json with E1-E10, AMENDMENT_A8_2.json, RESULTS.json, REVIEW.json with findings R1-R6, REPORT_KO.md with its correction notice, PHASE2_NOTES.txt, run_phase2.py, references.py, runner/), the result paragraph of addendum A8 in ${EV}/PREREGISTRATION.md (section "Results, wave 3, group 8"), and the FBP code (git diff d4e2772f HEAD -- crates).
STEPS:
1. ${EV}/router_breakeven_v27b/NOTES.txt first (commit within 15 minutes).
2. E7 diagnosis on P_scalar_fd (y' = -1e6 (y - cos t)), allowed because it is not part of the new deciding set: reproduce the error ratios 50.6 and 3.53 of V27 against Rodas5p::solve at tolerances A and L; test the hypothesis that the handover starts RODAS5P with h0 = H (the pilot's horizon) and any other cause you find (pilot state, error norm, dense output, controller history); run the variants that isolate the cause; record raw logs; state the cause with evidence. If the cause is a defect of the route (not an inherent property of handing over), make the minimal correction with a failing-first test and re-run the V27 contract tests (unified_breakeven_contracts, unified_router_contracts, default_path_golden).
3. A draft ${EV}/router_breakeven_v27b/PREDECLARED.json (strict JSON) containing: the corrected E1 (the maximum ratio over undeclared cases only; geometric means as before over all cases where both succeed), the corrected E3 (excluding the runs that owner decision D1 ends; state the exact set), E2 and E4-E10 unchanged unless the diagnosis requires a change (then state it), the E7 rule unchanged (err(N) <= 3 max(err(D), err(R))); a FRESH problem set disjoint from the 35 problems of V27 (at least 30 problems: stiff, non-stiff and late-stiff families, small and large n, finite-difference and analytic Jacobians, with y0, span and the reason for each; none may have been run with the FBP route before this file is pushed), tolerances and options as V27; run settings; the frozen decision (the Auto route becomes FBP iff all of E1-E10 pass on the fresh set); and expectations without rule.
4. Report: the E7 cause, any code change with its tests, the draft's sha256, and anything the orchestrator must decide before pushing.`

const items = [
  { key: 'V33', prompt: V33, schema: RESULT_SCHEMA, label: 'build:v33',
    review: (b) => parallel([() => agent(V33_REVIEW('soundness')(b), { label: 'review:v33-soundness', phase: 'Review', schema: REVIEW_SCHEMA }),
                             () => agent(V33_REVIEW('mechanics')(b), { label: 'review:v33-mechanics', phase: 'Review', schema: REVIEW_SCHEMA })]) },
  { key: 'V23', prompt: V23, schema: RESULT_SCHEMA, label: 'build:v23-v34',
    review: (b) => agent(V23_REVIEW(b), { label: 'review:v23-v34', phase: 'Review', schema: REVIEW_SCHEMA }) },
  { key: 'V22', prompt: V22_REVIEW, schema: REVIEW_SCHEMA, label: 'review:v22', review: null },
  { key: 'V30', prompt: V30, schema: RESULT_SCHEMA, label: 'build:v30-phase1', review: null },
]
phase('Build')
const results = await pipeline(items,
  (item) => agent(item.prompt, { label: item.label, phase: item.key === 'V22' ? 'Review' : 'Build', schema: item.schema }),
  (out, item) => item.review ? item.review(out).then(review => ({ key: item.key, build: out, review })) : ({ key: item.key, build: out, review: null }))
return { results }
