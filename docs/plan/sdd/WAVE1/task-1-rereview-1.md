### Finding Verdicts
- F1 (avoided counter S*m -> (S-1)*m) — ADDRESSED. residual_first.rs:191,196 use (S-1)*basis.len(); economy_robustness.rs:61 expects 21; no other writer of the counter (grep).
- F2 (zero scale rejected at solver.rs:326) — ADDRESSED. validate accepts atol==0 with rtol>0 (solver.rs:474), full_check rejects only negative scale, WRMS uses scaled_error_component with nonzero-over-zero-scale -> +inf (reject). Regression tests/zero_tolerance_scale.rs (candidate on eager and residual-first; atol=0,rtol=0 and negative atol still Input).
- F3 (staged MF exception) — ADDRESSED. Doc comment at rodas5p_matrix_free_fast.rs ~2666; node-doc section in docs/plan/nodes/a1-defects.md; test staged_coupled_target_refuses_a_zero_start_weight_with_a_typed_error pins typed InvalidInput for Coupled and CoupledGuarded (atol=0) and unstaged success.

### New Breakage in the Fix Diff
None. Minor: fallback.rs:110 passes c.atol onward (not in diff; atol=0 now reaches it), not checked.

### Out-of-Scope Observations
None.

### Verdict
Fix round: All findings addressed, no new Critical/Important breakage. Test results taken from the report (not re-run).
