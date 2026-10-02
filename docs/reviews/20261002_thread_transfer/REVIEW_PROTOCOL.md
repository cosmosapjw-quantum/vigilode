# Thread-transfer review protocol

Date: 2026-10-02
Repository: cosmosapjw-quantum/vigilode
Reviewed source commit: d1e9ba3b0125ee478c28d0b2c280ca0869289c15
Reviewed source tree: 25e00400a0b8a76ee41171e717094344069a1b28
Existing publication branch: claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark

## Scope and authority

This is a user-requested code/research review and isolated mathematical reproduction in a documentation folder. It is not a newly admitted timing campaign, a production solver change, or a replacement of any existing research-node verdict. No protected R-JF path, coefficient snapshot, holdout, clock contract, timing authority, existing ledger row, or existing evidence is changed. Subsequent native performance studies must follow docs/RESEARCH_LEDGER.md and append their own preregistered ledger entries. The latest L-0033 verdict remains FAIL and timing authority remains HOLD.

Read-only exploration before this protocol included README, repository refs/PRs, the research ledger, R4 closure, fast-v2 preregistration/evaluation, stage-target semantics, homotopy and outward-certificate source, and the attached thread reports. No new numerical probe described below has run before this protocol is committed. The prior thread's results are prior information, not fresh tests of this repository.

## Questions and fixed probes

1. Radius closure: for the current strict-lower eight-stage quadratic target, form H(D)=H0+D H1 and E(D)=sum(H(D)^k a, k=0..7). Determine the geometry of the inequalities sum_j |alpha_ij| E_j(D) <= D. Compare the existing six-point factor-four schedule D=0.001*4^k with independently verified candidate radii on the five diagonal cases of r4_studies::homotopy_cost_study (dimensions 1,2,4,8,16). Preserve exact native coefficient bits and interval endpoints. A schedule failure is not proof of infeasibility. A Python real-arithmetic replay is not a Rust directed-rounding certificate.
2. Certificate action: compare the exact finite path-sum matrix construction with action-first doubling, e <- e + H^(2^level)e. Count scalar arithmetic and storage separately from critical-path depth. No wall-clock speedup or binary64 bit identity is assumed. Require exact equality on rational inputs, upper-bound validity against an independently solved triangular target, and explicit failure on an invalid structural premise.
3. Transfer safety: reproduce selected RVJ counterexample/estimator/Darboux identities and derive explicit physical-coordinate error transport. These are regression fixtures and optional chart contracts, not grounds to replace the existing RODAS5P method. Distinguish state error, distance to a stage target, embedded proxy, and finite-root certification.
4. Performance triage: recompute diagnostic Amdahl and batch-budget bounds using the committed fast-v2 evaluation and R4 cost model. Do not tune parameters against holdouts or rerun a timing campaign.

## Acceptance and rejection

All exact identities and bound assertions must be checked by executable code. Preserve failed assertions and classify implementation/environment failures separately from scientific counterexamples. New code uses a recorded test-first red/green cycle where an implementation is added. Report the actual number of tests and their scope; do not use a single PASS to promote the solver.

A proposed radius is admissible only after all required inequalities are independently re-evaluated. Grid sampling alone never proves global infeasibility. Action-first real-arithmetic equality does not establish bitwise equality with the production evaluation order. Nonfinite/negative data and a non-strict-lower coupling are rejected.

## Runtime and publication

The available local runtime has Python and C tooling but no cargo/rustc in PATH. GitHub source reads and writes work through the connector; direct network git access is unavailable. Rust workspace build, native contract tests, and native performance validation are therefore not claimed. Python proof/probe code, exact source selections, results, detailed Korean review and machine-readable development DAG will be published only under this folder on the existing branch. Before final publication, re-read the branch head and use a non-force fast-forward update. Any concurrent changes must be preserved.
