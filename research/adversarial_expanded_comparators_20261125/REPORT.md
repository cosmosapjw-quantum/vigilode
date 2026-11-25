# Expanded Diffsol / Julia SciML adversarial benchmark
Date: 2026-11-25 KST. Measured Rust adapter commit 7b8ea57ec8321196254c7a40fc99220cc8bd38ae. Parent original 1032-row replay 7cbbede5a46afe016376203c8c79d3e1087a1706 remains intact. No production solver change.

## Coverage and actual completion
| Suite | fresh rows | full output success | failed |
|---|---:|---:|---:|
| canonical Rust (old 126 + new 51) |177|163|14|
| spatial Rust (old 378 + new 135) |513|513|0|
| full-scope Diffsol supplement |141|138|3|
| native Julia canonical |390|369|21|
| native Julia extended ODE/mass/event |369|361|8|
| Julia spatial shared-Rust callbacks |486|486|0|
| Total |2076|2030|46|

504 original Rust keys remeasured; 1572 supplemental rows beyond these, but full Diffsol also deliberately repeats some previously measured method/configuration aliases, so do NOT call 1572 entirely distinct new configurations. All 21 actual run invocations exited 0; failures remain explicit solver records. No process timeout or missing requested cell in this bounded expansion. Original reference arrays (14 canonical/spatial) equal prior values; all 16 Rust corpus executions VERIFIED_BUILD_SOURCE, no protocol violations. 8 related adapter tests passed; actual new arm RED failed on unregistered variant, GREEN same test passed; full workspace not tested. The newly added arms are native Diffsol TR-BDF2 and ESDIRK34 with dense/sparse Faer LU, and explicit Tsit45. Diffsol BDF is retained. Julia 26 new method constructors canonical, 13 representative ODE methods, 9 mass methods and 9 event methods extended, spatial 11 dense/explicit and 7 sparse choices. Exact installed Julia 1.12.7 manifest unchanged. Method availability is from installed source; not exhaustive every SciML algorithm, no artificial split/symplectic/IMEX reformulation.

## Important empirical findings
All timings diagnostic, same nominal rtol only unless otherwise noted. Different physical CPU pins with concurrent groups share the socket, not isolated interleaving. See PROTOCOL.md. At rtol1e-6:
- Diffsol sparse BDF/TR-BDF2/ESDIRK34 (ms), Brusselator n100 1.213742/5.223471/6.704138, 3D semilinear advdiff 5.317633/6.089841/4.188511, 3D sphere-photon 11.653830/11.167761/10.578268, periodic advdiff n256 .994299/2.466844/1.495137, Brusselator n400 7.670320/19.943532/23.609684. Different achieved errors, no uniform ranking. All 135 new Diffsol spatial rows completed. New canonical Diffsol 11 nonlinear-failure caps: six VDP, four Oregonator, one HIRES. All 42 Tsit45 canonical/spatial cells completed. Diffsol explicit per-problem wall cap, Julia explicit maxiters, different constraints.
- Julia canonical new methods: RadauIIA9 Robertson .05742ms error2.34693e-7, Rodas5 .07757ms error3.32047e-8, AutoVern7(Rodas5P) VDP .38636ms error1.40494e-6, Tsit5/Vern7 Brusselator n100 .48156/.69606ms errors2.83765e-7/4.68744e-8. Canonical references disputed, no qualified ranking. 21 MaxIters failures (18 explicit and 3 Rosenbrock32).
- Julia full: Robertson mass Rodas6P .207753ms error3.87593e-9, constraint4.44e-16; RadauIIA9 .077948ms error4.89505e-7. Julia QNDF periodic advdiff .622246ms error4.85525e-6, Brusselator n400 4.076395ms error1.68424e-4; RadauIIA9 n400 7.843186ms error2.98629e-6. PR RadauIIA9 .014517ms error1.02795e-2: successful retcode does NOT mean accuracy pass. Eight Julia full failures MaxIters, including TRBDF2 mass cases.
- Event: new Julia nine methods x3 tolerances x2 contracts 54/54 complete correct root count and actual stop; Diffsol four native methods x3 x2 23/24 complete, one Tsit45 nonterminal loose tol stop-time error. At 1e-6 terminal Diffsol Tsit45 .009227ms root error7.54836e-8; Julia Rodas5Pr .085511ms root error1.58087e-9. Different root error, not speed ranking. No tangent/DAE reset expansion.
- Julia spatial bridge all 486 rows; y0 checked against reference at each setup, 18 directional Jacobian checks max6.96e-11 relative, actual C ABI and sparse conversion overhead included. No assertion of fully native Julia source performance. Shared source is exactly the original Rust Model; no extra PDE discretization.
- At target observed common error <=1e-7, actual sampled points (no interpolation) on the two *undisputed* sphere-photon problems:
  2D best VigilODE dense .414923ms err9.70032e-8 (rtol1e-6); Diffsol Tsit45 .114186ms err2.28031e-9 (1e-8); Julia shared Rust Tsit5 .085081ms err1.22302e-8 (1e-6).
  3D best VigilODE banded dense 2.261040ms err5.68571e-8 (1e-6); Diffsol Tsit45 .310948ms err2.13490e-9 (1e-8); Julia shared Rust Tsit5 .240275ms err8.41668e-9 (1e-6).
  At target <=1e-8, 3D VigilODE banded dense 3.021887ms error1.17138e-9, Diffsol Tsit45 .310948ms error2.13490e-9, Julia shared Rust Tsit5 .240275ms error8.41668e-9. Same error ceiling, different measured error and timing environment; informative priority, not exact matched-achieved-error speedups.
- Original 12/14 reference disputes unchanged. New implicit Diffsol matched records: canonical 360 diagnostic-only disputed; spatial 936 total, 30 MATCHED (two undisputed photon models, original harness interpolation), 906 diagnostic-only. Explicit Tsit45 isolated protocols contain no VigilODE baseline, so no original-harness matched row for Tsit45. Do not conflate target-ceiling sampled comparison with exact interpolation. M0 still required.

## Decisions and blockers
Retain all original and added arms. No change to default solver, original failure records, or release qualification. Extra Julia methods in installed modules not individually benchmarked (e.g. every ROS/ESDIRK variant, every adaptive solver). No custom Diffsol tableau added: all built-in methods covered. External additional packages (ODEInterface, LSODA Julia) not installed/updated. Shared-socket timing, resource caps, disputed references, and source-interface differences prevent global ranking. See FOLLOWUP_ADDENDUM.md.
