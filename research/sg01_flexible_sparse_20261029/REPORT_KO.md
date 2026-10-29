# VigilODE SG01 종료 보고서: Scope-limited standalone, native production HOLD

**실제 시작점:** GitHub PR104 #104, PR105 #105, PR106 #106, PR107 #107의 native monitored GMRES 및 원잔차 연구. 수행 중 추가된 #108 CMODE01은 sibling 연구로 확인했다. SG01은 flexible right PC 저장 Z와 exact-zero sparse interval certificate라는 별도 개선을 다룬다.

## Code-path reality audit

PR104 native `crates/rodas5p-krylov/src/gmres_into.rs` blob `90bfaaa62afeda9dec27b3dfd594e67cd3034da1`, right monitored path lines 617–758: 매 열 least-squares→`apply_preconditioner(correction)`→candidate→monitor. 따라서 wrapper 하나로 trigger-before-backsolve 절감은 불가능하다. Native `Preconditioner` trait의 선형·고정성 계약이 실제 증거로 강제되지 않음. PR107 checkpoint도 native integration은 `NOT_RUN`; PR108 CMODE01도 별개 synthetic IVP다.

## 새 코드와 검증

`src/lib.rs`: frozen dense `A=I−σJ`, per-column saved-Z FGMRES, incremental Givens selective trigger, outward exact-represented original residual, weighted lognorm stage error, source epoch guard, transactional output, finite itinerary verifier. `SparseFrozen`은 0만 제거하고 같은 stored J에 묶임. Default VigilODE code, dispatch, root API 변경 0.

현재 세션 Rust 1.94.1: **16/16 tests PASS**, Clippy `-D warnings` PASS, fmt PASS. 독립 Python Fraction-from-binary64 oracle는 320문제 × dense/sparse × 4개 부등식 = **2560/2560 PASS**. 이는 sampled point arithmetic의 독립 교차검산이지 모든 binary64 입력의 형식 증명이 아니다.

첫 버전 benchmark control은 매 prefix에서 early accept해 출력이 달라 실패했다. 동등한 stop trigger만 허용하도록 수정 후 동일 input/achieved error/열수와 check counts를 확인했다. Route language unit test의 잘못 작성된 좋은 예시도 교정한 후 재실행했다. 실패 기록을 지우거나 PASS로 전사하지 않는다.

## 3회 제한된 성능 루프

1. Dense original residual, every-build negative-cost 대조: n16/n64/n128에서 후보 수 감소의 이점이 있으나 O(n²) 증명 비용이 지배하여 큰 차원에서 제한적.
2. Frozen exact-zero sparse reuse: 모든 샘플에서 같은 achieved output 및 accepted Arnoldi count. Dense trigger / sparse trigger 7-block median n16 **1.6664×**, n64 **5.2582×**, n128 **10.1190×**. 실행 fixture는 모든 J가 diagonal이고 0 성분은 정확히 0. **이 값은 캐시 생성 제외 단일 선형 stage 경로**, full-IVP/native 성능이 아니다.
3. Microprofiling: 7블록 × 200회 반복 호출의 median measured certificate cost n128 dense **238941.5 ns**, sparse **3372.1 ns**; cache construction **35791.6 ns**. 동일 fixture에서 구축비 회수는 1회 certificate 호출 이내. 이것은 workload/CPU/optimization에 종속된 수치이며 구조적으로 일반화하지 않는다. 더 이상 같은 노드에서 performance tuning하지 않는다.

Raw: `evidence/{bench.csv,bench_summary.json,profile.csv,oracle.txt,fraction_oracle.json,bench_ratios.png}`. 그림은 n128 dense trigger-only가 일부 block에서 negative speed ratio를 보이는 경고를 보존한다. 프리프로파일링 없이 구조적 sparse 효과만 선택적으로 재인용하지 않는다.

## Risk ledger

P0: owner physical source/rate/JVP callback rounding 없음; weighted norm을 physical33으로 임의 재해석 금지. P0: native PR104 integration/tests/full-IVP matching 안 됨. P1: freeze epoch와 callback mutation 사이 TOCTOU/원자성 검증 없음. P1: fail-closed happy-breakdown threshold와 overflow/capacity/nonfinite의 full input coverage 아님. P1: dynamic preconditioner 지원은 prototype에 한정. P2: sparse build-once 비용/메모리/actual nonzero pattern에서 성능 다름. P2: timed synthetic n128의 일부 ratio<1 negative cost evidence.

**Claim gates:** standalone code numerically checked; no native integration; no observational prediction; no production acceleration claim. PR104 structural native historical speed, PR107 sketch, PR108 CMODE01은 separate evidence. No false DAG completion promotion.

DAG current: original VigilODE **7/23 inherited scoped**, central REI **28/88**, PREFIX01 **0/1**, PREFIX02 **0/1**, CMODE01 successor scope **8/10 research, 0/1 native admission**. Newly proposed SG01-to-production steps **0/12 admission**, SG01 bounded prototype *local tested 1/1; independent review OPEN*.