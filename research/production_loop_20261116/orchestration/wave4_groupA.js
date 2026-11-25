export const meta = {
  name: 'vigilode-wave4-groupA',
  description: 'Wave 4 group A: L1 V31 (default check All on wide bands) and V32 (protected stage_batch probe), KS-N7 (two-for-one real FFT), KS-N8 (field-of-values theory); each with an independent review',
  phases: [
    { title: 'Build', detail: 'four builders, one worktree each' },
    { title: 'Review', detail: 'independent read-and-run review of each result' },
  ],
}

const SCR = '/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad'
const LOCKA = SCR + '/locks/cargo_a.lock'
const LOCKB = SCR + '/locks/cargo_b.lock'
const WT = '/home/cosmosapjw/worktrees/'
const TL1 = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-'
const TKS = '/home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/'
const EV = 'research/production_loop_20261116'
const KS = 'research/kernel_spectral_foundation_20261124'

const COMMON = (lock) => `COMMON RULES
- Work only in your own worktree and only in the paths your task allows; everything else is read-only. Never modify /home/cosmosapjw/vigilode (main checkout; its untracked files belong to the owner), other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, git stash, git rebase, git commit --amend, cargo clean, or any force operation. Stage files with explicit paths.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you. Do not run tools/agent_gate.py start, finish, abort or approve.
- A "Fact-Forcing Gate" hook blocks the first Bash command, each new-file Write and the first Edit of a file. When it does, state the requested facts in one short paragraph and retry the identical call (who reads or calls the file, that no file with the same purpose exists, its format, and the instruction "owner 2026-11-25 09:21 KST: 남은 노드들을 이어서 마저 진행해. 수정은 전부 승인할게.").
- The shell working directory resets after every Bash call: use absolute paths or "cd <dir> && <command>" in one call.
- Rust: toolchain 1.94.1 ("cargo +1.94.1"), --locked, "-j 4", the CARGO_TARGET_DIR of your task. EVERY cargo command is prefixed with "flock -w 14400 ${lock} taskset -c 0-5,12-17" (a host-wide build lock shared with other agents; never hold it while you think, only for the command). Never run an unfiltered "cargo test -p rodas5p-integrators": name the test targets. Other projects' batch jobs load the host (load average 9-14) and memory is tight (about 13 GB free): one heavy process at a time; runs are slow; commit often.
- No wall-time measurement decides anything in this group: deciding timing cells are preregistered and run later by the orchestrator on a quiet host. Diagnostic timings you take are labelled diagnostic.
- Evidence: numbers in result files are parsed by a script from raw output of executed runs; failed and negative runs are kept; strict JSON. Do not report an unrun check as passed. If the same failure repeats twice, stop and report it.
- A guard refuses .md files created with the Write tool: render any new report .md with a script (editing an existing .md with Edit is allowed where your task allows it).
- Commit on your branch in small steps (notes first, then code, then logs and results); commit messages end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>". Do not push.
- Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.`

const L1PRE = (lock) => `You are one agent of VigilODE research loop L1 (since 2026-11-16), wave 4 group A. VigilODE is a Rust workspace of stiff ODE integrators (crates rodas5p-core, rodas5p-krylov, rodas5p-integrators) with the facade crate crates/vigilode; RODAS5P is the protected default method.
${COMMON(lock)}
- Protected, never edit unless your task names the file as an exception: fixtures/rodas5p_coefficients_snapshot.json; crates/rodas5p-core/src/{coefficients.rs,work.rs}; crates/rodas5p-integrators/src/{lib.rs,problem.rs,ppsw1.rs,sequential.rs,parallel.rs,stage_batch.rs,ptsw2b.rs,candidates.rs,source_owned_graph_schur.rs,rodas5p_fast.rs,rodas5p_matrix_free_fast.rs,rodas5p_fast_dense.rs,rodas5p_lagged.rs,client_stage.rs,adaptive.rs,routing.rs,stage_clients/mod.rs,stage_clients/dense_lu.rs,stage_clients/timed.rs}; crates/rodas5p-integrators/tests/{ppsw1_6b_contracts.rs,ptsw2b_contracts.rs,stage_clients_dense_identity.rs,stage_clients_alloc.rs}; crates/rodas5p-krylov/**; crates/vigilode/**; every Cargo.toml and Cargo.lock; rust-toolchain.toml. Do not append to research/LEDGER.jsonl and do not edit docs/agent/**, ${EV}/PREREGISTRATION.md, ${EV}/PROGRESS.md or ${EV}/CHECKPOINT.json.
- Loop checker from your worktree root: "python3 -I ${EV}/check_checkpoint.py" (a complaint that only names a pinned or protected file your task changes is expected; report it verbatim).
All L1 worktrees of this group start at commit d4e2772f (owner decisions OD-L1-26..30 in docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json; node contracts in TASK_CONTRACTS.json).`

const KSPRE = (lock) => `You are one agent of the kernel/spectral PLAN of VigilODE, branch research/stage-lattice-referee-20261124 (unit ${KS}). VigilODE is a Rust workspace of stiff ODE integrators with the facade crate crates/vigilode; RODAS5P is the protected default method. Read CLAUDE.md, ${KS}/PLAN.json (rules, decisions_applied, your task), ${KS}/RUN.json, and the THEORY.md sections your task names.
${COMMON(lock)}
- Brief rules of this unit: never edit CLAUDE.md, .claude/**, .github/**, Cargo.lock by hand, rust-toolchain.toml, docs/agent/switcher_v2_remediation/**, tools/agent_gate.py; never append to research/LEDGER.jsonl; do not edit ${KS}/RUN.json or ${KS}/PLAN.json (the orchestrator appends RUN.json at merge). New results go under ${KS}/ as new files (results/KS_N*.json or .jsonl, a node directory). Performance verdicts need owner-host preregistered cells (the orchestrator runs them).
- Facade research-gates: some research modules are behind the cargo feature research-gates (SL-N6); the default build of the facade must not need them.
Your worktree starts at commit 05b5e3e3 (KS-N1 done: ${KS}/owner_host/, results/KS_N1_*).`

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

const V31 = `${L1PRE(LOCKA)}
TASK: node V31_ASSEMBLY_DEFAULT_WIDE_BAND (owner decision OD-L1-27): the default first-use check of the coloured route of the opt-in AssemblyStageClient becomes CheckColumns::All where CheckColourings::total_groups of the declared pattern is >= n (All is exact up to rounding and not more expensive there), and stays CheckColumns::MultiColouring otherwise; the dense route keeps Sample(8). Ruling already made by the orchestrator: ">= n" (at equality All costs the same and is exact).
Worktree ${WT}vigilode-prod-loop-20261116-w4-v31, branch research/production-loop-20261116-w4-v31. export CARGO_TARGET_DIR=${TL1}v31.
Allowed paths: crates/rodas5p-integrators/src/stage_clients/assembly.rs and crates/rodas5p-integrators/src/stage_clients/assembly/**; crates/rodas5p-integrators/tests/stage_clients_assembly_*.rs (existing tests: only lines that pin the default check of the coloured route or its counts, each listed with its reason in RESULTS.json); ${EV}/assembly_wide_band/**.
READ FIRST: node V29 (${EV}/assembly_default/: NOTES.txt, RESULTS.json, REPORT_KO.md with its correction notice, REVIEW.json and BUILD.json, especially review finding F4 and the cost survey; run.sh; parse_results.py); the recording program crates/rodas5p-integrators/tests/stage_clients_assembly_default_record.rs and the tests stage_clients_assembly_default*.rs; AssemblyOptions::default_first_use_check, CheckColourings::of and total_groups, Engine::prepare_multi, AssemblyStageClient::new.
ORDER OF WORK:
0. ${EV}/assembly_wide_band/NOTES.txt, first commit within 15 minutes: the plan; how total_groups is known at construction without any right-hand side; which V26 corpus cases and V08 identity cells have total_groups >= n (compute them with a small program BEFORE any client edit and list them).
1. Commit A before any client edit: a recording with today's code (the V29 default) of the V08 identity cells (Jacobian, linearization-point, output and final-state fingerprints, steps, every counter) and of the V26 corpus decisions of the default (reuse the V29 recording program; add a mode or a new ignored test target stage_clients_assembly_wide_band_record.rs if needed).
2. The change, with rustdoc that states the rule and the reason. Sequential clients must still allocate nothing after new(); with_first_use_check, request_check and verify keep every variant selectable.
3. Recording B with the same program and the same commands.
4. New tests (not ignored): a wide declared band picks All; the equality case total_groups == n picks All; a narrow band keeps MultiColouring; the dense route keeps Sample(8); the allocation contract for a client whose default became All.
5. ${EV}/assembly_wide_band/parse_results.py writes RESULTS.json and renders REPORT_KO.md (Korean, for the owner): identity cells A vs B (bits and counters), corpus decisions A vs B (every wrong declaration detected by A stays detected by B; the cases where the rule picks All with their decisions and costs; correct patterns rejected), the first-use check cost in right-hand sides A vs B per case class.
6. Checks: "cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --no-fail-fast --test 'stage_clients_*' --test ppsw1_6b_contracts --test ptsw2b_contracts"; clippy -D warnings on the library and your assembly test targets; "cargo +1.94.1 fmt -p rodas5p-integrators -- --check"; the loop checker.
Discriminators: identity unchanged; rule per route implemented; corpus detection not worse than A; suites.`

const V32 = `${L1PRE(LOCKB)}
TASK: node V32_STAGE_BATCH_PROBE (owner decision OD-L1-28). Exception: you may edit the protected crates/rodas5p-integrators/src/stage_batch.rs, but only the concurrency probe (run_concurrency_probe) and private helpers it needs. Goal: the probe waits, with a bound, until a second task is active, so that parallel-observed and the scientific checksum of the stage-batch feasibility report no longer depend on the scheduler; the checksum composition stays.
Context: ${EV}/ci/TRIAGE_KO.md section 12.1 and ${EV}/ci/v3_first_hosted/ (checksum_flip.py, local_runs.log, the hosted log excerpt): on a hosted runner (4 vCPU, coverage build, tests under strace -f) the second of two CLI runs observed no concurrency (flipping parallel-observed alone reproduces the hosted checksum). The probe's 8 tasks each sleep 1 ms on a 4-thread rayon pool; when the other workers start late (thread creation under strace -f, load), one worker can run every task alone.
Worktree ${WT}vigilode-prod-loop-20261116-w4-v32, branch research/production-loop-20261116-w4-v32. export CARGO_TARGET_DIR=${TL1}v32.
Allowed paths: crates/rodas5p-integrators/src/stage_batch.rs (probe only); new tests crates/rodas5p-integrators/tests/stage_batch_probe_*.rs; ${EV}/stage_batch_probe/**.
ORDER OF WORK:
0. ${EV}/stage_batch_probe/NOTES.txt, first commit within 15 minutes: why the old probe can see 1; the design of the bounded rendezvous (each task marks itself active and waits in short sleeps until the largest number of simultaneously active tasks seen is >= 2 or a deadline passes; state the bound and why; a one-thread pool can never see 2, so the deadline ends the wait and the result stays as today); effect on run time.
1. Old behaviour: build the CLI binary (cargo +1.94.1 build --locked -j 4 --release -p rodas5p-cli) and try to provoke the failure: run "rodas5p stage-batch-feasibility --profile smoke --output <file>" many times (say 40) under "strace -f -qq -e trace=none" with taskset on 1 and on 2 CPUs; count runs whose stage_parallelism_observed is false; keep the raw outputs under ${EV}/stage_batch_probe/logs/. If no failure can be provoked, say so (that item NOT_DECIDABLE) and keep the logs.
2. The change.
3. Tests: crates/rodas5p-integrators/tests/stage_batch_probe_contracts.rs: repeated run_stage_batch_feasibility on the smoke profile (state N) - stage_parallelism_observed true every time and one checksum; on this host the checksum must equal 4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950 (same composition; if it differs, report exactly why).
4. The provocation of step 1 again with the new code; logs.
5. ${EV}/stage_batch_probe/parse_results.py writes RESULTS.json and renders REPORT_KO.md (Korean).
6. Checks: "cargo +1.94.1 test --locked -j 4 --release -p rodas5p-cli --test cli_contracts"; the existing tests that use stage_batch (grep the tests of rodas5p-integrators and rodas5p-cli for stage_batch or stage-batch and run those targets by name); your new test; clippy -D warnings on the integrators library and your test; fmt; the loop checker (a complaint about the protected blob of stage_batch.rs is expected: report it verbatim; the orchestrator re-pins it under OD-L1-28).
Discriminators: old failure provoked (PASS if provoked, NOT_DECIDABLE if not); new code never shows the failure under the same provocation; checksum unchanged; suites.`

const KSN7 = `${KSPRE(LOCKA)}
TASK: PLAN.json KS-N7 "Two-for-one real FFT on the periodic box". Read THEORY.md section 2 (in particular 2.3, the involution) and section 7 (implementation contract); crates/vigilode/src/spatial/separable.rs (BoxKernel, plans, solve); crates/vigilode/tests/spatial_separable.rs (the exactness and mass tests T1/T2 and the ignored diagnostics).
Worktree ${WT}vigilode-stage-lattice-ksn7, branch research/stage-lattice-referee-20261124-ksn7. export CARGO_TARGET_DIR=${TKS}ksn7.
Allowed paths: crates/vigilode/src/spatial/separable.rs; crates/vigilode/tests/spatial_separable.rs (append tests; existing assertions unchanged); ${KS}/ks_n7/**; ${KS}/results/KS_N7_*.
DESIGN CONSTRAINT: keep today's full complex transform selectable and keep it the DEFAULT (for example a public enum BoxTransform { Complex, PairedReal } on SeparableStages with a with_box_transform setter). The owner-host timing cell (acceptance: box stage solve time <= 0.6 of the current one at 256x256; kill: no gain) is preregistered and run later by the orchestrator; only that cell may make PairedReal the default.
ORDER OF WORK:
0. ${KS}/ks_n7/NOTES.txt, first commit within 15 minutes: the plan; the algebra of the pairing (rows y and y+1 packed as u + i v along x; separation by R Z(m) = conj Z(nx - m)); where the Hermitian half is transformed along y and z; odd row counts; plan reuse and memory.
1. PairedReal.
2. Tests (append to spatial_separable.rs, not ignored): PairedReal agrees with Complex to round-off on every box grid of the existing tests (assert a stated bound and print the largest difference); the T1/T2 bounds of the existing tests hold for PairedReal (backward error and mass identity as stated there); even and odd sizes; the z range of the existing tests.
3. An ignored measurement test measure_box_transform_one_arm that reads the environment variables KS_N7_ARM (complex or paired), KS_N7_GRID (for example 256x256 and 128x128x64 if the box is 3-D; use the box shapes the code supports) and KS_N7_SOLVES, runs ONE arm per process and prints one JSON line (arm, grid, solves, median seconds per stage solve, largest backward error), so that perf stat can count one arm per process. Run it yourself only as a smoke test with few solves; label the numbers diagnostic.
4. ${KS}/ks_n7/make_results.py writes ${KS}/results/KS_N7_IMPLEMENTATION.json from your test output logs (largest differences, backward errors, smoke timings marked diagnostic).
5. Checks: "cargo +1.94.1 test --locked -j 4 --release -p vigilode --test spatial_separable"; "cargo +1.94.1 clippy --locked -j 4 -p vigilode --all-targets --release --no-deps -- -D warnings"; "cargo +1.94.1 fmt -p vigilode -p rodas5p-integrators -- --check"; "python3 -I ${KS}/scripts/verify_run.py". Do not run the whole facade suite (the orchestrator does at merge).
Discriminators: agreement with Complex (PASS/FAIL), T1/T2 bounds for PairedReal (PASS/FAIL), timing NOT_RUN (orchestrator).`

const KSN8 = `${KSPRE(LOCKB)}
TASK: PLAN.json KS-N8 "Field-of-values replacement for advection (open)". Theory task; no Rust change. Read THEORY.md completely (sections 2, 4 inertia certificate, 5 contractivity, Gershgorin spectral bound and stiffness certificate, 7, 8) and skim crates/vigilode/src/spatial/separable.rs (SpectralBounds, inertia) and crates/vigilode/src/spatial (DiffusionGrid) for the discrete operator.
Question: for non-normal J (advection-diffusion with ring-constant velocity on the sphere grid), what replaces the inertia certificate and the contractivity bound? Ring-constant velocity keeps the longitude symmetry, so each Fourier mode block is a tridiagonal non-Hermitian matrix. Work out a computable certificate, for example: the logarithmic 2-norm mu_2(B_m) = lambda_max((B_m + B_m^*)/2) per mode, giving solvability and the resolvent bound ||(zI - J)^{-1}||_2 <= 1/(z - max_m mu_2(B_m)) for z > max_m mu_2(B_m) (the role inertia plays for the symmetric case), and a computable enclosure of the numerical range W(B_m) (Hermitian and skew-Hermitian parts, or the boundary by rotation) as the spectral and stiffness bound replacing the Gershgorin bound; compare its tightness with Gershgorin. State exactly what is proven and what is only checked; mark the rest UNVERIFIED.
Acceptance (PLAN): a stated, checked bound on 3 small grids. Kill: no computable bound tighter than Gershgorin. Fallback: the separable route stays diffusion-only.
Worktree ${WT}vigilode-stage-lattice-ksn8, branch research/stage-lattice-referee-20261124-ksn8. Allowed paths: ${KS}/THEORY.md (append a new section after section 9 numbered 10, existing text unchanged); ${KS}/scripts/verify_fov.py (new); ${KS}/results/KS_N8_FOV.json (written by that script); ${KS}/ks_n8/** (notes, logs).
ORDER OF WORK: 0. ${KS}/ks_n8/NOTES.txt first commit within 15 minutes (plan). 1. Theory: statement, proof sketch with every step, the discrete operator with ring-constant velocity (centered or upwind: say which and why; build it from the grid of the code). 2. ${KS}/scripts/verify_fov.py (python3 -I, numpy 2.4.2 is installed): on 3 small grids (dense matrices, exact eigen-decompositions) and several velocities and z values, check (a) the resolvent bound and solvability, (b) eigenvalues inside the computed enclosure, (c) the ratio of your bound to the Gershgorin bound; write ${KS}/results/KS_N8_FOV.json; exit 0 only if every check holds. Optional: symbolic checks with the mathharness or wolfram MCP tools (load them through ToolSearch). 3. The THEORY.md section 10. 4. Checks: "python3 -I ${KS}/scripts/verify_fov.py", "python3 -I ${KS}/scripts/verify_foundation.py /tmp/ks_foundation_check_ksn8", "python3 -I ${KS}/scripts/verify_run.py".
Discriminators: bound holds on 3 grids (PASS/FAIL); tighter than Gershgorin (PASS/FAIL/KILL).`

const reviewPrompt = (pre, node, where, focus, target) => `${pre}
YOU ARE THE INDEPENDENT REVIEWER (read and run; you did not build this). Node ${node}. Builder worktree ${where} (read-only for you; do not commit there). Use your own CARGO_TARGET_DIR=${target} for any build. Write scratch files only under ${SCR}/review_${node}/.
Check scope (git diff of the builder branch against its base: only allowed paths), protected files untouched (except the named exception), the order of commits (notes, recordings before changes where the task says so), that every number in the result files is parsed from raw logs (re-run the parser into your scratch directory and compare), rebuild and re-run the tests and checks the task names, recompute each discriminator yourself, and try to break the change: ${focus}
Return accept, accept_with_corrections or reject with findings (critical, major, minor) and the commands you ran.`

phase('Build')
const B = [
  { key: 'V31', prompt: V31, label: 'build:v31',
    review: (b) => reviewPrompt(L1PRE(LOCKA), 'V31_ASSEMBLY_DEFAULT_WIDE_BAND', WT + 'vigilode-prod-loop-20261116-w4-v31', 'edge cases of the rule (total_groups == n, n small, empty or diagonal patterns, the band back ends, threaded execution), allocation after new(), that every case detected by the old default stays detected, and the rustdoc statements. Builder result: ' + JSON.stringify(b).slice(0, 6000), TL1 + 'v31-review') },
  { key: 'V32', prompt: V32, label: 'build:v32',
    review: (b) => reviewPrompt(L1PRE(LOCKB), 'V32_STAGE_BATCH_PROBE', WT + 'vigilode-prod-loop-20261116-w4-v32', 'whether the change touches anything in stage_batch.rs beyond the probe, the deadline on a one-thread pool, deadlock or livelock risk, run-time cost, the checksum composition (it must be unchanged), and whether the provocation evidence is real. Builder result: ' + JSON.stringify(b).slice(0, 6000), TL1 + 'v32-review') },
  { key: 'KSN7', prompt: KSN7, label: 'build:ks-n7',
    review: (b) => reviewPrompt(KSPRE(LOCKA), 'KS-N7', WT + 'vigilode-stage-lattice-ksn7', 'the algebra of the two-for-one split (Hermitian symmetry, the m = 0 and Nyquist modes, odd sizes), agreement with Complex on grids the builder did not test, that Complex stays the default, and that the one-arm measurement test really runs one arm per process. Builder result: ' + JSON.stringify(b).slice(0, 6000), TKS + 'ksn7-review') },
  { key: 'KSN8', prompt: KSN8, label: 'build:ks-n8',
    review: (b) => `${KSPRE(LOCKB)}
YOU ARE THE ADVERSARIAL CHECKER of a theory result (KS-N8, field-of-values certificate for advection-diffusion). Worktree ${WT}vigilode-stage-lattice-ksn8 is read-only for you; write scratch only under ${SCR}/review_KS-N8/. Try to REFUTE the stated bound: re-derive the proof sketch step by step, run ${KS}/scripts/verify_fov.py, and add your own numerical tests (more grids, other velocities including large Peclet numbers, z close to the bound, complex z if the claim covers it, both centered and upwind discretizations if relevant); check the comparison with Gershgorin and whether 'tighter' holds on every tested case; check that THEORY.md section 10 states only what is proven or checked. Return accept, accept_with_corrections or reject with findings. Builder result: ` + JSON.stringify(b).slice(0, 6000) },
]
const results = await pipeline(B,
  (item) => agent(item.prompt, { label: item.label, phase: 'Build', schema: RESULT_SCHEMA }),
  (build, item) => agent(item.review(build), { label: 'review:' + item.key, phase: 'Review', schema: REVIEW_SCHEMA }).then(review => ({ key: item.key, build, review })))
return { results }
