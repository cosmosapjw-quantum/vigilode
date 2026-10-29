# Independent review of the NCP follow-up

Reviewer: separate `followup_reviewer` agent. Base inspected: `7a63876a97fc17f640771ec06658269947049994`. This review rederived the block algebra and remap inequalities and read the changed implementation and targeted tests. Historical solver evidence was not rerun. Native execution results belong to the corresponding validation receipts, not to the act of reviewing code.

## Findings and corrections

| ID | Finding | Consequence | Correction / status |
|---|---|---|---|
| IR-01 | Initial bordered constructor used `vec![..]` after usize-product checks only | Extreme capacities could panic rather than return a typed refusal | Checked byte length against `isize::MAX` and `try_reserve_exact` added; extreme-capacity regression present. Closed by source review; execution tracked in Schur tests. |
| IR-02 | Initial cyclic stencil accumulation could yield `J=-1` for a one-cell advection case with diffusion=1, velocity=1e16 although its RHS is identically zero | New opt-in path could change the IVP in a degenerate but valid dimension | Explicit one-cell zero Jacobian and two-cell coalesced coefficients added. Adversarial regression is present in accuracy tests. Closed by source review; final execution is separately recorded. |
| IR-03 | Stencil complexity metadata appeared even when no stencil assembly executed | Telemetry could misidentify the measured code path | Complexity metadata is now null unless actual stencil build counter is positive. Closed. |
| IR-04 | Photon phase-space measure was initially labelled total `dN` without volume | Normalization ambiguity between occupation, number density and total/comoving number | Remap theory now uses `dn`, states per proper volume and requires explicit volume/geometry mapping. Closed. |
| IR-05 | Logarithmic GMRES iteration formula omitted trivial boundary cases | `q=0` makes the displayed logarithm undefined | Schur theory now states `0<q<1`, `0<epsilon<1`; identity and initially satisfied targets are separate. Closed. |

## Schur kernel

For `M=[[A,B],[C,D]]`, substitution of `u=A^-1 r_r`, `W=A^-1 B`, `z=(D-CW)^-1(r_m-Cu)`, and `x_r=u-Wz` yields both block equations exactly. The implementation uses this ordering. Reviewed asymmetric lower/upper bandwidth indices, the Schur row-swap/permutation order, and active-dimension refactors. Every failed refactor invalidates old factors; invalid factors cannot silently be reused. The default unknown preconditioner identity forces recycle-image refresh under the existing trait contract.

Strict row diagonal dominance is a sufficient admission condition for no-pivot elimination, not an error certificate or a property of every radiation Jacobian. Schur nonsingularity is separate. Full-operator true residual must decide Krylov success. A failed `apply` from arithmetic overflow may partially overwrite the output, as explicitly documented; callers must discard it. Successful refactor/apply have no allocation operations, and a dedicated allocator-count test covers variable active dimensions.

For lower/upper widths `l,u`, phase size `n`, border size `m`, work is `O(n*l*u+n*(l+u)*m+n*m^2+m^3)` for setup and `O(n*(l+u)+n*m+m^2)` per apply, with diagonal-loop terms understood when widths are zero. Fixed bandwidth and fixed border give linear-in-n cost. Periodic wrap entries and dense/low-rank scattering cannot be dropped from the true operator merely to meet the band contract; include them in a border/low-rank correction or use the block as a measured approximate preconditioner.

The conditional GMRES bound follows by selecting `p_k(z)=(1-z)^k` when `||I-M P^-1||_2<=q<1`; changing the preconditioner within a solve requires a different flexible-Krylov analysis. The logarithmic iteration formula applies for `0<q<1`, `0<epsilon<1`; `q=0` is the identity case. The exact triangular Schur factor gives a nilpotent residual block of index two. Neither statement guarantees a universal 3–5 iterations for approximate radiation systems.

The later T5 addition and `LOW_RANK_EXTENSION.md` were independently checked by eliminating `z` from `-V^T u+z=0`. The augmented first equation is exactly `(A+UV^T)u+Bv=f`, while the matter equation is unchanged. Inputs `B_ext=[B,U]`, `C_ext=[C;-V^T]`, `D_ext=diag(D,I)` therefore fit the delivered API with border size `m+r` and zero auxiliary RHS. An original-dimension Krylov interface still needs an explicit embedding/restriction wrapper; the enlarged kernel's dimension must not be silently treated as `n+m`. Cost uses `m+r`; Thomson rank is two per energy group, potentially `2*N_E` globally. This extension is accepted as derived algebra/API mapping, with a dedicated native augmented-border client test still pending as the document states.

## Remap kernel

The overlap matrix is nonnegative and column stochastic on equal covered domains. Summing columns proves conservation; applying the triangle inequality proves L1 contraction of cell integrals, equivalently measure-weighted L1 contraction of densities. The sorted two-pointer algorithm visits at most `n+m-1` positive overlaps. The independent cumulative-defect recurrence and `L*h_j^2/3` cell-average bound in `REMAP_THEORY.md` follow from the triangle inequality and the exact double integral of `|x-y|`.

The code operates on nonnegative integrals, validates all widths/endpoints, stages every result, and copies to caller output only after all checks. Overflow, positive-contribution underflow and exceeded mass-defect thresholds refuse atomically. Exact-arithmetic proofs are correctly separated from floating diagnostics. The L1 diagnostic is not an unknown-solution error estimator and is not inserted into the existing Euclidean certificate API.

Number and energy are distinct moments; signed Stokes/PSTF quantities do not satisfy this nonnegative-scalar API. The one-dimensional measure-coordinate kernel is a valid building block for angular-cell energy sweeps and separable mappings. It is not a generic spherical intersection engine.

## Accuracy adapter and physics connection

The opt-in heat stencil removes the repeated basis-JVP RHS construction and preserves dense storage/clearing costs. Tests compare the original analytic JVP columns, cyclic neighbour collisions and a full fast-driver trajectory. Negative advection velocity in matrix-equality tests establishes derivative correspondence only; the inherited left-biased advection stencil is not thereby proved monotone for negative velocity.

The LRS Bianchi-I reference uses both log-energy and cosine-angle cell measures and satisfies the checked identity `3H+3*qdot+div_mu(v_mu)=0`. The unpolarized cold Thomson operator has nonlocal rank-two gain, not nearest-neighbour angular scattering. Its number cancellation and isotropic null mode agree with the written formula. The absorption ledger conserves photon energy plus ionization and thermal energy under the stated single-photon assumptions. Generic Bianchi, polarization, recoil and live BASS/rei coupling remain explicit scope extensions.

## Accepted scope

The reviewed changes support adoptable guarded Schur/remap kernels and an opt-in benchmark assembly improvement, subject to their recorded native tests. They do not implement an accepted-step AMR driver by slicing workspace alone. A production connection still needs atomic mesh/state/measure/tolerance updates; Jacobian, preconditioner, recycler and dense-output identity invalidation; remap and linear-solve error budgets; and independent full-horizon physical references. The current RODAS5P implementation has eight stages. Frozen nonlinear RHS coefficients can introduce first-order global consistency error even if the inner integrator has higher order; keeping only the Jacobian/preconditioner frozen is a distinct operation.

Final per-file review identities and execution-scope status are recorded in `INDEPENDENT_REVIEW.json`.
