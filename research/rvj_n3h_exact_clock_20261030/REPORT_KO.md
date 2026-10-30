# N3H | represented-time source horizon fail-closed repair

## 대상 및 반례

상위 PR139 HEAD `c04447a6d5930d4b1898c933f33a9c222b60f32d`의 **정확한 연구용 Rust 소스**를 복제해 비가역 변경 없이 검사했다. `Source::toy(12)`는 종료시각 T=0.5를 선언한다. 기존 `propose` 조건은 `t+h>T+1e-13`인 경우만 거부하므로, h=0.5+2^-48을 실제로 허용한다. 더욱 강한 반례는 t=1/4, h=nextUp(1/4)에서 exact binary64-as-real 덧셈이 T보다 크지만 RN의 `fl(t+h)=T`이다. 단순 `fl(t+h)>T`로는 이 반례를 막지 못한다.

## 수정과 정확성

Knuth TwoSum의 error-free 변환으로 finite binary64 t,h의 정확한 합 `t+h=s+err`를 복원한다. `s>T` 또는 `s==T && err>0`이면 **실제로 표현된 두 입력을 정확한 실수로 간주한 합**이 종료시각을 넘는다. 다른 영역은 RN rounding half-ulp와 이웃 binary64 값의 간격을 이용해 판별한다. 원래 `T+1e-13` 우회 경로는 제거하고, 원본 시간창을 정확히 닫는다. 테스트된 실행 호스트는 Rust1.99.1 x86_64, IEEE-754 RN, gradual subnormal이다. 이는 f64 출력의 물리 오차 증명이 아니고 clock/domain 검사의 exact source representation 판정이다.

## 실제 RED/GREEN 증거

* 최초 네이티브 반례 2개 모두 FAIL (Rust test exit101): T를 약 3.55e-15 / 1.78e-15 만큼 초과하는 창을 허용.
* 순진한 `fl(t+h)>T` 수선 이후에도 RN tie witness는 FAIL(exit101).
* TwoSum 수선으로 horizon 3/3 PASS, 기존 25/25 PASS, 총 **28/28 PASS**. strict Clippy `-D warnings`, rustfmt 모두 PASS.
* 독립 Python Fraction oracle: 100,000 deterministic exact represented sums/clock comparisons **PASS**.
* 원래 source outputs 및 716 mode-window row의 `re/im/bound` CSV는 **byte-identical** (SHA256 `b3d288f51464ad8129036e90823d35ffa88af754ddbb7be97ee2faa26cabf508`).
* 변경은 `src/lib.rs`의 source horizon 판단 부분과 새로운 통합 테스트 `tests/horizon_contract.rs`뿐. 비선형 solver, tableau, tolerances, speed claims, physical precision claims는 변경하지 않음.

## 한계와 상태

실제 exponential/convolution/carrier floating-point 계산은 outward-certified 아님. 전 구간 물리 오차, 원래 RODAS5P 8-stage 잔차, distinct-author C9 미검증. N3H는 N3R의 경계 버그 수선일 뿐 RVJ DAG 4/8로 산입하지 않는다.

VigilODE 7/23 · REI 28/88 · RVJ 3/8 scoped · successor admitted 0/12 · C9 different-author 0/1 · C10 NOT_STARTED · production HOLD · merge_ready false · auto_next false.
