# VigilODE PR121 successor: source-owned five matched shear histories, actual physics data

2026-10-30 KST. Native 193×10 source-owned chemical/geometry states. NO IVP, no modification to PR121/PR120/PR274 or BASS source owner, no production. Original BASS commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`, PR121 parent HEAD `53933d370c4bcc020dc03a6fce276f7b1fa2950b`. The raw original files were transferred by BASS GitHub `fetch_file(base64)` → explicitly temporary Google Doc transport → raw export to this runtime, decoded after stripping text-export UTF8 BOM. Every original binary SHA-256 was matched against the corresponding same-commit original `RESULT.json`, and Zip CRC passed. Technical transport does not modify model or source data.

## Objective and measured scientific data

See THEORY_KO.md. Five pairs: m00_r00, m02_r02, m04_r04, m00_r04, m04_r00. Two saved initial shear settings Σ_F 0 and 0.6, 10 original 193-epoch NPZ, total 1,930 direct native source `AtomicNetwork.evaluate` calls. Exact pointwise Δ(dτ/dx) decomposition integrated in all five histories (Simpson and trapezoid reported separately). New 5×193 row physical CSV files, 5 per-pair JSON, one aggregate JSON, 3 diagnostic PNGs. All 5 original saved Δτ <0, geometry shares 92.02–92.12%; midpoint n_b term zero. Original five-pair subset post-hoc chosen, not a preregistered 25-case statistical design. The 127-endpoint PR118 atlas and independent latest BASS CE1 optical sky are historical/source-context only, NOT assumed to be one chemistry provider.

## Exact code/test execution

- `python code/paired_replay.py --write` exit=0; reconstructs full per-pair 193 rows.
- `python -m unittest discover -s tests -p test_paired_replay.py -v` exit=0, **15/15** test methods. Checks source binary SHA, source identity failure negative case, 193×5 matched grids, actual native source `gamma`, gas photo-energy replay, max baryon charge/mass residual, exact midpoint identity including 25 Fraction exact-real samples, stored BDF vs sampled quadrature, nonnegative ionization/geometry contributions relative to B, and no IVP code path.
- `python code/plot_data.py` exit=0 produced three plots.
- `WolframLanguageEvaluator`: 2 independent symbolic product identities `{0,0}`. Different-author R6/C9 NOT_RUN.
- Prior PR121 original 20/20 tests and PR118 13/13 are frozen previous-turn results. This new node's 15 tests are separate, not replacement for native workspace/CI.

## Key quantitative distinction

Original BASS Σ=.6 vs 0 yields Δτ ~−1.23...−1.25×10⁻⁴. In five actual matched saved histories a **92% geometric contribution, 8% electron-history contribution** is the finite exact 193-point source-model decomposition; small Δx_e at the final z=2 is inadequate to infer the integrated contribution. No causal counterfactual split or observed shear data inferred; BDF errors not bounded solely by postprocessed composite Simpson difference. Native F/J is used for exact saved-state chemistry, not to change derivative-free solver priority.

## Active independent branches/constraints examined

VigilODE PR120 derivative-free N01 C6 (native CI evidence mixed, HOLD), PR119 C5 and PR117 derivative-free value-only precedence remain untouched. REI PR274 lithium off-manifold exact BASS-internal mode, PR273 eventwise REI/BASS mismatch H collisional and HII RR, PR272 photon-budget hypothetical LP, and BASS PR261 Liouville sky are distinct source/model components and **not physically joined**. PR121 R6/C9 NOT_RUN, PR118 prereg ledger CI FAIL, inherited failed tests, closed resource budgets, Q-LEDGER/Q-API/Q-GIVENS, 8 suites, 10 interactions unchanged. No native source/epoch RODAS transaction or production promotion. New branch only additive.

## Actual source/code integrity and preservation limits

Locked full package includes nested unchanged PR121 ZIP and nine newly restored original NPZ (tenth original embedded in PR121); BASS historical atomic net/model byte hashes provided by PR121 and `native_full_rate.source_manifest_check()`; coefficient source numeric 129/129 originally verified, raw coefficient JSON byte identity NOT newly established. No dense Jacobian used as production algorithm. Historical resource budget cannot be reset or transferred; present execution cost under current runtime only, host overhead UNKNOWN_NOT_ZERO. Failed intermediate transfer attempt (Google Doc draft due tool-call count) and UTF8 BOM decoding failure preserved in local provenance note; both were diagnosed and recovered; never cited as scientific failure.

## Claim decision

Auxiliary `BASS21_MATCHED_SHEAR_OPACITY_5_PAIRS_20261030`: 1/1 scoped science data emitted, independent review HOLD; R/C 18/20 scoped and R6/C9 genuine independent NOT_RUN. Original VigilODE 7/23, central REI 28/88 (working33/88), successor 0/12 admitted, F4 PARTIAL, L4 false, PRODUCTION HOLD. Preserve conditional scope, no main merge, no force push, no tolerance weakening and no new IVP.
