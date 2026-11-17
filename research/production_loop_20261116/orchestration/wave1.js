export const meta = {
  name: 'vigilode-loop-l1-wave1',
  description: 'Loop L1 wave 1: adversarial audit, stage theory, Schwarz screen, default-path golden oracle, trial merge of switcher lineage, stage client kit',
  phases: [
    { title: 'Audit', detail: 'three review dimensions over both code lines' },
    { title: 'Verify', detail: 'independent refutation attempt per new major finding' },
    { title: 'Synthesize', detail: 'audit report, merged findings, release-gate proposal' },
    { title: 'Theory', detail: 'exact-arithmetic statements about the Rodas5P stage structure' },
    { title: 'Screen', detail: 'Python screen of spatial Schwarz stage-chain iteration' },
    { title: 'Golden', detail: 'bitwise oracle of default-facing entry points' },
    { title: 'TrialMerge', detail: 'merge switcher lineage on trial branch and run gates T1-T6' },
    { title: 'ClientKit', detail: 'dense reference and timing stage clients' },
  ],
}

const LOOP = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116'
const EV = LOOP + '/research/production_loop_20261116'
const HEADS = '/home/cosmosapjw/vigilode'
const S_SHA = '933922495045796c99c530b0f1f1efd8fec83ed0'
const L_SHA = '2dc9a2c5778ac5b3ac960b0f6741aa5916122384'
const TGT = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116'

const PRE = `You are one agent of VigilODE research loop L1 (2026-11-16). VigilODE is a Rust workspace of stiff ODE integrators; RODAS5P (8-stage Rosenbrock-Wanner, order 5) is the protected default method.
HARD RULES
- Work only in the paths named in your task. Never modify ${HEADS} (the main checkout: it holds the owner's untracked files and a hook blocks edits there; reading is fine), other worktrees, branch main or any remote. Never run git push, git reset --hard, git clean, git gc, cargo clean, or any force operation.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does NOT apply to you: its gate tasks are all done. Do not run tools/agent_gate.py start/abort.
- A "Fact-Forcing Gate" hook may block your first Bash command and each new-file Write. When it does, state the requested facts in one short paragraph (the task in one sentence; what the command verifies, or for a new file: who reads/calls it, that no file with the same purpose exists, its data format, and the instruction "execute loop L1 node") and retry the identical call. If it flags a destructive command, do not run that command; find a non-destructive way.
- Rust: the worktree pins toolchain 1.94.1 (use "cargo +1.94.1"); always pass --locked and "-j 6"; export CARGO_TARGET_DIR exactly as given. Never run an unfiltered "cargo test -p rodas5p-integrators" (251 test targets): always name targets. The host is heavily loaded: builds are slow, be patient, do not start parallel cargo processes.
- Evidence: numbers in result files must be produced by executed runs and parsed from their raw output (no typed-in results). Keep failed runs. Strict JSON (no NaN/Infinity). Wall time on this shared host is diagnostic_only; deterministic counters are primary.
- Protected, never edit: fixtures/rodas5p_coefficients_snapshot.json; crates/rodas5p-core/src/{coefficients.rs,work.rs}; crates/rodas5p-integrators/src/{sequential.rs,parallel.rs,stage_batch.rs,ptsw2b.rs,candidates.rs,source_owned_graph_schur.rs,rodas5p_fast.rs,rodas5p_matrix_free_fast.rs,rodas5p_fast_dense.rs,rodas5p_lagged.rs,client_stage.rs,adaptive.rs}; crates/rodas5p-krylov/src/block_gmres.rs; crates/vigilode/src/{lib.rs,automatic.rs,switching.rs}; Cargo.toml; Cargo.lock; rust-toolchain.toml. Tolerances, controller constants, default routing and the G0 gate are immutable.
- Write full details to files. Your final answer is ONLY the compact structured result; keep "summary" under 1200 characters.
REFERENCE PATHS
- LOOP worktree (branch research/production-loop-20261116, commit ${L_SHA}): ${LOOP}. Evidence root: ${EV} (contains PLAN_KO.md = approved plan with plan-stage findings in section 1, PREREGISTRATION.md = frozen discriminators).
- Switcher lineage (unmerged, commit ${S_SHA}) is the checked-out tree of ${HEADS}; read files there or with "git -C ${HEADS} show ${S_SHA}:<path>".
`

const RESULT_SCHEMA = { type: 'object', properties: {
  status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
  summary: { type: 'string' },
  branch: { type: 'string' }, head_commit: { type: 'string' },
  files_written: { type: 'array', items: { type: 'string' } },
  checks: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, command: { type: 'string' }, exit_code: { type: 'integer' }, result: { type: 'string' } }, required: ['name', 'result'] } },
  key_numbers: { type: 'array', items: { type: 'string' } },
  open_issues: { type: 'array', items: { type: 'string' } },
}, required: ['status', 'summary', 'checks', 'open_issues'] }

const FINDINGS_SCHEMA = { type: 'object', properties: {
  dimension: { type: 'string' }, summary: { type: 'string' }, file: { type: 'string' },
  plan_items_checked: { type: 'array', items: { type: 'object', properties: { item: { type: 'string' }, verdict: { type: 'string', enum: ['confirmed', 'corrected', 'refuted', 'not_checked'] }, note: { type: 'string' } }, required: ['item', 'verdict'] } },
  findings: { type: 'array', items: { type: 'object', properties: {
    id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] }, is_new: { type: 'boolean' },
    title: { type: 'string' }, claim: { type: 'string' }, evidence: { type: 'string' }, reproduce_cmd: { type: 'string' }, impact: { type: 'string' }, minimal_fix: { type: 'string' },
  }, required: ['id', 'severity', 'is_new', 'title', 'claim', 'evidence'] } },
}, required: ['dimension', 'summary', 'findings'] }

const VERDICT_SCHEMA = { type: 'object', properties: {
  finding_id: { type: 'string' }, verdict: { type: 'string', enum: ['confirmed', 'refuted', 'inconclusive'] }, evidence: { type: 'string' }, correction: { type: 'string' },
}, required: ['finding_id', 'verdict', 'evidence'] }

const AUDIT_TAIL = `
METHOD. First read ${EV}/PLAN_KO.md section 1: those items were found by the orchestrator at plan stage. In "plan_items_checked" give your own verdict for every plan item that falls in your dimension, re-derived with commands (do not trust the text). In "findings" list at most 10 findings, most important first; id format "<DIM>-NN"; set is_new=false for items already stated in the plan and is_new=true for what the plan missed or got wrong. Every finding needs file:line or command evidence and a reproduce_cmd where possible. Say for each defect whether it invalidates a recorded verdict or only limits its scope. You are read-only on sources: write ONLY your findings file ${EV}/audit/findings_<dim>.json (create the directory if needed; do not commit; if you need scratch space use a directory under /tmp/claude-1000 that you create). Final answer = the structured object (the same findings, compact).`

const DIMS = [
  { key: 'evidence', prompt: `${PRE}
TASK: audit dimension EVIDENCE (id prefix EVD): evidence authenticity and governance.
A. Switcher lineage (read-only at ${HEADS}): for every research node directory added on that line (research/*_2026110[34]*/, plus docs/agent/production_solver_qualification_20261106_v2 and _20261107_v3 evidence) classify each evaluation script as EXECUTES_SOLVER (invokes cargo/a binary/valgrind, or reads raw output files produced by one: name them and check they exist in the tree) or SYNTHETIC (numbers typed in or computed from a formula). List the ledger rows of research/LEDGER.jsonl (L-0205..L-0214) whose claims rest on SYNTHETIC scripts. Check whether the benchmark examples that docs/agent/switcher_v2_remediation/DESIGN.md prescribed for those nodes exist. Check the cross-solver benchmark (bench/out/*.json, bench/*.py, crates/vigilode-bench) for a real runner and raw data.
B. Unification line (in LOOP; identical to commit 074a5699 for these paths): for U03, U04, U09, U10, U11 under research/production_unification_20261115/ check that the RESULTS/EVALUATION JSON can be regenerated from the raw logs by the included scripts (run scripts on COPIES in scratch; never overwrite repository files) and that at least 5 numbers per node sampled from each REPORT_KO.md match the JSON. Re-run at most these three already-built test targets with CARGO_TARGET_DIR=/mnt/extdrive/vigilode-loop-20261116/target from ${LOOP}: "cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test ppsw1_independent_qualification", "... -p vigilode --test unified_method_capabilities", "... -p vigilode --test geometry_mol_contracts"; compare pass counts with the reports.
C. Governance: ledger continuity (rows for U nodes?), "python3 -I tools/check-research-node.py --base bee5563c51264a7da19c519295bdda2fb317caef" in LOOP, which GitHub workflows would run for a push to a research branch (.github/workflows/production.yml triggers), and the root cause of the flaky test crates/rodas5p-krylov/tests/sp02_workspace_charge.rs (the loop baseline saw 2 failures that vanish with --test-threads=1).${AUDIT_TAIL}` },
  { key: 'design', prompt: `${PRE}
TASK: audit dimension DESIGN (id prefix DSG): experiment design and regime coverage.
1. U11 holdout (${LOOP}/research/production_unification_20261115/production_holdout/: PREREGISTRATION.json, EVALUATION.json, evaluate.py, runner/, logs/): recompute from the raw runner outputs, separately for the PPSW1 1-thread and 6-thread arms, the matched-error step ratio and speed against RODAS5P, and the per-step wall cost ratio PPSW1(1 thread)/RODAS5P per problem and tolerance. Read the runner source and state exactly what the timed region includes (starter, thread-pool creation, how each arm obtains its Jacobian, build profile). State which arm the G0 speed criterion used and whether the preregistration fixed that before the run.
2. PPSW1 controller provenance: ACCEPTED_STEP_SIGMA_MAX and the hysteresis rule in crates/rodas5p-integrators/src/ppsw1.rs; the text of OD-15, OD-25, OD-26 in docs/agent/unified_local_agent_20261109/OWNER_DECISIONS_20261110.json; if step histories are logged, the fraction of accepted steps at the cap (else NOT_AVAILABLE).
3. Regime coverage: build a table of every matched-error or speed evaluation you can locate in research/LEDGER.jsonl and research/*/RESULTS*.json (aim for at least 25 evaluations across the project history): problem, dimension n, Jacobian type, linear backend, whether the RHS cost dominates. Then count evaluations with n > 1000, with 2D/3D operators, and with an expensive RHS.
4. Cross-solver benchmark on the switcher line (${HEADS}/bench/out/UNIFIED_BENCHMARK_REPORT.md, bench/protocols/*.json, crates/vigilode-bench/src): is it at matched achieved error, which VigilODE arms are clipped to the 101-point output grid, reproduce three table rows from the raw JSON.
5. U04 matrix-free verdict (${LOOP}/research/production_unification_20261115/matrix_free_w/): did any arm have a preconditioner, which n range, is "no crossover" a regime statement or a method statement.${AUDIT_TAIL}` },
  { key: 'code', prompt: `${PRE}
TASK: audit dimension CODE (id prefix COD): the frontend and the candidate code paths.
1. ${LOOP}/crates/vigilode/src: selection.rs (construct concrete inputs where MethodRegistry admission disagrees with GeneralOde::select, for example a banded problem with n above max_dense_dimension; list every constraint select enforces that descriptors lack), PeerOde (lowering, the error mapping in callback_to_core, the time grid starting at t0+2*h0, switch_history_to), production.rs, automatic.rs, solve.rs (what Tier::Performance does), capabilities.rs.
2. ${LOOP}/crates/rodas5p-integrators/src/ppsw1.rs and ppsw1_linear_backend.rs: per-step heap allocations and repeated work (tableau(sigma) recomputation, Vec allocation in ParallelExecution::map_ordered, dense Jacobian built from n JVPs) that explain a measured 17-22 microseconds per step at n=3..8 against about 1 microsecond for the protected driver; give file:line and an estimate of each cost.
3. Switcher line, read-only at ${HEADS}: crates/vigilode/src/unified/{router,analyzer,fallback,options,solution,workspace}.rs, registry.rs (WindowDriver returns Result<_, String>), hybrid_automaton.rs, production.rs, switching.rs. List every place where a label or doc comment claims an engine that is not dispatched, where a failure reason is replaced by a generic one, where work counters are dropped, where a fallback is silent or uncharged.
4. Hygiene in LOOP: tracked crates/rodas5p-integrators/src/dense_output_v2.rs.rej; research-only modules compiled into the production crates (count the mod lines of crates/rodas5p-integrators/src/lib.rs by category); crates outside the workspace; README claims against code.
Do not edit any source file.${AUDIT_TAIL}` },
]

const verifyPrompt = (f, dim) => `${PRE}
TASK: independent verification of ONE audit finding (dimension ${dim.key}). Try to REFUTE it. Reproduce it from primary files and commands only; do not rely on the wording of the finding or on ${EV}/PLAN_KO.md. If you cannot reproduce it exactly as stated, answer "refuted" or "inconclusive" and say what you observed instead; put a corrected statement in "correction" when the finding is partly right. Read-only: write no files. Limit yourself to what is needed to decide this one finding.
FINDING: ${JSON.stringify(f)}`

const synthPrompt = (dims) => `${PRE}
TASK: synthesize the loop L1 audit. Inputs: the three findings files in ${EV}/audit/ (findings_evidence.json, findings_design.json, findings_code.json) and the verification verdicts below. Also read ${EV}/PLAN_KO.md sections 1 and 2 and ${LOOP}/docs/agent/production_solver_program_20261112/RELEASE_GATE_MATRIX.json.
Write (do not commit) into ${EV}/audit/:
1. audit_findings.json: merged list; each finding with dimension, severity, is_new, verification {verdict, evidence} or "not_sent" and the orchestrator-reproduced flag for plan items; drop nothing, mark refuted ones as refuted.
2. AUDIT_KO.md: Korean, at most 220 lines, plain technical prose: (a) 판정 요약 (5-8 lines), (b) 확정된 finding 표, 심각도순, each with 근거 경로 and 최소 교정, (c) 계획 단계 항목 검증표 (confirmed/corrected/refuted), (d) 기록된 판정에 미치는 영향 (which recorded verdicts stay valid, which are limited in scope, which are invalid), (e) 미실행 항목. No promotion language; separate executed from not run.
3. R_GATE_PROPOSAL.json: for each axis id of RELEASE_GATE_MATRIX.json a proposed status in {NOT_MET, PARTIAL, MET, NOT_RUN} with reasons and evidence paths taken from the findings; a proposal only.
4. LINEAGE_ADDENDUM.md: English, short: the switcher lineage (${S_SHA}) and the rei lineage are absent from research/production_unification_20261115/lineage.json; list the corrections the findings support (do not edit lineage.json: it is hash-pinned evidence).
VERDICTS AND DIMENSION SUMMARIES: ${JSON.stringify(dims.map(d => d && ({ dimension: d.dimension, summary: d.summary, plan_items_checked: d.plan_items_checked, findings: d.findings.map(f => ({ id: f.id, severity: f.severity, is_new: f.is_new, title: f.title })), verdicts: d.verdicts })))}`

const V05 = `${PRE}
TASK: node V05, theory note with machine-checked statements. Write (do not commit) under ${EV}/theory/: verify_tableau.py (exact rational arithmetic with fractions.Fraction on the decimal strings of ${LOOP}/fixtures/rodas5p_coefficients_snapshot.json; reconstruct Gamma=(I/gamma-C)^-1, alpha=A*Gamma, beta=alpha+Gamma, b=Gamma^T*b_code exactly as ${LOOP}/crates/rodas5p-core/src/coefficients.rs lines 168-212 do), verify_independent.py (a second implementation, mpmath at 60 digits or sympy Rational, that does not import the first), RESULTS.json (written by the scripts), THEORY.md (English). Tag every statement [exact], [zero by design within representation: residual value], [numerical] or [conjecture].
Statements to establish or refute:
S1. alpha (the coupling of RHS arguments) is a full strictly lower triangle, so in the black-box RHS model the eight evaluations form a chain of depth 8. State precisely what this excludes and what it does not.
S2. beta21 = 0 by design (alpha21 = 3*gamma, gamma21 = -3*gamma), so N = beta - gamma*I has nilpotency index 7; stability function R(z) = P(z)/(1-gamma z)^7 with deg P <= 6; R(z)-exp(z) = O(z^7); give P, R(infinity), and a numerical check of |R(iy)| <= 1.
S3. Sweep amplification: for the simultaneous-sweep iteration of ${LOOP}/research/vigilode_r6_eightstage_causal_frontier_20261101/THEORY_R6.md (Theorem 1) on y' = lambda*y the error propagation matrix is z/(1-gamma z)*N with stiff limit -N/gamma; tabulate ||(N/gamma)^m|| in 1-, 2- and infinity-norm for m = 1..7, and the minimum over diagonal scalings for m = 1 (to show the growth is not a scaling artefact).
S4. W-conditions: sum b_i gamma_i (order 2) and the order-3 W-trees for the main weights b and for the embedded weights (row 7 of beta). Consequence for a Jacobian error E: leading perturbation of the main solution, of the embedded solution and of the error estimate. Check the primary literature on whether Rodas5P (Steinebach, BIT Numerical Mathematics 2023, "Construction of Rosenbrock-Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package") and Rodas4P2 were designed with the order-2 W condition; use web search if you have it (generic queries only, no project or user names); cite equation numbers or mark UNVERIFIED.
S5. Orders of the stage arguments g6, g7, g8 as approximations of y(t_n+h) under the classical ROW order conditions up to order 5; the structural identities alpha_8 = beta_7, b = beta_8, alpha_7 = beta_6.
S6. Speculation band: if the next step's Jacobian and factorisation are overlapped with stages 7-8 the saving is min(T_JLU, T_chain/4): speed-up at most 1.25 and at least 1.15 exactly when T_JLU/T_chain is in [0.15, 0.9167]; with stage 8 only the bound is 1.125.
S7. Cost model per step (clearly labelled model, to be confronted with measurements): dense (LU 2n^3/3, solve 2n^2), banded (LU about n*b^2, solve about 4nb), matrix-free (8 Krylov solves), finite-difference Jacobian (n RHS): closed forms for the share of the eight-stage chain and for the bound of each lever (parallel Jacobian assembly, LU reuse, shared Krylov basis, speculation).
S8. Schwarz locality: for W = I - gamma*h*D*Laplace_h (1D 3-point and 2D 5-point) the discrete Green's function decays like rho^d; derive rho in 1D exactly from the characteristic equation, relate it to ell = sqrt(gamma*h*D)/dx, verify numerically, and state the expected per-sweep contraction for overlap delta and how the transient depends on S3.
S9. Level-structured ROW feasibility: count order conditions for order 5 (17 trees) plus the additional conditions Rodas5P imposes (cite) against the free coefficients of a two-processor tableau with L = 5 or 6 levels; a counting argument only, go/no-go.
Both scripts must exit 0 and agree; RESULTS.json must be generated by them. Record the exact commands.`

const V14 = `${PRE}
TASK: node V14 screen (Python only: numpy/scipy; no Rust, no cargo). Write (do not commit) under ${EV}/schwarz_screen/: screen.py, RESULTS.json (written by the script), README.md.
MODEL. RODAS5P in U-form with the exact tableau of ${LOOP}/fixtures/rodas5p_coefficients_snapshot.json, as in ${LOOP}/crates/rodas5p-integrators/src/rodas5p_fast.rs lines 1160-1202: W = I/(h*gamma) - J; stage i: rhs = f(y + sum_j A_ij U_j) + sum_j (C_ij/h) U_j; W U_i = rhs; y_new = y + sum_j b_code_j U_j; the embedded error is U_8.
PROBLEMS on 2D grids with N in {64, 128}: P1 heat equation with a smooth source; P2 2D Brusselator with standard parameters; P3 Gray-Scott with pattern-forming parameters. Use sparse matrices and scipy.sparse.linalg.splu.
STEP SIZES. Integrate with the exact global solve and a standard embedded-error controller (WRMS norm, atol = rtol in {1e-4, 1e-6}, safety 0.9, factor limits [0.2, 5]) past a short transient and record accepted h. This is a stand-in for, not a copy of, the protected Rust controller: say so in the README.
ITERATION. For a sample of at least 20 accepted steps per (problem, tolerance): restricted additive Schwarz on the full eight-stage system with P x P subdomains (P in {2, 4}): every subdomain runs the whole eight-stage chain on its extended block (owned cells plus overlap of delta = m*ell cells, ell = sqrt(gamma*h*D_max)/dx, m in {1, 2, 4}) with a local LU of W, taking stage values outside the block from the previous sweep (Jacobi update, all subdomains read the old iterate), then writes back only its owned cells. Initial halo guesses: g0 zero increments; g1 the previous accepted step's stage increments scaled by the step-size ratio.
RECORD per sweep: WRMS of (y_new_sweep - y_new_exact) in tolerance units, the same relative to the step's embedded error estimate, and the WRMS of the ORIGINAL global stage residuals. "Sweeps needed" = first sweep with WRMS endpoint difference <= 0.1 tolerance units.
REPORT per configuration: median and max sweeps needed, halo width in cells, halo/subdomain-width ratio, overlap work factor (1+2*delta/H)^2, the diffusion number sigma = h*D/dx^2 of the accepted steps. Evaluate the frozen criteria: PASS = median sweeps <= 2 at overlap <= 4*ell with predictor g1, endpoint difference <= 0.1 tolerance, and median halo <= H/8; KILL = median sweeps > 4. State plainly which configurations pass, which fail and which are killed; do not tune thresholds.
LIMITS. Single process, total runtime under 25 minutes; if N = 128 is too slow do N = 64 fully and N = 128 for P1 only and say so. RESULTS.json must be written by screen.py and include the command line and the wall time.`

const V03G_WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-v03g'
const V03G = `${PRE}
TASK: node V03G, bitwise oracle of the default-facing entry points. Your worktree: ${V03G_WT} (branch research/production-loop-20261116-v03g, at ${L_SHA}). export CARGO_TARGET_DIR=${TGT}/v03g. You may create only: crates/vigilode/tests/default_path_golden.rs, files under crates/vigilode/tests/golden/default_path/, and research/production_loop_20261116/golden_oracle/ (receipt). No source edits.
PURPOSE. The oracle is the neutrality test for a later merge of another code line: it must detect any bit change of results or work counters on the routes a default user reaches.
DESIGN. One integration test file. Each case calls a public API of the vigilode crate and serialises: status, final time, number of output times, the final state as hex of f64::to_bits, every work counter exposed by the result type, and for dense-output cases the bits of five interpolated states at fixed fractions of the span. Compare against a golden JSON file; when the environment variable VIGILODE_BLESS_DEFAULT_PATH=1 is set, write the golden files instead of comparing. Golden files are deterministic JSON (sorted keys).
CASES (read crates/vigilode/src/{lib.rs,solve.rs,capabilities.rs,selection.rs,production.rs,automatic.rs,explicit.rs,problem.rs,options.rs} and existing tests such as facade_contracts.rs, production_selection_contracts.rs, p0_contracts.rs for API usage):
- Rodas5p::solve on Robertson (n=3), HIRES (n=8), Van der Pol mu=1000 on a short span, 1D Brusselator N=32 (n=64); rtol in {1e-4, 1e-6, 1e-8} with explicit atol; covering every FacadeRoute you can reach: explicit dense Jacobian, finite-difference Jacobian, banded Jacobian, JVP (matrix-free), dense_output=true (dense and banded), mass matrix or InteriorError::Enforce (sequential route).
- Dopri5::solve on a non-stiff problem (rotation or a smooth nonlinear system).
- GeneralOde::solve with the default policy and with a Stiff policy that permits derivatives.
- ProductionSolver::solve with the default ProductionConfig, and AutomaticOde::solve with its default policy.
- A decision grid: exhaustive enumeration of GeneralOde::select over problem features (Jacobian kind Dense/Banded/Jvp/FiniteDifference, autonomous or not, time derivative supplied or not, mass, events, declared band), Options (dense_output, interior error), dimension in {8, 64, 65, 500} and every GeneralOdePolicy combination (regime x derivatives x allow_matrix_free x max_dense_dimension in {64, 1000}); record the decision or the refusal variant; use callbacks that increment an atomic counter and assert it stays zero.
If an entry point is not deterministic (threads) say so and exclude it with the reason; never loosen to a tolerance.
STEPS. Write the test. Bless once. Run it twice without bless: both must pass. Also run it once in the debug profile (no --release) without bless to learn whether debug and release agree bit for bit; report the answer (a disagreement is a finding, not a failure). Commands: "cargo +1.94.1 test --locked -j 6 --release -p vigilode --test default_path_golden"; "cargo +1.94.1 clippy --locked -j 6 --release -p vigilode --test default_path_golden --no-deps -- -D warnings"; "rustfmt +1.94.1 --check --edition 2024 crates/vigilode/tests/default_path_golden.rs". Write research/production_loop_20261116/golden_oracle/RECEIPT.json (commands, exit codes, number of cases, number of grid entries, sha256 of each golden file; values produced by a script or the test itself, not typed in). Commit everything on your branch with a normal descriptive commit message ending with the line "Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>". Do not push. Report head_commit.`

const V15_WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-v15'
const V15 = `${PRE}
TASK: node V15, stage client kit. Your worktree: ${V15_WT} (branch research/production-loop-20261116-v15, at ${L_SHA}). export CARGO_TARGET_DIR=${TGT}/v15. You may create or edit only: crates/rodas5p-integrators/src/stage_clients/{mod.rs,dense_lu.rs,timed.rs}, crates/rodas5p-integrators/tests/stage_clients_*.rs, and research/production_loop_20261116/client_kit/ (receipt).
BACKGROUND. The protected fast RODAS5P driver has a public extension point: the trait StageLinearSystem in crates/rodas5p-integrators/src/client_stage.rs, used through integrate_rodas5p_fast_client_observed_with_atol (rodas5p_fast.rs about line 2094; reference client BandedStageLu about line 1587; LU kernels re-exposed as "pub mod lu_research" about lines 1471-1519; the dense LU choice in choose_lu about line 859: in-place LU when n <= 64 or density <= 0.1, otherwise faer through rodas5p_core::LuFactorization). Later loop nodes (cost anatomy, shared Krylov basis, speculative Jacobian, threaded Jacobian assembly) will be written as clients, so the kit must prove that a client can reproduce the protected driver exactly.
DELIVER.
1. dense_lu.rs: DenseLuStageClient implementing StageLinearSystem with two explicit modes (InPlace, Faer) so that the client driver reproduces the protected dense driver (integrate_rodas5p_fast_observed_with_atol or the matching public entry, same options and LU policy) bit for bit: accepted and rejected step counts, every accepted time and state (f64 bits), and the work counters the trait lets you charge.
2. timed.rs: TimedStageClient<C> forwarding to an inner client and accumulating nanoseconds (std::time::Instant) and call counts for the linearize, factor and solve phases in a PhaseTimes struct; no heap allocation per call.
3. tests/stage_clients_dense_identity.rs: (i) bit identity on Robertson (n=3), HIRES (n=8), 1D Brusselator N=32 (n=64) and N=100 (n=200) at rtol 1e-4, 1e-6, 1e-8 for both LU modes where the protected policy offers them; (ii) the timed wrapper is result-neutral and its call counts match the driver's counters (one factorisation per attempt, eight solves per attempt).
4. tests/stage_clients_alloc.rs: exactly ONE #[test] in its own test binary using a counting global allocator: zero allocations per accepted step after warm-up for the dense client (a process-global counter is racy when several tests share a binary).
5. research/production_loop_20261116/client_kit/RECEIPT.json written from executed output: commands, exit codes, test counts, and a diagnostic wall ratio client/protected on Brusselator N=100 (7 interleaved repetitions, labelled diagnostic_only).
If bit identity cannot be reached through the trait, stop and report exactly where the client path diverges from the protected path (file:line and the first differing quantity) instead of forcing it; the fallback (a sibling driver like rodas5p_lagged.rs) is a decision for the orchestrator.
COMMANDS: "cargo +1.94.1 test --locked -j 6 --release -p rodas5p-integrators --test stage_clients_dense_identity --test stage_clients_alloc"; "cargo +1.94.1 clippy --locked -j 6 --release -p rodas5p-integrators --lib --test stage_clients_dense_identity --test stage_clients_alloc --no-deps" (the loop baseline has 8 pre-existing lints in source_owned_graph_schur.rs: report new warnings separately and fix yours); "rustfmt +1.94.1 --check --edition 2024 <your files>". Commit on your branch with a descriptive message ending with "Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>". Do not push. Report head_commit.`

const TRIAL_WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-trial'
const v02t = (g) => `${PRE}
TASK: node V02, trial merge of the switcher lineage. Your worktree: ${TRIAL_WT} (branch research/production-loop-20261116-trial, at ${L_SHA}). export CARGO_TARGET_DIR=${TGT}/trial. The owner decided: adopt the merge only if the 1.94.1 build, the tests and bitwise identity of the default-facing paths pass. You produce the evidence; you do not adopt anything. Protected files MAY change on this trial branch only through the merge itself and the fix-ups listed below.
GOLDEN ORACLE from node V03G: ${g ? JSON.stringify({ status: g.status, branch: g.branch, head_commit: g.head_commit, summary: g.summary }) : 'UNAVAILABLE (run T3 as NOT_RUN)'}
STEPS.
1. Merge: "git merge --no-ff --no-commit ${S_SHA}". Resolve conflicts only: crates/rodas5p-integrators/src/lib.rs keeps the module lines of both sides; research/LEDGER.jsonl keeps the loop branch's existing lines in place through row L-0300 and then appends the switcher rows L-0205..L-0214 in their original order (the ledger is append-only relative to the loop branch); anything else: prefer a union and report it. No reverts and no content edits inside the merge commit beyond conflict resolution. Commit the merge with a message that lists the resolutions.
2. Fix-ups, each a separate minimal commit after the merge commit: (a) rust-toolchain.toml back to channel 1.94.1 and rust-version = "1.94.1" in the root Cargo.toml and crates/vigilode-coupled/Cargo.toml (report what crates outside the workspace declare); (b) std APIs newer than 1.94.1 (for example AtomicU64::try_update) replaced by equivalents with identical semantics; (c) the new enum variant Jacobian::SparseJvp handled in crates/vigilode/src/selection.rs (CapabilitySet::from_problem_and_policy) with the same permissions as Jvp; report it as a decision that needs review; (d) lint expectations that are unfulfilled on 1.94.1. Do not edit the merged .claude/settings.json, CLAUDE.md or tools/agent_gate.py; report what hooks the merged tree would install for a session started in it.
3. T1: "cargo +1.94.1 check --workspace --all-targets --locked -j 6". If --locked fails because the merge changed Cargo.lock, report the dependency delta (new crates and versions) and continue with the merged lock. If T1 needs fix-ups outside classes (a)-(d), document the first three offending errors and stop T1 there.
4. T2: run with --release --no-fail-fast, logs under research/production_loop_20261116/trial_merge/logs/: "-p vigilode -p rodas5p-core -p rodas5p-krylov"; the peer targets "-p rodas5p-integrators --lib --test ppsw1_6b_contracts --test ppsw1_independent_qualification --test ppsw1_parallel_rhs_contracts --test ppsw1_session_boundary_contracts --test ppsw1_linear_backend_contracts --test ptsw2b_contracts"; and the integrators/krylov test targets the switcher line added (find them with "git diff --name-only --diff-filter=A ${L_SHA} HEAD -- 'crates/*/tests'"), named explicitly. Baseline on the loop branch: 789 passed / 2 failed (the thread-raced sp02_workspace_charge; rerun that binary with "-- --test-threads=1") / 21 ignored, and 218 passed / 0 failed / 3 ignored.
5. T3: bring the oracle with "git cherry-pick <head_commit of V03G>" and run "cargo +1.94.1 test --locked -j 6 --release -p vigilode --test default_path_golden" WITHOUT blessing. Report every differing case with its first differing field. For differences, find the responsible source hunks ("git diff ${L_SHA} HEAD -- <file>") and say whether excluding a hunk is possible without breaking the switcher tests. Never bless on this branch.
6. T5: "cargo +1.94.1 clippy --locked -j 6 --release -p vigilode -p rodas5p-core --all-targets --no-deps -- -D warnings" and clippy for "-p rodas5p-integrators --lib"; compare the warning set with the loop baseline (8 pre-existing lints in source_owned_graph_schur.rs).
7. T6: "python3 -I tools/check-research-node.py --base ${L_SHA}"; list ledger ids after the merge; read research/k1_shared_krylov_basis_20261104/eval_k1.py, research/c1_coloured_assembly_20261104/eval_c1.py, research/b7_first_batch_20261104/eval_b7.py, research/d1_stiffness_gate_20261104/eval_d1.py and state for each whether it executes a solver. Do NOT append ledger rows.
8. Write under research/production_loop_20261116/trial_merge/: REPORT.md, BLOB_DELTA.json (for each of the protected paths listed in research/production_loop_20261116/CHECKPOINT.json: blob id before and after, changed or not, diffstat; produced by a script), T_GATES.json (T1, T2, T3, T5, T6 each PASS / FAIL / NOT_RUN with command and log path). Commit the report on the trial branch (message ending with "Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>"). Do not push. Never touch the loop branch or other worktrees.
STOP RULES: at most two attempts per failing step; a failing gate is a result to report, not something to force green.`

phase('Audit')
const out = await parallel([
  () => pipeline(
    DIMS,
    (d) => agent(d.prompt, { label: `audit:${d.key}`, phase: 'Audit', schema: FINDINGS_SCHEMA, effort: 'high' }),
    async (r, d) => {
      if (!r) return null
      const cand = r.findings.filter(f => f.is_new && f.severity !== 'minor')
      const targets = cand.slice(0, 3)
      if (cand.length > targets.length) log(`${d.key}: ${cand.length - targets.length} new major findings not sent to verification (cap 3 per dimension)`)
      const verdicts = await parallel(targets.map(f => () => agent(verifyPrompt(f, d), { label: `verify:${f.id}`, phase: 'Verify', schema: VERDICT_SCHEMA, effort: 'medium' })))
      return { ...r, verdicts: verdicts.filter(Boolean) }
    },
  ).then(async (dims) => {
    const ok = dims.filter(Boolean)
    const synth = ok.length ? await agent(synthPrompt(ok), { label: 'audit:synthesis', phase: 'Synthesize', schema: RESULT_SCHEMA, effort: 'high' }) : null
    return { dims: ok.map(d => ({ dimension: d.dimension, summary: d.summary, plan_items_checked: d.plan_items_checked, findings: d.findings.map(f => ({ id: f.id, severity: f.severity, is_new: f.is_new, title: f.title })), verdicts: d.verdicts })), synth }
  }),
  () => agent(V05, { label: 'theory:V05', phase: 'Theory', schema: RESULT_SCHEMA, effort: 'high' }),
  () => agent(V14, { label: 'screen:V14', phase: 'Screen', schema: RESULT_SCHEMA, effort: 'high' }),
  () => agent(V03G, { label: 'golden:V03G', phase: 'Golden', schema: RESULT_SCHEMA, effort: 'high' })
    .then(async (g) => ({ v03g: g, v02t: await agent(v02t(g), { label: 'trial:V02', phase: 'TrialMerge', schema: RESULT_SCHEMA, effort: 'high' }) })),
  () => agent(V15, { label: 'kit:V15', phase: 'ClientKit', schema: RESULT_SCHEMA, effort: 'high' }),
])
return { audit: out[0], v05: out[1], v14: out[2], golden_trial: out[3], v15: out[4] }
