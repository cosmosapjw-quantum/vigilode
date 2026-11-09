# VigilODE T01: historical lineage and revised short DAG

**HOLD_NOT_RELEASE_QUALIFIED.** This turn modified documentation/plans only; no Rust solver source was changed.

GitHub REST enumeration captured **187 PR metadata rows and 361 branch refs**. PR #176's 28-row inventory was recovered exactly. Selected source/body audit covered G0, global-error, coupled, N01/C9, applied REI and RVJ lineages. All metadata were inventoried, but it would be inaccurate to say all 187 diffs received a scientific peer review.

## Research priority changed

The pinned C5 source already contains `rodas5p-fair-ab` with common output-grid/error metrics, reference provenance, clipped/dense output-policy pairs, adaptive comparison and work/timing reports. T02 therefore replays the **current** source rather than rebuilding an evaluator. The frozen v0.10 commit `f2e9b1e8883bc49c124ce5ab421c92fb58516b74` has no Git common ancestor with the C5 source; its old tag and successor branch are absent from current refs. The v0.10 result is historical comparison, not current source proof.

C5's original full-IVP result remains negative: median graph-Schur/fast-direct wall ratio `2.242032452000776`, all 12 original clipped-grid pairs slower. Physical cases have path length one and zero affine pair operations. `scan_into` is serial, allocates per chain and clones per stride. The R10 ring96 count 538 versus 94 is synthetic operation-count evidence, not physical or parallel speedup.

New order: **T02 current global-error replay** and **T03 scan cost/long-path preregistration** proceed in parallel. T04 measures long path-forest IVPs. T05 implements an ordered work-efficient scan only if T04 proves end-to-end bottleneck/crossover. T06 qualifies BDF/Radau and exact Peer-W/SEPP math. Frontend and spatial work follow qualification.

The official G0 DAG remains unchanged: 23 nodes / 34 edges, progress 7/23, successor 0/12, production HOLD. This is a short execution overlay only.

## T01 outputs

`HISTORY_MATRIX.json`, `LINEAGE_DAG.mmd`, `SOLVER_COMPONENT_INVENTORY.json`, `NEGATIVE_EVIDENCE_INDEX.json`, `REPLAY_COMMANDS.json`, revised `SHORT_TERM_DAG_20261109.json`, `METHOD_PORTFOLIO_20261109.json`, `LITERATURE_DELTA_20261109.json`, `SOURCE_LOCK_20261109.json`, and `NEXT_TURN_AGENT_PROMPT_KO.txt`.

`$physmath-research-loop` was not exposed by the active ChatGPT tool catalog or plugin search. The current context, recent summaries, memory and indexed Library were searched, but a complete account-wide archived transcript export is not exposed. No local clone exists in this runtime, so Cargo/Rust tests, format, benchmark and CI are NOT_RUN.

Blelloch (1990), work-efficient scan: https://www.cs.cmu.edu/~guyb/papers/Ble93.pdf. Ordered affine composition is associative in exact arithmetic but noncommutative; floating reassociation must pass original-W residual/fallback gates.

Work branch: https://github.com/cosmosapjw-quantum/vigilode/tree/research/solver-portfolio-shortdag-20261109
