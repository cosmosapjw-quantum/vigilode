# Structure-exact stage solves: a foundation that joins the side threads

Run `kernel_spectral_foundation_20261124`, branch `research/stage-lattice-referee-20261124`.
It follows the stage-lattice referee run of the same day (`research/stage_lattice_referee_20261124`).

Claim status: **D** derived here, **I** imported (standard result, cited by name), **A** assumption,
**T** tested in this run (script or Rust test named), **N** negative result, **P** proposal,
**C** conjecture, **O** open.

## 0. The question and the answer in one paragraph

The side threads of VigilODE were Fourier transforms and convolution, involutions, algebraic
topology (Hodge, H^0), singular and algebraic geometry (singular loci, inertia), and dynamical
systems (stiffness, contractivity). Each had produced gates and screens but no shipped
optimization. One numerical object joins them: the RODAS5P stage operator
`W(z) = z I - J` with `z = 1/(h gamma)`, solved eight times per step. Every thread is a statement
about `W` or `J`. Symmetry block-diagonalizes `W`. Topology fixes the kernel of `J` and gives a
conserved functional that checks the solves. Singular geometry says where `W(z)` fails and how to
certify that it has not. Dynamics bounds `spec J` and turns that bound into a route decision. The
optimization is a **structure-exact stage solver**: when the declared structure holds, the
eight stage solves cost two real FFT sweeps and one tridiagonal sweep per Fourier mode,
with no Krylov iteration, and they carry certificates that cost O(n). The previous run showed
that the eight stages cannot be run in parallel (V05, depth 8). Within one stage, the separable
solver has `floor(np/2) + 1` independent mode systems, so parallel width comes from symmetry,
not from the stage loop (§6).

## 1. The stage problem and its cost (D)

One RODAS5P attempt does 8 RHS calls, one Jacobian action or assembly, one preparation of `W(z)`
(factorization or preconditioner setup) and 8 solves with `W(z)`. On the matrix-free
route, each solve is a GMRES run: `k` iterations, each with a JVP, a preconditioner application
and `O(k n)` orthogonalization. On the banded route each solve is two triangular sweeps (the
stage lattice of the previous run). The levers are:
(a) a cheaper solve, by structure;
(b) a shorter critical path, by the stage lattice;
(c) factorization reuse;
(d) certificates that replace verification work.
This run works on (a) and (d) for operators with a symmetry group. It keeps (b) and (c) from the
earlier runs.

## 2. Fourier, convolution and involution: block diagonalization by symmetry

**2.1 (I, Schur / convolution theorem).** If a finite group `G` acts on the cells, and `J`
commutes with every permutation of the action, then `J` preserves each isotypic component of
the representation. For the cyclic group `Z_N`, those components are the Fourier modes, and a
`Z_N`-equivariant operator on `C^N` is a circulant, i.e. a convolution, which the DFT diagonalizes.

**2.2 The sphere grid (D, T: F3).** The latitude-longitude finite-volume heat operator of
`spatial.rs` reads, at ring `r` and longitude `x`,

    (L u)[r,x] = a_r (u[r,x-1] - 2 u[r,x] + u[r,x+1]) + n_r (u[r-1,x] - u[r,x]) + s_r (u[r+1,x] - u[r,x]).

Here `a_r` is the azimuth rate, `n_r` the north rate and `s_r` the south rate; all three are
constant along a ring. The longitude rotations `Z_np` therefore commute with `L`. Take the DFT
`u[r,x] = sum_m U_r(m) e^{2 pi i m x / np}`. It turns `L` into `np` independent tridiagonal
blocks:

    (L_m U)_r = -(n_r + s_r + a_r (2 - 2 cos theta_m)) U_r + n_r U_{r-1} + s_r U_{r+1},
    theta_m = 2 pi m / np.

With `J = L + c I` (a ring-independent linear rate `c`), the stage matrix of mode `m` is the real
tridiagonal

    W_m(z) = tridiag(-n_r, z - c + n_r + s_r + a_r (2 - 2 cos theta_m), -s_r).

The script `verify_foundation.py` builds `L` cell by cell and conjugates it with the DFT. Off-mode
coupling and block mismatch are below 1e-9 (check F3).

**2.3 Two involutions (D, T: F3, F8).**
- *Reflection* `x -> -x` commutes with `L`, so `W_m = W_{np-m}`: the blocks depend on `m` only
  through `cos theta_m`. Only `floor(np/2) + 1` distinct tridiagonal matrices exist, so a step
  needs `floor(np/2) + 1` factorizations, not `np`.
- *Complex conjugation* acts on the spectrum of a real field as `U(m) -> conj U(np - m)`, so
  real data have Hermitian spectra. Two consequences:
  (i) each real block `W_m` solves the real and imaginary parts of `U(m)` as two real right-hand
      sides with one factorization;
  (ii) two real rings `u, v` share one complex FFT of `u + i v`, separated by
       `U = (Z + R Z)/2` and `V = (Z - R Z)/(2i)`, where `R Z(m) = conj Z(np - m)`.
  Check F8 confirms (ii) to 1e-12. The packing halves the FFT count. The dihedral group
  `D_np = Z_np ⋊ Z_2` is the full symmetry used. Its real irreducible representations have
  dimension 1 (`m = 0` and, for even `np`, `m = np/2`) and dimension 2 (every other `m`).

**2.4 Periodic box (D).** `Z_nx × Z_ny × Z_nz` acts transitively and `L` has constant
coefficients, so the multidimensional DFT diagonalizes it completely:
`W(k) = z - c + sum_axis rate_axis (2 - 2 cos theta_axis)`. A stage solve is one forward
transform, a pointwise division and one inverse transform.

**2.5 Cost (D).** With ring pairs packed, one separable stage solve on the sphere costs
`nt/2` complex FFTs of length `np` forward and the same number inverse
(`O(n log np)` together), plus `floor(np/2)+1` Thomas solves, each with two right-hand sides
(`O(n)`). Preparing `W(z)` costs `O(n)` and happens once per attempt; all 8 stages reuse it,
because `J` is constant (§7, T3).

## 3. Algebraic topology: the kernel, the conserved mass, the stage mass identity

**3.1 Discrete Hodge form (D, T: F1).** Write `δ: C^0 -> C^1` for the coboundary (differences
across cell faces) and `K` for the diagonal of face conductances. Then
`L = -A^{-1} δ^T K δ`, where `A` is the diagonal of cell areas. The code's rates satisfy
`A_r s_r = A_{r+1} n_{r+1}` (the same face conductance `K_{r+1/2}`), and the azimuthal faces are
symmetric by construction. So `A L` is symmetric negative semidefinite: `L` is self-adjoint in
the area inner product `<u, v>_A = sum A_i u_i v_i`. Check F1 gives a relative asymmetry of
`A L` at round-off and no positive eigenvalue.

**3.2 Kernel = H^0 (D, T: F2).** `ker L = ker δ` is the space of functions that are locally
constant on the cell graph, i.e. the 0-th cohomology. The sphere's cell graph is connected
(the polar caps are rings of cells joined in longitude), so `dim H^0 = 1`. Check F2 finds a
one-dimensional kernel aligned with the constants. The same holds for the torus (periodic
box). The kernel lives in mode `m = 0`, and only there.

**3.3 Conserved mass (D).** `1^T A L = (A L 1)^T = 0`, so `M(u) = sum A_i u_i` is invariant
under pure diffusion.

**3.4 Stage mass identity (D, T: F7, Rust test `separable_stage_solves_are_exact_and_mass_consistent`).**
For `W = z I - (L + c I)`, apply `1^T A` to `W x = b`:

    (z - c) M(x) = M(b).

Every stage solve therefore has an O(n) a-posteriori check in the H^0 direction. The check
does not depend on the solver, so it also verifies the FFT round trip.

**3.5 Linear invariants of RODAS5P (D, T: `separable_route_conserves_mass_and_matches_the_reference`).**
Suppose `1^T A f(u) = 0` for every `u` and `1^T A J = 0`. The U-form stage equations are
`W k_i = f(Y_i) + sum_j (C_ij / h) k_j`. Applying `1^T A` with `c = 0` and using induction over
the stages gives `M(k_i) = 0`, so every RODAS5P step conserves `M` up to round-off when the
solves are exact. With an inexact (Krylov) solve, `M` drifts by the solver residual's mass. The
exact separable solve removes this drift by construction.

**3.6 Remark (I).** On the torus, `H^1` has dimension 2: two harmonic flux fields that are not
gradients. They do not touch the scalar heat equation. They matter for flux-form transport, where
a Hodge-based splitting must carry the harmonic part separately. The sphere has `H^1 = 0`.
Recorded as **O** for advection.

## 4. Singular and algebraic geometry: where the stage pencil fails, and inertia

**4.1 Singular locus (D).** The stage pencil `P(z) = z I - J` is singular exactly on
`Σ = spec J`. In parameter space `(h, p)` this is the hypersurface `det(I/(h gamma) - J(p)) = 0`.
Near `Σ`, `κ(W) ~ 1 / dist(z, spec J)`. Every RODAS5P failure that involves the linear algebra
sits near this locus.

**4.2 Inertia certificate (D, T: F4, Rust test `inertia_counts_the_modes_beyond_the_stage_pencil`).**
For `A`-self-adjoint `J`, `A W_m` is symmetric. Thomas elimination of `W_m` without pivoting
gives `W_m = (A^{-1} L A)(A^{-1} D L^T)`, where `A W_m = L D L^T`. Its pivots are therefore
`d_r / A_r`, and they have the signs of `D`. Sylvester's law of inertia (I) then gives

    #{non-positive Thomas pivots over all modes} = #{λ ∈ spec J : λ ≥ z}.

Check F4 confirms the equality in 40 random `(c, z)` trials, including 64 crossings in one trial.
Consequences:
(i) all pivots positive ⇔ `z > λ_max(J)` ⇔ `W(z)` is SPD in `<.,.>_A`, and then
    `||W^{-1}||_A = 1/(z - λ_max) ≤ 1/(z - c)`;
(ii) a non-positive pivot means the step crossed the singular locus in that mode. The solver
     refuses the preparation with a typed linear-solve error, and the driver rejects the
     attempt and shrinks `h`.
The certificate costs nothing beyond the factorization it reads.

**4.3 The pole as a chart singularity (D, T: F6).** The latitude-longitude chart collapses each
polar cap to a point. The polar ring's area is `2 R^2 sin^2(dθ/2) dφ`, and its azimuth rate is
`a_0 = κ dθ / (2 R^2 sin^3(dθ/2) dφ^2) ≈ 4κ/(R^2 dθ^2 dφ^2)`. Relative to the equator,
`a_0 / a_eq → 4 nt^2 / π^2` (F6: quotient 0.99997 at `nt = 256`). This is the classical pole
problem (I). Explicit methods pay for it with an `O(nt^2)` smaller stable step. Krylov methods
pay with `κ(W)` growing like `a_0 / z`. In the mode decomposition, the polar stiffness enters
`W_m` only on the diagonal, as `a_0 (2 - 2 cos θ_m)`. High modes at the polar ring are therefore
damped exactly, by a diagonally dominant row. The separable solve resolves the chart singularity
at no extra cost: the large entries sit where Thomas elimination is most stable.

## 5. Dynamical systems: contractivity, spectral bounds, a stiffness certificate

**5.1 Contractivity (D).** The logarithmic norm of `J = L + cI` in `<.,.>_A` is
`μ_A(J) = λ_max(J) = c`. So `||u(t) - v(t)||_A ≤ e^{ct} ||u(0) - v(0)||_A`. `J` is `A`-normal, so
`||R(hJ)||_A = max_λ |R(hλ)|`. RODAS5P is A-stable (I), so for `c ≤ 0` the discrete map is
`A`-contractive for every `h`.

**5.2 Spectral bound (D, T: F5).** Gershgorin's theorem applied to each `W_m` gives
`spec J ⊂ [c - ρ̄, c]` with `ρ̄ = max_r 2(n_r + s_r) + 4 a_r`. F5 measured `ρ/ρ̄ = 0.982`
on a 9×16 grid. The bound costs `O(nt)`.

**5.3 Stiffness certificate (D, P for the router).** DOPRI5 is stable on `[-β, 0]` of the real
axis with `β ≈ 3.3`. For `c ≤ 0`, an explicit run over `T` needs at least `T ρ̄ / β`
steps of 6 RHS calls. An implicit run with exact separable solves needs `N` steps, each costing
`8 RHS + 8 S + F`, where `S` is a separable solve and `F` a preparation. Implicit integration
pays off when `6 T ρ̄ / β · c_f > N (8 c_f + 8 c_S + c_F)`. The left side is known before the
first step. `SeparableStages::spectral_bounds` returns `ρ̄`, `c` and the explicit step bound
`β / ρ̄`, so the facade or the user can make this choice. Wiring it into the automatic router is
**P** (not done here).

## 6. The stage lattice, revisited

The previous run proved that the RODAS5P stage dependency has depth 8 (V05). It also showed
that on the banded route the critical path is the 16 triangular sweeps (SL-1). For a separable
operator the picture inside one stage changes. The FFTs are independent across ring pairs
(width `nt/2`). The mode solves are independent across modes (width `floor(np/2)+1`). Each mode
solve has depth `2 nt` (Thomas), or `O(log nt)` with cyclic reduction (**P**). The eight stages
stay sequential (**N**, unchanged). Parallel width inside a stage is the symmetry group's number
of irreducible blocks. This is the correct form of "parallelizing the 8-stage loop" for
symmetric problems: parallelize inside each stage, across irreducible representations.

## 7. Implementation contract (what the code does, and its claims)

`vigilode::spatial::SeparableStages` (built by `DiffusionGrid::separable_stages(c)`) runs
inside the protected client driver `integrate_rodas5p_fast_client_observed_with_atol` through
`vigilode::unified::solve_separable`. Its claims:

- **T1 exactness.** For `J = L + cI` on the sphere and the box, the separable solve meets
  `||b - W x||_inf ≤ 1e-13 ((|z - c| + ρ̄) ||x||_inf + ||b||_inf)` on random right-hand
  sides (7 grids including odd `nt`, odd `np` and a 3D box, `z` from 1e-3 to 1e4; measured
  maximum 3.4e-16).
- **T2 stage mass.** `|(z - c) M(x) - M(b)| ≤ 1e-13 (Σ A_i) ((|z - c| + ρ̄) ||x||_inf + ||b||_inf)`
  (measured maximum 5.1e-17). The normalization by the backward-error scale, not by
  `Σ A_i |b_i|`, is needed for `z ≪ ρ̄`, where `||x||` exceeds `||b||` by `1/z`.
- **T3 route.** A whole IVP on the separable route matches an independent dense
  matrix-exponential reference within the tolerance envelope. It conserves `M` to round-off for
  `c = 0`, and agrees with the shipped matrix-free route within the same envelope.
- **T4 inertia.** For `c > z`, preparation refuses with a typed error, and the refusal count
  equals the number of eigenvalues above `z` on a small grid.
- **T5 verified mode.** If the problem's true Jacobian differs from the declared `L + cI` (a
  non-constant reaction), `Verified` mode computes the true residual with the problem's JVP and
  runs defect correction `x ← x + W_sep^{-1} r`. This converges when
  `||W_sep^{-1} (J - J_sep)||_A < 1`, guaranteed for `h γ < 1 / (||J - J_sep||_A + c_+)` (D).
  Beyond that bound the solve refuses, and the step shrinks.
- **T6 cost.** Diagnostic timing only, against the shipped JVP + GMRES route, on this 2-vCPU
  host. Owner-host cells are preregistered in `PLAN.json`.

## 8. What does not carry over (N, O)

- Longitude-dependent coefficients break `Z_np` (**N** for exactness). `Verified` mode turns
  the separable solve into a preconditioner with a checked residual (T5).
- Advection is non-normal. The inertia certificate (§4.2) and the contractivity bound (§5.1)
  do not apply: **O**, field-of-values replacement needed. §10 gives the replacement (KS-N8).
- Nonlinear reaction `f(u)` with `f'` not ring-constant: same as the first point. A ring-wise
  linearization (`c_r` per ring) keeps the separability, since a ring-constant diagonal commutes
  with `Z_np`. The implementation takes one global `c`; per-ring `c_r` is **P**.

## 9. Where the side threads now sit

| Thread | Object | Use in code |
| --- | --- | --- |
| Fourier / convolution | `Z_np` isotypic decomposition | block-diagonal stage solve |
| Involution | reflection, conjugation | half the factorizations, half the FFTs |
| Algebraic topology | `H^0`, Hodge form | mass identity check, conservation test |
| Singular geometry | stage-pencil locus, pole chart | inertia refusal, polar stiffness in diagonal |
| Algebraic geometry (inertia) | Sylvester, Sturm | pivot-sign certificate |
| Dynamical systems | log norm, Gershgorin | contractivity statement, `spectral_bounds` |
| Stage lattice | critical path | width from symmetry inside a stage |

## 10. Advection: the field of values replaces inertia and Gershgorin (KS-N8)

This section answers the open advection item of §8. The derivation, with every step, is in
`ks_n8/DERIVATION.txt`. The checks are in `scripts/verify_fov.py`, which writes
`results/KS_N8_FOV.json` (225 cases on 3 grids, reproducible to the byte). The closed forms of 10.2
are also checked symbolically in `ks_n8/logs/wolfram_l4_symbolic.txt`. The code has no advection
term, so this section defines one on the code's grid. No Rust code changes.

**10.1 The operator (D).** The velocity is constant on rings: a zonal speed `u_r` per ring and a
meridional speed `v_f` per latitude face, positive towards larger colatitude. The transport is in flux
form, the same finite-volume form as `L`. The zonal volume flux through the meridian faces of ring `r`
is `ψ_r = u_r R dθ`. The meridional flux through face `f` is `φ_f = v_f R sin(f dθ) dφ`, which is zero
at the polar faces. With `b_r = ψ_r / A_r` and `div_r = (φ_{r+1} - φ_r) / A_r`:

    A_r dq[r,x]/dt  ⊃  ψ_r (q̂[r,x-1/2] - q̂[r,x+1/2]) + φ_r q̂[face r] - φ_{r+1} q̂[face r+1].

The primary scheme takes centred face values (the mean of the two cells). It is second order. For
divergence-free flow it is A-skew-adjoint, so the advection adds non-normality without damping, which
is the hard case for a certificate. Its A-Hermitian part is still the diffusion operator of §4.2, so the
certificate can be compared with §4.2 directly. The secondary scheme is donor-cell upwind. It equals
the centred scheme plus an artificial diffusion with conductance `|flux|/2` on every face, which is
A-self-adjoint and negative semidefinite (D), so every Hermitian-part statement below covers it. The
cell Péclet numbers are `|u_r| R sin θ_r dφ / (2κ)` (zonal) and `|v_f| R dθ / (2κ)` (meridional).

- *Divergence-free implies zonal (D).* `φ_0 = 0` and `φ_{r+1} = φ_r + A_r div_r`, so `div ≡ 0` if and
  only if `φ ≡ 0`. Also `Σ_r A_r div_r = 0`, so a divergent flow has `max_r(-div_r) > 0`.
- *Mode blocks (D, T: A0, worst 1.9e-15).* Ring-constant coefficients keep `Z_np`, and mode `m` gives
  the tridiagonal block `B_m = L_m + cI + T_m`, with

      centred: T_m = diag(-i b_r sin θ_m) + V,
               V[r][r-1] = φ_r/(2A_r),  V[r][r+1] = -φ_{r+1}/(2A_r),  V[r][r] = -div_r/2;
      upwind:  T_m = diag(-|b_r| (1 - cos θ_m) - i b_r sin θ_m) + V_up   (donor cell).

  `B_{np-m} = conj(B_m)`. The reflection of §2.3 is lost because it reverses the flow. The conjugation
  involution stays, so real data still need only the half spectrum, now with complex blocks.
- *Mass (D, T: A6, worst 5.6e-17).* In flux form `1^T A T = 0`, so the stage mass identity
  `(z - c) M(x) = M(b)` of §3.4 holds unchanged.

**10.2 Hermitian and skew parts (D, T: A1, worst 2.9e-16; symbolic for `nt = 4`).** The DFT along
rings is unitary in `<.,.>_A`, so `J` is A-unitarily equivalent to the direct sum of
`B̃_m = Â B_m Â^{-1}`, with `Â = diag(√A_r)`. The A-Hermitian part `H̃_m = (B̃_m + B̃_m^*)/2` is real
symmetric tridiagonal:

    diagonal   c - n_r - s_r - a_r σ_m - div_r/2   [upwind: - |b_r|(1 - cos θ_m) - (|φ_r| + |φ_{r+1}|)/(2A_r)]
    off-diag   √(n_{r+1} s_r)                       [upwind: + |φ_{r+1}| / (2 √(A_r A_{r+1}))]

The skew part `K̃_m = (B̃_m - B̃_m^*)/(2i)` is Hermitian tridiagonal, with diagonal `-b_r sin θ_m` and
off-diagonal `i φ_{r+1} / (2 √(A_r A_{r+1}))`. Centred zonal advection is therefore purely skew, which
means it conserves A-energy. Meridional flow adds `-div/2` to the Hermitian part, and upwind adds a
negative semidefinite part. `H̃_m` depends on `m` only through `cos θ_m`, so it has `floor(np/2) + 1`
distinct matrices, as in §2.3.

**10.3 Hermitian-part inertia certificate (replaces §4.2; D from I, T: A3).** The A-logarithmic norm is
`μ_A(J) = max Re W_A(J) = max_m λ_max(H̃_m)`. Here `W_A(J) = conv ∪_m W(B̃_m)` (I: numerical range of a
direct sum), and `spec J ⊂ ∪_m W(B̃_m)`. Sylvester's law (I), applied to `zI - H̃_m`, gives

    C(z) := #{non-positive Thomas pivots of zI - H̃_m over all modes} = #{eigenvalues of (J + J^†)/2 at or above z},

and `C(z) = 0 ⇔ z > μ_A(J)`. The count equality needs `z` not to be an eigenvalue of a leading
principal submatrix of `zI - H̃_m`; a zero pivot is counted as a refusal. In all 225 cases, for `z` above and below `μ_A`, the counts over all
modes and over the half spectrum match the dense eigenvalue counts. When `C(z) = 0`:

- (a) *Solvability and resolvent.* `Re <W(z) x, x>_A ≥ (z - μ_A) ||x||_A^2`, so by Cauchy–Schwarz
  `W(z)` is invertible and `||W(z)^{-1}||_A ≤ 1/(z - μ_A)`. For complex `w` outside `W_A(J)`,
  `||(wI - J)^{-1}||_A ≤ 1/dist(w, W_A(J))` (I). T: A4 tested 900 (case, `z`) pairs and found no
  violation beyond round-off (largest normalized excess 2.6e-16).
- (b) *Thomas on the complex blocks.* Every pivot of `W_m = zI - B_m` (no pivoting) satisfies
  `Re p_r ≥ z - λ_max(H̃_m) ≥ z - μ_A`. The multipliers satisfy `|l_r| ≤ |B_m[r][r-1]| / (z - μ_A)`.
  Proof (D): `W_m` and `Â W_m Â^{-1}` have the same pivots, and `p_r` is a Schur complement. With
  `x = (-M_r^{-1} w y; y)`, `Re(p_r) |y|^2 = Re x^* M_{r+1} x ≥ (z - λ_max(H̃_m)) |x|^2`, by Cauchy
  interlacing for the Hermitian part. This is the quantitative form of "all pivots positive". T (A5):
  every pivot exceeds its bound by at least 3.5e-5 of the scale, and the solves have backward error at
  most 1.4e-15 in the tested range (`z - μ_A` at least 1e-4 of the scale, cell Péclet at most 30).
  Corrected after the independent check (finding F1): the multiplier bound grows like `1/(z - μ_A)`,
  and without pivoting the complex sweep is not backward stable near the threshold for centred
  divergent flow at high Péclet (growth up to 1e8, backward error up to 5.5e-6 at `z - μ_A` of 1e-12
  to 1e-6 of the scale, against 4e-17 with partial pivoting; upwind stays below 2e-16). A solver of
  the complex blocks (P) needs pivoting or a refusal margin there.
- (c) *Reduction, and only one direction.* For zero velocity, `C(z)` is the §4.2 count. For A-normal
  `J` (zero velocity, and solid-body rotation, where `b_r` is constant), `C(z) = #{λ : Re λ ≥ z}` and
  `||W(z)^{-1}||_A = 1/dist(z, spec J)` (T: A9). For non-normal `J`, only `C(z) = 0` carries
  information. The certificate may refuse a regular `W(z)`, and `C(z) > 0` does not count eigenvalues:
  `[[1, 4], [0, 1]]` at `z = 1/2` has two eigenvalues with `Re λ ≥ z` but `C = 1`.
- *Cost.* One real symmetric Thomas sweep of `zI - H̃_m` per distinct mode and preparation, which is
  O(n). Zonal flow needs none, because there the certificate is `z > c` (10.4).

The Euclidean variant, `μ_2(J) = λ_max((J + J^T)/2) = max_m μ_2(B_m)`, satisfies the same theorems in
the 2-norm (T: A4, 708 pairs), but it does not reduce to §4.2. For zero velocity, `μ_2 - c` ranges from
0.0055 to 0.19 on the three grids. It is positive whenever the ring areas differ: `x = 1` gives a zero
Rayleigh quotient, while `(L + L^T) 1 = L^T 1 ≠ 0`. The area-weighted norm is therefore the right one.

**10.4 The value of `μ_A`, and the abscissa against Gershgorin (D, T: A10, A11, C1).**

- (i) `μ_A ≥ c`: take `x = 1` and use mass conservation.
- (ii) For divergence-free (zonal) flow, centred or upwind, `μ_A = c` exactly, and the resolvent bound is
  attained because `J 1 = J^† 1 = c 1`. The certificate is `z > c`, the same as for diffusion. T: over
  153 cases, `|μ_A - c| ≤ 3.8e-17` of the scale, and the bound is attained to 1.0e-12.
- (iii) For general ring-constant flow, `c ≤ μ_A ≤ c + max_r(-div_r)/2` (Weyl). The exact value is a
  bisection on `C(z)`.
- (iv) The Gershgorin abscissa of the mode blocks (the §5.2 construction) satisfies
  `α_G ≥ c + max_r(-div_r)`. Take row `r` at `m = 0`: for the centred scheme the bound follows from the
  triangle inequality; for upwind, the row is Metzler with row sum `c - div_r`. Hence
  `μ_A - c ≤ (α_G - c)/2`: the field of values at least halves the Gershgorin growth bound of a
  divergent flow, and ties with it (`= c`) for zonal flow. Over the 72 divergent cases, the measured
  `(μ_A - c)/(α_G - c)` ranges from 0.023 to 0.48.

Gershgorin in grid space (the max-norm log norm and Varah's resolvent bound) does worse for the centred
scheme at cell Péclet above 1. At Péclet 30 in zonal flow, its abscissa exceeds `c` by 3.3 to 788,
while `μ_A = c`. The Varah bound does not apply at 193 of the 900 tested `z`; the field of values
certifies all 900.

The following is measured, not proven. For a monotone operator (upwind, or centred below Péclet 1),
the area-weighted column log norm `μ_{1,A}` equals `c` to round-off (at most 5.7e-13), by the Metzler
property and mass conservation. In its own (L1) norm, this is sharper than `μ_A` for divergent flow:
the L2 energy of a convergent flow can grow while its mass cannot, and `μ_A - c` is the true L2 rate.
For divergent flow the certificate is conservative. At `z = μ_A + 1e-4` of the scale, the true
`||W^{-1}||_A` is 3.0 to 851 times below the bound, and `μ_A` exceeds the spectral abscissa by 0.0026
to 45.

**10.5 Contractivity (replaces §5.1; I + D, T: A7, A8).** `||e^{tJ}||_A ≤ e^{t μ_A}` (I, Dahlquist).
For a rational `R` bounded on `Re w ≤ h μ_A`, `||R(hJ)||_A ≤ sup_{Re w ≤ h μ_A} |R(w)|`. This is von
Neumann's theorem in an inner-product norm (I, Hairer and Wanner II §IV.11), applied to `hJ - h μ_A I`
and to `w ↦ R(w + h μ_A)`. RODAS5P has `R(w) = 1 + w b^T (I - w β)^{-1} 1`, whose only pole is at
`1/γ`. The hypothesis is therefore `z = 1/(hγ) > μ_A`, which is the certificate condition. For
`μ_A ≤ 0` (zonal flow with `c ≤ 0`), the RODAS5P step is A-contractive for every `h`, as in §5.1. It is
now an inequality, because `J` is not A-normal and the norm is no longer a maximum over the spectrum.
T: the script builds the stability function from `fixtures/rodas5p_coefficients_snapshot.json`
(sampled `max |R(iy)| = 1 + 2.2e-16`, `|R(-1e12)| = 1.3e-11`). In all 225 cases, `||R(hJ)||_A` never
exceeds the bound by more than 1.6e-13 (relative), with equality attained by the constants in zonal
flow at `c = 0`, and `||e^{tJ}||_A` never exceeds its bound by more than 1.1e-14 (corrected after the
independent check, finding F3: for divergent flow the norms sit below their bounds).

**10.6 Spectral and stiffness enclosure (replaces §5.2; D from I, T: B0 to B3, C2).** The support
function of `W(B̃_m)` is `h_m(φ) = λ_max(cos φ H̃_m + sin φ K̃_m)`, the top eigenvalue of a Hermitian
tridiagonal matrix. With `K` directions, the polygon
`P_m = ∩_k {w : Re(e^{-i φ_k} w) ≤ h_m(φ_k)}` contains `W(B̃_m)` and `spec B_m`. `K = 4` gives
Bendixson's rectangle `[λ_min(H̃_m), λ_max(H̃_m)] × i[λ_min(K̃_m), λ_max(K̃_m)]` (I). A larger `K` gives
Johnson's outer polygon (I). Then `spec J ⊂ ∪_m P_m` and `W_A(J) ⊂ conv ∪_m P_m`. Each `h_m(φ_k)`
can be computed with a certified bracket by bisection on the Sturm count of 10.3 (P); the script uses
dense `eigvalsh` instead. With `K = 256`, every eigenvalue of `J` and of each `B_m` lies in the
enclosure to round-off (largest normalized excess 6.5e-15). The stiffness radius `ρ_F = max |v - c|`
over the vertices bounds both the spectrum and the numerical range of `J - cI`.

The measured ratio `ρ_F/ρ_G` over the 225 cases, where `ρ_G` comes from the row Gershgorin discs of
the `B_m` (the §5.2 construction applied to the advective blocks):

| Velocity | Cases | `ρ_F/ρ_G` | Notes |
| --- | --- | --- | --- |
| Zero | 9 | 0.958 to 0.982 | the F5 ratio; the Hermitian part is exact |
| Zonal | 144 | 0.958 to 0.997 | `ρ_F` equals the spectral radius to within 2.2e-4 on the tested set; imaginary-extent ratio 0.12 to 0.99, because `K̃_m = diag(-b_r sin θ_m)` is exact while Gershgorin adds `n_r + s_r` |
| Divergent | 72 | 0.64 to 1.10 | `ρ_F > ρ_G` in 9 cases (below) |

Corrected after the independent check (finding F2): the looser cases below are an artifact of
sampling Péclet only at 0.3 and 30. With more Péclet numbers, `ρ_F > ρ_G` occurs for divergent flow
under both schemes: centred near cell Péclet 1 (74 of 1480 centred divergent cases, up to 1.054;
there a meridional off-diagonal vanishes and Gershgorin is exact for the spectrum) and upwind at high
Péclet in many families (601 of 1480, up to 1.2071 = (1 + √2)/2, a Jordan-type 2×2 limit). Zonal
flow is never looser. In the 225 frozen cases, the 9 looser cases are all upwind Hadley-plus-jet flow at Péclet 30, for every `c`: `ρ_F/ρ_G` is 1.104
(G1), 1.037 (G2) and 1.033 (G3). The excess is at the left end: the leftmost point of `H̃_m` lies
beyond the leftmost Gershgorin disc by the same ratio. The numerical range of these strongly non-normal
blocks reaches further than their spectrum; there the Gershgorin radius exceeds the spectral radius by
only 0.6 to 3.1 %.

Gershgorin encloses only the spectrum. For non-normal `J`, the spectrum does not bound `||R_E(hJ)^n||`
for an explicit method `R_E`, but the numerical range does: `||f(X)|| ≤ (1 + √2) max_{W(X)} |f|`
(I, Crouzeix and Palencia 2017). So if `h conv ∪_m P_m` lies inside the DOPRI5 stability region, then
`||R_E(hJ)^n||_A ≤ 1 + √2` for every `n`. The §5.3 step bound `β/ρ̄` becomes a two-dimensional fit of
the enclosure into the DOPRI5 region: **P** for the router (KS-N5), UNVERIFIED here.

**10.7 Verdict (KS-N8).** The acceptance criterion, a stated and checked bound on 3 small grids, is
met. D1, "bound holds on 3 grids", is **PASS**: every check of A0 to A11, B0 to B3 and C1 holds. The
225 cases are 7 velocity families, Péclet 0.3 and 30, centred and upwind, 3 values of `c`, and `z`
both above and below `μ_A`. D2, "tighter than Gershgorin", is **FAIL** under the rule frozen before
the run (`ks_n8/NOTES.txt`), because `ρ_F` exceeds `ρ_G` in 9 of 225 cases (upwind, divergent,
Péclet 30). The kill criterion (no computable bound tighter than Gershgorin) does not apply: every
case is strictly tighter in `ρ` or in the abscissa, the abscissa is never looser (proven), and the
radius is tighter in 216 of 225 cases. The kill fallback is therefore not forced. The separable route
in the code stays diffusion-only only because no complex-block solver exists yet (P). For divergent
flow (centred near cell Péclet 1, upwind at high Péclet), the Gershgorin radius can be the tighter
*spectral* radius, but only the field of values bounds norms.

- **Proven** (D, from the imported results named above): 10.1, 10.2, 10.3 (a) to (c), 10.4 (i) to
  (iv), 10.5, and the enclosure property of 10.6.
- **Only checked** (T): the ratios and margins quoted here, on 3 grids (`nt × np` = 6×10, 9×16, 11×15),
  7 velocity families and 2 Péclet numbers.
- **Negative (N; corrected after the independent check, finding F4):** `ρ_F ≤ ρ_G` in general. It is
  false for divergent flow (9 counterexamples in the frozen run, 687 in the independent check, all
  confirmed against the true numerical range); zonal flow has no counterexample.
- **UNVERIFIED or open:**
  - the certified Sturm bisection for `h_m(φ_k)` (P);
  - a solver with complex mode blocks, the half spectrum and the Hermitian-part certificate (P, not
    implemented);
  - the DOPRI5 explicit step bound from the enclosure (P);
  - grids larger than those checked;
  - longitude-dependent velocity, which breaks `Z_np` (§8, O);
  - the harmonic flux part on the torus (§3.6, O).
