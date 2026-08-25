# IMPLEMENTER / RECOVERY PROMPT — VigilODE A1

You are resuming VigilODE A1 after the originally required durable handoff ref was missing. The ref now exists. Work autonomously in the local repository and do not ask the user to execute commands for you.

## Refs

```text
canonical base
main@4e3a75e5b2843dc1e135dcadba72edb1d09be94c

a1 implementation
research/a1-inner-tolerance-parity@67ec3ad77d0a88f3ff9c096b309d3a12da72b600

read-only handoff
handoff/a1-inner-tolerance-parity-20260825

existing draft PR
#18
```

The implementation branch already contains an A1 candidate. **Do not start from scratch. Audit first.** Do not create a second PR.

## 1. Intake before mutation

1. Fetch `main`, `research/a1-inner-tolerance-parity`, `handoff/a1-inner-tolerance-parity-20260825`, and PR #18 metadata.
2. Confirm the live identities and ancestry against `CURRENT_STATE.json`.
3. Check out the implementation branch in a clean worktree.
4. Check out the handoff branch in a separate detached worktree. Never merge it.
5. Read the handoff `AGENTS.md` and the complete mandatory read order.
6. Run:

```bash
python <handoff>/acceptance/test_handoff_contract.py \
  --repo <implementation> \
  --handoff <handoff>

python <handoff>/tools/discover_a1_tolerance_sites.py \
  --repo <implementation>
```

If any pinned remote identity moved, stop before mutation with `BLOCKED_BY_REMOTE_DRIFT`.

## 2. Reconstruct authority from source

Compare the canonical base and the current feature diff. Establish independently that the pre-A1 production phi law was:

```text
relative = max(3.0e-2 * outer_rtol, 1.0e-12)
absolute = max(3.0e-4 * outer_rtol, 1.0e-14)
```

Do not infer authority from prose or literal frequency alone. Use production call paths and executable tests.

Build an evidence table with:

- exact file and line;
- pre-A1 expression;
- post-A1 route;
- nominal/retry/fallback/matrix-free/alternate/test-only/dead-code classification;
- outer-error input available;
- proof that the linear and phi paths consume the same stored values.

If production evidence supports more than one incompatible authority law, stop with `BLOCKED_BY_UNRESOLVED_SPEC`. List every competing formula and call path. Do not choose one yourself.

## 3. Audit the existing candidate

Inspect exactly these expected changed paths:

```text
.github/workflows/a1-inner-tolerance-parity.yml
crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs
crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs
crates/rodas5p-integrators/src/lib.rs
crates/rodas5p-integrators/tests/a1_inner_tolerance_parity_contracts.rs
```

Verify:

1. one checked immutable policy owns the shared values;
2. nonfinite and nonpositive outer tolerance is rejected before solver work;
3. both paths share relative and absolute floors;
4. the exact pre-A1 multiplication arithmetic is preserved;
5. every protected atlas linear lane supplies `adaptive.rtol`;
6. GMRES method/restart/maxiter and phi dimension/increment/orthogonalization/substep settings are unchanged;
7. no A2/A3, timing, dependency, fixture, equation, controller, or work-accounting change exists;
8. the current tests prove both numerical parity and production wiring rather than only testing a detached helper.

Do not add retrospective design or plan documents to PR #18. This handoff is the execution contract.

## 4. Repair rule

The preferred result is no mutation. Make a source change only if you can show a concrete A1 contract failure.

For any repair:

1. write or strengthen the failing test first;
2. run it and capture the expected failure;
3. implement the smallest A1-only correction;
4. run the focused test and the full invariant matrix;
5. commit normally on `research/a1-inner-tolerance-parity`;
6. push without force;
7. reuse PR #18 and leave it draft.

Do not weaken an existing test, change expected scientific outputs, or use a test-only source-string assertion as a substitute for executable behavior.

## 5. Mandatory verification

Execute every entry in `INVARIANT_TEST_MATRIX.yaml`, including:

```bash
cargo metadata --locked --format-version 1
cargo test -p rodas5p-integrators --test a1_inner_tolerance_parity_contracts --locked
cargo test -p rodas5p-integrators --test g4_s5b0_regime_atlas_contracts --locked
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --locked
cargo test -p rodas5p-integrators --all-targets --no-run --locked
cargo test -p rodas5p-krylov --all-targets --locked
cargo test --workspace --all-targets --no-run --locked
cargo clippy -p rodas5p-krylov -p rodas5p-integrators --all-targets --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
```

For the exact final head, verify that both the A1 workflow and the E4 fresh-clone online/offline workflow are green. Do not reuse an older green run after a repair commit.

## 6. PR #18 evidence update

If the PR body remains stale or RED-only, update the body to include:

- exact base/head/tree identities;
- the pre-A1 authority expression and original location;
- complete callsite matrix;
- current five-file diff surface;
- RED/GREEN evidence and exact test counts;
- exact-head A1 and E4 workflow evidence;
- forbidden-diff ledger;
- explicit statement that A2, A3, wall timing, ranking, active switching, PM-4 Task 2, merge, tag, and release were not performed.

Do not change the PR title, base, draft state, or merge state unless a concrete metadata defect requires correction. Do not open another PR.

## 7. Final output

When no repair was required, return:

```text
A1_REVIEW_READY
base: <sha/tree>
head: <sha/tree>
PR: #18 open/draft/unmerged
authority: <exact expression and source>
callsite coverage: PASS
A1 contract: 5/5 PASS
G4/S5B0 behavioral contract: 10/10 PASS
v3.8-D schema contract: 5/5 PASS
workspace/build/clippy/fmt: PASS
A1 exact-head workflow: PASS
E4 exact-head workflow: PASS
forbidden diff: PASS
A2/A3: NOT PERFORMED
wall-time/ranking: NOT PERFORMED
merge/tag/release: NOT PERFORMED
```

When a repair was required, return `A1_REPAIRED_REVIEW_READY` plus the original failure and repair commit.

Do not report partial verification as success.