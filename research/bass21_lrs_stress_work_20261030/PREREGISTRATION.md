# Prospective research node | BASS21 LRS polarized photon stress and anisotropic work
2026-10-30 KST. Registered before executing the 50-case angular stress atlas. The exploratory three-case derivation/preflight and existing PR132 experiments happened before this registration and are NOT presented as prospectively registered results.

## Immutable source and bounds
Parent: VigilODE Draft PR132 `f97b0df6e13e09a67db3050a0e8e925e2e009022`. Owner: `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`, 50 original full62 histories x 193 saved epochs. Nested BASS ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`; immutable PR132 ZIP SHA256 `ff17ce0c5883de8363898e83ebcb8b30322d35402d0cdbe4c678437493f6103a`.

### Physical question and claim ceiling
Compute photon **energy/stress response** of parent source-frozen coupled deltaI and Q Thomson transfer under metric ds² = -c²dt² + a²e^{-2b}(dx₁²+dx₂²) + a²e^{4b}dx₃², signature (-,+,+,+), normal-comoving cold electrons, x=ln(a/a_ref), I_0 baseline held fixed. Re-evaluate parent transfer on independent Gauss-Legendre half-sky rays, integrate six even Legendre moments delta J_l for l=0,2,4,6,8,10. Construct delta rho, delta P_perp, delta P_parallel and delta pi using normalized brightness `F=a⁴I/(a_ref⁴ I_ref)`. Q is a polarization degree and not added twice to energy tensor. Calculate all 50 x193 x6 source-conditional physical deltaJ_l values, plus 25 matched pairs x193 x6; 50/25 endpoint comparisons and stress-work history.

### Theorem to audit
For Bolometric anisotropic Liouville operator, dmu/dx = -3 b' mu(1-mu²); dln(aE)/dx=b'(1-3mu²). Hence
`d deltaJ0/dx = -2 b' deltaJ2`
without Thomson collision monopole source, and
`d deltaJ2/dx = b'*(-8 deltaJ0/5-10 deltaJ2/7+36 deltaJ4/35) - g*(9 deltaJ2/10+3 B_Q/40)`
with `g=c sigma_T n_e/H`, `B_Q=2 integral_0^1 Q(1-mu²)dmu`.
Mechanical work identity: `deltaJ0(x) = -2 integral_0^x deltaJ2 db` if initial deltaJ0=0. Stress tensor deltaT^a_b = diag(-deltaJ0,(deltaJ0-deltaJ2)/3,(deltaJ0-deltaJ2)/3,(deltaJ0+2deltaJ2)/3) times a physically specified reference intensity scale. Trace zero. This is an exact continuum condition, but the numerical budget uses saved-grid derivatives/Simpson only; no global BDF certificate.

### Experiment and falsifiers
Derive exact angular projection independently (Wolfram/symbolic). Re-solve full parent Volterra on original 50 source histories, using angular GL48 fixed point, and evaluate output on GL64 and GL96 for the six moments, plus source Q. Quantify angular disagreement, new moments, energy-work closure with stored b' by second-order nonuniform-grid derivative and sampled-grid Simpson, and the quadrupole differential budget on saved grid. Compare independent density/pressure directional integrals. Require no positivity violation and distinguish fail vs exploratory. No tolerance weakening, no new original geometry/chemistry or polarized IVPs, no new electron/Einstein feedback, no borrowed KM2E/CE1 event data, no observed CMB E/B/tau. Preserve original PR132 and all historical failures. Independent-author blind R6/C9 remains NOT_RUN unless actual native dispatch.

### Outputs and admission
Full source-inclusive self-checking ZIP with original nested source binary, runnable Python, actual 50x193x6 and 25x193x6 arrays, CSV, report, exact source hashes and numerical checks. Publish on isolated child Draft PR, never merge or force-push; binary backups to Library and Google Drive only with actual provider ACK/readback. Record **official DAG unchanged**: VigilODE 7/23, REI central 28/88, REI working 33/88, BASS CE1 9/15, successor 0/12. One new auxiliary scoped science dataset if calculations/physics checks pass; production HOLD, AUTO_NEXT_NODE=false after this bounded unit.
