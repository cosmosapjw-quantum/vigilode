# Constructive Bianchi-I redshift-surface optical-depth calculation

Metric with signature (-,+,+,+): `ds²=-c²dt²+a(t)²exp(-2b(t))(dX²+dY²)+a(t)²exp(4b(t))dZ²`. Define `x=ln(a/a_ref)` (BASS stored `a_ref=1/21`), observer x_o=ln 7 at saved source z_FRW=2; no electron bulk velocity.

Conserved covariant spatial photon momenta imply at fixed observer local cosine mu_o and db=b(x)-b(x_o):
`R(x,mu_o)²=(1-mu_o²)exp(2db)+mu_o² exp(-4db)`,
`E(x)/E_o=exp(x_o-x)R(x,mu_o)`,
`mu(x)=mu_o exp(-2db)/R`.
Choose an observer-defined photon redshift z_s by root
`F(x_em;mu_o,z_s)=x_o-x_em+ln(R(x_em,mu_o))-ln((1+z_s)/(1+z_o))=0`.
Source roots z_s = 4,6,10,14 all remain inside 193-state BASS source history.

With source-normal comoving electrons, `g(x)=c sigma_T n_e(x)/H(x)` (original source `gamma_T_per_H`). The Thomson optical-depth past emission surface is
`tau(mu_o,z_s)=int_{x_em(mu_o,z_s)}^{x_o} g(x) dx = tau_T(x_o)-tau_T(x_em(mu_o,z_s))`.
At **fixed common x_em**, the integral is direction-independent. Therefore nonzero angular multipoles at fixed photon z_s are due to **geometry-induced boundary variation**, not artificially direction-dependent electron density. `tau_l=(2l+1)/2 int_-1^1 tau(mu) P_l(mu)dmu`. First-order small db gives `tau_2≈2 g(x_iso)[b(x_iso)-b_o]`; normal signs retained.

Physical dimensions: g=d tau/dx dimensionless, sigma_T m², n_e m^-3, c m/s and H s^-1. b dimensionless. No new source IC/BC: source saved BASS Y/H/temperature/UV, x and tau all frozen.

Validation: independent Wolfram identity `d(ln R)/d(db)=1-3mu(x)²` exact zero residual. Source ZIP and 50 case checksums verified. Brent+PCHIP root/optical depth vs TOMS748+piecewise b+independent g quadrature (600 common rows): max abs difference 1.792218141533075e-7; x interpolation max difference 3.884606089632481e-6. These are **representation checks**, not bounds on the original IVP. 15/15 controlled physical tests include reflection, zero shear, equal-time null, axis, positive g and paired input identity.

Mean matched-pair redshift surface contrast at z_s=10 decomposes algebraically `-0.0010656089752203526 = -0.00011141510955137989 + (-0.0009541938656689726)`, where fixed-x and direction-boundary terms are an **additive identity**, not isolated physical causal effects.

Scientific literature reuse: Fleury, Pitrou and Uzan, Phys Rev D 91, 043511 (2015), DOI 10.1103/PhysRevD.91.043511 (Bianchi I light propagation); Challinor, Phys Rev D 62, 043004 (2000), DOI 10.1103/PhysRevD.62.043004 (polarized transport), not a source owner; Norman et al., Astrophys J Suppl Ser 216, 16 (2015), DOI 10.1088/0067-0049/216/1/16 (coupled ionization/RT perspective, different assumptions).