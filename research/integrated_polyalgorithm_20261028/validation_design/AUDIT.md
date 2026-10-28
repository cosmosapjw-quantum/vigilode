# Cumulative independent-PR validation design

## Applied scope and actual work

`skill_context_used`: physmath-research-loop 1.3.0; the parent-supplied
`SKILL_CONTEXT.md` and `RESUME.md` were read. This worker applies R3 plus the
requested C4/C10 handoff design: read-only source/history/CI inspection, concrete
coverage and execution contracts. No solver build, scientific experiment,
benchmark, profile, CI rerun, publication or recursive delegation was performed.

## 1. What the independent PR must contain

The exact ancestry was checked with `git merge-base --is-ancestor` (exit 0 on
every adjacent pair):

| Object | Exact commit |
|---|---|
| Observed main | `8d0c79184e09efb5bdadc24a6315c60a71a44264` |
| Pre-conversation implementation | `3064d453091af1dec7f99f0c8afabd98b447ea34` |
| PR90 | `68a28357d960c4c2743c299770f0a33ef63245b2` |
| PR91 | `206298b4068d923bb47c7160b111b7439098660c` |
| PR92 | `7eb2dd0ffda39afb2871d76700a720986ff09fe0` |
| PR93 | `384dba091a3d9e1aac0ecddec264958298239253` |

The current integration research commit will descend from PR93. A separate
validation branch can start at that exact delivered commit and target `main`.
Its contents therefore include all four PRs without waiting for them to merge.
Do not cherry-pick four already nested heads or reapply their changes.

At handoff execution, re-read main and all source PR heads. Preserve the delivered
snapshot as the initial baseline. If main advanced, record the new base, compute
the merge base, then make a normal merge in the new worktree and record conflict
resolutions. Never reset a user's checkout or force-push the source branches.
Re-test the resolved cumulative tree; old head tests do not qualify a merge result.

The diff against main also includes substantial older research predating this
conversation. The validation PR must report three scopes separately:

1. **Source inclusion:** every pinned PR head is an ancestor of the validation head.
2. **Conversation coverage:** all changed crate/workflow paths in the range
   pre-conversation commit through PR93, plus current integration additions.
3. **Inherited/release qualification:** the baseline suite and seven Q-* obligations.

The per-PR changed crate/workflow path counts are 13, 20, 44, 0 for PR90--93;
the cumulative range has 73 unique paths. PR93 changes research artifacts, not the
Rust runtime. `COVERAGE.json` records the complete 73-path union and source hashes.
Counting paths is coverage bookkeeping, not scientific validation.

Ancestry proves that the commits are included in history; a later commit can still
replace or revert their behavior. The current-tree path map and semantic gates
therefore remain necessary. For superseded implementations retain the old source
as an attribution baseline and verify the intended invariant on the final code.

## 2. New remote evidence: PR93 CI

### E4 fresh-clone workflow

Observed run [40609485096](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485096),
job [122557234098](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485096/job/122557234098).
The default fresh-clone dependency graph, workspace compile and touched default
contract passed. The broader integrator Clippy command failed:

```text
cargo clippy -p rodas5p-integrators --all-targets --locked -- -D warnings
```

The current log contains the same 23 diagnostic sites described by PR92's
`post_push/SETUP_REVIEW.json`. The compiler summaries of 17 lib and 22 lib-test
errors overlap. Independent vendor construction, offline graph comparison,
offline compile/test and final formatting were skipped after this failure.
Thus default build success does not certify E4's offline half. This is the
existing **Q-LINT** gate; add the skipped E4 stages to its positive follow-through.

### Research authority workflow

Observed run [40609485121](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485121),
job [122557234399](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40609485121/job/122557234399).
Checker unit tests and research-node/append-only ledger validation passed.
The full authority scan failed with 10,133 citations / 797 distinct object strings:
53 missing, 27 missing+allowlisted, 220 non-commit, 497 reachable.

The checked-in workflow uses `fetch-depth: 0`. The concrete failures therefore
must not all be attributed to a shallow checkout. Source inspection establishes
at least three different identity classes:

- `adapters/julia/Manifest.toml` uses **git-tree-sha1** for package source trees.
  The current keyword scanner treats `sha1` text as a potential commit authority;
  an absent external tree cannot be resolved as a local VigilODE commit.
- `BASS_REI_RECENCY.json` explicitly binds its commits to **bass** or
  **rei_bianchi**, but `resolve()` always checks this repository's object database
  and remote branches. These need repository-qualified validation.
- Historical local commit IDs already have **published equal-tree mappings**,
  for example `production_frontier_20261023/PUBLISH_MAP.json`. Such a mapping
  preserves distinct commit identities while supporting source-tree reuse.
  Validate its published target and exact tree; never silently rewrite the old
  receipt to claim it ran at a different commit.

Other missing local authorities can remain genuine provenance gaps. Full
classification and repair belong to **Q-LEDGER**. The minimal constructive change
is typed `(repository, object_kind, identifier, required_relation)` resolution
with explicit mapping evidence and regressions for malformed/unmapped authority.
Do not add all 53 identifiers to an allowlist, exclude whole research directories,
or treat a missing scientific source as a harmless external object by default.
The current root's scoped 27-commit success remains scoped evidence; it is not
the full-repository authority result.

The minimal observations and exact selected log excerpts are preserved in
`CI_OBSERVATIONS.json`. The old failed jobs remain failed.

## 3. Required coverage, with strong oracles

| Gate group | Included changes | Semantic checks beyond compilation |
|---|---|---|
| PR90-LU/MODE | aligned LU scratch, into partial-time, borrowed residual rows | original result bits/counters; dimensions around alignment boundaries; warmed allocation count; independent true residual |
| PR90-ADAPTER | conservative early comparator stop and evidence enum | complete versus incomplete evidence, fatal priority, exact accepted checkpoint, all attempted work, default-off parity |
| PR91-CAUSAL | projected-input and prefix reuse | equal input+epoch only, one-component drift invalidation, cached versus uncached stage/output bits, worker failure accounting |
| PR91/92-STRUCTURAL | two channels, charged full-space enclosure | independently recomputed original residual; dimension1 and arbitrary transverse states; outward bounds; failed workers charged and joined |
| PR91/92-HOMOTOPY | accepted-stage seed, scalar/algebraic/rational roots | original outward residual, physical grid error, root/update/clock budget, monotone branch; old/new planner × fixed backend comparison |
| PR91/92-ADAPTER | integer same-window decisions, checkpoint/pilot reuse, sentinel | tie and >2^53 boundaries, overflow, wrong replay endpoint; FSAL/controller/schedule/epoch state; rejected/prefix/fatal work |
| PR92-PRODUCTION | strict JSON config and production API | RHS-only versus selective supplied-JVP permissions, unknown/missing fields, partial failure output, exact callback counts |
| PR92-LAGGED | physical work reconciliation | original failed regression unchanged; admitted callback failures; local identity A-P distinct from solver attempts |
| PR92-HYGIENE/API | format/lint changes and exported fields | operation-order preservation and downstream exhaustive literals; do not infer timing identity from numerical parity |
| PR93-THEORY | three exact-check lanes and 17 candidates | rerun each source check with new receipts; new mathematics separate from unimplemented binary64 certification |

Whole-package tests on the cumulative tree are required in addition to focused
interaction tests. Default-feature builds do not cover declared research feature
targets. The inherited main workspace excludes `vigilode-bench` and
`vigilode-hybrid`; qualifying either needs its own manifest/environment and must
be named explicitly. Bench/SUNDIALS remains Q-BENCH; hybrid is a separately
scoped predecessor dependency, not a new PR90--93 runtime change.

### Cross-PR interactions to add before promoting new routes

These are prospective tests, not currently executed passes:

1. **I-STATE:** probe winner continuation × rejection × requested outputs; compare
   uninterrupted session against resumed session at equal input/state/budget.
2. **I-EPOCH:** warm LU/polynomial/causal reuse × provider/step/norm/model change;
   unchanged identity reuses, changed identity invalidates, result stays certified.
3. **I-WORK:** budget refusal, admitted callback failure, diagnostic residual,
   early-stop comparator and all scheduled workers; actual callbacks reconcile
   independently with attempted/admitted/refused counters.
4. **I-BUDGET:** physical step budget × polynomial/root error × lifted/clock error;
   no acceptance from component budgets that omit or double-count a defect.
5. **I-POLY:** Distinct/SameVector × fused-only/column-return × degree≤128/>128;
   cancellation, zero/subnormal, rounded chart boundary, refusal and fallback.
6. **I-ROOT:** old/new planner × identical scalar/rational backend; Y=0, g=0,
   strong cubic, roundoff floor and update/clock exhaustion; preserve branch.
7. **I-STRUCTURE:** exact quotient / near structure / full RHS; perturbation
   enclosure and provider invalidation, compare with original full residual.
8. **I-NEGATIVE:** nonnormal spectral trap, two-root residual ambiguity, unstable
   variable-step Verlet product and historical W-filtered cost failure.
9. **I-PROFILE:** at least test, measurement and actual release/LTO profiles for
   identified floating/branch-sensitive fixtures; source+binary+libm identity.
10. **I-CLIENT:** full returned trajectory/observations on the actual client;
    persistent RHS-only baseline and strongest eligible structural/CG/coupled
    comparator, equal physical accuracy, charged preparation/failure work.

N01--N06 implementation does not exist just because these tests are listed. Keep
each candidate `pending_local` until its code, scientific oracle and receipts exist.

## 4. Preserve and execute the inherited qualification DAG

The source plan is
`research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json`.
Keep its 23 nodes, 34 edges, eight native execution suites and correction counts.
The new validation coverage overlays those tasks rather than silently renaming
them or manufacturing fresh retry budgets. Old structural/adapter loops are 3/3,
homotopy1/3 and separate integration-accounting1/3. The six N tasks and seven
Q tasks retain their explicit existing counters.

**Q-GIVENS:** export exact A/b/Jacobi bits, independently evaluate every returned
Ok residual before historical expected-status checks. The earlier Ok(3640) at
RHS1e14 is not by itself false convergence. Do not flip the expectation blindly.

**Q-FAIR-AB:** test A=E+R, E≤B with independent callback entry counters for budget
refusal and admitted failure. This is not the already-fixed lagged A-P ledger.

**Q-BENCH:** qualify the actual SUNDIALS version/compiler/MPI include and link
discovery. Version changes become experiment factors; build success alone does
not establish matching comparator physics or outputs.

**Q-LINT:** address the actual 23 sites, preserving arithmetic/API intent, then
complete E4's previously skipped vendor/offline stages.

**Q-API:** choose compatibility preservation or an explicit versioned migration;
check downstream exhaustive literals against PR91 and the pre-conversation API,
not only the previous research head. New enum variants from PR90 also matter.

**Q-NEGATIVE:** preserve the historical 15/7>1.3 FAIL and inaccurate Off arm.
An expected-negative scientific regression can assert that known failure is
reproduced; it cannot convert the historical performance criterion into PASS.
Execute/record the rest of ignored coverage rather than blanket skipping it.

**Q-LEDGER:** complete the typed authority classification above while keeping
append-only records, genuine unresolved cases and validation-source binding.

The positive route remains L-RELEASE → L-HARDWARE → R-QUALIFY, with the inherited
dependencies. Theory and disjoint code changes can proceed in parallel; builds,
native tests, benchmark and profiling share `/tmp/vigilode-cargo-20261027.lock`.
Freeze one bounded performance hypothesis before execution and preserve unused
holdout where the project's performance-authority policy requires it.

## 5. Executable command design and evidence handling

`COMMANDS.json` supplies exact argv or inherited runner references and classifies
every command as prospective. The parent integration harness should expand output
and target placeholders without shell interpolation. For every executed command,
record source commit/tree, source/input hashes, executable hash where present,
Rust/libm/CPU/profile identity, argv/cwd/env, timeout, exit, raw log and reached tests.
Never sum overlapping target/filtered reruns as distinct tests.

Four source snapshots plus the cumulative final snapshot provide distinct roles:
PR90/91/92 support attribution and selected reproductions; PR93 source checks
support theory; the final integrated tree is the actual qualification candidate.
Do not replay every old expensive campaign merely for reassurance. Preserve the
complete original evidence, reproduce each unique changed behavior at the final
tree, and use exact predecessor snapshots when a comparison or failure attribution
actually depends on them. A full prior-claim table must say reproduced, reused
under stated identity, superseded, failed or unexecuted.

Several inherited Python checks write beside their own script, including
`dopri_phase_exact.py`, `exact_checks.py`, `exact_constant_checks.py` and
`exact_structural_constants.py`. Run them in a fresh replay mirror retaining
repository-relative paths and required fixture bytes, or a disposable worktree;
copy new outputs into the new evidence node. Do not overwrite the frozen results.
Elapsed time/path metadata need semantic comparison, not byte equality. PR93's
CHECK.py programs emit stdout and can be captured without replacing old receipts.

Fresh target directories must be separate for baseline and candidate builds.
PR90 already observed a stale local-package artifact in a shared target. Paired
measurements must bind the exact executables and include cold/warm preparation,
certificate computation, copy, callback failure and fallback. Service times are
measured hardware properties, while admission constants come from proofs.

An independent PR provides an independently reproducible source/validation
boundary. Independent mathematics still needs a different derivation or oracle
(e.g. full dense/original equation, exact rationals/high precision), not merely
running the same production routine from another branch.

## 6. Present status and constructive exit

This plan closes ambiguity about source inclusion, known CI causes and what a
complete cumulative validation must cover. It does not close any native Q/N gate.
The next concrete action is to materialize the delivered source in a fresh
validation worktree, run its dry-run/provenance checks, then execute/fix the
inherited qualification gates and selected implementation interactions in the
recorded budget. A failed gate keeps its exact diagnostic and moves unaffected
work forward; it never becomes a generic reason to repeat the entire research loop.
