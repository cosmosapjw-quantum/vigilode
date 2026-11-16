#!/usr/bin/env bash
# Loop L1 node V02 (trial merge): gate T2, three test suites run one after another
# (never in parallel). Raw logs go to trial_merge/logs/. Writes logs/t2_done.marker last.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
G="$HERE/run_gate.sh"

# A: facade, core, krylov (loop baseline: 789 passed / 2 failed / 21 ignored)
bash "$G" t2_a_facade_core_krylov.log \
  cargo +1.94.1 test --locked -j 6 --release --no-fail-fast \
  -p vigilode -p rodas5p-core -p rodas5p-krylov

# B: integrators lib and the peer targets (loop baseline: 218 passed / 0 failed / 3 ignored)
bash "$G" t2_b_integrators_peer.log \
  cargo +1.94.1 test --locked -j 6 --release --no-fail-fast -p rodas5p-integrators \
  --lib --test ppsw1_6b_contracts --test ppsw1_independent_qualification \
  --test ppsw1_parallel_rhs_contracts --test ppsw1_session_boundary_contracts \
  --test ppsw1_linear_backend_contracts --test ptsw2b_contracts

# C: integrators and krylov test targets added by the switcher line
#    (git diff --name-status 2dc9a2c5 HEAD, status A, under crates/*/tests/)
bash "$G" t2_c_switcher_added_integrators.log \
  cargo +1.94.1 test --locked -j 6 --release --no-fail-fast -p rodas5p-integrators \
  --test causal_depth_stage_residual_contracts --test dopri5_stiffness_guards \
  --test real_collective_client_contracts --test shared_basis_mf_contracts
bash "$G" t2_c_switcher_added_krylov.log \
  cargo +1.94.1 test --locked -j 6 --release --no-fail-fast -p rodas5p-krylov \
  --test givens_ls_contracts --test secant_preconditioner_contracts \
  --test shared_basis_contracts

# D (extra, not part of the task's T2 list): integrators test targets the switcher
#    line modified.
bash "$G" t2_d_switcher_modified_integrators.log \
  cargo +1.94.1 test --locked -j 6 --release --no-fail-fast -p rodas5p-integrators \
  --test conditioned_polynomial_contracts --test frontier_polynomial_provider \
  --test int03_banded_fast_contracts --test pexprb_level2_continuation_accounting \
  --test scientific_corpus_v2_contracts

date -Is >"$HERE/logs/t2_done.marker"
