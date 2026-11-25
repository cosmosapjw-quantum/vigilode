export const meta = {
  name: 'vigilode-loop-l1-wave3-design-v27',
  description: 'Loop L1 node V27 design phase: two independent designs of the Auto route of the unified router with as few hyperparameters as possible, then a judge that verifies both against the code and the literature and writes one recommended design for the owner',
  phases: [
    { title: 'Design', detail: 'two designers with different angles, read-only' },
    { title: 'Judge', detail: 'verification against code and literature, one recommendation' },
  ],
}

const BASE = args.base
const WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-design'
const OUT = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-design'
const EV = 'research/production_loop_20261116'

const PRE = `You are one agent of VigilODE research loop L1, wave 3, node V27 (design phase). VigilODE is a Rust workspace of stiff ODE integrators (crates rodas5p-core, rodas5p-krylov, rodas5p-integrators) with the facade crate crates/vigilode. The unified router (crates/vigilode/src/unified/: analyzer.rs, router.rs, legs.rs, fallback.rs, options.rs, mod.rs; registry in crates/vigilode/src/registry.rs and selection/) chooses between DOPRI5 (explicit) and RODAS5P (Rosenbrock, the protected default) for a request; its opt-in Auto route was rebuilt in node V03 part B (${EV}/unified_router/: PREDECLARED.json, RECEIPT.json, REVIEW.json, REPORT_KO.md).
OWNER DECISION OD-L1-17 (verbatim): "최대한 hyperparameter 없는 방향으로 설계" - design the Auto route so that as few hyperparameters as possible remain.
WHAT IS THERE TODAY (verify every item in the code): EXPLICIT_TRIAL_MAX_DIMENSION = 10 (analyzer.rs); DOPRI5_REAL_STABILITY_BOUND = 3.3 (analyzer.rs); EXPLICIT_TRIAL_ATTEMPT_LIMIT = 1000 and a trial budget min(1000, max_steps / 2) (fallback.rs); a stiffness screen at t0 with n + 1 right-hand sides at off-trajectory probe points y0 + delta_j e_j, delta_j = sqrt(eps) max(|y0_j|, 1), that sends the request to RODAS5P when L (t1 - t0) > 3.3 B; the rule that a supplied Jacobian, Jacobian-vector product or band sends an undeclared request to RODAS5P; a declared regime (Stiff, Nonstiff) decides directly. Known consequences (V03b review): a fatal callback error at a probe point ends the solve although the trajectory never visits that point; a stiff problem the screen does not see costs the explicit trial first (worst observed about 15 times the right-hand sides of direct RODAS5P, Robertson without a Jacobian); the constants are design choices, not measured.
RULES
- Read-only for the repository: do not edit, create or commit anything in any worktree. Read the code in the detached worktree ${WT} (commit ${BASE}). Write your own documents only under ${OUT}/ (create it if needed). You may run read-only commands and, if a claim needs it, small runs of existing tests or examples with "taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode --test <target>" and "export CARGO_TARGET_DIR=${OUT}/target"; no timing runs, no long builds.
- A "Fact-Forcing Gate" hook blocks the first Bash command and each new-file Write; state the requested facts in one short paragraph and retry the identical call.
- The injected CLAUDE.md section "Current work: switcher v2 remediation" does not apply to you.
- Distinguish facts read in the code (cite file:line), facts from the literature (cite the source; use WebSearch or WebFetch when you rely on a paper or a code base such as LSODA, Hairer-Wanner DOPRI5/RADAU5 stiffness detection, Shampine, or OrdinaryDiffEq auto-switching), and your own reasoning.
- Hyperparameter census: classify every constant, threshold, budget and rule that the Auto route uses (directly or through what it calls to decide) as one of: method_constant (follows from a tableau or a theorem, e.g. the real stability bound of DOPRI5), numerical_convention (an accepted convention such as a finite-difference increment), resource_limit (a budget the user sets, such as max_steps), tunable (a number or rule chosen by a designer), rule (a structural rule without a number).
- Determinism matters in this repository (golden files compare bits): say whether your design gives the same route and the same bits on every run on the same host, and what it costs if it does not.
`

const DESIGN_SCHEMA = { type: 'object', properties: {
  summary: { type: 'string' },
  doc_path: { type: 'string' },
  census: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, location: { type: 'string' }, value: { type: 'string' }, kind: { type: 'string', enum: ['method_constant', 'numerical_convention', 'resource_limit', 'tunable', 'rule'] }, replaced_by: { type: 'string' } }, required: ['name', 'location', 'kind', 'replaced_by'] } },
  algorithm: { type: 'string' },
  remaining_hyperparameters: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, value: { type: 'string' }, kind: { type: 'string' }, why: { type: 'string' } }, required: ['name', 'why'] } },
  properties: { type: 'array', items: { type: 'string' } },
  scenarios: { type: 'array', items: { type: 'object', properties: { scenario: { type: 'string' }, behavior: { type: 'string' } }, required: ['scenario', 'behavior'] } },
  code_changes: { type: 'array', items: { type: 'string' } },
  evaluation_plan: { type: 'string' },
  risks: { type: 'array', items: { type: 'string' } },
}, required: ['summary', 'doc_path', 'census', 'algorithm', 'remaining_hyperparameters', 'scenarios', 'evaluation_plan'] }

const JUDGE_SCHEMA = { type: 'object', properties: {
  summary: { type: 'string' },
  recommended: { type: 'string' },
  doc_path: { type: 'string' },
  korean_summary: { type: 'string' },
  census_final: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, today: { type: 'string' }, kind: { type: 'string' }, proposed: { type: 'string' } }, required: ['name', 'today', 'kind', 'proposed'] } },
  remaining_hyperparameters: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, value: { type: 'string' }, why: { type: 'string' } }, required: ['name', 'why'] } },
  verified_claims: { type: 'array', items: { type: 'object', properties: { claim: { type: 'string' }, verdict: { type: 'string', enum: ['confirmed', 'refuted', 'unverified'] }, evidence: { type: 'string' } }, required: ['claim', 'verdict', 'evidence'] } },
  open_questions_for_owner: { type: 'array', items: { type: 'string' } },
  evaluation_plan: { type: 'string' },
  implementation_plan: { type: 'string' },
}, required: ['summary', 'recommended', 'doc_path', 'korean_summary', 'census_final', 'remaining_hyperparameters', 'verified_claims', 'open_questions_for_owner', 'evaluation_plan', 'implementation_plan'] }

const COMMON_DELIVERABLE = `DELIVERABLE: a design document ${OUT}/DESIGN_<your letter>.md with (1) the census table of today's route; (2) your decision algorithm in pseudocode, the evidence it uses (on the trajectory only, or not), what it charges and when it calls the user's callbacks; (3) its properties with arguments: a cost bound against the better single method under a stated cost model, determinism, no off-trajectory calls if possible; (4) the hyperparameters that remain, each with the reason it cannot be removed and its kind; (5) the behaviour on these scenarios: stiff from t0; stiffness that develops later (Robertson); non-stiff; oscillatory non-stiff (eigenvalues near the imaginary axis); non-normal transient growth; a fatal callback error off the trajectory; budget exhaustion; a supplied Jacobian on a non-stiff problem; large n with a cheap right-hand side; small n with an expensive right-hand side; (6) the code changes (files, what the drivers must expose, for example a per-step stiffness estimate of DOPRI5 - look for what the switcher lineage already provides, such as crates/rodas5p-integrators/src/dopri5_stiffness.rs or the switching solver, and the counters the drivers return); (7) an evaluation plan that can be pre-registered against today's route (problems including those of ${EV}/unified_router/PREDECLARED.json and new ones, measures such as charged right-hand sides, Jacobians, factorizations, attempts, achieved error, discriminators that are comparisons where possible). Return the structured result; keep "summary" under 1500 characters.`

const A = `${PRE}
TASK: designer A. Angle: evidence on the trajectory and competitive switching. Use only quantities the integration produces anyway (for example the stiffness quotient of DOPRI5 from its own stages, the step sizes and error estimates of both methods, the charged work counters) and decide when to leave DOPRI5 with a rule that has a provable bound, such as a ski-rental (break-even) rule, against the better of the two single methods. Remove the off-trajectory screen if you can. Say exactly which cost units you compare (callback units, operation counts of the linear algebra, or both) and how you convert between them without a tuned number, or why you cannot.
${COMMON_DELIVERABLE.replace('<your letter>', 'A')}`

const B = `${PRE}
TASK: designer B. Angle: an economic comparison of measured costs. Decide by the expected cost per unit of integrated time of each method, from quantities measured during the solve (for example one RODAS5P step taken on the trajectory as a probe, the step size each method accepts, the counted right-hand sides, Jacobians, factorizations and solves) and from structural facts the registry knows (dimension, Jacobian form, band, sparsity). Compare with LSODA (Petzold 1983) and other automatic stiffness switching schemes and list their hyperparameters. Address the conversion between callback costs and linear-algebra costs and the determinism of the decision explicitly; if a measurement of time is unavoidable, say how the decision stays reproducible.
${COMMON_DELIVERABLE.replace('<your letter>', 'B')}`

const judgePrompt = (a, b) => `${PRE}
TASK: judge of node V27. Two designers wrote independent designs of the Auto route. You did not write either; your job is to find what is wrong with them, to verify their claims against the code and the literature, and then to write ONE recommended design.
DESIGN A (structured result; the document is at its doc_path):
<<<
${JSON.stringify(a).slice(0, 12000)}
>>>
DESIGN B (structured result; the document is at its doc_path):
<<<
${JSON.stringify(b).slice(0, 12000)}
>>>
CHECK: (1) every claim about the code (does the driver expose what the design needs; are the counters there; is the census complete - search the route and what it calls for every constant); (2) every claim from the literature (cite); (3) hidden hyperparameters (a number inside a rule that looks parameter-free, such as a count of consecutive steps, a safety factor, a window, a probe interval); (4) the cost bound under its stated model and whether the model is honest about linear-algebra costs; (5) determinism; (6) the scenarios, in particular Robertson, oscillatory non-stiff problems and fatal callbacks.
THEN write ${OUT}/DESIGN_RECOMMENDED.md: at the top a short summary in Korean for the owner (what changes, which hyperparameters disappear, which remain and why, what has to be decided), then the technical design in English: the census table (today, kind, proposed), the algorithm in pseudocode, the properties with their arguments, the remaining hyperparameters, the code changes and an implementation plan in steps, and an evaluation plan ready for pre-registration (problems, arms = today's route and the new route, measures, discriminators as comparisons where possible, what would refute the design). Return the structured result; "korean_summary" is the Korean summary for the owner (under 2500 characters); keep "summary" under 1500 characters.`

const designs = await parallel([
  () => agent(A, { label: 'design:A', phase: 'Design', schema: DESIGN_SCHEMA }),
  () => agent(B, { label: 'design:B', phase: 'Design', schema: DESIGN_SCHEMA }),
])
const [a, b] = designs
if (!a && !b) { log('both designers returned nothing'); return { a, b, judge: null } }
const judge = await agent(judgePrompt(a, b), { label: 'judge:v27', phase: 'Judge', schema: JUDGE_SCHEMA })
return { a, b, judge }
