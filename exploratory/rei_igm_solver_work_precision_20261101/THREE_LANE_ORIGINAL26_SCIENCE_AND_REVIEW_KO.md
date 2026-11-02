# VigilODE → REI F05 original 26-event three-lane thermal-root reuse

2026-11-01 | one finite theory/coding optimization loop | Draft PR #183 only | no original REI owner mutation

## Original problem and traceability

Goal: reduce actual original 46,080-cell, 26-event H/He/OTS/thermal original F05 native full/half1/half2 step [160,161] compute cost without changing science or owner acceptance. Source READ-ONLY original REI PR348 commit `fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`, source TAR SHA256 `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`, state `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`, 26-event kernel `579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644`. Original accepted tick is only 160/131072. Original 14 physical ledgers, photon groups, global H/He neutral absorber moments, OTS, energy, positivity and table-event checks unchanged.

The newer original owner PR #356 independently adopts a **versioned V2 source-scoped discrete defect predicate**, not historical V1 acceptance, and still says official tick161 NOT_ACCEPTED. This VigilODE performance research is separate. Exact pre-registration HEAD published before new three-lane computation: `92a13a6cfe1f8017d29fa674f5aa01d2c47f974b`.

## Constructive theory and implementation

The exact original implicit interval thermal root is F(x)=E(x)-U0-h*(constant+weight*R(x)) where E(x)=ctx.ecoef*exp(x). The unchanged original `thermal_root` evaluates interval F(lo),F(hi) inside the 20-attempt expansion loop, including when it immediately breaks with unchanged endpoints; it then redundantly computes the SAME two expressions at the same endpoints and unchanged `ctx` before applying the sign bracket.

The VigilODE opt-in shadow retains previous interval endpoint results exactly on early break. On loop exhaustion (endpoints may have changed on the final attempt), `for...else` recomputes both endpoints. Sign bracket, interval Newton, positive-derivative check, outward operations, temperature root coefficients, source forcing, all three original native map calls are unchanged. This is exact code-path reuse **conditional on deterministic pure F/ctx**, NOT a proof of full correctly rounded interval math. No time-history acceptance or default solver substitution is involved.

There are 3 thermal roots per original map and 3 original maps per arm: 9 roots and two duplicate F endpoint evaluations per root. The observed work reduction 90→72 thermal RHS calls per arm agrees with the predicted 18 eliminated calls.

## Native original physical experiment and results

Frozen three original shape lanes: LOCAL_NEUTRAL_HAZARD_PRIMARY, RECOMBINATION_WEIGHTED_AUDITOR, SCRIPT_SELF_SHIELDING_AUDITOR. Each lane runs original A/shadow B/shadow B/original A, each arm executing REAL original full+half1+half2 46,080-cell 26-event source maps. 36 actual original maps total; the three lanes share **one** physical source dataset, not three independent astrophysical realizations. Runs are pinned to disjoint CPU-affinity groups (0-3, 8-11, 16-19), OPENBLAS/OMP/MKL thread limit 1. Therefore cross-lane wall comparisons are diagnostic; within-lane A/B is the fairer comparison.

| Lane | original A0 / A3, s | shadow B1 / B2, s | median A/B |
| --- | --- | --- | --- |
| Primary | 16.308284 / 16.584611 | 15.823873 / 15.183503 | 1.060809x |
| Recombination | 16.039000 / 16.814484 | 14.336492 / 14.721358 | 1.130623x |
| Self-shielding | 16.948553 / 16.103533 | 15.351882 / 15.229156 | 1.080803x |

All **12/12** arms original source-native PASS for three full/half1/half2 maps, **14/14 original raw nuclei/photon-group/OTS/energy ledgers per arm**, 0 table events; original population five species + logT lower/upper array results (all three maps) BYTE-IDENTICAL across all 12 arms, SHA256 `fe260af02b4ad84ae4c90ed0066e42713ad056a7582d33b0708e5eb6d4b6c6cc`. Source-specific median of the three within-lane speed ratios **1.080803x** (DESCRIPTIVE ONLY); 90→72 original thermal RHS calls every arm. Actual original 26-event source calculation performed, not a reduced photo-only surrogate.

**Negative evidence explicitly retained:** historical V1 independent endpoint logT error bound 2.1245050576368385e-4 remains FAIL versus original strict 2e-4. Original source-owner PR356 V2 alternative shadow criterion does NOT authorize tick161. No tick162, no canonical history, no production. Local imported optional NumPy extensions emitted NumPy2 ABI warnings in stderr, processes exit0; no formally pinned clean replay of this new VigilODE optimization. The exceptional 20-expansion branch was algorithmically protected but NOT separately exercised as a negative branch fixture.

## Actual tests and reproducibility

`analyze_original26_three_lane.py` is a fresh evidence consumer validating the complete 12-arm source SHA, state SHA, kernel SHA, 36 map statuses, 14 raw ledgers, 0 events, exact SHA, 90/72 work, original V1 fail, arm timing and source authority. Tamper/negative tests reject invalid source identity, one bad ledger, erased historical error, mismatched array SHA, fabricated speedup and missing lane. Python 20/20 normal and 20/20 -O tests PASS. Reused existing native VigilODE opt-in PR181 nonauton method 6/6 Rust tests PASS; source-based native four-route comparison 5/5 Rust PASS. Native 15-state five-clock tests are **not** full original 26-event solver validation, and no generic solver default changes.

Receipts in this directory: `ORIGINAL26_ROOT_ABBA_{PRIMARY,RECOMB,SHIELD}_3LANE.json`, `ORIGINAL26_THREE_LANE_AUDIT.json`, `PREREGISTRATION_3LANE_THERMAL_ROOT_20261101.md`, unchanged previous `ORIGINAL26_ROOT_ABBA.json`, executable scripts and test suite. Original source owner path remains detached and clean.

## One bounded self-hostile referee / drift

1. Failure of an arbitrary original bracket expansion or transition across Hummer-Seaton events is not excluded by this single tick's branch. Do not replace original owner source without exhaustive active-branch negative tests and clean source-owner CI.
2. Full continuum truncation error, globally validated interval arithmetic and accepted history **NOT_ESTABLISHED**; original V1 failure preserved. No false V2/tick161 claim.
3. Timings use same host, same source dataset and CPU-affinity scheduling; meaningful local work reduction measured, but no generic hardware/horizon speedup guarantee. Apparent 3-lane agreement does not create independent physics validation.
4. In this bounded objective, the constructor and hostile reviewer are the same assistant. No native distinct-author referee. PR #183 is Draft/HOLD; inherited unrelated full production CI failures are not swept under the rug.
5. The candidate avoids previous drift towards fourth photon atlases: this specifically reduces the **original complete 26-event runtime** while preserving all original output bytes. Next move is the smallest SCIENCE-OWNED change: a separate opt-in owner-controlled version of this identical bracket optimization, first 0–64 old protocol parity and three-lane zero events/raw ledger parity, and independently pinned runtime before any source-owner release decision. No additional pure governance loop.

## Final stage checkpoint (reported counters unchanged at every subphase)

R5 constructive endpoint-reuse argument DERIVED; R6 one self-hostile pass COMPLETED_SAME_AUTHOR; R7 real 3-lane full source experiment NUMERICALLY_CHECKED; C2 original hotspots mapped; C5 opt-in research implementation IMPLEMENTED; C6 original-/negative-test implementation VERIFIED; C7 source 14 raw ledgers/physical array parity VERIFIED_SCOPED; C8 same-error work-per-arm comparison MEASURED_SCOPED; C9 self-hostile code review SAME_AUTHOR; C10 Draft GitHub and cold backup tracked separately. These are bounded **research stage results**, not completion of all official science DAG nodes.

VigilODE OFFICIAL 7/23; successor 0/12; REI CENTRAL 28/88; REI WORKING 33/88; FAST 4/7, canonical scientific delta zero. Original tick160/131072 accepted only. PRODUCTION HOLD, NO_MERGE, NO_FORCE_PUSH, NO_DEFAULT_CHANGE, NO_TOLERANCE_RELAXATION, AUTO_NEXT_NODE=false. Next artifact should be an actual changed source-owner scientific run/admission, not another 3-lane benchmark report.
