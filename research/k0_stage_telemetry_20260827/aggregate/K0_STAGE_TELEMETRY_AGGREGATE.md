# VigilODE K0 stage-telemetry aggregate

- Verdict: `COMPLETE`
- Claim class: `EXPLORATORY_NONAUTHORITATIVE`
- Execution head: `321c63ee8ca0f216001bf41b30d58c1858a4781a`
- Frozen contract: LegacyFixed (`rtol=1e-10`, `atol=1e-12`), two kernels, six families.
- Claim boundary: raw receipt, geometry, residual, and work observations only; no performance or activation inference.

| Family | Kernel | State | Stages | Linear | Diagnostic | Checked total | Telemetry JVP | A1 accepted | A1 rejected |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| hires-ramped | incremental-givens-candidate | COMPLETE | 368 | 1964 | 736 | 2700 | 322 | 22 | 1 |
| hires-ramped | legacy-restarted-gmres | COMPLETE | 368 | 3266 | 368 | 3634 | 322 | 22 | 1 |
| nonautonomous-stiff-forcing | incremental-givens-candidate | COMPLETE | 2960 | 9914 | 5920 | 15834 | 2590 | 177 | 8 |
| nonautonomous-stiff-forcing | legacy-restarted-gmres | COMPLETE | 2960 | 37950 | 2960 | 40910 | 2590 | 177 | 8 |
| robertson-ramped | incremental-givens-candidate | COMPLETE | 400 | 2296 | 800 | 3096 | 350 | 23 | 2 |
| robertson-ramped | legacy-restarted-gmres | COMPLETE | 400 | 4990 | 400 | 5390 | 350 | 23 | 2 |
| rotating-nonnormal | incremental-givens-candidate | COMPLETE | 1616 | 11452 | 3232 | 14684 | 1414 | 100 | 1 |
| rotating-nonnormal | legacy-restarted-gmres | COMPLETE | 1616 | 25270 | 1616 | 26886 | 1414 | 100 | 1 |
| semilinear-advection-diffusion-ramped | incremental-givens-candidate | COMPLETE | 512 | 55712 | 1024 | 56736 | 448 | 29 | 3 |
| semilinear-advection-diffusion-ramped | legacy-restarted-gmres | COMPLETE | 512 | 63940 | 512 | 64452 | 448 | 29 | 3 |
| van-der-pol-ramped | incremental-givens-candidate | COMPLETE | 1104 | 6642 | 2208 | 8850 | 966 | 65 | 4 |
| van-der-pol-ramped | legacy-restarted-gmres | COMPLETE | 1104 | 17526 | 1104 | 18630 | 966 | 65 | 4 |
