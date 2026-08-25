# A1 Inner-Tolerance Parity Handoff Authority

This branch is a read-only execution and audit handoff for VigilODE A1. It must never be merged into `main` or into the implementation branch.

## Canonical identities

- Repository: `cosmosapjw-quantum/vigilode`
- Canonical base: `main@4e3a75e5b2843dc1e135dcadba72edb1d09be94c`
- Canonical base tree: `c6d4e20b54f84e6894b1954fc61681b881350b85`
- Implementation branch: `research/a1-inner-tolerance-parity`
- Intake implementation head: `67ec3ad77d0a88f3ff9c096b309d3a12da72b600`
- Intake implementation tree: `4d5070a35cbc546efc1dd350feeb4a45e08c7e01`
- Existing draft PR: `#18`

The implementation branch already contains an A1 candidate. Do not assume an unmodified feature branch and do not reimplement A1 from scratch. First audit the existing diff and executable evidence. A repair commit is allowed only when a concrete contract violation is demonstrated.

## Source-of-truth precedence

1. The user's explicit approval to start A1 and the prohibition on A2/A3, timing claims, merge, tag, and release.
2. The live remote identities above, re-read before work.
3. Production source and executable tests on the implementation branch.
4. GitHub Actions evidence bound to the exact implementation head.
5. This handoff package.
6. Prose in PR descriptions or prior chat summaries.

If live remote identity differs from `CURRENT_STATE.json`, stop before mutation with `BLOCKED_BY_REMOTE_DRIFT` and report the exact old/new identities. Do not silently rebase, force-push, or reinterpret the state.

## Mandatory read order

1. `CURRENT_STATE.json`
2. `AUDIT_COMPILED_EXEC_PLAN.yaml`
3. `P0_P1_THREAT_CATALOG.yaml`
4. `INVARIANT_TEST_MATRIX.yaml`
5. `IMPLEMENTER_PROMPT.md`
6. `FRESH_REVIEW_PROMPT.md`
7. `CODEX_LAUNCHER.md`

Then run:

```bash
python acceptance/test_handoff_contract.py --repo /path/to/implementation/worktree --handoff /path/to/this/handoff/worktree
python tools/discover_a1_tolerance_sites.py --repo /path/to/implementation/worktree
```

## A1 scientific contract

The pre-A1 exponential/phi path is the authority for the outer-to-inner law:

```text
relative = max(3.0e-2 * outer_rtol, 1.0e-12)
absolute = max(3.0e-4 * outer_rtol, 1.0e-14)
```

A1 factors this law into one checked immutable policy and routes both protected RODAS5P/GMRES and exponential phi-Krylov configuration through it. This is a tolerance-policy parity change only.

The exact floating-point arithmetic of the pre-A1 phi expression is preserved. Tests must compare against the same multiplication expression rather than an independently rounded decimal literal.

## Allowed actions

- Read and audit all implementation-branch source, tests, workflows, and PR metadata.
- Run the complete invariant matrix.
- Add one minimal repair commit to `research/a1-inner-tolerance-parity` when evidence demonstrates an A1 defect.
- Update PR #18's body to reflect verified evidence.
- Leave PR #18 draft and unmerged.

## Forbidden actions

- Any mutation of this handoff branch after publication except a separately justified handoff correction.
- Merging this handoff branch anywhere.
- Creating a second A1 PR.
- Implementing A2 inner GMRES convergence, A3 incremental Givens/QR, controller refactors, timing changes, benchmark retuning, or scientific fixture changes.
- Changing `Cargo.toml`, `Cargo.lock`, tracked Cargo configuration, dependencies, solver equations, residual definitions, restart sizes, iteration caps, preconditioners, or work-count accounting.
- Wall-time ranking, speedup claims, active switching, PM-4 Task 2, merge, tag, or release.
- Force push.
- Post-hoc design documents presented as pre-implementation evidence. This handoff is the execution contract; do not inflate the implementation PR with retrospective process artifacts.

## Stable terminal states

- `A1_REVIEW_READY`: all required gates pass on an exact head and no repair is needed.
- `A1_REPAIRED_REVIEW_READY`: a minimal evidence-backed repair was pushed and all gates pass on the new exact head.
- `BLOCKED_BY_REMOTE_DRIFT`: a pinned remote identity moved before mutation.
- `BLOCKED_BY_UNRESOLVED_SPEC`: production evidence supports more than one incompatible authority law.
- `BLOCKED_BY_VERIFICATION_FAILURE`: a required gate fails and no bounded A1-only repair is yet justified.

Never report partial verification as success.