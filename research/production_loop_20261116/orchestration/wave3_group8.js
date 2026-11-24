export const meta = {
  name: 'vigilode-loop-l1-wave3-group8',
  description: 'Loop L1 wave 3 group 8: V27 phase 2, the pre-registered evaluation of the forward break-even pilots Auto route (arms T, N, D, R; E1-E10; counters only) followed by an independent read-and-run review',
  phases: [
    { title: 'Evaluate', detail: 'builder runs every arm on the deciding set and parses E1-E10' },
    { title: 'Review', detail: 'independent review of phase 1 code and phase 2 evidence' },
  ],
}

const BASE = args.base
const WTP = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-'
const TGP = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-'
const EV = 'research/production_loop_20261116'
const RH = EV + '/router_hyperparameter_free'
const OTHERS = 'taskset -c 0-5,12-17'

const PRE = `You are one agent of VigilODE research loop L1 (started 2026-11-16), wave 3, group 8. VigilODE is a Rust workspace of stiff ODE integrators with the facade crate crates/vigilode; RODAS5P is the protected default method; node V27 replaces the Auto route of the unified router (opt-in) by forward break-even pilots (FBP).
HARD RULES
- Work only in the worktree and paths your task allows; every other path is read-only for you. Never modify /home/cosmosapjw/vigilode (main checkout; its untracked files belong to the owner), the loop worktree /home/cosmosapjw/worktrees/vigilode-prod-loop-20261116, other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, git stash, git rebase, cargo clean, or any force operation. Stage files with explicit paths.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you. Do not run tools/agent_gate.py start, finish or abort.
- A "Fact-Forcing Gate" hook blocks the first Bash command, each new-file Write and the first Edit of a file. When it does, state the requested facts in one short paragraph and retry the identical call (for a new file: who reads it, that no file with the same purpose exists, its format, and the instruction "execute loop L1 node V27 phase 2 as assigned by the orchestrator").
- The shell working directory resets after every Bash call: use absolute paths or "cd <dir> && <command>" in one call.
- Rust: toolchain 1.94.1 ("cargo +1.94.1"), --locked, "-j 4", the CARGO_TARGET_DIR given in your task, one cargo process at a time, prefix builds and runs with "${OTHERS}". Counters only: no wall-time measurement is part of this node.
- Memory: about 8 GB available; one heavy process at a time.
- Evidence rules: ${RH}/PREDECLARED.json is frozen (committed 2026-11-24 20:04:58 KST in 1ffbda20, on origin since 23:29:45 KST) together with ${RH}/AMENDMENT_A8_2.json (fixture spans; exploratory arms N_prod and N_cont not run) and the errata A8.1 and A8.3 in ${EV}/PREREGISTRATION.md. Thresholds and the decision rule may not be moved or re-interpreted after seeing data. Every number in RESULTS.json is parsed by a script from raw runner output; failed and negative runs are kept; strict JSON (no NaN or Infinity). If a discriminator cannot be evaluated as written, report the exact obstacle (status partial or blocked); do not substitute another criterion.
- No change of the router, the session pass-throughs or any test assertion in this phase: a defect found now is reported, not fixed (a fix would need a new pre-registered round).
- Do not append to research/LEDGER.jsonl and do not edit docs/agent/**, ${EV}/PREREGISTRATION.md, ${EV}/PROGRESS.md or ${EV}/CHECKPOINT.json.
- Commit on your branch in small steps; commit messages end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". Do not push. A guard refuses .md files that you create with the Write tool: render REPORT_KO.md with a script.
- Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.
`

const RESULT_SCHEMA = { type: 'object', properties: {
  status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
  summary: { type: 'string' },
  branch: { type: 'string' }, head_commit: { type: 'string' },
  first_deciding_run_utc: { type: 'string' },
  files_written: { type: 'array', items: { type: 'string' } },
  checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['name', 'result'] } },
  discriminators: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, frozen_threshold: { type: 'string' }, measured: { type: 'string' }, verdict: { type: 'string', enum: ['PASS', 'FAIL', 'NOT_DECIDABLE', 'NOT_RUN'] } }, required: ['name', 'measured', 'verdict'] } },
  frozen_decision: { type: 'string' },
  key_numbers: { type: 'array', items: { type: 'string' } },
  rulings: { type: 'array', items: { type: 'string' } },
  open_issues: { type: 'array', items: { type: 'string' } },
}, required: ['status', 'summary', 'checks', 'discriminators', 'frozen_decision', 'open_issues'] }

const REVIEW_SCHEMA = { type: 'object', properties: {
  node: { type: 'string' },
  verdict: { type: 'string', enum: ['accept', 'accept_with_corrections', 'reject'] },
  summary: { type: 'string' },
  scope_ok: { type: 'boolean' }, protected_ok: { type: 'boolean' }, predeclared_before_results: { type: 'boolean' },
  reran: { type: 'array', items: { type: 'object', properties: { command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['command', 'result'] } },
  discriminator_checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, builder_value: { type: 'string' }, reviewer_value: { type: 'string' }, reviewer_verdict: { type: 'string', enum: ['PASS', 'FAIL', 'NOT_DECIDABLE', 'NOT_RUN'] }, agrees: { type: 'boolean' } }, required: ['name', 'reviewer_verdict', 'agrees'] } },
  findings: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] }, claim: { type: 'string' }, evidence: { type: 'string' }, correction: { type: 'string' } }, required: ['id', 'severity', 'claim', 'evidence'] } },
}, required: ['node', 'verdict', 'summary', 'scope_ok', 'protected_ok', 'discriminator_checks', 'findings'] }

const EVAL = `${PRE}
TASK "v27-phase2": the pre-registered evaluation of node V27 (phase 2 of the order of work in ${RH}/PREDECLARED.json).
Worktree ${WTP}v27, branch research/production-loop-20261116-w3-v27 (phase 1 head c998963d: the FBP route, the failing-first and property tests, the runner ${RH}/runner, IMPLEMENTATION_NOTES.md). export CARGO_TARGET_DIR=${TGP}v27. Arm T is built from the copy of the runner at ${TGP}v27/runner_t with path dependencies on the detached base checkout ${WTP}v27base (commit ${BASE}; read-only; its builds use CARGO_TARGET_DIR=${TGP}v27base).
Allowed paths: ${RH}/** except PREDECLARED.json, AMENDMENT_A8_2.json, DESIGN_A.md, DESIGN_B.md, DESIGN_RECOMMENDED.md, DESIGN_RESULTS.json, judge_model.py, judge_model_results.jsonl, MODEL_RECHECK_ORCHESTRATOR.jsonl and PHASE1_BUILD.json; the runner sources (both copies); doc comments only (no code, no assertion) of crates/vigilode/examples/unified_production_solver.rs (lines that still describe the removed screened trial) and of contract_4 in crates/vigilode/tests/unified_router_contracts.rs.
READ FIRST: ${RH}/PREDECLARED.json, ${RH}/AMENDMENT_A8_2.json, the errata A8.1 and A8.3 at the end of ${EV}/PREREGISTRATION.md, ${RH}/IMPLEMENTATION_NOTES.md and ${RH}/PHASE1_BUILD.json, the runner, and ${RH}/DESIGN_RECOMMENDED.md sections 4 and 8.
ORDER OF WORK (the order is part of the evidence):
0. ${RH}/PHASE2_NOTES.txt, first commit within 15 minutes: the case list you will run (problem x tolerance x options x arm), how each discriminator is computed from which runner fields, the reference of every problem, and how a closure entry is classified for E5.
1. References: closed form where the pre-registration allows (linear problems, oscillator, heat by its modes, the scalar FD problem by its closed form if you derive and state it), else Rodas5p::solve at rtol 1e-12, atol 1e-14; record them with the command; commit.
2. Every case of the deciding set for the arms T, N, D and R, each three times (two runs in one process, one in a fresh process), raw output kept under ${RH}/logs/phase2/; the time of the first run (UTC) is reported in first_deciding_run_utc. Note: Dopri5::solve on stiff problems may stop at max_steps; that is a result, not an error.
3. ${RH}/parse_results.py reads only the raw runner output and the references and writes ${RH}/RESULTS.json: per case the measures of the pre-registration (charged callbacks by kind, closure entries by kind, operations by the one formula, attempts, pilots with H, window cost and pilot cost, handover time, status, stop, error against the reference in the weighted RMS norm, bits and digests of the three runs), each discriminator E1-E10 with the numbers it was decided on, the frozen decision, and the declared expected losses next to the measured values. It also renders ${RH}/REPORT_KO.md (Korean, for the owner: what changed, the table of costs of T and N against D and R, every discriminator, the decision, the losses).
4. The doc-comment edits of the example and of contract_4 (describe the break-even route; no code change), then "${OTHERS} cargo +1.94.1 test --locked -j 4 --release -p vigilode --test unified_router_contracts --test unified_breakeven_contracts" and "cargo +1.94.1 test --locked -j 4 --release -p vigilode --example unified_production_solver" or a build of the example, with logs.
5. Strict-JSON load of every JSON file you wrote; regenerate RESULTS.json and REPORT_KO.md once more and check they are byte-identical; commit.
Report head_commit, every discriminator, the frozen decision, and open issues.`

const REVIEW = (build) => `${PRE}
TASK: independent review of node V27 (phase 1 code and phase 2 evidence), built by other agents. You did not write this work; your job is to find what is wrong with it. Read-and-run only: no edit, no commit in any worktree; scratch only under ${TGP}v27/review-scratch/; for builds use CARGO_TARGET_DIR=${TGP}v27/review-target (and ${TGP}v27base/review-target for the base checkout), never the builder's directories.
Branch research/production-loop-20261116-w3-v27 in ${WTP}v27 (base ${BASE}); base checkout ${WTP}v27base. Frozen: ${RH}/PREDECLARED.json, ${RH}/AMENDMENT_A8_2.json, errata A8.1 and A8.3 of ${EV}/PREREGISTRATION.md; owner decision OD-L1-22 in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json; design ${RH}/DESIGN_RECOMMENDED.md.
The phase-2 builder's final report (JSON):
<<<
${JSON.stringify(build).slice(0, 9000)}
>>>
CHECK in this order and give evidence:
1. Scope and order: git diff --name-only ${BASE}..HEAD against the phase-1 and phase-2 allowed paths (phase 1: crates/vigilode/src/session.rs, crates/vigilode/src/unified/**, crates/vigilode/tests/unified_breakeven_*.rs, the D4/D5 edits of unified_router_contracts.rs listed in IMPLEMENTATION_NOTES.md, ${RH}/**); protected files untouched (crates/vigilode/src/lib.rs, automatic.rs, switching.rs, solve.rs, golden files, Cargo files); the failing-first commit A precedes the implementation and its log shows assertion failures on the base; no run of the deciding set before 2026-11-24 23:29:45 KST (14:29:45 UTC) (log headers, file times, commit times).
2. Code against design: break_even_length(n) = ceil((2n^2 + 54n + 384)/156); the located rule (first accepted attempt not growth-capped, or after a rejection; exact equality against max_factor); pilot times b * 2^k with at least b located attempts; H and the horizon; one RODAS5P attempt with max_steps 1 and h0 = H, charged; handover from (t_D + H, y_pilot) with h0 = H and the attempts left; a lost pilot leaves the DOPRI5 session bit-identical; D1 (fatal in a pilot ends the solve with CallbackFailed); D2 start rule; no callback before DOPRI5's first stage. Are the U1-U5 tests able to fail (try one mutation in a scratch copy)? Are the D4/D5 contract edits limited to what the owner decided?
3. Re-run with your own builds: the vigilode tests unified_router_contracts and unified_breakeven_contracts and the default-path golden test; at least eight deciding cases for T and N (robertson_40, lorenz96_40, F2, N2, P_heat_fd, P_scalar_fd, osc_1000, arenstorf) and compare counters, bits and digests with the builder's raw logs; check that T was built against the base checkout.
4. Recompute E1-E10 and the frozen decision from the raw logs with a short script of your own; audit the E5 classification of closure entries and the reference of E7; trace at least five numbers of REPORT_KO.md to raw lines.
Verdict "accept" only if the evidence supports every claim; "accept_with_corrections" if specific claims or labels must be corrected; "reject" if a discriminator verdict or the frozen decision is unsupported, or a scope, order or protection rule was broken. Keep "summary" under 1200 characters.`

const items = [{ key: 'v27' }]
const results = await pipeline(
  items,
  () => agent(EVAL, { label: 'evaluate:v27', phase: 'Evaluate', schema: RESULT_SCHEMA }),
  (build) => {
    if (!build) { log('evaluation returned nothing; review skipped'); return { build: null, review: null } }
    log('evaluation finished: ' + build.status + '; decision: ' + build.frozen_decision)
    return agent(REVIEW(build), { label: 'review:v27', phase: 'Review', schema: REVIEW_SCHEMA }).then(review => ({ build, review }))
  },
)
return { results }
