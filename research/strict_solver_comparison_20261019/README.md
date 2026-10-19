# Strict solver comparison: reading order

**Read [FINAL_REVIEW_KO.md](FINAL_REVIEW_KO.md) first.** This node supersedes the interpretation of the earlier SciPy-1.17 VODE timings, not their historical bytes.

|Artifact|Purpose|
|---|---|
|[Final critical review](FINAL_REVIEW_KO.md)|Source identities, actual matched-error results, explanations, limitations and failure corrections|
|[Matched-error cost CSV](readback_runs/39263860973/COST_AT_MATCHED_ERROR.csv)|34 solver/case/grid configurations, 12 fresh process summaries each|
|[Matched-error work CSV](readback_runs/39263860973/WORK_AT_MATCHED_ERROR.csv)|Counters at the actual selected tolerance, not a different-tolerance proxy|
|[Full matched observations](refined_runs/39261895983)|270 calibration rows, 408 timing rows with individual repeat times, 189 deduplicated full-state objects, references and hashes|
|[Final readback](readback_runs/39263860973/AUDIT.json)|PASS_SCOPED_READBACK_WITH_INITIAL_ROUNDING; exact initial bit check remains FAIL_PRESERVED|
|[Initial-value diagnosis](readback_runs/39263860973/INITIAL_VALUE_SUMMARY.json)|18 Diffsol rows, max absolute difference 2.22e-16, original observations retained|
|[Raw linkage audit](refined_runs/39261895983/LINK_AUDIT.json)|Raw states/times to metrics to frozen accuracy selection to timing summaries|
|[Main screening](runs/39257390707)|756 accuracy cells and 1152 timing processes; two large raw files were NOT archived, as explicitly listed in large_files_excluded.txt|
|[SciPy1.17 differential control](runs/39257390707/SCIPY117_DIAG_SUMMARY.json)|24 cases showing known Jacobian-transpose regression and remaining dense-path anomaly|
|[Reproduction](REPRODUCE.md)|Readback without rerunning solvers, or explicit new pinned-binary experiment|
|[DAG](DAG.json)|Scoped completion versus held/unclaimed conclusions|

Measured numerical library source: `a21d573980f66fe02891bd216153443b5352f921`.
Final matched-error benchmark executable source: `9c6a6d9145795e35b978652a567b806ddc84a6bd`.
Readback code source: `65ef8cde8b3c6fdbfef4d06b52c63df673467651`.
Later report/publication commits are not new measured binaries.

Production crates and root Cargo files were not changed. No whole-workspace test or independent-personnel review claim. Wall-clock conclusions remain conditional shared-runner diagnostics. No merge or default solver promotion.

The one-time historical source-repair helpers exist to explain the recorded CI transition, not as a general installation entry point. Use REPRODUCE.md rather than rerunning an old mutating CI bootstrap script on an arbitrary branch.
