export const meta = {
  name: 'vigilode-loop-l1-wave3-group7',
  description: 'Loop L1 wave 3 group 7: CI test selection v3 (measured inputs per target; owner decisions OD-L1-19..21) with two adversarial reviewers, one fix round and a recheck; and phase 1 of the V27 implementation (forward break-even pilots, OD-L1-22) without any evaluation run',
  phases: [
    { title: 'Build', detail: 'CI v3 builder and V27 phase-1 builder, one worktree each' },
    { title: 'Review', detail: 'two adversarial reviewers of CI v3 (soundness, mechanics)' },
    { title: 'Fix', detail: 'CI v3 builder applies confirmed findings' },
    { title: 'Recheck', detail: 'one reviewer verifies the fixes' },
  ],
}

const BASE = args.base
const WTP = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-'
const TGP = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-'
const EV = 'research/production_loop_20261116'
const OTHERS = 'taskset -c 0-5,12-17'

const PRE = `You are one agent of VigilODE research loop L1 (started 2026-11-16), wave 3, group 7. VigilODE is a Rust workspace of stiff ODE integrators (crates rodas5p-core, rodas5p-krylov, rodas5p-integrators, rodas5p-fair-ab, rodas5p-cli) with the facade crate crates/vigilode and the separate crate vigilode-coupled; RODAS5P (8-stage Rosenbrock-Wanner) is the protected default method.
HARD RULES
- Work only in your own worktree and only in the paths your task allows; every other path is read-only for you. Never modify /home/cosmosapjw/vigilode (main checkout; its untracked PDF and zip files belong to the owner), the loop worktree /home/cosmosapjw/worktrees/vigilode-prod-loop-20261116, other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, git stash, git rebase, cargo clean, or any force operation. Stage files with explicit paths (never "git add -A" or "git add .").
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you: its gate tasks are all done. Do not run tools/agent_gate.py start, finish or abort.
- A "Fact-Forcing Gate" hook blocks the first Bash command, each new-file Write and the first Edit of a file. When it does, state the requested facts in one short paragraph and retry the identical call. Facts for Bash: the task in one sentence and what the command verifies or produces. For a new file: who reads or calls it, that no file with the same purpose exists (search first), its data format, and the instruction "execute loop L1 wave 3 group 7 task <your task> as assigned by the orchestrator". For an Edit: the files that import or call the edited file (search), the public items affected, the data formats touched, and the same instruction. If the hook flags a destructive command, do not run it; find a non-destructive way.
- The shell working directory resets after every Bash call: use absolute paths, "git -C <worktree>", or "cd <dir> && <command>" in one call.
- Rust: toolchain 1.94.1 ("cargo +1.94.1"), always --locked and "-j 4", and export CARGO_TARGET_DIR exactly as given in your task. Never run an unfiltered "cargo test -p rodas5p-integrators" (about 250 test targets): always name the test targets. One cargo process at a time. Prefix builds and runs with "${OTHERS}" (cores 6-11 are reserved for timing runs of other agents). No wall-time measurements are needed in this group.
- Memory is tight (about 8 GB available, swap in use, two other builders are running): one heavy process at a time, single allocations below 2 GB.
- Protected, never edit: fixtures/rodas5p_coefficients_snapshot.json; crates/rodas5p-core/src/{coefficients.rs,work.rs}; crates/rodas5p-integrators/src/{lib.rs,problem.rs,ppsw1.rs,sequential.rs,parallel.rs,stage_batch.rs,ptsw2b.rs,candidates.rs,source_owned_graph_schur.rs,rodas5p_fast.rs,rodas5p_matrix_free_fast.rs,rodas5p_fast_dense.rs,rodas5p_lagged.rs,client_stage.rs,adaptive.rs,stage_clients/**}; crates/rodas5p-integrators/tests/**; crates/rodas5p-krylov/**; crates/vigilode/src/{lib.rs,automatic.rs,switching.rs,solve.rs}; crates/vigilode/tests/default_path_golden.rs and crates/vigilode/tests/golden/**; every Cargo.toml and Cargo.lock of the repository workspace; rust-toolchain.toml. Tolerances, controller constants, default routing and the research gate G0 are immutable. Do not append to research/LEDGER.jsonl and do not edit docs/agent/**, ${EV}/PREREGISTRATION.md, ${EV}/PROGRESS.md or ${EV}/CHECKPOINT.json: the orchestrator records results at merge time.
- Commit on your branch in small steps so that partial work survives an interruption; commit messages end with the line "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". Do not push.
- Write full details to files. Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.
- Loop checker, run from your worktree root: "python3 -I ${EV}/check_checkpoint.py" (verifies protected blobs and that changed paths are inside the programme allowlist; a complaint that only names a path your task explicitly allows, or the pins of implemented sources that your task changes, is expected and must be reported verbatim).
All worktrees of this group start at commit ${BASE}.
`

const RESULT_SCHEMA = { type: 'object', properties: {
  status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
  summary: { type: 'string' },
  branch: { type: 'string' }, head_commit: { type: 'string' },
  files_written: { type: 'array', items: { type: 'string' } },
  checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['name', 'result'] } },
  finding_status: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, status: { type: 'string', enum: ['fixed', 'not_applicable', 'open'] }, how: { type: 'string' } }, required: ['id', 'status', 'how'] } },
  key_numbers: { type: 'array', items: { type: 'string' } },
  rulings: { type: 'array', items: { type: 'string' } },
  open_issues: { type: 'array', items: { type: 'string' } },
}, required: ['status', 'summary', 'checks', 'open_issues'] }

const REVIEW_SCHEMA = { type: 'object', properties: {
  lens: { type: 'string' },
  verdict: { type: 'string', enum: ['sound', 'sound_with_fixes', 'unsound'] },
  summary: { type: 'string' },
  earlier_findings: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, closed: { type: 'boolean' }, evidence: { type: 'string' } }, required: ['id', 'closed', 'evidence'] } },
  findings: { type: 'array', items: { type: 'object', properties: {
    id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
    claim: { type: 'string' }, evidence: { type: 'string' }, fix: { type: 'string' } },
    required: ['id', 'severity', 'claim', 'evidence', 'fix'] } },
  checked: { type: 'array', items: { type: 'string' } },
}, required: ['lens', 'verdict', 'summary', 'earlier_findings', 'findings', 'checked'] }

const CI_SCOPE = `Allowed paths: tools/ci/**; tools/check_ignored_tests_in_ci.py; .github/workflows/production.yml; .github/workflows/ignored-tests.yml; .github/workflows/a1-inner-tolerance-parity.yml (ONLY remove the step named "Run the workspace test closure"); ${EV}/ci/** except ${EV}/ci/SELECTION_REVIEW.json.`

const CI_BUILD = `${PRE}
TASK "ci-v3": implement the CI test selection v3 that the owner approved (owner decisions OD-L1-19, OD-L1-20 and OD-L1-21, verbatim in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json; read them first).
Worktree ${WTP}ci3, branch research/production-loop-20261116-w3-ci3. export CARGO_TARGET_DIR=${TGP}ci3.
${CI_SCOPE}
READ FIRST: tools/ci/select_tests.py (v2: rules, tokenizer classification noop/body/structural, coverage map, baseline lookup), .github/workflows/production.yml and ignored-tests.yml, tools/check_ignored_tests_in_ci.py, ${EV}/ci/TRIAGE_KO.md sections 8-10, and ${EV}/ci/SELECTION_REVIEW.json (every finding of the two reviewers of v1, with evidence and suggested fixes: F1-F14 and F-01-F-12). The reviewers' scratch files are read-only under /home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-ci-review/ (prototype select_tests_fixed.py, harness.py, harness_fixed.py, sweep.txt, logs of the hosted runs); copy what you need.
DESIGN v3 (approved; implement it; write every choice the design leaves open as a ruling in ${EV}/ci/V3_NOTES.md):
A. The map records, for every test target, its measured inputs: "compile" = repository files in the rustc dep-info of its artifact (every package in scope; integration tests, lib and bin unit tests with profile.test, examples for compile inputs only); "executed" = files with covered regions (the v2 coverage machinery); "read" = repository files the test process opened successfully; "dirs" = repository directories it opened. Paths outside the repository, under the cargo target directory or under .git are dropped. Bump the map and outcome schema versions.
B. Measurement at every run for every target that runs: dep-info from the build; coverage as in v2; reads by strace attached through cargo's runner setting (CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER or the equivalent config) so that only the test executable and its children are traced, with a seccomp filter (for example strace -f --seccomp-bpf -qq -e trace=open,openat,openat2 -e status=successful -o <dir>/trace.%p ...; O_DIRECTORY opens are dirs). strace 6.8 is installed on this host (/usr/bin/strace); the workflows install it when it is missing (sudo apt-get install -y strace) and record its version. Doc tests are not traced (they always run for a selected package). A target whose measurement fails gets no map entry (so it runs next time) and the outcome says so.
C. Selection rules: (1) a global change (Cargo.toml, Cargo.lock, rust-toolchain*, .cargo/, tools/ci/, the workflow file) means a full run; (2) no baseline, no map, a different runner image (the ImageOS and ImageVersion environment variables of GitHub runners, recorded in the outcome) or a different pull-request base sha (github.event.pull_request.base.sha, recorded) since the baseline means a full run; (3) every changed path (added, modified, deleted, both sides of a rename; parse "git diff -z --name-status -M") selects every mapped target whose compile, executed or read set contains it, and an added or deleted path also selects every mapped target whose dirs contain its parent directory; (4) the v2 classification of package sources stays: structural -> every target of the package and of its dependents; body -> rule 3 plus the doc tests of the package and its dependents; noop (only comments and whitespace) -> no package rule, but rule 3 still applies to the read sets and to targets that include the file textually (include_str!, include_bytes!, include!, #[path]) from another crate or test crate; state the exact rule and why it is sound for text consumers; (5) a target in scope without a map entry runs whenever the diff is not empty; (6) baseline failures are re-run, filtered to current targets (a failure of a deleted target is dropped with a note); (7) an example whose compile set contains a changed path is compiled; (8) decide whether the literal matcher of v1/v2 is still needed (for example only for notes about files read by no test) and record the ruling.
D. Mechanics: job-level concurrency with cancel-in-progress false for the test job of production.yml and for the ignored-tests job (queued; GitHub keeps the newest pending run per group); the other jobs of production.yml keep cancelling on non-main refs. Negative-control failures must not let the baseline advance (give the steps ids and upload the outcome only when no control failed, or merge the failed controls into outcome.failed; choose and record). Targets with unmet required-features are never named (cargo metadata lists them). python3 -u for every selector step; a failing "has" call fails its step; errors of the baseline listing get their own note; upload-artifact with overwrite: true; branch names through env, not expanded inside run scripts; git output parsed with -z; dep-info paths normalised against the repository root. tools/check_ignored_tests_in_ci.py counts --workspace coverage only for tests under workspace member directories. a1-inner-tolerance-parity.yml: remove only the step "Run the workspace test closure". No schedule. Keep the full-mode inputs (workflow_dispatch full default true; main runs full), the job time limits 300 and 330 minutes, the artifact names and the coverage instrumentation.
VERIFY and keep the logs under ${EV}/ci/v3_logs/:
1. Extend "python3 -I tools/ci/select_tests.py self-test" with cases for every finding of SELECTION_REVIEW.json that code can show (classification, -z parsing, required-features filtering, baseline filtering, image and base triggers, map rules 3-5) and run it.
2. A local end-to-end run with coverage and strace on a set of targets that contains at least rodas5p-core residual_gain_contract, rodas5p-integrators constants_provenance_contracts, ptsw2b_contracts and ppsw1_6b_contracts (if one of these takes more than 20 minutes, replace it by another target that hashes a test source and record why), generic_scope_contracts, rodas5p-cli alg02_predictive_controller and the integrators lib unit tests if they finish within 20 minutes. The local LLVM tools are a separate rustup home: export VIGILODE_LLVM_BIN to the bin directory of the llvm-tools under /home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-llvm (see ${EV}/ci/TRIAGE_KO.md section 10). Then, with the map that run produced, selections for the review's cases: research/reaudit_accuracy_speed_20261010/gain_witness.rs (F1 -> residual_gain_contract), crates/rodas5p-cli/src/main.rs (F2 -> constants_provenance_contracts), crates/rodas5p-integrators/tests/rnext_common/mod.rs (F3 -> cli alg02_predictive_controller), crates/rodas5p-integrators/tests/unified_candidate_contracts.rs (F4 -> ptsw2b_contracts or ppsw1_6b_contracts), crates/rodas5p-integrators/tests/support/a02_packet_counter_contract.rs (F5 -> integrators lib unit tests), research/generic_policy_redesign_v33/results/calibration_analysis/FROZEN_ZETA34_POLICY.json (F6 -> constants_provenance_contracts), a new file under crates/rodas5p-krylov/src (F7 -> generic_scope_contracts); each must select the named target. Also negative cases: a comment-only change in a file that no target reads or includes textually selects nothing; a body change selects only the targets that executed the file plus doc tests.
3. "python3 tools/check_ignored_tests_in_ci.py"; a YAML parse of the three workflows (python3 with yaml if available, else actionlint if installed); the loop checker.
4. ${EV}/ci/TRIAGE_KO.md: a new section 11 in Korean: what v3 measures, the rules, a table finding id -> fixed or not applicable and how, the residual gaps and their cost, and the expected cost of strace.
Report finding_status for every finding id of SELECTION_REVIEW.json. Report head_commit.`

const reviewPrompt = (lens, focus, build) => `${PRE}
TASK: adversarial review of the CI test selection v3 built by another agent (lens: ${lens}). You did not write this change; your job is to find what is wrong with it. Read-and-run only: do not edit, create or commit anything in any worktree; scratch files only under ${TGP}ci3/review-${lens.split(' ')[0]}/ (create it). export CARGO_TARGET_DIR=${TGP}ci3/review-target-${lens.split(' ')[0]} if you need builds (the builder has finished; keep builds small).
The change: branch research/production-loop-20261116-w3-ci3 in the worktree ${WTP}ci3, base ${BASE} (git -C ${WTP}ci3 diff ${BASE}..HEAD). Owner decisions: OD-L1-19, OD-L1-20, OD-L1-21 in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json. The findings of the review of v1 are in ${EV}/ci/SELECTION_REVIEW.json; the v1 reviewers' harness and sweep are read-only under /home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-ci-review/.
${CI_SCOPE}
The builder's final report (JSON):
<<<
${JSON.stringify(build).slice(0, 9000)}
>>>
CHECK, with evidence (commands you ran, file:line):
1. For every finding id of SELECTION_REVIEW.json that belongs to your lens: is it closed? Reproduce the original failing case against v3 where possible (fill earlier_findings).
2. ${focus}
3. Scope: only allowed paths changed; protected files untouched; "git status --short" clean.
Keep "summary" under 1200 characters.`

const SOUND_FOCUS = 'Soundness: find changes that can change the outcome of a test that v3 does NOT select. Try at least: files read through environment variables set in CI or by build scripts, subprocesses started by tests (are they traced?), files opened by other means than open/openat/openat2 (mmap of an already open fd, execve of scripts, stat-only checks, readlink), directories walked with read_dir (is a new or deleted file in a walked directory selected?), renames, the noop rule (comment changes that alter include_str! consumers, file!/line!/column! uses), map staleness (a target not run for many runs keeps an entry: can a later change in an input it read only conditionally be missed?), targets whose measurement failed, doc tests, examples, lib and bin unit tests, the full-mode triggers (runner image, pull-request base), and the structural rule. Re-run the sweep of every compile-time input with the v3 map logic if feasible, and the seven cases F1-F7 of the old review. For every hole say what it would miss and the smallest fix.'

const MECH_FOCUS = 'CI mechanics: will the workflows run as intended on GitHub-hosted runners? Check job-level concurrency (does a new push still cancel the test job? does queueing coalesce?), installation and use of strace on hosted runners (ptrace permission, the cargo runner setting, seccomp filter flags of the installed version, output size), coverage plus strace together, artifact upload conditions (negative controls), the outcome schema and its baseline lookup, required-features handling, image and base triggers (which variables exist on hosted runners), python3 -u, error paths of has and of the listing, env handling of branch names, the reachability checker, the A1 workflow change (only the one step removed), time limits, and whether the first run after the change is full. Run the self-test and a short local end-to-end run on one or two targets.'

const recheckPrompt = (reviews, fix) => `${PRE}
TASK: recheck of the fixes of the CI test selection v3. Read-and-run only (scratch only under ${TGP}ci3/recheck/). Worktree ${WTP}ci3, branch research/production-loop-20261116-w3-ci3, base ${BASE}.
The two reviews (JSON):
<<<
${JSON.stringify(reviews).slice(0, 12000)}
>>>
The builder's fix report (JSON):
<<<
${JSON.stringify(fix).slice(0, 6000)}
>>>
For every critical and major finding of the two reviews: is it fixed? Reproduce the failing case against the current head and give evidence. Also re-run the self-test, the reachability checker and the loop checker. Use lens "recheck"; put each review finding id in earlier_findings; new problems in findings. Keep "summary" under 1200 characters.`

const fixPrompt = (reviews) => `${CI_BUILD}
FIX ROUND: you are continuing the work on the same branch (the earlier builder has finished; read ${EV}/ci/V3_NOTES.md and the git log of the branch first). Two adversarial reviewers reported (JSON):
<<<
${JSON.stringify(reviews).slice(0, 14000)}
>>>
Apply every critical and major finding you can confirm (reproduce it first; if you cannot confirm one, say why in rulings). Minor findings: fix when cheap, else record a ruling. Re-run the checks of VERIFY that the fixes touch and update TRIAGE_KO.md section 11 and V3_NOTES.md. Report finding_status for the review findings (ids as given by the reviewers, prefixed S- for the soundness review and M- for the mechanics review).`

const V27_SCOPE = `Allowed paths: crates/vigilode/src/session.rs (crate-private pass-throughs only); crates/vigilode/src/unified/** (new module breakeven.rs included); new test targets crates/vigilode/tests/unified_breakeven_*.rs; crates/vigilode/tests/unified_router_contracts.rs only for the changes owner decisions D4 and D5 require (list every edited test and why in ${EV}/router_hyperparameter_free/IMPLEMENTATION_NOTES.md); crates/vigilode/tests/unified_production_solver_contracts.rs and crates/vigilode/examples/unified_production_solver.rs only where a removed public name must be replaced; ${EV}/router_hyperparameter_free/** except PREDECLARED.json, DESIGN_A.md, DESIGN_B.md, DESIGN_RECOMMENDED.md, DESIGN_RESULTS.json, judge_model.py, judge_model_results.jsonl and MODEL_RECHECK_ORCHESTRATOR.jsonl. crates/vigilode/src/lib.rs is protected (it already re-exports unified::*).`

const V27_BUILD = `${PRE}
TASK "v27-phase1": implement node V27_ROUTER_HYPERPARAMETER_FREE, phase 1 only: the forward break-even pilots (FBP) Auto route approved by owner decision OD-L1-22 (verbatim in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json), with D1 (a fatal callback inside a pilot ends the solve with CallbackFailed and the accepted prefix), D2 (a supplied Jacobian, JVP or band starts RODAS5P), D3 (one-way), D4 (the V03b assertion "no DOPRI5 leg" for P_heat_fd and P_scalar_fd becomes the work bound W plus a recorded handover), D5 (the public names of today's route are removed: EXPLICIT_TRIAL_MAX_DIMENSION, DOPRI5_REAL_STABILITY_BOUND, EXPLICIT_TRIAL_ATTEMPT_LIMIT, StiffnessScreen, the removed RouteBasis values, UnifiedStop::Preflight if nothing needs it), D6 (no price option, no counting in the protected dense kernel).
Worktree ${WTP}v27, branch research/production-loop-20261116-w3-v27. export CARGO_TARGET_DIR=${TGP}v27. A detached checkout of the base commit for arm T is ${WTP}v27base (read-only; for builds against it use CARGO_TARGET_DIR=${TGP}v27base).
${V27_SCOPE}
READ FIRST: ${EV}/router_hyperparameter_free/DESIGN_RECOMMENDED.md (sections 3, 4, 7, 8), ${EV}/router_hyperparameter_free/PREDECLARED.json (frozen; phase 1 of its order_of_work is your task), the current router (crates/vigilode/src/unified/*.rs), crates/vigilode/src/session.rs and the native DOPRI5 session (crates/rodas5p-integrators/src/dopri5.rs around the session, attempt telemetry and checkpoint), crates/vigilode/tests/unified_router_contracts.rs (V03b contracts and fixtures), ${EV}/unified_router/ (PREDECLARED.json with the work bound W, REVIEW.json), Rodas5p::solve_observed_bounded in the facade.
ORDER OF WORK (the order is part of the evidence):
0. ${EV}/router_hyperparameter_free/IMPLEMENTATION_NOTES.md, first commit within 15 minutes: file:line of everything you will change, how the DOPRI5 session exposes attempts, how the pilot calls RODAS5P with max_steps = 1 and h0 = H, and the plan of the tests.
1. Commit A: failing-first tests in crates/vigilode/tests/unified_breakeven_contracts.rs against the base API: F2 (y' = -y, fatal for y > 1, y0 = 1, span [0,1]) succeeds under Auto; Lorenz-96 n = 40 under Auto has no RODAS5P leg and costs at most 2x Dopri5::solve; Robertson FD span 40 under Auto charges at most 2 cost(Rodas5p::solve) + 2 b c_D + pilots; the oscillator over [0,1000] and the Arenstorf orbit have no RODAS5P leg (definitions in PREDECLARED.json). Run them on the base sources and keep the log showing that each fails by assertion (not by a compile error; write them so that they compile against both APIs, or record why one cannot). Commit the tests and the log.
2. session.rs: crate-private set_attempt_telemetry and last_attempt pass-throughs; a test that telemetry and quota-1 advances keep the bits and counters of Dopri5::solve.
3. unified/breakeven.rs (break_even_length(n) in integers, the FBP loop, PilotRecord) and the edits of analyzer, fallback, router, legs, solution, mod and options as in section 7 of the design; VigilodeSolution gains the pilot records and a route-history entry for a handover. Tests of the properties U1-U5 (neutrality and the 2 C_D bound, one budget, no callback before DOPRI5's first stage, evidence of a handover, determinism) in unified_breakeven_contracts.rs.
4. The contract changes of D4 and D5 in unified_router_contracts.rs; every other existing test passes unchanged.
5. Suites: "${OTHERS} cargo +1.94.1 test --locked -j 4 --release -p vigilode --no-fail-fast" with its log; clippy with -D warnings on vigilode (all targets); "cargo +1.94.1 fmt -p vigilode -- --check"; cargo doc of vigilode without new warnings; the default-path golden test passes unchanged; the loop checker (the pins of the unified sources and of unified_router_contracts.rs in CHECKPOINT.json will be reported as changed: expected, report verbatim).
6. The evaluation runner: a standalone cargo package ${EV}/router_hyperparameter_free/runner (empty [workspace] table, path dependencies on the crates of the worktree; create its Cargo.lock by copying the repository Cargo.lock and building once with --offline, then --locked) that runs one (arm, problem, tolerance, options) case and prints one JSON line of raw counters, status, stop, callback entries by kind (counted in the closures), pilot records, route digest and final-state bits; arm T is built from a copy of the runner placed under ${TGP}v27/runner_t with absolute path dependencies on ${WTP}v27base/crates (the runner uses only names that exist in both APIs, and a cargo feature for the pilot records of N). Check the plumbing on the toy problem y' = -y over [0,1] only. NO run of any problem of the deciding set in this phase: the pre-registration must be on origin first, and phase 2 is a later task.
Report head_commit, the failing-first evidence (test names and the assertion each failed with on the base) and open issues.`

const items = [
  { key: 'ci3', kind: 'ci' },
  { key: 'v27', kind: 'v27' },
]

const results = await pipeline(
  items,
  it => agent(it.kind === 'ci' ? CI_BUILD : V27_BUILD, { label: 'build:' + it.key, phase: 'Build', schema: RESULT_SCHEMA }),
  async (build, it) => {
    if (it.kind !== 'ci') return { key: it.key, build }
    if (!build) { log('ci3 builder returned nothing; reviews skipped'); return { key: it.key, build: null } }
    log('ci3 builder finished: ' + build.status)
    const reviews = (await parallel([
      () => agent(reviewPrompt('soundness of the selection', SOUND_FOCUS, build), { label: 'review:ci3-soundness', phase: 'Review', schema: REVIEW_SCHEMA }),
      () => agent(reviewPrompt('CI mechanics', MECH_FOCUS, build), { label: 'review:ci3-mechanics', phase: 'Review', schema: REVIEW_SCHEMA }),
    ])).filter(Boolean)
    const serious = reviews.flatMap(r => r.findings || []).filter(f => f.severity !== 'minor')
    log('ci3 reviews: ' + reviews.map(r => r.lens + '=' + r.verdict).join(', ') + '; critical/major findings: ' + serious.length)
    if (!serious.length) return { key: it.key, build, reviews, fix: null, recheck: null }
    const fix = await agent(fixPrompt(reviews), { label: 'fix:ci3', phase: 'Fix', schema: RESULT_SCHEMA })
    const recheck = fix ? await agent(recheckPrompt(reviews, fix), { label: 'recheck:ci3', phase: 'Recheck', schema: REVIEW_SCHEMA }) : null
    return { key: it.key, build, reviews, fix, recheck }
  },
)
return { results }
