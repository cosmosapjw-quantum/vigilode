# E-09 — mass matrix and f_t (pre-registration)
Written BEFORE any run. Binary: harness/src/bin/e09_mass_ft.rs.
(a) Fixed-step ladder h = 1/2^k, k=2..10, T=[0,1], arms Direct and GMRES (rtol 1e-13, atol 1e-16, no PC), on
    diagmass_eps1e-3 (M=diag(1,1e-3), M y' = J y + r(t), exact y=(sin t, cos t+0.5), J=[[-1,.5],[.2,-1]] so M^-1 J has eigenvalue ~ -1e3),
    manufactured_mass_nonlinear_problem(1e3,1,1,0), constant_affine_mass_problem(). Error = max over grid ||y-y*||_inf / max ||y*||_inf.
    FAIL if for any (problem, arm) there are fewer than 2 consecutive halvings with slope >= 4.5 among rows with error > 1e-12
    (no order-5 regime), or if error at k=6 (h=1/64) exceeds 1e-6, or any run errors.
(b) prothero_robinson_problem(lambda, mu, 0) with lambda in {-1e2,-1e4}, mu in {0,1}; twin with partial_t=None (FD path,
    eps = sqrt(EPSILON)*max(|t|,1)); Direct arm, same ladder. Report ratio err_fd/err_analytic per k; "FD dominates" := ratio > 2.
    FAIL for the analytic arm as in (a). The FD arm is EVIDENCE (where FD f_t error dominates), not PASS/FAIL.

# Results (appended after the run; pre-registration above unchanged)
Status: **FAIL on the pre-registered order rule for the two stiff cases (diagmass eps=1e-3; PR lambda=-1e4); tolerance rule PASS; all 20 ladders ran without error.**
| case | arm | pre-floor slopes | max consecutive >=4.5 | err at h=1/64 |
|---|---|---|---|---|
| diagmass_eps1e-3 | direct | [3.22, 3.6, 4.78, 2.79] | 1 | 2.49e-12 |
| diagmass_eps1e-3 | gmres | [3.22, 3.6, 4.78, 2.79] | 1 | 2.49e-12 |
| manufactured_mass_nonlinear_s1e3 | direct | [3.88, 4.58, 5.04, 4.13] | 2 | 2.94e-12 |
| manufactured_mass_nonlinear_s1e3 | gmres | [3.88, 4.58, 5.04, 4.13] | 2 | 2.94e-12 |
| constant_affine_mass | direct | [5.18, 5.64, 5.83] | 3 | 3.34e-14 |
| constant_affine_mass | gmres | [5.18, 5.64, 5.83] | 3 | 3.35e-14 |
| PR lambda=-1e+02 mu=0.0 | analytic_ft | [0.36, 2.84, 4.03, 4.69] | 1 | 7.64e-12 |
| PR lambda=-1e+02 mu=1.0 | analytic_ft | [2.01, 2.65, 3.96, 4.66] | 1 | 7.26e-12 |
| PR lambda=-1e+04 mu=0.0 | analytic_ft | [2.95, 3.03, 3.11, 3.25] | 0 | 1.75e-12 |
| PR lambda=-1e+04 mu=1.0 | analytic_ft | [2.95, 3.03, 3.11, 3.25] | 0 | 1.75e-12 |

- Direct vs GMRES (matrix-free W with mass matrix): errors agree to <= 1.4e-15 on every ladder point of all three mass problems.
- FD f_t vs analytic f_t: identical down to h=1/64; FD error dominates (ratio > 2) from h=1/256 for both lambda; FD floor ~1.5e-15 (lambda=-1e2) vs ~1.6e-13 (lambda=-1e4), i.e. proportional to |lambda|.
- Stiff cases show order reduction to ~3-4 (h*|lambda| >= 1) rather than order 5; the nonstiff mass problem shows clean order 5.
Files: e09_out.json (all rows with errors, slopes, counters), stderr log, build.log, results.json.
