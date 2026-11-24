# Public matrix-free heat problems for a periodic box and the sphere

The public constructor returns an ordinary `vigilode::Problem`. For a time-independent linear heat operator the same stencil computes both the RHS and the exact action on an arbitrary Krylov vector. The caller selects serial or Rayon row evaluation. A small-grid caller that needs dense output can explicitly replace the JVP with the existing finite-difference provider. This opts into dense allocation and factorization. No routing threshold depends on a single timing table.

## Derivation and scope

Let W be diagonal with the cell measures. Choose one orientation for each shared physical face, give B one row with entries -1,+1 for that face, and let C contain the face conductances. The operator is L = -W^-1 B^T C B. Cancellation at shared faces gives 1^T W L = 0 and u^T W L u = -sum_e C_e (Bu)_e^2 for real u and C_e >= 0. Thus the spatial discretization conserves the weighted mass and dissipates the weighted squared norm. These are statements about the semidiscrete ODE. Roundoff and finite time-step error remain in the returned solution. The construction stores Cartesian axis rates or four numbers per latitude ring. Each independent output row reads the old input and writes to its own disjoint slice.

The d-dimensional Cartesian domain is periodic on each axis with a uniform spacing h_j and arbitrary constant diagonal diffusivities kappa_j >= 0. The symbol is lambda_h(k) = -4 sum_j kappa_j sin^2(pi k_j/n_j)/h_j^2. Units of L are inverse time.

On a radius-R sphere, theta is colatitude, phi is periodic longitude, and u is sampled in each cell centre. With spacings b = pi/n_theta and p = 2pi/n_phi, the area W_i = 2 R^2 p sin(theta_i) sin(b/2) avoids subtracting nearly equal cosines at the poles. Each internal meridional face has conductance kappa sin(theta_face) p/b. Each zonal face has conductance kappa b/(sin(theta_i) p). Polar faces have zero length and zero conductance. The stencil supports longitude-dependent fields. For u = cos(theta_i) the latitude face sum is proportional to sin^2(theta_i-b/2) - sin^2(theta_i+b/2), giving the exact discrete eigenvalue -2 kappa sin(b)/(b R^2). The nonzonal test uses an area-weighted residual against the continuum l=1 eigenvalue; it measures spatial error independently of time integration. The polar cells have shorter longitudinal spacing and can increase stiffness. Neither a spherical volume solver nor a nonlinear reaction term is included in this adapter.

## Independent checks

`crates/vigilode/tests/geometry_mol_contracts.rs` supplied prior test-only Cartesian and latitude-longitude operators. The new public operator allows an ordinary matrix-free solve, reusable coefficients, and in-place RHS/JVP application. Its continuum spectrum and pole interpretation are consistent with Durran, *Numerical Methods for Fluid Dynamics*, §6.4, and Chebfun's [heat equation on the unit sphere](https://www.chebfun.org/examples/sphere/SphereHeatConduction.html). The incidence assembly in `reference.py` is independent of the row implementation. It constructs full multimode semidiscrete endpoints by SciPy's matrix-exponential action, then evaluates BDF and Radau with the constant sparse Jacobian. All three time solvers receive the same nominal tolerance; the fixture records achieved error and work separately.

Run from the repository root:

```sh
python research/spatial_review_20261124/reference.py
cargo test --locked -p vigilode --test spatial_diffusion -- --nocapture
cargo test --locked -p vigilode --doc
cargo run --locked -p vigilode --release --example spatial_rows -- 4
```

`row_kernel_timings.jsonl` contains ten interleaved pairs per geometry, with identical input, repeated calls, and a bitwise equality check before timing. The measured quantity is one stencil application. This does not measure the duration of a complete solve or concurrency between different RODAS5P stages. The host was a loaded Ryzen 9 5900X, 24 logical CPUs; an observation shortly after the run gave a 1-minute load average of 73.58. The recorded rows used rustc 1.94.1, opt-level 3, one codegen unit, panic=abort, compiling the example and spatial module together against cached release dependencies. The four-worker median paired serial/parallel ratios ranged from 0.30 to 0.84. Production keeps serial as the default spatial execution mode. A reserved pool and held-out full-solve, matched-achieved-error timings remain necessary for a default crossover rule.

The local native verification used CARGO_TARGET_DIR=/tmp/vigilode-spatial-review-target-20261124 and --offline. The full facade documentation suite passed four runnable examples (including the spatial example) and one compile-fail contract. The benchmark example also passed cargo check.

## Latitude-line preconditioner for a shifted sphere stage

With the same finite volumes, a positive real Rosenbrock shift σ = γh, and constant κ, write Kφ = −Lφ and Kθ = −Lθ. Both directional components are nonnegative and self-adjoint in the area norm. Define A = I + σ(Kφ + Kθ) and P = I + σKφ. Put α = κ cot(b/2)/(R² b). At every latitude, including the two polar rows, the sum of the two meridional off-diagonal conductances divided by the area is α. This follows by adding sin(θ−b/2) and sin(θ+b/2). Gershgorin bounds the spectrum of Kθ by [0, 2α]. The preconditioned Rayleigh quotient is 1 + σ ⟨u,Kθu⟩_W / ⟨u,Pu⟩_W. Since P ≥ I, it lies between 1 and 1 + 2σα. A constant field attains the lower endpoint. The zonal latitude-alternating field u_ij = (−1)^i has Kφu = 0 and Kθu = 2αu, so it attains the upper endpoint. Consequently the weighted, symmetrically preconditioned spectral condition number is exactly **1 + 2σκ cot(b/2)/(R² b)**. The bound is independent of longitude resolution. Euclidean left-preconditioned restarted GMRES has a different norm; its observed iteration counts are recorded separately.

The ring operator has Fourier eigenvalues 4 a_i sin²(πm/nφ), where a_i is the azimuthal rate in the code. Longitude FFTs implement P^-1 with independent rows. Retaining meridional coupling and applying W^1/2 gives one real-symmetric tridiagonal shifted matrix per longitudinal Fourier mode. The independent complex RHS for each mode can then be solved directly. The program below exercises both constructions against a sparse physical-grid incidence matrix, including the true residual of every returned stage solution. Spherical elliptic solvers and longitude decomposition have a long history, including [FISHPACK HWSSSP](https://www.eecs.yorku.ca/~roumani/fortran/slatecAPI/hwsssp.f.html). This result supplies an exact condition formula for this cell-area stencil and a concrete VigilODE port target. Computational evidence covers a single real shift and three meshes. A native hook, full adaptive trajectories, amortized setup, and comparisons at matched error remain open.

```sh
OPENBLAS_NUM_THREADS=1 python research/spatial_review_20261124/polar_lines.py
```
