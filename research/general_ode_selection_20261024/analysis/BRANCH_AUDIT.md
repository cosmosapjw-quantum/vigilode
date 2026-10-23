# Branch integration and public ODE interface audit

The native followup and the separate general-development branch are already ancestors of the selected base. They should not be merged a second time. The divergent readiness branch contains research preparation files, with no production Rust changes useful to transplant into the present general-ODE scope.

The audit compares the fixed base `057fecf0698fa262d881f101625e6b0bb93791cf`; it does not follow mutable branch tips. Commands were read-only (`git merge-base`, `git rev-list`, `git diff`, `git show`, and targeted source inspection). No compiler, solver test, benchmark or NCP run was launched by this reviewer.

| Source | Fixed commit | Common ancestor | Commits unique to source | Decision |
|---|---|---|---:|---|
| Native followup | `1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef` | source itself | 0 | Reuse existing implementation and receipts |
| Separate general development | `c03ca90f19a9e64ba7751c069325a48b54b39c2e` | source itself | 0 | Already included; avoid duplicate merge |
| Readiness preparation | `9e6856703a7a9be58f8a397bebcd9f516d76c1ec` | `cff15d6da06effe494664f1ef50e20d47d3ad619` | 2 | Preserve historical reference; no source transplant |

The current base has two commits beyond the native followup. Their entire file delta consists of `S0_CONTRACT.json`, `S1_CHECKPOINT.json`, and `THEORY.md` in `research/v1_portable_norm_20261024/`. Therefore the committed production source at the base is the native followup source. The current general-development worktree additions are reviewed separately below.

The separate development branch is 250 commits behind this base in ancestry. Its history includes the nonfinite-shift panic correction (`604a91e7`) and strengthened benchmark triage rules. These are historical inputs to the present code, not missing branch changes or newly executed experiments. Ancestry alone does not prove that every old behavior remains unchanged; this audit does not make that stronger claim.

The readiness branch adds 15 files exclusively under `research/v1_readiness_20261023/`. Its report describes fixed application-specific affine-norm preparation, 12 preparation tests and five C comparisons. Those reported counts were not replayed here and do not establish native general-ODE qualification. Its useful general lesson is retained: evaluating an embedded-error norm, including an interval enclosure of that arithmetic, is distinct from proving an actual local truncation or global trajectory error bound. The fixed application lift and application receipt rules are excluded from the selected solver specification.

## Read-only review of the new general ODE surface

No new blocking correctness defect was found in the inspected `selection.rs` and `session.rs` snapshot. The machine-readable companion records SHA256 and size for the reviewed sources and contract tests. This finding is a static review result; compilation and execution evidence belong to the separately recorded parent campaign.

`GeneralOde::select` delegates no callback and creates no numerical workspace. It refuses mass matrices, events and dense-output requests, selects DOPRI5 for the declared unknown/nonstiff regimes, and requires separate derivative authorization for RODAS5P. `ProvidedOnly` rejects both an implicit finite-difference Jacobian and an absent nonautonomous time derivative. Dense callbacks still trigger the dense dimension ceiling when paired with a declared band. The JVP route requires explicit opt-in and inherits the underlying capability refusals.

`Dopri5::with_session` retains a single native session across calls. Its workspace, controller, first-same-as-last derivative, output schedule and global attempt count persist. The native kernel poisons callback output buffers; unwritten or nonfinite successful output is converted to a fatal failure with diagnostics. Rejected trials do not replace the accepted checkpoint. Terminal results prevent further callback execution. Returning early from the driver produces an inspectable partial solution with `success == false`.

A targeted refresh also inspected the native schedule lookup change from linear search to binary search. `OutputSchedule::new` requires finite, strictly increasing times; the comparison retains the prior signed-zero equality. This reduces target-membership lookup from linear to logarithmic comparisons without changing the accepted target set. It is not a measurement of total solver speed. The session documentation now makes explicit that terminal calls check epochs but do not revalidate the target.

These contracts have limits that should remain visible to external readers:

1. The selector uses a caller-declared regime. It is not an automatic stiffness detector or an adaptive explicit/implicit switcher. The existing opt-in switching adapter is a different API.
2. The dense dimension ceiling is not a total memory budget. Compact band storage is exempt, and a very wide band can still be expensive. A future memory/work ceiling should account for dimension and bandwidth with checked arithmetic.
3. Session epochs detect declared changes. They cannot identify undeclared mutation of parameters behind an interior-mutable callback. A changed model requires a new session from the last accepted state.
4. The public scoped session supports forward integration on a fixed landing schedule. Arbitrary later observations, continuous event location, mass matrices, affine physical norms and backward integration are not provided by this interface.
5. `GeneralSolution::ensure_success` retains the method-specific partial result but does not retain its `GeneralSelection` wrapper inside the error. Callers needing the policy receipt should copy it first; a later result-carrying error can improve this observability contract.

The first four items constrain correct use rather than invalidate the implementation. The fifth is an API observability improvement, not a loss of the accepted physical state. Broader production suitability still depends on the scoped numerical coverage, failure contracts and comparative measurements documented elsewhere in the new research package.

## Reuse and remaining work

Reuse the existing native implementations and their published evidence. Do not cherry-pick the application-specific readiness branch into the selected general solver. Keep the new selector/session contract tests tied to the final source hashes, and refresh this static review only if those contracts or implementations change materially. Broader speed or accuracy claims must come from the parent campaign's new receipts; this audit generated none.
