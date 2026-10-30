# BASS21 LRS spin-2 high-rank boundary-memory atlas (50×193, no new source IVP)

## Scope and authoritative definitions

Original source: `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`, original source archive within parent PR143, child VigilODE PR145 HEAD `6bb55eed42e01db9c3715d17c858d8e5632d6a38`, corrected complete archive SHA256 `c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564` (60,191,082 bytes). Parent PR143 full SHA256 `a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee`. No replacement of owner photons, electrons, Bianchi background, geometry or clock. No new original IVP. Source case/clock grid is x=ln(a/a_ref), 50 original cases, 193 x epochs. Signature (-,+,+,+). Source x-end 1.9459101490553132, source g=c*sigma_T*n_e/H>=0, tau(x0)=0 increasing.

For original transported-axis LRS Q and U=0, plus-mode spin E is `E_l0+=k_l c_l` with `k_l²=4 pi [(l+2)!/((2l+1)(l-2)!)]` for even l>=2. The fixed-screen shear Galerkin operator is `A_E[l,m]=k_l A_c[l,m]/k_m`, with exact all-rank nearest-neighbour coefficient `A_c[L,L+2]=3(L-1)(L+3)(L+4)/[(2L+3)(2L+5)]`. The supplied PR145 high-rank source flux is `F_L=b'(x)A_E[L,L+2]E_(L+2)`, L=16,18,20,22. This input formula was checked against ORIGINAL parent b'(x), not a regenerated synthetic b'.

## Causal post-processing test functional (definition, not solution claim)

Define for each L and original source case the signed and unsigned damping-weighted boundary-source integrals:

    M_L(x) = exp(-tau(x))*integral_{x0}^x exp(tau(s)) F_L(s) ds
    G_L(x) = exp(-tau(x))*integral_{x0}^x exp(tau(s)) |F_L(s)| ds.

The intended kernel is the scalar original-Thomson extinction factor, valid as an auxiliary TEST projection for L>=4 and source-frozen tau. It is not the full coupled matrix propagator of the spin-2 E hierarchy. Differentiating formally gives `M_L'(x)+g(x) M_L(x)=F_L(x)` with `M_L(x0)=0`, but here M_L is calculated **directly by quadrature on existing saved arrays**, not by evolving any owner photon/electron/Einstein IVP. This is a separate response functional to an *omitted input term*, NOT the actual difference of full and truncated solutions. The exact inequality `|M_L| <= G_L` holds for positive extinction kernel, and numerically it does on all 38,600 points.

Numerical method: evaluate the integral by composite source-grid trapezoid on `exp(tau)F`, then multiply by `exp(-tau(x))`; an independently coded one-step decay recurrence gives the same discrete functional to roundoff (max 2.262e-26). Also check independent SciPy cumulative Simpson quadrature as a discretization sensitivity only, not a time/BDF error certificate. No change to original source or its fixed 1e-10 angular precision gate.

## Actual new physical outputs (source-conditioned)

New original-source diagnostic samples: 50 * 193 * 4 = **38,600 signed M**, and **38,600 unsigned G** = **77,200 newly evaluated physical source-functional values**. Twenty-five genuine original `s04_mXX_rYY` vs `s00_mXX_rYY` matched pairs; 50 endpoint rows; 193 ensemble time rows. The full NPZ and two plots are included. S04 higher E levels were resolved against angular GL64/96 in the parent, but S00 E_l at l>=12 were not; S00 high-l memory values must therefore not be interpreted as physical detection.

Max |M_L| for L 16/18/20/22: [2.9404924207515054e-11, 6.350723801045897e-12, 1.4911158937392245e-12, 3.6204574554767766e-13]. Mean signed s04-s00 terminal difference over 25 pairs: [9.60258944845742e-12, -1.5894846646656712e-12, 3.0861784598816623e-13, -6.597847377531762e-14]. Max |M_L| is a bounded source-specific diagnostic, not a rigorous upper bound on actual finite closure errors. The alternating signs reflect original parent source, not generated random data.

## Interpretation/gates

Derived: all-rank LRS spin2 coefficient and positive causal envelope |M|<=G. Verified numerically on actually restored original saved BASS source: cross-owner flux b'*A_E*E max gap 6.46235e-27, actual recurrence max gap 2.26182e-26, 19/19 tests. The Simpson-vs-trapezoid global max difference 3.13567e-16 reflects only postprocessing quadrature. Existing 50-case source BDF time accuracy, independent arbitrary-screen rotation, fully general observer boost/PSTF, full matrix transfer, observed CMB E/B, and real coupled E_l/J_l evolution remain NOT certified.

Published primary literature: Anthony Challinor, *Microwave background polarization in cosmological models*, Phys Rev D 62, 043004 (2000), DOI:10.1103/PhysRevD.62.043004 (general covariant polarization tensor, but explicit multipole hierarchy almost-FRW); Andrew Pontzen and Anthony Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387-1398 (2007), DOI:10.1111/j.1365-2966.2007.12221.x (nearly-FRW homogeneous Bianchi; not exact arbitrary strong-shear); Cyril Pitrou, *The radiative transfer at second order: a full treatment of the Boltzmann equation with polarization*, Classical and Quantum Gravity 26, 065006 (2009), DOI:10.1088/0264-9381/26/6/065006 (second-order perturbative transport).

Blind-style self adversarial audit: screen-gauge transformations do not create physical B modes; no total truncation error claim; source b' and tau checked by exact array identity and positivity; no independent-author R6/C9; global CI not green. One negative source mutation and one negative decay-sign test in test set. No strong universal finite-closure no-go asserted.

Official progress unchanged: VigilODE 7/23, REI central 28/88, REI working 33/88, BASS CE1 9/15, admitted successor 0/12, official increment 0. New scoped source-functional dataset locally complete, not an admitted DAG node. Production HOLD; NO_MORE_IVPS; no main merge/force-push/production promotion. AUTO_NEXT_NODE false.

## Registration and archival scope
This new secondary integral diagnostic was selected and executed in the same turn after checking restored original inputs. It was not prospectively Git-preregistered; do not reuse the PR145 preregistration receipt as prospective registration of this newly defined memory functional. Source identity and original PR145 performance are independent evidence.
