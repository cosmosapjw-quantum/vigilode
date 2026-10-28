# Profile summary (diagnostic build: frame pointers + line tables; timing diagnostic_only)

## case5__adaptive: perf 333.1 us/solve (n=8000); callgrind Ir total 13,142,757 over n=3 (incl. process start)
perf self top 18:
  6.83%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  5.10%  [.] adapter_checkpoint_trace::models::Model::component
  4.11%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  4.00%  [.] adapter_checkpoint_trace::models::Model::apply
  3.93%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  3.51%  [.] __sincos_fma
  3.39%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  3.12%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work
  2.92%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  2.75%  [.] rodas5p_krylov::kernels::two_pass_mgs_into
  2.70%  [.] rodas5p_core::work::WorkCounters::delta
  2.51%  [.] rodas5p_core::norms::safe_l2
  2.44%  [.] vigilode::fd::Guard::vector_call
  2.01%  [.] rodas5p_core::operator::apply_counted
  1.93%  [.] __memmove_avx_unaligned_erms
  1.68%  [.] rodas5p_krylov::gmres_into::arnoldi_column
  1.62%  [.] pulp::x86::v3::V3::vectorize::imp
  1.57%  [.] faer::linalg::householder::apply_block_householder_sequence_transpose_on_the_left_in_place_with_conj

## case5__rodas-dense: perf 41.5 us/solve (n=50000); callgrind Ir total 7,900,602 over n=10 (incl. process start)
perf self top 18:
  24.38%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  10.05%  [.] adapter_checkpoint_trace::models::Model::apply
  9.61%  [.] adapter_checkpoint_trace::models::Model::component
  9.60%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  6.49%  [.] __sincos_fma
  6.07%  [.] vigilode::fd::Guard::vector_call
  3.41%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  2.86%  [.] rodas5p_integrators::problem::OdeProblem::eval_rhs_into
  2.74%  [.] rodas5p_integrators::output::land_capped
  2.10%  [.] rodas5p_integrators::rodas5p_fast::lu_in_place
  1.98%  [.] adapter_checkpoint_trace::catalog_problem::{{closure}}
  1.79%  [.] rodas5p_integrators::rodas5p_fast::integrate_fast_in
  1.76%  [.] rodas5p_integrators::output::step_to
  1.73%  [.] __memmove_avx_unaligned_erms
  1.16%  [.] rodas5p_integrators::rodas5p_fast::Workspace::factor
  1.15%  [.] vigilode::solve::build::{{closure}}
  1.11%  [.] rodas5p_integrators::adaptive::AdaptiveStepConfig::validate
  0.85%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated

## case5__rodas: perf 378.4 us/solve (n=6000); callgrind Ir total 14,986,817 over n=3 (incl. process start)
perf self top 18:
  8.23%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  5.45%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  5.37%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  4.68%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work
  4.46%  [.] adapter_checkpoint_trace::models::Model::component
  4.26%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  4.13%  [.] __sincos_fma
  3.44%  [.] adapter_checkpoint_trace::models::Model::apply
  3.36%  [.] rodas5p_core::norms::safe_l2
  3.32%  [.] rodas5p_krylov::kernels::two_pass_mgs_into
  2.83%  [.] rodas5p_core::work::WorkCounters::delta
  2.65%  [.] vigilode::fd::Guard::vector_call
  2.36%  [.] rodas5p_core::operator::apply_counted
  2.12%  [.] rodas5p_krylov::gmres_into::arnoldi_column
  1.86%  [.] pulp::x86::v3::V3::vectorize::imp
  1.82%  [.] faer::linalg::householder::apply_block_householder_sequence_transpose_on_the_left_in_place_with_conj
  1.64%  [.] <rodas5p_core::operator::ClosureOperator<F> as rodas5p_core::operator::LinearOperator>::apply_with_work
  1.59%  [.] pulp::x86::v3::V3::vectorize::imp_fastcall

## decay__adaptive: perf 14.1 us/solve (n=200000); callgrind Ir total 6,369,330 over n=30 (incl. process start)
perf self top 18:
  15.33%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  5.30%  [.] __memcmp_avx2_movbe
  4.71%  [.] __memmove_avx_unaligned_erms
  4.70%  [.] alloc::collections::btree::map::BTreeMap<K,V,A>::insert
  4.47%  [.] _int_malloc
  4.32%  [.] _int_free
  4.11%  [.] <str as serde_json::value::index::Index>::index_or_insert
  3.84%  [.] rodas5p_integrators::output::land_capped
  3.82%  [.] malloc
  3.70%  [.] adapter_checkpoint_trace::solve_arm
  3.63%  [.] vigilode::switching::drive_windows
  2.61%  [.] rodas5p_integrators::output::step_to
  2.55%  [.] cfree@GLIBC_2.2.5
  2.37%  [.] <str as serde_json::value::index::Index>::index_into
  2.36%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  2.07%  [.] alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,alloc::co
  1.91%  [.] rodas5p_core::work::WorkCounters::delta
  1.29%  [.] adapter_checkpoint_trace::model::{{closure}}

## decay__dopri: perf 4.8 us/solve (n=400000); callgrind Ir total 2,929,459 over n=30 (incl. process start)
perf self top 18:
  31.25%  [.] rodas5p_integrators::dopri5::integrate_dopri5_impl
  10.33%  [.] rodas5p_integrators::dopri5::evaluate_rhs
  9.09%  [.] rodas5p_integrators::output::land_capped
  7.41%  [.] rodas5p_integrators::output::step_to
  3.76%  [.] adapter_checkpoint_trace::model::{{closure}}
  3.48%  [.] __memmove_avx_unaligned_erms
  3.11%  [.] rodas5p_integrators::dopri5::Dopri5Workspace::accept_fsal
  2.56%  [.] __ieee754_pow_fma
  2.29%  [.] rodas5p_integrators::dopri5::error_norm
  1.90%  [.] rodas5p_integrators::output::OutputCollector::limit_step
  1.74%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated
  1.71%  [.] adapter_checkpoint_trace::rec
  1.64%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::trial
  1.57%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::accept
  1.56%  [.] rodas5p_integrators::adaptive::adaptive_next_step_after_attempt_prevalidated
  1.51%  [.] _int_free
  1.41%  [.] rodas5p_integrators::output::OutputCollector::accept_prechecked
  0.88%  [.] vigilode::explicit::Dopri5::solve_impl

## gyro-tight__adaptive: perf 3160.4 us/solve (n=600); callgrind Ir total 51,364,404 over n=1 (incl. process start)
perf self top 18:
  24.17%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  23.58%  [.] adapter_checkpoint_trace::models::Model::component
  17.36%  [.] adapter_checkpoint_trace::models::Model::apply
  3.55%  [.] rodas5p_integrators::output::land_capped
  3.42%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  3.29%  [.] rodas5p_integrators::dopri5::error_norm
  2.94%  [.] rodas5p_integrators::output::step_to
  2.00%  [.] adapter_checkpoint_trace::catalog_problem::{{closure}}
  1.68%  [.] __ieee754_pow_fma
  0.87%  [.] __memmove_avx_unaligned_erms
  0.70%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::accept
  0.68%  [.] rodas5p_integrators::adaptive::adaptive_next_step_after_attempt_prevalidated
  0.65%  [.] rodas5p_integrators::output::OutputCollector::limit_step
  0.61%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated
  0.58%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  0.56%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::trial
  0.52%  [.] adapter_checkpoint_trace::rec
  0.47%  [.] _int_malloc

## gyro-tight__dopri: perf 2574.2 us/solve (n=900); callgrind Ir total 44,890,764 over n=1 (incl. process start)
perf self top 18:
  27.45%  [.] adapter_checkpoint_trace::models::Model::component
  20.96%  [.] adapter_checkpoint_trace::models::Model::apply
  20.39%  [.] rodas5p_integrators::dopri5::integrate_dopri5_impl
  7.67%  [.] rodas5p_integrators::dopri5::evaluate_rhs
  4.30%  [.] rodas5p_integrators::output::land_capped
  4.01%  [.] rodas5p_integrators::dopri5::error_norm
  3.47%  [.] rodas5p_integrators::output::step_to
  2.32%  [.] adapter_checkpoint_trace::catalog_problem::{{closure}}
  1.50%  [.] __ieee754_pow_fma
  0.81%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated
  0.81%  [.] rodas5p_integrators::adaptive::adaptive_next_step_after_attempt_prevalidated
  0.77%  [.] adapter_checkpoint_trace::rec
  0.75%  [.] rodas5p_integrators::output::OutputCollector::limit_step
  0.73%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::trial
  0.64%  [.] __memmove_avx_unaligned_erms
  0.61%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::accept
  0.51%  [.] rodas5p_integrators::dopri5::Dopri5Workspace::accept_fsal
  0.48%  [.] rodas5p_integrators::output::OutputCollector::accept_prechecked

## gyro-tight__rodas: perf 170289.1 us/solve (n=12); callgrind Ir total 2,160,641,206 over n=1 (incl. process start)
perf self top 18:
  13.64%  [.] adapter_checkpoint_trace::models::Model::component
  11.26%  [.] __ieee754_pow_fma
  6.31%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  4.52%  [.] rodas5p_krylov::kernels::two_pass_mgs_into
  4.38%  [.] rodas5p_core::norms::safe_l2
  3.86%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  3.74%  [.] adapter_checkpoint_trace::models::Model::apply
  3.43%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work
  3.08%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  2.89%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  2.41%  [.] vigilode::fd::Guard::vector_call
  2.14%  [.] rodas5p_krylov::gmres_into::arnoldi_column
  2.10%  [.] rodas5p_core::operator::apply_counted
  1.99%  [.] pow@@GLIBC_2.29
  1.90%  [.] rodas5p_core::work::WorkCounters::delta
  1.49%  [.] pulp::x86::v3::V3::vectorize::imp
  1.40%  [.] rodas5p_krylov::gmres_into::least_squares_correction
  1.32%  [.] pulp::x86::v3::V3::vectorize::imp

## relaxation__adaptive: perf 98.6 us/solve (n=25000); callgrind Ir total 7,303,277 over n=5 (incl. process start)
perf self top 18:
  16.74%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  9.64%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  4.87%  [.] __sincos_fma
  4.79%  [.] rodas5p_integrators::problem::OdeProblem::eval_rhs_into
  4.47%  [.] vigilode::fd::Guard::vector_call
  4.02%  [.] __expm1_fma
  3.87%  [.] __memmove_avx_unaligned_erms
  3.47%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  3.07%  [.] rodas5p_integrators::output::land_capped
  2.83%  [.] adapter_checkpoint_trace::model::{{closure}}
  2.62%  [.] _int_malloc
  2.16%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  2.14%  [.] rodas5p_integrators::output::step_to
  2.02%  [.] __tanh
  1.65%  [.] _int_free
  1.29%  [.] rodas5p_integrators::adaptive::AdaptiveStepConfig::validate
  1.28%  [.] malloc
  1.28%  [.] vigilode::solve::build::{{closure}}

## rotation__adaptive: perf 394.3 us/solve (n=8000); callgrind Ir total 19,056,971 over n=3 (incl. process start)
perf self top 18:
  26.70%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  6.58%  [.] rodas5p_integrators::output::land_capped
  5.57%  [.] rodas5p_integrators::output::step_to
  4.83%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  2.86%  [.] rodas5p_integrators::dopri5::error_norm
  2.78%  [.] __memmove_avx_unaligned_erms
  2.66%  [.] adapter_checkpoint_trace::model::{{closure}}
  2.10%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  1.82%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  1.59%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  1.30%  [.] __ieee754_pow_fma
  1.27%  [.] rodas5p_core::work::WorkCounters::delta
  1.23%  [.] rodas5p_integrators::adaptive::adaptive_next_step_after_attempt_prevalidated
  1.14%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated
  1.13%  [.] adapter_checkpoint_trace::rec
  1.13%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  1.10%  [.] rodas5p_integrators::output::OutputCollector::limit_step
  1.08%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work

## rotation__dopri: perf 231.5 us/solve (n=10000); callgrind Ir total 13,162,496 over n=3 (incl. process start)
perf self top 18:
  35.82%  [.] rodas5p_integrators::dopri5::integrate_dopri5_impl
  13.74%  [.] rodas5p_integrators::dopri5::evaluate_rhs
  10.09%  [.] rodas5p_integrators::output::land_capped
  7.69%  [.] rodas5p_integrators::output::step_to
  4.44%  [.] adapter_checkpoint_trace::model::{{closure}}
  3.60%  [.] rodas5p_integrators::dopri5::error_norm
  3.07%  [.] rodas5p_integrators::dopri5::Dopri5Workspace::accept_fsal
  2.98%  [.] __ieee754_pow_fma
  2.19%  [.] __memmove_avx_unaligned_erms
  2.11%  [.] rodas5p_integrators::adaptive::adaptive_next_step_after_attempt_prevalidated
  1.93%  [.] rodas5p_integrators::output::OutputCollector::limit_step
  1.87%  [.] rodas5p_integrators::adaptive::AdaptiveControllerState::propose_factor_prevalidated
  1.86%  [.] adapter_checkpoint_trace::rec
  1.69%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::trial
  1.45%  [.] rodas5p_integrators::adaptive_output::AdaptiveOutputSession::accept
  1.32%  [.] rodas5p_integrators::dopri5_dense::Dopri5OutputSession::accept
  1.03%  [.] rodas5p_integrators::output::OutputCollector::accept_prechecked
  0.86%  [.] rodas5p_integrators::output::adaptive_end_step

## stale-return__adaptive-sentinel: perf 249.3 us/solve (n=10000); callgrind Ir total 11,121,833 over n=3 (incl. process start)
perf self top 18:
  22.36%  [.] __sincos_fma
  11.07%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  9.72%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  5.40%  [.] adapter_checkpoint_trace::model::{{closure}}
  4.36%  [.] __expm1_fma
  3.96%  [.] __tanh
  3.67%  [.] rodas5p_integrators::output::land_capped
  3.59%  [.] vigilode::fd::Guard::vector_call
  3.12%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  2.61%  [.] rodas5p_integrators::output::step_to
  2.25%  [.] __memmove_avx_unaligned_erms
  2.05%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  1.69%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  1.53%  [.] rodas5p_integrators::problem::OdeProblem::eval_rhs_into
  1.29%  [.] _int_malloc
  0.98%  [.] rodas5p_integrators::adaptive::AdaptiveStepConfig::validate
  0.90%  [.] __ieee754_pow_fma
  0.83%  [.] _int_free

## stale-return__adaptive: perf 306.3 us/solve (n=8000); callgrind Ir total 13,851,231 over n=3 (incl. process start)
perf self top 18:
  21.39%  [.] __sincos_fma
  18.33%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  6.52%  [.] vigilode::fd::Guard::vector_call
  5.67%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  5.04%  [.] adapter_checkpoint_trace::model::{{closure}}
  3.60%  [.] __tanh
  3.22%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  3.20%  [.] rodas5p_integrators::output::land_capped
  2.83%  [.] rodas5p_integrators::problem::OdeProblem::eval_rhs_into
  2.83%  [.] __expm1_fma
  2.24%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  2.17%  [.] rodas5p_integrators::output::step_to
  2.11%  [.] __memmove_avx_unaligned_erms
  1.50%  [.] vigilode::solve::build::{{closure}}
  1.41%  [.] rodas5p_integrators::adaptive::AdaptiveStepConfig::validate
  1.30%  [.] rodas5p_integrators::rodas5p_fast::integrate_fast_in
  1.08%  [.] rodas5p_integrators::rodas5p_fast::Workspace::factor
  0.85%  [.] _int_malloc

## stiff-forced__adaptive: perf 48.0 us/solve (n=50000); callgrind Ir total 7,816,379 over n=10 (incl. process start)
perf self top 18:
  9.69%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  9.60%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  5.52%  [.] __sincos_fma
  5.24%  [.] _int_malloc
  5.02%  [.] __memmove_avx_unaligned_erms
  3.52%  [.] rodas5p_integrators::output::land_capped
  3.21%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  3.13%  [.] rodas5p_integrators::output::step_to
  3.05%  [.] vigilode::fd::Guard::vector_call
  2.98%  [.] _int_free
  2.26%  [.] malloc
  2.05%  [.] adapter_checkpoint_trace::model::{{closure}}
  1.88%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  1.70%  [.] cfree@GLIBC_2.2.5
  1.69%  [.] vigilode::solve::Rodas5p::solve_observed_bounded
  1.58%  [.] __memcmp_avx2_movbe
  1.55%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  1.42%  [.] alloc::collections::btree::map::BTreeMap<K,V,A>::insert

## stiff-forced__rodas: perf 11.7 us/solve (n=150000); callgrind Ir total 2,872,646 over n=10 (incl. process start)
perf self top 18:
  23.56%  [.] rodas5p_integrators::rodas5p_fast::Workspace::attempt
  8.25%  [.] rodas5p_integrators::rodas5p_fast::lu_solve_in_place
  7.49%  [.] vigilode::fd::Guard::vector_call
  7.34%  [.] __sincos_fma
  4.79%  [.] rodas5p_integrators::rodas5p_fast::Workspace::solve
  3.78%  [.] __memmove_avx_unaligned_erms
  3.42%  [.] rodas5p_integrators::output::land_capped
  3.28%  [.] rodas5p_integrators::problem::OdeProblem::eval_rhs_into
  2.53%  [.] rodas5p_integrators::adaptive::AdaptiveStepConfig::validate
  2.46%  [.] rodas5p_integrators::output::step_to
  2.33%  [.] adapter_checkpoint_trace::model::{{closure}}
  1.82%  [.] _int_free
  1.78%  [.] rodas5p_integrators::rodas5p_fast::integrate_fast_in
  1.70%  [.] vigilode::solve::Rodas5p::solve_observed_bounded
  1.52%  [.] vigilode::solve::build::{{closure}}
  1.38%  [.] <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
  1.32%  [.] rodas5p_integrators::rodas5p_fast::lu_in_place
  1.26%  [.] rodas5p_integrators::rodas5p_fast::Workspace::factor

## transition-39__adaptive: perf 6705.0 us/solve (n=300); callgrind Ir total 82,755,525 over n=1 (incl. process start)
perf self top 18:
  10.02%  [.] rodas5p_integrators::dopri5::Dopri5Session<F>::advance_to
  5.84%  [.] __sin_fma
  5.31%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  5.07%  [.] adapter_checkpoint_trace::model::{{closure}}
  3.27%  [.] vigilode::session::<impl vigilode::explicit::Dopri5>::with_session::{{closure}}
  3.26%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  3.24%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  2.96%  [.] __cos_fma
  2.95%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  2.60%  [.] __expm1_fma
  2.40%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work
  2.14%  [.] rodas5p_core::norms::safe_l2
  2.09%  [.] rodas5p_krylov::kernels::two_pass_mgs_into
  2.03%  [.] vigilode::fd::Guard::vector_call
  2.00%  [.] rodas5p_integrators::output::land_capped
  1.82%  [.] rodas5p_core::work::WorkCounters::delta
  1.78%  [.] rodas5p_core::operator::apply_counted
  1.71%  [.] rodas5p_krylov::gmres_into::arnoldi_column

## transition-39__rodas: perf 10618.5 us/solve (n=200); callgrind Ir total 123,875,057 over n=1 (incl. process start)
perf self top 18:
  8.57%  [.] faer::linalg::qr::col_pivoting::factor::qr_in_place
  5.51%  [.] rodas5p_krylov::gmres_into::solve_gmres_into_with_accounting
  5.25%  [.] rodas5p_integrators::rodas5p_matrix_free_fast::Rodas5pMfFastWorkspace::attempt_inner
  4.46%  [.] rodas5p_krylov::small::LeastSquaresWorkspace::solve_into
  4.40%  [.] <rodas5p_core::operator::ShiftedOperator as rodas5p_core::operator::LinearOperator>::apply_with_work
  3.69%  [.] rodas5p_core::norms::safe_l2
  3.26%  [.] rodas5p_krylov::kernels::two_pass_mgs_into
  3.14%  [.] vigilode::fd::Guard::vector_call
  3.13%  [.] rodas5p_core::operator::apply_counted
  2.85%  [.] rodas5p_core::work::WorkCounters::delta
  2.64%  [.] rodas5p_krylov::gmres_into::arnoldi_column
  1.87%  [.] faer::linalg::householder::apply_block_householder_sequence_transpose_on_the_left_in_place_with_conj
  1.86%  [.] <rodas5p_core::operator::ClosureOperator<F> as rodas5p_core::operator::LinearOperator>::apply_with_work
  1.83%  [.] faer::linalg::householder::make_householder_imp
  1.83%  [.] __expm1_fma
  1.83%  [.] pulp::x86::v3::V3::vectorize::imp
  1.64%  [.] rodas5p_core::operator::apply_preconditioner
  1.62%  [.] pulp::x86::v3::V3::vectorize::imp_fastcall

