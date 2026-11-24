export const meta = {
  name: 'vigilode-ci-selection-review',
  description: 'Adversarial review of the change-based CI test selection (tools/ci/select_tests.py and the two workflows): one reviewer for the soundness of the selection rules, one for the CI mechanics',
  phases: [
    { title: 'Review', detail: 'two independent read-and-run reviewers with different lenses' },
  ],
}

const WT = '/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-ci-review'
const OUT = '/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-ci-review'

const PRE = `You review a change to the CI of the VigilODE repository (Rust workspace: crates rodas5p-core, rodas5p-krylov, rodas5p-integrators, rodas5p-fair-ab, rodas5p-cli, vigilode, vigilode-coupled). Owner instruction (verbatim): "test job은 누적해서 실행하지 말고 새로 실행해야 하는 것이나 이전 변경에 영향받는 것만 재실행하는 방향으로 수정. full suite는 최종 release 전에 실행 예정." (CI test jobs must not re-run everything cumulatively; only tests that are new or affected by earlier changes; the full suite runs before the final release.)
THE CHANGE (commit a4c891eb, in the detached worktree ${WT}): tools/ci/select_tests.py (read its docstring: rules 1-7), .github/workflows/production.yml (job test), .github/workflows/ignored-tests.yml, tools/check_ignored_tests_in_ci.py, and the description in research/production_loop_20261116/ci/TRIAGE_KO.md section 9.
RULES: read-and-run only. Do not edit, create or commit anything in any worktree. Scratch files only under ${OUT}/ (create it). Builds, if you need them: "export CARGO_TARGET_DIR=${OUT}/target" and "taskset -c 0-5,12-17 cargo +1.94.1 ... --locked -j 4"; no timing; no long builds of whole packages unless a claim needs it. The shell working directory resets after every Bash call. A "Fact-Forcing Gate" hook blocks the first Bash command and each new-file Write: state the requested facts in one short paragraph and retry. The injected CLAUDE.md section "Current work: switcher v2 remediation" does not apply to you. You did not write this change; your job is to find what is wrong with it. Give evidence (commands you ran, file:line). Keep "summary" under 1200 characters.`

const SCHEMA = { type: 'object', properties: {
  lens: { type: 'string' },
  verdict: { type: 'string', enum: ['sound', 'sound_with_fixes', 'unsound'] },
  summary: { type: 'string' },
  findings: { type: 'array', items: { type: 'object', properties: {
    id: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
    claim: { type: 'string' }, evidence: { type: 'string' }, fix: { type: 'string' } },
    required: ['id', 'severity', 'claim', 'evidence', 'fix'] } },
  checked: { type: 'array', items: { type: 'string' } },
}, required: ['lens', 'verdict', 'summary', 'findings', 'checked'] }

const SOUND = `${PRE}
LENS: soundness of the selection. Find changes that can change the outcome of a test that the selector does NOT select. Try concretely, in this repository, at least: modules included with #[path] or include!, include_str!/include_bytes! with concat!/env!, files read by build.rs (cargo:rerun-if-changed), tests that read files through constants defined in another file or through env vars, doc tests and examples (are they run, and when), test targets with harness = false or required-features, cargo features enabled only for some packages, dev-dependencies between workspace packages, files under crates/<pkg>/ that are neither src nor tests (README included in docs?), a change of a test source shared by targets of two packages, deleted and renamed files (both sides), the parser of string literals (raw strings, byte strings, char literals, nested comments, lifetimes, escaped quotes), normalise_literal and literal_matches (false negatives that matter more than false positives), the dep-info parser (escaped spaces, multiple rules), Level-1 mapping when cargo test --no-run fails. Run python3 tools/ci/select_tests.py self-test and simulate selections on real commits of the branch (select with a baseline.json you write whose base is a parent commit; use --profile release and your own CARGO_TARGET_DIR only if a test-source-only case needs a build). For every hole say what it would miss and the smallest fix.`

const MECH = `${PRE}
LENS: CI mechanics. Check: the baseline subcommand (GitHub REST endpoint and query for runs of a workflow file on a branch, status filter, exclusion of the current run, gh run download of an artifact by name, behaviour when the artifact is missing, rate and time cost when there are many runs, what happens on the very first run); permissions (actions: read at workflow level, GITHUB_TOKEN in GH_TOKEN); pull_request runs (checkout of the merge commit with fetch-depth 0, the head used for the diff, github.head_ref; does the baseline head exist in the clone); the full expression for workflow_dispatch and main; the outcome file and its 'complete' flag (can a run be complete although its tests did not run, or incomplete although they did); upload-artifact conditions and artifact names per job; the order and conditions of the steps of the ignored job (negative controls through 'has'); the run subcommand (per-package commands, --no-fail-fast, parsing of failed targets from cargo output, exit status); the reachability checker (does it still fail for a new ignored test that no step can reach, and for a scope that misses a package). Read the hosted logs of the first runs of commit a4c891eb if they exist ("gh run list --branch research/production-loop-20261116" and "gh run view <id> --log" in your scratch directory). For every defect say the failure scenario and the smallest fix.`

const results = await parallel([
  () => agent(SOUND, { label: 'review:ci-soundness', phase: 'Review', schema: SCHEMA }),
  () => agent(MECH, { label: 'review:ci-mechanics', phase: 'Review', schema: SCHEMA }),
])
return { results }
