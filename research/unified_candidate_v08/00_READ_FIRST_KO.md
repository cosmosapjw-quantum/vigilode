# RODAS5P unified candidate research cycle v0.8 — 먼저 읽기

이번 cycle은 기존 sequential/Krylov/SABR/homotopy 후보를 같은 Rust 기준으로 비교하고, 앞으로 BDF·Radau/IRK·Peer/W·parallel SDC·ROK/BOROK·Leja가 반드시 같은 registry와 gate를 통과하도록 만드는 작업이다.

## 정본 결과

- functional source commit: `20bdda908326521f5993169263bcbf6a1e6d3a98`
- candidate catalog: 163 entries
- executable: 156
- deferred: 7
- canonical Tier-N rows: 936
- numerical failures: 0
- uncertified rows: 0
- research-only C0 reference fallbacks: 4, all rejected
- one-thread/four-thread scientific checksum: identical
- authoritative joint promotion: 0

## 판정

- **PROMOTE:** 통합 연구 하네스, common certification/work/thread contract
- **REFERENCE:** sequential/direct, Tier-L GMRES/OFF
- **HOLD:** 모든 비기준 implemented candidate
- **DEFERRED:** BDF, Radau/IRK, Peer/W, parallel SDC, ROK, BOROK, exponential/Leja

## 읽는 순서

1. `reports/UNIFIED_CANDIDATE_CYCLE_REPORT_KO.md`
2. `closeout/PHASE10_PROMOTE_HOLD.md`
3. `results/RESULT_SUMMARY.json`
4. `reviews/PHASE09_INDEPENDENT_DIFF_REVIEW.md`
5. `handoff/NEXT_CYCLE_PLAN_KO.md`

`results/CANONICAL_EXACT_T1.json`과 `CANONICAL_EXACT_T4.json`이 full evidence이며, CSV tables는 비교 편의를 위한 파생 산출물이다.
