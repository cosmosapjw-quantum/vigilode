# VigilODE PR #183 | Original REI F05 verified native Rust interval MPRK22 corrector

## Source and scope

2026-11-02. Preregistered at 49e107b3a32c87642222e7947697c1734f6505f3, based on immutable VigilODE predecessor df3bc5752970e0c0840b596c76175b4143a8edd8. Original REI source-owner HEAD fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120, original 26-event sealed kernel SHA256 579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644, accepted tick160 parent cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01, source tar a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b.

Original REI PR #368 exact derived source branch f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa was read only. This native code does not replace the source owner 26-event thermal/OTS IVP; it certifies the original 2x2 H and 3x3 He MPRK22(1) FINAL population correctors on the three full/half1/half2 source maps, 46,080 real cells per map. Source interval binary contains 70 f64 per cell: five parent lower/upper, 25 original directed flux lower/upper, five predictor denominator lower/upper. 77,414,472 source bytes, SHA256 95b2cdbfeb5777c2b893910f1663c271d0ead673629180ece068bf3e0a6fb001. All source and worker files remain unchanged.

## R5 mathematical result

For frozen-stage positive source transfer F_{ij} >= 0, denominator sigma_j > 0 and timestep h > 0:

\[
 A_{ij}=\begin{cases}
 1+h\sum_{k\ne j}F_{kj}/\sigma_j, & i=j,\\
 -hF_{ij}/\sigma_j,&i\ne j.
 \end{cases}
\]

Every column sums to one and is strictly diagonally dominant. Therefore A is a nonsingular M-matrix, A^{-1} >= 0, 1^T A^{-1}=1^T and ||A^{-1}||_1=1. Let p be parent and delta=y-p. A delta=-(A-I)p. For any finite trial increment z with rigorously outward residual interval r=-(A-I)p-A z, the exact solution has |delta_i-z_i| <= rho, with rho=sum_j sup |r_j| computed upward. The point trial need NOT be exact or converged. Further physical intersections use y_i>=p_i/A_ii and y_i<=sum_j p_j; conservation implies sum_j delta_j=0 in each H/He block. All operations are implemented by the ORIGINAL VigilODE rodas5p_core::directed arithmetic, including subnormal behavior. Inactive cross-element source graph entries are exact zeros, although the source interval representation gives [-minsub,+minsub]; guards restore exact topology ONLY for these documented structural zeros. Any true cross-element flux is rejected.

## Actual scientific data

The native output contains original 46,080 x 3 x 5 source-corrector lower/upper population increments. Raw size 11,059,216 bytes, SHA256 d9122688c286f19650b2ba78b778d5df2356791b6778d105ad3616945dbfd4cc. The 138,240 genuine source-cell correctors comprise 276,480 small verified H/He linear systems. Release native core timing 0.697 seconds for this linear-substep computation ONLY, not an achieved-error matched full-IVP speedup.

Directed total electron count change (electrons):

| original map | native source-box delta Ne interval |
|---|---|
| full | [-1.566121952059835e60, -1.5534947026944666e60] |
| half1 | [-7.831016768489932e59, -7.767879284799048e59] |
| half2 | [-7.830339177309794e59, -7.767003422835449e59] |

All upper bounds are strictly negative: the original source interval MPRK final corrector robustly decreases material electron counts for this source step. This does NOT certify net electron change for the entire exact continuous-time uncertain IGM evolution, but is stronger than the earlier frozen point-only diagnostic. The interval has actual physical units of electrons.

Original native full-corrector normalized *increment tube widths*, not local error bounds: x_HII maximum 6.444710269581756e-6; x_HeII and x_HeIII maximum 4.617046933966123e-6. It is mathematically invalid to use those widths as the full owner local 2e-4 defect check.

## Verification and same-author hostile review

- Actual original 3-map source owner replay: all 14/14 original FINAL interval set ledgers PASS, zero source-table events, original common-parent source-discrete gate remains unchanged. This is an ORIGINAL OWNER ledger check, NOT a native Rust 14-ledger implementation.
- Cargo offline/locked native unit tests: 4/4 PASS including positive conservation, structural zero -/+minsub and genuine negative flux fail-closed.
- Rust 138,240 native interval corrector rows with no nonfinite/reversed intervals; separate stdlib 120-digit Decimal 24-cell (120-component) trial checks all enclosed. Decimal is high precision sampling, not a separate formal rational theorem.
- Independent full/twohalf source owner HII, HeII, HeIII increment reference intervals: 276,480/276,480 native overlap, no failures. Overlap does not imply equality of sets. Original owner CSV SHA256 11df439390209bd48742ca9f4303c8743c9e14900289805e13d1ed489450a5dd.
- Previously published fully native 46,080-cell x3 full/half1/half2 point-worker increments: 691,200/691,200 lie inside current verified interval corrector outputs. This does not port the full original uncertainty.
- Two complete export/native/audit research runs returned byte-identical source binary, native interval binary and independent audit JSON. Normal and Python -O audits byte-identical. Source hash tampering, false official tick161 and optimized Python -O state tampering all rejected.

The hostile pass found and repaired (1) original outward zero-flux columns being incorrectly treated as physical cross-element flux and (2) independent Python asserts being optimized away by Python -O. Both have targeted negative tests. No native subagent tool was available and no distinct-author blind referee is claimed. Source REI #370 exact seven-pin V2SC1 research worker and #371 sampled continuous RHS defect are separately classified; neither changes this result's fixed source or certifies full continuum IVP. Positive HeII local f64 underflow remains unresolved.

## Decision, drift watchdog, next handoff

ADMIT for specified use only: source-authentic native interval MPRK corrector, independent sampled source checking, strict negative original-corrector electron sign. HOLD: original full interval H/He/OTS thermal IVP, Alexander SDIRK2 interval thermal roots, all 14 ledgers in native Rust, continuum positive tube / nonlinear global three-moment Volterra stability, original HeII trace, full-IVP matched achieved-error speedup, official tick161.

V1 historical logT failure 2.1245050576368385e-4 > 2e-4 remains FAIL. V2 research source-discrete candidate is separate. Canonical VigilODE 7/23, successor 0/12; REI central 28/88, working 33/88, fast 4/7. Scientific delta=0, original official accepted tick160/131072, PRODUCTION=HOLD. No merge, force push, main/default change, original REI worker/history replacement, or threshold relaxation.

NEXT ONE REAL NODE: extend present directed Rust interval corrector by original stage-dependent H/He/OTS energy enclosures and Alexander SDIRK2 positive thermal roots, including all 14 original final source set ledgers on the three maps. Reuse original source-owner event-box, REI native Rust physics, VigilODE directed arithmetic/RODAS fallback; do not substitute a nonpositive generic solver, another photon atlas or a source-only timing proxy.
