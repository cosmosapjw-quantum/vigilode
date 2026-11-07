# VigilODE production-solver qualification v2

**Verdict: HOLD_NOT_RELEASE_QUALIFIED.** This is a bounded C9/C10 audit with supporting C2/C7/C8 checks, not completion of the full C1-C10 loop.

## Source lock
Repository: cosmosapjw-quantum/vigilode  
Branch: research/switcher-v2-20261103  
HEAD: 1403e1082c3a8d0d52d33a85924791238460b9d6  
Tree: 8a236871948798cb2a40c8a5ea65112a045bf16d  
Local and origin tracking heads agreed at intake. Rust: 1.99.0, Cargo: 1.99.0. The prior v1 package is pinned to parent e70b4a4012cc59f39d2ba2756cb33bba6ca2c913 and is historical, not current source authority.

## Claim-level verdict
1. Ordinary adaptive floating-point IVP use: PARTIAL_SCOPED_RESEARCH_IMPLEMENTATION, limited to the documented smooth forward identity-mass / endpoint-observation API and provider restrictions.
2. Validated IVP solver / rigorous global trajectory enclosure: NOT_QUALIFIED. Embedded estimates, sampled observers, finite grids and reference discrepancies do not enclose the exact continuous trajectory.
3. Matched-achieved-error performance: NOT_ESTABLISHED. All 900 matched rows in the current Rust benchmark are DIAGNOSTIC_ONLY with reason reference-disputed.
4. Live controller: SwitchingSolver uses DriveState. HybridRouteAutomaton is exported but was not found in the live dispatch call path.
5. API: Ok(solution) does not imply success=true. Inspect success, final_time, message and stats.accounting_complete.

## Bounded evidence
- switching_v2_remediation_contracts: 17/17 PASS.
- rhs_homotopy_contracts: 17/17 PASS.
- cargo fmt --all -- --check: PASS.
- git diff --check: PASS.
- Rust adversarial benchmark: 126 rows, 123 successful, 3 failed; 21 failed repetitions; 0 protocol violations.
- Julia comparator: 45 rows, 44 successful, 1 unstable, one missing common-error value.
- Both timing-authority fields are diagnostic_only. Julia records the Rust reference file SHA-256.
- Current SHA-256/index-sensitive epoch tests pass, but the authority record cited by a test comment was not found in current docs/research.

Validate with python3 validate_package.py and sha256sum -c SHA256SUMS. Only this new directory is added. No code, official DAG, ledger, threshold, previous untracked path or Git history was modified. No commit/push/merge/release was performed.
