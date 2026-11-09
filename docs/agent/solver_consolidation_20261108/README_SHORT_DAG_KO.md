# VigilODE T01: historical lineage and revised short DAG

**HOLD_NOT_RELEASE_QUALIFIED.** This turn modified documentation/plans only; no Rust solver source was changed.

GitHub REST enumeration captured **187 PR metadata rows and 361 branch refs**. PR #176's 28-row inventory was recovered exactly. Selected source/body audit covered G0, global-error, coupled, N01/C9, applied REI and RVJ lineages. All metadata were inventoried, but it would be inaccurate to say all 187 diffs received a scientific peer review.

## Peer-W priority addendum (2026-11-09)

Peer-W is now the first new whole-IVP implementation family. PTSW2C is the first order-3 prototype; PTSW2B is the separate L-stable order-2 control; p>=5 primary-table recovery is mandatory for G0. The exact tableau/API/source map is in PEERW_FIRST_IMPLEMENTATION_PLAN_KO.md. The original G0 DAG remains immutable.

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


## T01A: 통합 연구 아키텍처와 adaptive portfolio

핵심 설계는 모든 수치기법을 동시에 실행하는 pipeline이 아니다. 현재 IVP에서 필요한 **희소한 조합만 자동 선택·활성화**한다. 상위 selector는 전체 IVP method를, 중간 selector는 해당 method의 stage/linear backend를, 별도 selector는 source-specific transform과 CPU execution lane을 선택한다. 각 후보는 capability/compatibility, original-source residual, error, setup·probe·switch·fallback cost를 통과해야 한다.

- `ADAPTIVE_PORTFOLIO_ROUTER_SPEC_KO.md`: S0 permission/admissibility → S1 whole-IVP method → S2 solve backend → S3 optional structural worker → S4 execution lane.
- `ADAPTIVE_ROUTER_CONTRACTS.json`: 기계 판독 selector state, observation tiers, regime alphabet, cost gate, switching semantics 및 현 구현 상태.
- 기존 `AutomaticOde`/`SwitchingSolver`를 확장한다. 별도 switcher/WorkLedger/thread pool/benchmark framework를 만들지 않는다.
- 기존 switcher는 현재 DOPRI5와 허가된 RODAS 경로 중심이다. C8 finite-state router는 R2/R4 scoped, C8 numeric gate pending이다. 따라서 full adaptive portfolio는 아직 구현되지 않았다.
- 빠른 진동계의 약 32.5배 JVP-switching slowdown과 추후 stiff-transition 재탐색 실패 사례를 negative evidence로 보존한다. pressure-only 또는 한 번의 pilot에 의존하지 않는다.

공식 G0 23-node/34-edge DAG, 7/23, successor 0/12 및 production HOLD는 그대로 유지한다. 실행 overlay는 T01A 이후 T02 current global-error replay와 T03/T04 C5 economics, T06 method qualification을 진행하고, T07은 기존 switcher를 qualified candidate에 한해 확장하는 단계로 둔다.


Peer-W overlay PW00-PW06 is now the next primary implementation path. PW01/PW02 are separate method kernels; PW03 recovers a concrete p>=5 table; PW04 supplies transactional two-step history and the current fair-ab adapter; PW06 compares matched full IVPs. C5 scan research remains a separate preserved branch.


## PW07 conditional theory and mock research completed (2026-11-09)

Added original-W residual/operator-perturbation bound, method-switch/history-transfer global-error recurrence, RHS-only contraction assumptions, nonnormal product adversaries and affine-scan mock. Exact order moments and scalar mock slopes approach orders 3 (PTSW2C) and 2 (PTSW2B). Evidence is Python/SymPy mock only; no Rust solver, native concurrency, fair-ab matched-error run or G0 qualification. Prior-art boundary includes 2011/2015 Peer global-error control and Constantinescu 2017 coupled-GL estimation.
