# Source chemical Jacobian and quadratic remainder certificates

**Unit:** `V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028`  
**Root scientific decision:** `PASS_SCOPED` in `ADMISSION.json`  
**Scientific allocation:** `CLOSED_NO_REPLAY`; reuse the preserved evidence by default.  
**Central semantic completion:** 28/88, this unit delta0; full V8 remains open.

## What this package establishes

For the pinned BASS source `cc602e209e7deeb511ab2a1f3b2268d55206c43c` and binding `8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16`, the six charged coordinates are HII, HeII, HeIII, LiII, LiIII, LiIV (reduced30 indices23..28). The exact neutral totals are `T_H=9007199254740991/9007199254740992`, `T_He=T_Li=1`.

The real-arithmetic source restriction is

\[
F(z)=b+Pz+(e^Tz)Qz,\qquad J(z)=P+(e^Tz)Q+(Qz)e^T.
\]

The feedback outer product is required. For frozen supplied coefficients,

\[
F(z+d)-F(z)-J(z)d=(e^Td)Qd,\quad
D^2F[d,h]=(e^Td)Qh+(e^Th)Qd.
\]

`jacobian_extrema` returns all36 scalar Jacobian entry intervals and72 attaining state/rate witnesses over the physical product of H/He/Li simplexes and the declared independent12-rate box. State affinity at fixed rates gives24 state vertices; rate affinity at each vertex permits exact endpoint-sign optimization. Entry extrema need not be attained simultaneously by a single matrix. Shared temperature, UV and Hubble factors can make the actual correlated rate family narrower.

`remainder_bound` reuses the prior fixed physical33 Gram/inverse and exact PSD/sqrt machinery. It bounds the named chemical polynomial coefficient operator Qbar, giving `||R(d)||_G <= Gamma ||d||_G^2` and a Hessian factor2Gamma. Qbar is not a full Jacobian with omitted blocks set to zero. The implemented Gamma is for fixed supplied rates; an interval-rate uniform coefficient bound is a separate next obligation. A B6 carrier different from the exact chemical J additionally needs a verified `deltaJ*rho` term.

## Observed validation

The first authorized suite passed10/10 groups with zero failures/errors. A separately assembled nine-state conservative flux model independently checks F/J and all72 witnesses. An8-rate-corner subset with24 state vertices gives192 independent derivative evaluations. The exact hydrogen-only omitted-feedback discrepancy is `-15/7`. Signed remainder, Hessian polarization, conservation, domain boundaries, zero Q, metric rescaling, serialization and scope refusals pass.

With positive unit element weights and unit physical scales, `sigma^2=495/2`. For an H-only coefficient Q with kappa3/2, `sQ=9/4`, so the full-positive-electron sufficient Gamma radicand is `4455/8`; a rational upper square-root enclosure is returned. The separate restricted H-direction sharp squared coefficient is `297/8`, and is not equated with the full-positive-electron bound. Uniform scales×2 rescale Gamma×2 in the inherited dyadic enclosure.

One frozen Wolfram invocation returned24/24 True. `results/wolfram_first_raw.json` preserves the complete received response; the provider abbreviates long all-zero residue arrays, while all named flags and `all_pass` remain present. The saved-JSON-only SVG shows the deliberate H feedback omission and degree-two remainder scaling. The PNG and full original source snapshots are retained in the complete SOURCE/checkpoint archives.

Measured local CPU was3.040031s and wall3.089660323s. Wolfram tool wall was8.983s; remote CPU is unknown/null. No physical owner RHS/Jacobian/ODE/NCP/native-norm/lift/build call, execution repair or old campaign replay occurred. Packaging costs are separate.

## Source coefficients and provenance

The supplied rates are `gamma=Gamma_photo/H` and `kappa=n_b*alpha_rec/H`; their actual source/clock/geometry enclosure is external. Synthetic weights and rate inputs are labeled diagnostic. The optional inventory witness statically reconstructs finite source scalar preparation under declared IEEE binary64 RN/fsum assumptions, preserving hex traces. It is `SOURCE_RECONSTRUCTED_UNDER_RN`, not an observation of the original owner-prepared array. The original source_manifest body is not recovered. Owner modules are copied as pinned source data and never imported or initialized.

`deps/prior_matrix` is byte-identical reused exact matrix code and dependencies. `deps/source_bytes` contains the five runtime-read source/data files. Their identity manifests are included. The source intake additionally preserves the selected pinned source bodies and provenance. No new norm or native lift is implemented.

## Reproduction and archival execution

Start with `ADMISSION.json`, `reviews/INDEPENDENT_REVIEW.json`, `results/SOURCE_CONTROL_RESULT_SUMMARY.json`, `theory/THEORY_KO.md` and `code/IMPLEMENTATION_MAP.md`. The code depends on the Python standard library; the figure renderer additionally used matplotlib. No owner/model runtime is needed. JSON rationals use numerator/denominator string dictionaries.

The following are the exact historical runner commands, recorded for reproducibility. They are not instructions to repeat a closed allocation:

```bash
python -B code/run_bounded.py suite --go SCIENCE_EXECUTION_GO.json
python -B code/run_bounded.py figure --go SCIENCE_EXECUTION_GO.json
```

The saved invocation markers and first results intentionally prevent silent overwrite/replay. Any future execution needs a genuinely new source/contract/budget decision or an explicitly authorized targeted repair. Reuse the current source and observed results for unchanged dependencies. The prior test suites are not part of this suite and were not replayed.

`code/preexecution_original` preserves the original authored WL/proposal containing a corrected preexecution syntax typo. Those are historical evidence; only current `code/EXACT_CHECKS.wl` was evaluated. First-source manifests remain in `results/*_first_source/source_manifest.json`; the complete archives retain the redundant snapshot bodies. The original proposal's NOT_RUN mutable-budget hash is historical; the current actual budget legitimately differs, while the frozen budget and science code/theory hashes remain unchanged. The execution-time result summary's pending-admission label is also historical; `ADMISSION.json` supplies the current PASS_SCOPED decision.

## Remaining physical obligations

Actual effective-rate enclosures, source arithmetic/evaluation/query rounding and the original owner coefficient provenance are not supplied by this package. Chemical geometry columns `-Fchem*(grad log H)^T`, geometry rows, Thomson/tau rows and time variation are `OPEN_NOT_ZERO`. Full30 Jacobian/lognorm, actual accepted capsule/tube inclusion, initial/inherited/source/residual/continuation bounds, continuous-angle positivity and actual ODE solution enclosure remain separate. Do not promote the chemical partial block into a full model provider.

The maintained ceilings are query_rounding_bound:null, actual_query_rounding_bound:null, whole_domain_certificate:false, actual_ode_solution_enclosure:false, VerifiedModelProvider:null, V7fullrestore:NOT_RUN, production:HOLD, globalphysicalcompletion:null and L4:false. REI25 has a separate binding. See `NEXT_RESEARCH_KO.md` for the source-owned rate-enclosure continuation.

## Publication and archive scope

This is a text-only self-contained scientific source/evidence payload. Bulk raw branch/PR census, backup-provider raw responses, binaries, helper scripts, opaque prior archives and the standalone human report are excluded. Compact census and fresh preexecution classifications are retained; complete raw intake and PNG are preserved in the full SOURCE archive. Standalone report/storage/final completion acknowledgements are separate from scientific admission.

The previous central19-path return candidate was published as commit `0a7c46fed9e6e922e4c08dba01c36fa5956d43e3` with draft REI PR241. Its successful publication does not change the canonical28/88 count. The root will pin the fresh VigilODE base/branch and perform guarded publication for this package. No remote commit or publication of this new source payload is claimed by its local preparation record.
