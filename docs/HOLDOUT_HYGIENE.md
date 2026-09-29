# Holdout hygiene

Adopted 2026-09-29 after audit findings F-023 and F-047.

1. **Declared before the run.** Holdout dimensions, families and tolerances
   are fixed in the node's contract before any holdout row exists.
2. **Read by a script only.** Holdout rows are read only by a committed
   verdict script whose hash is in the contract. Nobody who can still change a
   policy constant or policy code reads them first.
3. **No holdout literals in the source.** No literal in `crates/*/src` may be
   derived from a holdout row. `G4S5B0Profile::is_holdout` marks the holdout
   profiles; the replayed k = 1 table has no rows for their dimensions
   (`holdout_dimensions_have_no_source_literals`) and is refused on them
   (`replayed_k1_table_is_rejected_on_holdout_profile`), including from the
   CLI.
4. **Consumed by the first verdict.** A holdout is calibration data after its
   first verdict. Any later rule needs a fresh holdout.
5. **Handoffs list calibration paths only.** Packages for coding agents
   exclude holdout directories.
6. **Minimum evidence.** A verdict is called discriminating only with at least
   five reference-unsafe positives and a verdict that survives a one-rank
   threshold move in both directions (`tools/holdout_verdict.py`). Otherwise
   it is `INCONCLUSIVE_INSUFFICIENT_POSITIVES` or
   `INCONCLUSIVE_RANK_UNSTABLE`.
7. **Safety labels are reference-anchored.** "Unsafe" means a reference local
   error above one tolerance unit, not the method's own estimate
   (`reference_unsafe` on the G4/S5B0 shadow rows).

Profiles flagged as holdout today: `canonical` (it contains N = 512),
`holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384`. The
N = 320 and N = 384 holdouts are already consumed.
