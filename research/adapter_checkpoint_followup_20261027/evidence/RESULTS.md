Verdict: **PASS** (timing `diagnostic_only`)

- G1 identity: 52 records, differences 0 → True
- G2 transition-39: allocator events per attempt 90.55 → 2.55 (0.028x)
- G2 case5: allocator events per attempt 90.95 → 2.95 (0.032x)
- G3 stale-return work 5099 → 3335, RHS 2445 → 2036, error 1.10e-08 / 1.74e-08 → True
- G4 case5: sentinel/adaptive work 1.022 (RHS 680 → 676), error 4.44e-08, pass True
- G4 stiff-forced: sentinel/adaptive work 1.039 (RHS 394 → 408), error 7.33e-10, pass True
- G4 relaxation: sentinel/adaptive work 1.000 (RHS 722 → 722), error 4.77e-09, pass True
- G4 transition-39: sentinel/adaptive work 1.074 (RHS 30515 → 34109), error 2.19e-10, pass True
- G5 tests_vigilode: 33 suites, 270 passed, 0 failed, 0 ignored
- G5 tests_integrators: 2 suites, 14 passed, 0 failed, 1 ignored
- G5 tests_krylov: 1 suites, 5 passed, 0 failed, 0 ignored

| case | a | b | b/a time | range | class |
|---|---|---|---:|---|---|
| case5 | adaptive | adaptive-sentinel | 1.042 | 1.039–1.047 | no_clear_timing_advantage |
| case5 | adaptive | rodas-dense | 0.137 | 0.136–0.151 | observed_paired_improvement |
| case5 | adaptive | rodas | 1.187 | 1.024–1.192 | no_clear_timing_advantage |
| case5 | rodas | rodas-dense | 0.113 | 0.112–0.125 | observed_paired_improvement |
| gyro-tight | adaptive | dopri | 0.866 | 0.861–0.869 | observed_paired_improvement |
| relaxation | adaptive | adaptive-sentinel | 0.997 | 0.883–1.023 | no_clear_timing_advantage |
| rotation | adaptive | dopri | 0.626 | 0.624–0.654 | observed_paired_improvement |
| stale-return | adaptive-sentinel | dopri | 3.258 | 3.244–3.408 | observed_paired_regression |
| stale-return | adaptive-sentinel | rodas | 1.054 | 1.054–1.055 | observed_paired_regression |
| stale-return | adaptive | adaptive-sentinel | 0.790 | 0.784–0.791 | observed_paired_improvement |
| stiff-forced | adaptive | adaptive-sentinel | 1.084 | 1.081–1.176 | observed_paired_regression |
| transition-39 | adaptive | adaptive-sentinel | 1.037 | 0.928–1.038 | no_clear_timing_advantage |
| transition-39 | adaptive | rodas | 1.615 | 1.592–1.886 | observed_paired_regression |
| transition-79 | adaptive | rodas | 2.571 | 2.561–2.590 | observed_paired_regression |

| cross-binary cell | candidate/base | identical records |
|---|---:|---|
| case5 rodas | 0.714 | True |
| transition-39 rodas | 0.801 | True |
| transition-39 adaptive | 0.857 | True |
| rotation adaptive | 0.931 | True |
| gyro-tight adaptive | 0.973 | True |
