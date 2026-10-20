# Coupled residual-first economy: next bounded native step

Read [CRITICAL_REVIEW_KO.md](CRITICAL_REVIEW_KO.md) first. The improvement is opt-in; existing eager API and upstream default solvers are unchanged. This is stacked on PR74, not merged into production.

|Artifact|Use|
|---|---|
|[Theory](THEORY.md)|Same-iterate residual/derivative reordering, sufficient equivalence assumptions, limits of candidate JVP caps|
|[API and replay](REPRODUCE.md)|Public entrypoints, budget semantics and source-pinned commands|
|[Original RED](runs/39353982085/RED.log)|3observed unnecessary-derivative failures and1output control|
|[Actual tests](runs/39355449893/TESTS.log)|Prior19+new14=33passed; no whole-workspace PASS|
|[Cost table](runs/39355449893/COST.csv)|All80configurations, root/LTE flags, work and failure cost|
|[Generated result report](runs/39355449893/REPORT.md)|50paired comparisons and their actual relations|
|[Raw run](runs/39355449893)|Every retained repeat time, references,71state objects, hashes|
|[Numerical readback](runs/39355449893/AUDIT.json)|684metrics,630work ledgers,270cap checks,27stage-bit comparisons,50paired medians|
|[DAG](DAG.json)|Completed scope and explicit holds|

Parent companion source:`15c43ce76734d0a560fcec5909ac49f1e6f2d347`.
Measured source:`f08d152a35fb2db748692570969cb1723c287c26`.
Measurement run:`39355449893`.
The generated/source-binding commit was published before build. Later prose/closeout commits are not relabelled as newly measured binaries. No old-host timing samples are pooled.

The JVP caps64/256are deliberately retained as sensitivity/negative results, not recommended optimal defaults. A cap limits candidate JVP attempts only; projected solves and fallback time still cost work. Root completion is distinct from embedded physical-step admission and continuous-time certification.
