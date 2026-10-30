# BASS21: source-owned full 25 matched histories (no IVPs)

Frozen BASS owner `cc602e209e7deeb511ab2a1f3b2268d55206c43c`; immutable PR124 parent `e48a049e143bb9d57a1d9e2fb2b570c32d0db2dc`.

**Physical data:** original 50/50 trajectories × 193 saved states, 9650 native `AtomicNetwork.evaluate` F/J; 25×193 pairwise geometry/ionization/density optical-depth decomposition; no IVP. `results/SCIENCE_SUMMARY.json` and `REPORT_KO.md` give original-unit results, limitations, and residuals. Original 50 NPZ, full 9650-row F/J and 4825-row history CSV and full source-code model are in the source-inclusive ZIP, **not** this Git text mirror.

**Full source artifact:** [Google Drive ZIP](https://drive.google.com/file/d/1kuKATa1oYeF496HrLIkjXvfFbwTX-l10/view), 23,770,654 bytes SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, with independently verified Drive remote raw readback and CRC; Library `/BASS/VIGILODE_BASS21_FULL25_20261030/`.

Clean extraction `bash VERIFY.sh` on full source ZIP: original 50 SHA, static hashes 92, 20/20 Python tests exit 0, 7 regenerated result SHA, and ZIP 114-member CRC PASS. The Git mirror is incomplete by design; its `VERIFY.sh` **requires the full archive**.

**Gates:** 17/20 R/C scoped complete, C8 PARTIAL, distinct-author R6/C9 NOT_RUN. Stage JSON files are retrospective snapshots, not distinct independent executions/preregistration. VigilODE DAG 7/23, REI central 28/88, working 33/88, successor 0/12, auxiliary 1/1 scoped conditional data. PR118 historic prereg/ledger CI failure remains; production HOLD; preserve PR117 causal RHS/value-only priority, Q-LEDGER/Q-API/Q-GIVENS, 8 suites, 10 interactions.

`NO_MORE_IVPS=true`; `NO_MAIN_MERGE=true`; `NO_FORCE_PUSH=true`; `NO_PRODUCTION_PROMOTION=true`; `NO_TOLERANCE_WEAKENING=true`.
