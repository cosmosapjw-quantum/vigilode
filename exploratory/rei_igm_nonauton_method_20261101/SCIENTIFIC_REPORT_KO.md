# VigilODE → Original REI IGM five-clock: exact nonautonomous f_t and honest method selection

2026-11-01 KST | Source-scoped real science: DERIVED + NUMERICALLY_CHECKED + IMPLEMENTATION_VERIFIED_LOCAL | canonical HOLD.

## Source and authority
- Original owner physics: original REI Bianchi IGM, 46,080 physical cells and three global neutral moments / four photon groups; PR #342 exact source at https://github.com/cosmosapjw-quantum/rei_bianchi/commit/cd136dc3c7d8b9bf2523a0182b726013f0bcfb04.
- Original reduced 15-state photoionization-only client: VigilODE PR #175 HEAD d99c3f06d46e233a7848e5b4562b4a59e2b5d900, five piecewise-linear photon forcing clock knots and 11 independent SciPy Radau/DOP853 source-reference rows. The five-clock interpolation is an additional astrophysical model assumption; original 26-event full thermal/OTS is NOT solved here.
- Parent: VigilODE PR #180 HEAD 8cad5fdbcbde883be61f50ffdb51b3212aa2eab9, with locally native-compiling source and original frozen source/reference. All original owner files remain unchanged.
- Prospective same-source experiment commit 89f8933a0cac07f83368b7ce47a0ab41b74060e5 was pushed BEFORE running new Rust trials.
- Source values and native original physically reduced RHS/JVP are reused verbatim; independent Python stdlib audit reconstructs the exact original parent example file after removing only the new optional dispatcher and checks default original two-arm JSON replay semantic equality. Original unchanged baseline SHA256 7efae27f1ef75afb886b41a20940631e86d7e12d0cc9d09f14e5d9e14dac3136.

## Derivation and identified defect
Original time-dependent client used autonomous=true, partial_t=None even for five-clock time-varying source. Its existing matrix-free RODAS5P therefore used f_t=0 in a nonautonomous stage. That does NOT imply original published source states violated their error budget, but can change truncation estimates, accepted-step economy and work.

For original neutral ratios n_s>0 and group photon current J_g, group species fractions w_gs and subgrid escape fraction e_g, use
 D_g=e_g+sum_s w_gs n_s,  P_s=sum_g J_g w_gs n_s/D_g,  E_g=J_g e_g/D_g.
Holding state fixed:
 D'_g=e'_g+sum_s w'_gs n_s,
 P'_s=sum_g n_s[(J'_g w_gs+J_g w'_gs)/D_g-J_g w_gs D'_g/D_g^2],
 E'_g=(J'_g e_g+J_g e'_g)/D_g-J_g e_g D'_g/D_g^2.
Primes are derivatives with respect to original dimensionless u, so the original h_seconds coefficient is retained once when mapping into the original 15-state ODE. Symbolic Wolfram checks returned exactly (0,0) for the species derivative and sum-of-photon-share conservation. All four original intervals use the *same five knots*; each native RODAS5P segment restarts on a knot, with its chosen one-sided exact interpolation slopes. No invented sixth source point or modified original RHS/JVP.

## Actually executed original-source result
Native Cargo test for the original example plus two new derivative/negative-path tests: 6/6 PASS. One initial finite-difference check FAILED at 1e-5 segment fraction with normalized discrepancy 2.807956e-6, diagnosed as binary64 subtraction cancellation at escaped-photon component 6. Independent 1e-3/1e-4/1e-5 sweeps over all four original source clock segments showed the larger 1e-3 fraction conserves smooth-segment finite-difference accuracy and reduces diagnostic error. Test differentiation step changed; original 2e-6 test tolerance, physical source and native integrator untouched. Final 6/6 PASS.

Original Radau reference: 11 requested times, 15 physical quantities each. Eight preregistered native experiments (2 methods × source-frozen/time-varying × rtol 2e-9 or 2e-11) all meet original 5e-10 source-error budget and 2e-11 physical invariant budget.

At rtol 2e-11, time-varying source and identical knot splitting:

| route | max abs vs Radau (15 states x 11 times) | max H/He/photon/binding invariant | accepted/rejected | RHS/JVP | f_t calls |
|---|---|---|---|---|---|
| Corrected 8-stage matrix-free RODAS5P (analytic f_t) | 3.3306690738754696e-16 | 3.8834674583231046e-16 | 17 / 0 | 171 / 897 | 17 |
| RHS-only DOPRI5 | 6.661338147750939e-16 | 7.584172338152823e-16 | 17 / 0 | 106 / 0 | 0 |
| Historical forced RHS misdeclared autonomous, NOW on SAME knots (post-hoc adversarial control) | 4.621671968740859e-14 | 7.771561172376096e-16 | 215 / 12 | 1816 / 10442 | 227 zero-valued calls |

The last route numerically meets the source-state budget, but its autonomous declaration is mathematically INVALID, and it is not one of eight preregistered science-method candidates. Because the negative control uses the exact same knot boundaries/output/reference/rtol, the 215→17 accepted-step reduction is attributable to the corrected time-dependence declaration for this source-conditioned problem, rather than solely to knot segmentation. The ratio 215/17=12.647058823529411 is a step/work reduction in this *one 15-state source system*, NOT a general stiff solver performance guarantee.

All nine source trajectories were audited independently in Python stdlib: 9×11×15 finite state values, H and He conserved population relations, group photon emission=absorption+escape ledger, ionization binding energy identity, source- and clock-time parity, full vector error versus independent Radau, and complete work counters. Independent physical audit: PASS; source original client default output semantic replay and byte-exact original RHS/JVP after stripping optional dispatcher: PASS. New arithmetic outputs: REAL_SOURCE_NONAUTON_METHODS.json, independent audit INDEPENDENT_AUDIT.json, LOCAL_TESTS.log and reproducible audit_source_independent.py. All numeric paths remain subject to Rust binary64 and original source/JVP trust; none is formal MPFR proof.

## Adversarial verdict and drift watchdog
1. This is a **corrected mathematical model declaration plus actual source computation**, not a fourth photon atlas, new astrophysical source or governance-only progress.
2. The source is the original REI source-derived aggregate 15-state photoionization model. It has no coupled 26-event recombination/thermal energy/OTS roots, no full 46k simultaneous 26-event IVP, and no Hummer–Seaton event partitions. The full REI D_h Jacobian/uniform remainder, canonical tick161 and observational/CMB claim remain NOT_RUN/NOT_ACCEPTED.
3. Neither global order≥4.8 nor generic original G0 speedup≥1.15 is certified. The two predeclared tolerances produced essentially roundoff-limited endpoints and the same accepted-step count; these are not a broad matched-error work-precision frontier. Local wall times are explicitly DIAGNOSTIC_ONLY.
4. DOPRI5 is most economical here in exact RHS/JVP work because this source-reduced model is effectively nonstiff on these segments; do not generalize to the full thermal/OTS event problem. PEXPRB54S4 native driver is autonomous-only in this version, thus was correctly excluded for actual time-varying source.
5. One same-author adversarial negative-control pass and one independently implemented Python state/physics audit were conducted. Different-author blind referee NOT_RUN, no subagents.
6. The original unmodified PR180 source and archived baseline remain intact. Do not change default production routing or original canonical acceptance. The next real REI application should target source-owned complete 26-event four-site H/He/thermal integration, not another photoionization-only atlas.

## Evidence status and exact progress
- R5 constructive photon-clock derivative: DERIVED, Wolfram CHECKED symbolic.
- R7 original source numerical comparison: NUMERICALLY_CHECKED (8/8 preregistered, one negative control).
- C5 scoped opt-in Rust integration and correct f_t: IMPLEMENTED.
- C6 Rust tests: 6/6 PASS_LOCAL; replayable source/independent Python audit PASS_LOCAL.
- R6/C9 same-author adversarial: PASS_SCOPED, DIFFERENT_AUTHOR_NOT_RUN.
- VigilODE original 7/23_REPORTED; successor 0/12; REI central 28/88, working 33/88, Fast 4/7; canonical delta=0; original accepted tick160/131072; production HOLD, no merge/force push/tolerance relaxation/default change/auto next.
- Paper context, not direct theorem for this source: discontinuous Rosenbrock one-sided integration https://arxiv.org/abs/1211.3933 and stiff time-dependent forcing/order reduction https://arxiv.org/abs/2201.07940. The actual result above is measured from the source/code, not claimed by these papers.

## Exact CI configuration local release replay
Verified on connected Linux host with original frozen source, first physics, existing solver implementation unchanged:
- cargo test --locked --release -p rodas5p-integrators --no-default-features --example rei_igm_five_clock_source -- --nocapture: **6/6 PASS**, including original native test.
- cargo run --locked --release -q -p rodas5p-integrators --no-default-features --example rei_igm_five_clock_source -- --nonauton-study: **8/8 preregistered source method runs PASS**, plus separate labeled invalid-autonomy posthoc control.
- Default original client release replay **byte-identical on same host** with original PR180 archived JSON; independent Python standard-library audit **9/9 real source trajectories PASS** including each of 11 times ×15 state fields, original photon/nuclei/binding, error and full work counters.
- Original source JSON SHA256: 9a10ccaab0dbdb9b508021f115d668973920fb6699dc54e4c3be61f17ee10eb6; original independent Radau source-reference SHA256: 5a609e29d8c17ffece1a16c8c396a4619183761f9facb5452f59b706f70aea20.
- Release real-source JSON SHA256: eac83dc6d94a9597542564613633a5128292a50620eb14a4e3af372a063bc1f3. Release independent audit SHA256: a67e899b398f04b8052c785871257721188ef8a1e6f2fb41c7cacff884bbbbf2.
- New scoped GitHub Actions science workflow is included. New GitHub HEAD CI still requires actual run status and artifact readback; no success claimed from workflow creation. Existing historical authority CI remains strict and unchanged. No original full 26-event/canonical tick161 promotion.
