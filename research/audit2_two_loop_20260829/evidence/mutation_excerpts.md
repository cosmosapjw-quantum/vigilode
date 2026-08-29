# Actual mutation-log excerpts
Full logs are retained in the downloadable research bundle. These excerpts remove backtrace frames only; they are not a new test execution.

## budget_none_false_pass
Original SHA-256: f7fd9ce4ce63985348d681f49b5c22d27270805362c9c2128472cdb42fa9a3fc
test missing_budget_is_not_pass ... FAILED
thread 'missing_budget_is_not_pass' (8693) panicked at crates/rodas5p-fair-ab/tests/output_accuracy_assessment_contracts.rs:10:41:
assertion `left == right` failed
  left: WithinBudget
 right: BudgetNotSpecified
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
error: test failed, to rerun pass `-p rodas5p-fair-ab --test output_accuracy_assessment_contracts`

## frozen_J_for_all_stages
Original SHA-256: cc64440ce6dae86b98e1d3eb1d62c8f8b100db895ffb440d9d5d81255e3e36eb
test common_w_matches_full_target_correction_without_stage_jacobians ... FAILED
thread 'common_w_matches_full_target_correction_without_stage_jacobians' (9004) panicked at crates/rodas5p-integrators/tests/audit2_structured_correction_contracts.rs:23:9:
assertion failed: eta <= 4096.0 * f64::EPSILON
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
error: test failed, to rerun pass `-p rodas5p-integrators --test audit2_structured_correction_contracts`

## wrong_correction_sign
Original SHA-256: 49a3e2103d6b2307ce95ab5c822518e9391f4a80e1a767ba329140b781cb507d
test common_w_matches_full_target_correction_without_stage_jacobians ... FAILED
thread 'common_w_matches_full_target_correction_without_stage_jacobians' (9315) panicked at crates/rodas5p-integrators/tests/audit2_structured_correction_contracts.rs:23:9:
assertion failed: eta <= 4096.0 * f64::EPSILON
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
error: test failed, to rerun pass `-p rodas5p-integrators --test audit2_structured_correction_contracts`
