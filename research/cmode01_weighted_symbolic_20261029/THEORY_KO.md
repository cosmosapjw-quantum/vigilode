# CMODE01: 가중 logarithmic norm · 검증된 symbolic-cell 인증 재사용
**2026-10-29.** Parent source PR106 head `c6cba2b62629e819dd163325c7c9fb9bb5e3080c`. Prior PR104/105/106 remain unmerged; preregistration on this branch at `0a8737e94319829c574fb5d954440b5de5931054`, committed before new execution. The independent standalone Rust implementation is for a synthetic affine-clock nonautonomous model, NOT native VigilODE RODAS/GMRES.

## Theorem 1: weighted lognorm and physical norm
For a real represented matrix J, positive w, sigma>=0, define
`mu_w(J)=max_i[J_ii + sum_{j!=i}|J_ij|w_j/w_i]`.
By semigroup/logarithmic norm and the resolvent Laplace integral, if `1-sigma mu_w(J)>0`,
`||(I-sigma J)^(-1)||_(w,inf) <= 1/(1-sigma mu_w(J))`.
If physical weights p differ, error of a proposed stage xhat is bounded by
`||xhat-x*||_(p,inf) <= max_i(w_i/p_i) * K_w * max_j |b_j - (Axhat)_j|/w_j`,
where `K_w=1/(1-sigma mu_w(J))`. Do not discard norm-conversion factor. Original residual, source clock, norm, epoch, and margin must be validated before committing. A matrix-free JVP with unbounded callback errors is outside this theorem.

Wolfram exact rational example J=[[-2,10],[1/10,-3]], w=[10,1], sigma=1/4: mu_unweighted=8 (no admissible margin), mu_weighted=-1, K_w=4/5; `A^(-1)=[[28/41,40/41],[2/205,24/41]]`. Changing norm changes certificate, not physical trajectory.

## Theorem 2: sound affine symbolic cell
If `J(theta)=J0 + theta E` and a certified whole-slab parameter `theta in [a,b]`, each lognorm row expression and hence `mu_w(J(theta))` is convex in theta, giving
`mu_w(J(theta)) <= max(mu_w(J(a)),mu_w(J(b)))`.
One certified upper bound may be cached for a dyadic parameter cell, guarded by immutable source plus model/source/clock/norm/epoch and true whole-slab inclusion. Guards are fail-closed. Mere equality of time-slab endpoint labels is insufficient for nonlinear parameter paths: `theta(t)=.02+.25 sin^2(pi*t)` has both endpoints in an early cell but an interior excursion. The finite state record is a *sound itinerary of a declared affine clock*, not a general symbolic conjugacy, nor universal bisimulation.

## Scientific status
Wolfram symbolic rational identity: checked. Rust 1.94.1 standalone tests: 8 pass; strict Clippy/fmt pass. Independent Python Fraction-from-binary64: 560 inequalities/40 cases pass. 256 commuting synthetic trajectories checked against matrix exponential: max achieved endpoint absolute error 2.0699365877591136e-8, identical final states between certificate strategies. This is NOT a rigorous global IVP error certificate.

**P0 pending:** public standalone `AffineBlock::certify` accepts a caller-provided `mu_upper`, while physical source/clock/epoch snapshots are caller-provided tokens. Production must replace with opaque provenance-verified certificate, live source ownership and bounded JVP/oracle errors. No performance or accuracy claim for actual BASS/REI physical rates or native adaptive integration.

## Literature boundary
Saad & Schultz, GMRES (1986) DOI 10.1137/0907058; Niesen & Wright, phi-action Krylov arXiv:0907.4631; Jawecki, defect bounds for phi functions DOI 10.1007/s11075-021-01190-x; Henzinger, Symbolic Approach to Hybrid Systems DOI 10.1007/3-540-45657-0_4; Sankaranarayanan & Tiwari, Relational Abstractions DOI 10.1007/978-3-642-22110-1_56. These support analytical approach, not code admission.
