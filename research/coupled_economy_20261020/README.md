# Coupled residual-first economy: native result and final archive

Read [CRITICAL_REVIEW_KO.md](CRITICAL_REVIEW_KO.md) for the scientific conclusions and [ARCHIVE_RECOVERY.md](ARCHIVE_RECOVERY.md) before replaying the frozen data. The one missing deterministic reference was recovered with its exact original SHA256; all101archive-file hashes and saved numerical readback then passed. No candidate or timing sample was regenerated.

|Artifact|Use|
|---|---|
|[Theory](THEORY.md)|Same-iterate residual/derivative reordering, sufficient equivalence assumptions, limits of candidate JVP caps|
|[API and new-run replay](REPRODUCE.md)|Public entrypoints, budget semantics and source-pinned commands|
|[Original RED](runs/39353982085/RED.log)|3observed unnecessary-derivative failures and1output control|
|[Actual tests](runs/39355449893/TESTS.log)|Prior19+new14=33passed; no whole-workspace PASS|
|[Cost table](runs/39355449893/COST.csv)|All80configurations, root/LTE flags, work and failure cost|
|[Generated result report](runs/39355449893/REPORT.md)|50paired comparisons and their actual relations|
|[Raw observations](runs/39355449893)|Every retained repeat time and71state objects; restore REFERENCES.json from the linked recovery gzip|
|[Numerical readback](runs/39355449893/AUDIT.json)|684metrics,630work ledgers,270cap checks,27stage-bit comparisons,50paired medians|
|[Final archive/source validation](reference_recovery/39357596507/VALIDATION.json)|101file hashes, exact reference recovery,33tests and unchanged source|
|[DAG](DAG.json)|Completed numerical scope and explicit holds|

Parent companion:`15c43ce76734d0a560fcec5909ac49f1e6f2d347`.
Measured source:`f08d152a35fb2db748692570969cb1723c287c26`.
Numerical run:`39355449893`. Deterministic-reference recovery:`39357596507`.
Later prose/archival commits are not new measured binaries. No old-host times are pooled.

JVP caps64/256are retained sensitivity/negative results, NOT recommended optimal defaults. A cap limits candidate JVP attempts only; projected solves and fallback still cost work. Root completion is distinct from embedded time-error admission and continuous-time certification. Existing eager API and upstream default numerical solvers are unchanged. PR75 is stacked on unmerged PR74.
