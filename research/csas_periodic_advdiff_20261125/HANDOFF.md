# CSAS periodic upwind implementation: C1-C10 bounded slice
Date: 2026-11-25 KST. Base recorded in BASE.txt. New branch research/csas-periodic-advdiff-20261125. Original checkout unchanged; no merge or push.

## Scientific contract
Uniform periodic 1D grid, signed first-order upwind advection, central diffusion, constant reaction in declared preconditioner. For d=D/dx^2, v=c_adv/dx, reaction c, symbol J(k)=c-4(d+|v|/2)sin^2(theta/2)-i*v*sin(theta), original stage W=zI-J. Fourier solves reuse the existing BoxKernel/paired-real transform, stable complex division and Verified original JVP defect correction. No RODAS5P coefficient, tolerance or default route change. A variable reaction is allowed only as a checked mismatch under the existing correction budget, not as an exact Fourier diagonalization. Coercivity screen z>c; not a general singularity theorem. Real-axis explicit stability bound deliberately unavailable (NaN) for nonzero drift. Mass diagnostic is not proof of full residual accuracy. Ordinary f64 backward error, not outward-rounded certificate.

## Actual validation
Baseline: spatial_separable 11 pass, 3 ignored. Red: test spatial_upwind upwind_stage_solves_original_stencil failed on deliberate not-implemented stub (101). Green same test passed; initial green run included unused-import warnings that were removed by expanded tests. Final regression: spatial_diffusion 7, spatial_separable 11, spatial_upwind 4 pass; 0 fail, 4 ignored. 90 stage cases including signed velocity, odd/even sizes, both FFT modes. Analytic n256 IVP and separately integrated variable-reaction test. Clippy --lib --test spatial_upwind -D warnings, fmt, diff check exit 0. Full workspace tests NOT_RUN. Same-author review only; native subagents unavailable.

## Reproduce
Use Rust 1.94.1 and locked dependencies. All commands relative to worktree.
cargo test --locked -j 4 -p vigilode --release --test spatial_upwind --test spatial_separable --test spatial_diffusion -- --nocapture
OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 RAYON_NUM_THREADS=1 taskset -c 0 cargo test --locked -j 1 -p vigilode --release --test spatial_upwind measure_upwind_work_precision -- --exact --ignored --nocapture --test-threads=1
cargo clippy --locked -j 4 -p vigilode --release --lib --test spatial_upwind --no-deps -- -D warnings
cargo fmt -p vigilode -- --check
python3 research/csas_periodic_advdiff_20261125/summarize.py

For reproduction of RESULTS.json use the preserved logs, not a new timing log. The original untracked full-scope harness source was read and hashed (BENCHMARK_SOURCE_SHA256.txt); its full contents were not imported. Test uses the actual original stencil, IC, T and common-error norm. Baseline is same-branch Rodas5p::solve, not a newly run external library or the older unified wrapper. 7 paired timed repetitions per arm, one warmup, CPU 0 pinned. Timing diagnostic only. Measured pairwise median ratio 51.42-53.21; log-log interpolation only in overlapping measured errors gives 51.43-52.99. All same-tolerance pairs have equal accepted/rejected/RHS counts, and error differences <=4.69e-16 absolute. No release promotion. This test checks deterministic measured endpoint errors, not bitwise equality of full trajectories.

## C9 risks / C10 decision
New optional public SeparableReport field periodic_upwind_rate is skipped in serialization for old routes, but source-level exhaustive construction/destructuring may require updating. Full facade/workspace regressions, ABI compatibility, new dense-output/event/mass support, full spatial corpus, perf counters and competitor work-precision not tested. Trusted mode retains historical source-declaration assumption, not a true-residual check. Verified backward error is an operational floating-point metric, not a certified forward-error bound. Existing KS-N3 negative cells remain valid. Keep new route opt-in; HOLD_NOT_RELEASE_QUALIFIED.

Next: C1-CSAS-BRUSSELATOR-BLOCK-DIRICHLET-02 (first inspect existing FixedWidth2 / sparse and Schur assets, then compare exact block-band vs preconditioned paths at matched error); alternatively extend original JVP correction into prepared Fourier preconditioned GMRES, then boundary-aware Schur. Do not infer a speed win outside the tested periodic n256 model.
