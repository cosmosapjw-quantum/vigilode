export const meta = {
  name: 'ks-n1-independent-review',
  description: 'Two independent read-and-run reviews of the KS-N1 owner-host cells (statistics/protocol lens and probe-equivalence lens)',
  phases: [{ title: 'Review', detail: 'two lenses in parallel, no repository edits' }],
}

const WT = '/home/cosmosapjw/worktrees/vigilode-stage-lattice-20261124'
const SCR = '/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad'
const COMMON = `You are an independent reviewer of one research result of VigilODE (Rust ODE integrators). You did not produce it.
Repository worktree (read only for you): ${WT}, branch research/stage-lattice-referee-20261124, local head af12afd8 (evidence commit; the protocol commit 3dff0183 is on origin since 2026-11-25 07:05:53 KST).
RULES: do not edit, create or delete any file inside the worktree or any other repository checkout; do not run git commands that change state (no commit, checkout, reset, stash, push); write scratch files only under ${SCR}/ks_n1_review_<your lens>/. The CLAUDE.md injected into your context describes a different task (switcher v2 gate) and does not apply to you. A build of the facade test suite may be running on the host; do not start cargo builds (no cargo build/test/clippy); you may run the already built binaries named below. Do not trust the producer's summary: recompute from raw files.
TASK UNDER REVIEW: PLAN.json task KS-N1 (research/kernel_spectral_foundation_20261124/PLAN.json): "Does the SL-N12 acceptance hold on the owner host: instructions <= 0.60 and cycles <= 0.80 of the Indexed route at n >= 1e4?" Kill: cycles ratio above 0.95 at both sizes. Frozen protocol: research/kernel_spectral_foundation_20261124/owner_host/KS_N1_PROTOCOL.json. Probe: owner_host/ks_n1_probe (one kernel per process). Series script owner_host/run_ks_n1.sh, parser owner_host/parse_ks_n1.py (corrected after the series for a reported-only field: libtest's prefix in front of the first diagnostic JSON line). Raw logs: owner_host/logs/ks_n1/. Results: results/KS_N1_OWNER_HOST.json and .jsonl. Producer's claimed verdict: PASS (instructions ratio 0.466 at n=1e4 and 0.469 at n=1e5; cycles ratio 0.655 and 0.670; cycles spread 0.016 and 0.088; every process started at load1 <= 3.0).
Built binaries you may run: probe /home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/ks-n1-probe/release/ks-n1-probe (usage: <indexed|fixed2> <rows> <solves>); test binary /home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/ws/release/deps/banded_kernel_entry_identity-519bc46678f9d2b9 (libtest; run the non-ignored tests with no arguments; do not run timing tests). perf is installed (perf stat -e instructions:u ...). Use taskset -c 5 for anything you run.
Return a structured verdict: confirm, confirm_with_corrections, or refute, with findings (each with evidence) and the commands you ran with exit codes.`

const LENSES = [
  { key: 'stats', prompt: `${COMMON}
YOUR LENS: statistics and protocol adherence.
1. Write your own script (not parse_ks_n1.py) that reads every <tag>.perf.csv, <tag>.meta.json and <tag>.out.json of logs/ks_n1 and recomputes per round, cell and arm the per-solve counts (C(1+N)-C(1))/N for instructions:u and cycles:u with N from the protocol, the ratio of medians fixed2/indexed per cell and metric, the per-round ratios and the spread max/min-1 of the cycles ratios, the quiet rule (load1_before <= 3.0 for all 24 series processes) and the identity of the probe lines within each cell (all fields but arm, solves and driver). Compare every number with results/KS_N1_OWNER_HOST.json; apply the frozen verdict rules of the protocol yourself.
2. Protocol adherence: (a) the protocol file at commit 3dff0183 (git show 3dff0183:<path>) equals the current one; (b) the hashes it states (PLAN.json, the test file, probe main.rs, Cargo.toml, Cargo.lock, rodas5p_fast.rs) equal the sha256 of those files at 3dff0183 and at af12afd8; (c) the first measured process started after the protocol commit time (git log -1 --format=%ci 3dff0183) and the push (07:05:53 KST); (d) the process order follows the protocol order (timestamps in meta files); (e) env.json records toolchain 1.94.1, core 3, governor performance; (f) the post-series parser change (git diff 3dff0183 af12afd8 -- research/kernel_spectral_foundation_20261124/owner_host/parse_ks_n1.py) touches only the reported-only diagnostic field and cannot change the verdict.
3. Look for anything that would make the verdict wrong: subtraction artifacts (e.g. C(1) differing between arms), negative or implausible per-solve values, a round with missing processes, units, events not counted, the diagnostic runs' pass status (test result line and exit 0).` },
  { key: 'equivalence', prompt: `${COMMON}
YOUR LENS: does the measured probe really measure the SL-N12 question, and do its identity claims hold.
1. Compare the probe source owner_host/ks_n1_probe/src/main.rs line by line with brusselator_strip, case(), adaptive() and the run closure of measure_fixed_width_against_indexed_whole_solves in crates/rodas5p-integrators/tests/banded_kernel_entry_identity.rs: same right-hand side, band fill, initial state, span, tolerances, output schedule, entry point and kernels. Report every difference.
2. Check that the facade's (2,2) banded route after SL-N12 (commit 3f9762a1; look in crates/vigilode/src for the banded kernel rule and in crates/rodas5p-integrators/src/rodas5p_fast.rs for FixedWidth2 and the entry point integrate_rodas5p_fast_banded_observed_with_kernel) runs the same driver arithmetic the probe calls with BandedKernel::FixedWidth2, and that BandedKernel::Indexed is the route before SL-N12. State whether the facade route adds work around the driver that the probe omits and whether that would matter for the ratio.
3. Re-run, pinned with taskset -c 5: perf stat -x, -e instructions:u for the probe at rows 5000 with solves 1 and 101 for both arms (4 processes), and at rows 50000 with solves 1 and 13 for both arms (4 processes). instructions:u of a deterministic program should repeat closely: compare with the logged counts of the same tags (any round) and recompute the instruction ratios. Check that every probe line you get has the same attempts, accepted, rejected, work, counters and state_fnv1a in both arms. Do not measure cycles.
4. Run the non-ignored tests of the test binary (no arguments) and report the result line.
5. Read crates/rodas5p-integrators/src/banded_exact_tests.rs::fixed_offsets_match_slices_under_pivoting_and_singular_columns briefly and say whether bit identity under pivoting is covered by a test that runs in normal test runs (not ignored).` },
]

const SCHEMA = {
  type: 'object',
  properties: {
    lens: { type: 'string' },
    verdict: { type: 'string', enum: ['confirm', 'confirm_with_corrections', 'refute'] },
    summary: { type: 'string' },
    recomputed: { type: 'object' },
    findings: { type: 'array', items: { type: 'object', properties: {
      id: { type: 'string' }, severity: { type: 'string', enum: ['blocking', 'major', 'minor', 'note'] },
      claim: { type: 'string' }, evidence: { type: 'string' }, correction: { type: 'string' } },
      required: ['id', 'severity', 'claim', 'evidence'] } },
    ran: { type: 'array', items: { type: 'object', properties: {
      command: { type: 'string' }, exit_code: { type: 'number' }, result: { type: 'string' } },
      required: ['command', 'result'] } },
  },
  required: ['lens', 'verdict', 'summary', 'findings', 'ran'],
}

phase('Review')
const results = await parallel(LENSES.map(l => () =>
  agent(l.prompt, { label: `review:${l.key}`, phase: 'Review', schema: SCHEMA })))
return { reviews: results }
