# Numerical and certificate design contract

## Claim namespaces
Keep LOCAL_ERROR_ESTIMATE, OBSERVED_REFERENCE_ERROR, A_POSTERIORI_GLOBAL_BOUND, VALIDATED_TRAJECTORY_ENCLOSURE and PERFORMANCE_EVIDENCE separate. None implies another.

## Validated IVP target
For y'=f(t,y), y(t0) in Y0, specify dimension, units, regularity, callback semantics, [t0,T], output grid and parameter uncertainty. Return interval or Taylor-model states Y(t) with existence/uniqueness evidence and exact y(t) in Y(t) for every t over the full interval.

For every slab [tn,tn+1], construct a candidate tube X and a validated inclusion. A Picard form P(X)(t)=Yn+integral F(s,X(s)) ds must be proved outwardly inside X, together with a contraction/Lipschitz or justified interval Newton/Krawczyk existence/uniqueness argument. Each slab starts from a set containing the previous terminal enclosure. No time gaps.

Embedded estimates, finite grids, convergence plots, ordinary high-precision references and scalar metadata are diagnostics, not enclosures.

## Arithmetic trust boundary
Every bound-affecting operation needs directed rounding or verified roundoff enclosure: arithmetic, abs, sqrt, norms, eigenvalues, quadrature, Taylor remainder, exp, factorization and serialization. Detect NaN, overflow, underflow, cancellation and subnormals. State FMA/reassociation assumptions. Taylor polynomial plus interval remainder jointly encloses truncation and floating-point effects.

## AUTH-001 contraction
For T(x)=x-PG(x), P invertible, prove on the complete closed B2(xc,r): q_hi>=sup ||I-P DG(x)||_2, q_hi<1; rho_hi>=||PG(xc)||_2; rho_hi<=(1-q_hi)r. Then Banach gives a unique fixed point, and invertibility of P gives G(x*)=0. Every bound is outward. A nonzero evidence_reference or caller q is not a derivative proof.

## AUTH-002 Leja
Bind witness to exact operator digest, dimension, scaling, basis/order where material, and model epoch. Validate at consumption. Approximate skew symmetry needs outward defect and spectral perturbation proof; otherwise EstimateOnly.

## AUTH-003 Gershgorin
Outwardly compute r_i>=sum_{j!=i}|A_ij| and endpoints A_ii +/- r_i. Verify matrix class permits the claimed real spectral interval. Caller intervals and Ritz values remain EstimateOnly unless independently verified and source-bound.

## OBS-001 observer
Sampled mu_H(t_k) or alpha_max(t_k) does not bound slab supremum. Validated mode needs enclosures for J,H,Hdot, positive H lower bound, eigenvalue, quadrature remainder, metric jumps, QR defects and roundoff on every slab. Current sampled mode is EstimateOnly/SampledDiagnostic.

## Switching/API
HybridRouteAutomaton and DriveState are distinct until runtime call path proves otherwise. Live transitions preserve accepted state/time/tolerance/work. Ok(SwitchingSolution) is not completion: inspect success, final_time, message and accounting_complete.

## Performance
Use same problem/data/observable/grid/norm/achieved error/compiler/binary/runner/CPU affinity/reference. Reference uncertainty must be below target or reference validated. Disputed reference means matched rows are diagnostic-only. Report failures/refusals/timeouts/rejected/speculative work, medians/spread and allocation scope. Same host proves host identity only.

## Epoch
Current SHA-256/index-sensitive golden tests pass, but owner authority file cited in code comment is absent from current docs/research. Resolve authority and persisted-ID migration.
