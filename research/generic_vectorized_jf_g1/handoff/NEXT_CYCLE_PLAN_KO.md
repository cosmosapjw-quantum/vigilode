# G2 Next Cycle — Parallel Fifth-Order Exponential Rosenbrock / EPIRK

1. Create `generic-parallel-exponential-v1.2` from the verified G1 closeout commit.
2. Freeze BDF2/Radau IIA3 hashes and physical-client exclusion tests.
3. Add a small-dense high-precision oracle for `exp(hA)v` and `phi_k(hA)v`.
4. Implement counted matrix-free adaptive Krylov phi actions with residual/error reporting.
5. Validate EXPRB2 and EXPRB43 order, stiff decay, oscillatory phase, nonnormal and mass-matrix behavior.
6. Implement one published parallel order-five exponential Rosenbrock scheme from exact coefficients.
7. Separate arithmetic work, critical path, and actual wall time; no call-count substitution.
8. Run generic calibration, freeze all thresholds, then run sealed generic holdout.
9. Compare at matched external global error against protected JF RODAS5P, held q1→q2, frozen BDF2 and frozen Radau IIA3.
10. Promote only if order ≥4.8, false convergence 0, step ratio ≤1.2 and median same-error speedup ≥1.15.
