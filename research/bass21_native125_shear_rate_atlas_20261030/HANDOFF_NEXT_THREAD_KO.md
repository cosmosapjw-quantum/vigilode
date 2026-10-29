# Copy/paste handoff: VigilODE × BASS21 native125

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
PARENT_PR=122
PARENT_HEAD=ded23f73366e492ad487b1614a895e8474cb215b
ORIGINAL_BASS_OWNER=cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c
OWNER_127_CSV_GIT_BLOB=a49aa5276b4256d6bf0ad443b6b830effe2ab738
OWNER_127_CSV_SHA256=f8020fe09d4ac34573de0e452f46a324f7dca2097b857131eadfee5ed8d6c866
PARENT_PR122_SOURCE_FULL_SHA256=93a305a9599cd7a87d5f574fc8105ea9096af24fc5763287ff1d4fbc190f5561
CURRENT_UNIT=BASS21_NATIVE125_SHEAR_RATE_ATLAS_20261030
NEXT_UNIT=BASS21_MATCHED25_FULL_HISTORY_SOURCE_REPLAY
AUTO_NEXT_UNIT=false

## Scope of what is really established

- 127 original endpoints (125 full 5x5x5 grid plus two cancellation cases) same immutable source; 125 native original F/J evaluations and 25 matched optical depth endpoints computed, no IVP. Original PR122 source-full ZIP with five 193-epoch matched pairs is nested unchanged.
- For all 25 shear0 vs shear.6 matched endpoints the stored Δτ <0, exact endpoint differential opacity split dominated by geometry 99.9999983%; this is NOT the total historical chemistry contribution (previous five full trajectories showed ~8% ionization integral).
- For each of 25 original saved 5-shear sets, nonuniform secant slopes of tau decrease across initial shear [0,.003,.03,.15,.6]; 7 first-step positive changes are tiny numerical observations, NOT a rigorously established continuum local maximum. Full 25 history decomposition is NOT_RUN; only five prior.
- Local 14/14 Python tests PASS and Wolfram exact identity 0. Native distinct-author R6/C9 NOT_RUN; previous research node CI FAIL must not be rewritten; no observed CMB/reionization posterior.

## Next executable physics-first task

1. Fetch fresh branch/PR and original ZIP; authenticate SHA256/source owner, inspect today's sibling repos and latest handoffs. Read `THEORY_KO.md`, `REPORT_KO.md`, `CONTRACT.json`, `DAG_DELTA.json`, `results/SCIENCE_RESULTS.json` and `results/BASS25_MATCHED_SHEAR_OPACITY.csv` in delivered package.
2. Restore remaining **40** original BASS `trajectory.npz` saved 193-sample histories by their original BASS owner SHA/blob; *do not* generate new IVPs. Matched 25 pairs total 50 trajectories; 10 exist inside PR122 predecessor.
3. For each of 25 matched states verify original T(x), UV(x), x,z grid, nb(x), initial Pm,Pr and shear 0/.6 and original `AtomicNetwork.evaluate`. Compute exact finite product polarization **at each** of 193 saved epochs, integrate geometry, electron ionization, baryon-density components via saved-grid Simpson and compare stored BDF τ. Preserve per-case differences, quadrature variation, uncertainties. Return physically actual 25x193 tables.
4. Conduct one bounded independent scientific R6/C9 when native delegation exists. Reject cross-owner REI full17 / BASS source pairing unless one rate/clock/temperature/photons/inventory owner is authenticated. Keep derivative-free PR117 priority and solver trials separate.
5. At each selected R/C stage make immutable checkpoint and report `VigilODE 7/23; REI central 28/88; working 33/88; current auxiliary 1/1` until real original DAG authority changes. GitHub Draft stack on exact PR122 HEAD only, no merge, no force-push, no new IVP, no production.

## Reproduction

Clean unzip source-full archive. `bash VERIFY.sh` to rebuild native125 F/J and 25-pair CSV/JSON in fresh extraction. Verify input checksums, source hashes, 14 tests and 5 generated text outputs. Native original full 193-sample time-history extension is a separate task.
