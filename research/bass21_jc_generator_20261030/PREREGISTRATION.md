# Prospective source-owned J/c coupled generator atlas | 2026-10-30

## Experiment registered before the NEW full 50-case generator evaluation

Parent: VigilODE Draft PR #134 immutable HEAD `21373cf012f348ad6911580461ab466016968433`; original BASS source owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; immutable original archive SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. Parent canonical source-full ZIP SHA-256 `d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16`. Baseline PR133 moment array and PR134 c2..c10 are previous results, not prospective outputs.

## Physics and frozen boundaries

Metric (-,+,+,+), axisymmetric Bianchi I, x=ln(a/a_ref), cold electron rest-frame elastic Thomson, g=c sigma_T n_e/H, x-dependent b'=db/dx, Q=I_meridional-I_azimuthal, even parity. All 50 original owner histories x193, 62-state exact inputs; original geometry, unpolarized J0..J8, electron/chemistry and optical clocks frozen, no original or new IVPs. The existing PR132 coupled second-kind Volterra deltaI/Q solve may be used on the same immutable source and is not a new standalone initial-value solve. No cross-source KM2G/KM2F/CE1 input substitution.

Construct exact-rational Galerkin shear matrices A^J_lm, A^c_lm for Legendre J_l=<I P_l> and screen c_l=<Q P_l^2>/N_l. Source-owned collision in the perturbation sector: C_deltaJ0=0, C_deltaJ2=-g(9 deltaJ2/10+sqrt(6) E2/10), C_deltaJl=-g deltaJl for l>=4, E2=(12/(5sqrt(6))) c2, C_c2=-g[(J2_orig+deltaJ2)/4+2 c2/5], C_c_l=-g c_l for l>=4. Higher even m=l+2 must be retained; truncation flux identified explicitly. Source frozen x and g arrays remain unchanged.

## Prospective new observables

Generate from actual same-source coupled deltaI/Q field additional screen c12,c14 and intensity deltaJ12 (and deltaJ14 if needed), source-qualified instantaneous (not IVP integrated) shear, Thomson and total d/dx generators for deltaJ_l (l=0,2,...10) and c_l (l=2,4,...12), stored at all 50*193 epochs. Measure pair differences, source-normalized extrema, hierarchy-flux contributions, frozen-generator vs sampled-grid derivatives as diagnostics. Claim ceiling: exact local generator of existing source-frozen model and new higher moments, NOT fully self-consistent evolving source-native BASS E_l or Einstein/chemistry feedback or observed CMB E/B. No new official DAG node without separate acceptance.

## Precommitted falsifiers / acceptance

Exact rational low-rank coefficients independently CAS checked; original source 50/50 SHA identities; PR133 deltaJ_0..10 and PR134 c2..10 on same GL/source agree within 1e-11 absolute; collision matrix eigenvalues -1,-3/10; angular GL64/GL96 new high modes agree within 1e-10 absolute; coupled consistency |deltaJ0' + 2 b' deltaJ2| sampled residual reported, not misidentified as global BDF convergence; zero-Q limit, zero-shear limit, parity and null initial c, positive photon coherency, no source changes. Numerical deviations retained, no tolerance weakening to hide discrepancies. Same-author adversarial review marked NOT DISTINCT AUTHOR. No force push, main merge, production promotion, global authority gate suppression; official 7/23, central REI 28/88, working REI 33/88, BASS CE1 9/15, successor admitted 0/12; production HOLD.

Publication: child Draft PR only after local results and checks, source-inclusive ZIP in Library and Drive if available; Dropbox only with actual provider receipt.
