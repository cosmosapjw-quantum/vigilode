# CPU-only matched-achieved-error benchmark protocol

Freeze repository, branch, commit, tree, Rust toolchain, coefficient source/hash, RHS source/hash, initial state, norm, time span, output schedule, tolerances, startup and step-ratio policy. Preregister before timing. GPU forbidden.

Corpus: linear stiff diagonal and nonnormal systems with exact solutions; Prothero-Robinson; Robertson, HIRES, Van der Pol; source-locked cooperative cubic and equilibrium-near cohort for normal-form lift; 2D diffusion/reaction and advection-diffusion; 3D anisotropic diffusion; sphere Laplace-Beltrami harmonic modes and anisotropic reaction-transport; PP05/PP06 Fourier-Volterra invariant-leaf source only.

Arms: protected native RODAS5P; selected peer-W coefficient table after order/stability prechecks; normal-form parent/lift only inside validated chart; Fourier direct/FFT predictor only on source-specific model; external BDF/Radau/exponential solver only if a genuine validated implementation exists. Internal BDF1/2 and Radau IIA1/3 are not production comparators by name.

Accuracy and timing:
- independent high-precision or converged reference with uncertainty;
- match achieved output error, not requested tolerance; target must be bracketed by measured points for every method; no extrapolation;
- at least seven randomized ABBA/BAAB paired repetitions on one CPU host and core budget;
- charge startup, transform, lift, path construction, FFT padding/transforms, interval certificate, factorization, Jacobian/JVP, preconditioner, Krylov, rejected attempts, fallback, output and memory;
- report wall time, accepted/rejected steps, stages, RHS/JVP, operator applications, factorization, Krylov iterations, reductions, allocations and peak memory;
- peer-W includes startup and step-ratio changes;
- normal-form lift includes chart validity, transform condition, lift dimension, defect transport and original-state achieved error;
- preserve negative controls and registered failures.

Record CPU model, cores, SMT, affinity, worker count, Rust toolchain, BLAS backend, NUMA/partitioning if relevant and background load. Test workers 1/2/4/8. No GPU device or accelerator path may be invoked.

Promotion: full-IVP achieved-error paired timings must support speed claim. Production readiness also requires method order/stability/estimator, startup/restart/fallback, API, clean-clone CI and one independent adversarial review. Lower nominal step count or faster kernel alone is insufficient.
