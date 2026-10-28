# 스텝 적응과 방법 선택의 연결: 소스 감사와 실행 가능한 수정

기준 소스: `d79326648a7273c0decd641eaf94d94f5bea7c63`. 이 노트의 구현 범위는 `crates/vigilode/src/switching.rs`와 전용 시험·예제다. PhysMath Research Loop 1.3.0의 R2–R5/R7/R9, C2–C8을 선택하여 조사–유도–pilot–검증을 연결하였다.

## 현재 구현이 하는 일

`GeneralOde::select`는 `Unknown`과 `Nonstiff`를 모두 DOPRI5에 할당한다. 사용자가 `Stiff`를 선언해야 RODAS5P를 선택한다. 따라서 이 API의 `Unknown`은 자동 강성 검출을 뜻하지 않는다.

별도 `SwitchingSolver`는 DOPRI5에서 시작하고 시도 수·거절률·시도 한도 초과를 관측하여 RODAS5P로 전환한다. 기본 `DerivativeFallback::Forbidden`에서는 하나의 지속 DOPRI5 세션만 실행한다. 미분 제공자를 명시적으로 허용한 경우에만 경쟁이 열린다. 자코비안/JVP 제공자에 따라 implicit 하위 경로가 달라지며, 구조의 발견이나 주파수 추정은 하지 않는다.

2026-10-23에 추가된 matched-prefix entry pilot은 같은 시작점에서 explicit가 도달한 시각까지 RODAS5P를 실행하여 결정론적 작업 점수를 비교한다. 이 구성은 작은 스텝을 강성으로 곧바로 해석하는 오류를 줄인다. 다만 과거 기본 설정은 pilot 비활성화, 횟수 제한 1이고, derivative 허용 경로는 **모든 batch/window에서 `h0`로 재시작**한다. 따라서 startup 비용이 압력 지표에 섞일 수 있다. implicit return probe는 이전 implicit window의 작업률과 비교하므로 위치가 같은 비교가 아니다.

최근 로컬 실행의 약 97.6% 수락률과 적은 진행량은 이 소스상의 startup 현상만으로 설명되지 않는다. 장기간의 작은 스텝은 원래 좌표의 안정성 제약에서도 발생한다. 두 효과를 별개로 계측해야 한다.

## 구현한 수정

`SwitchingConfig::evidence_driven(fallback)`는 다음을 한 번에 활성화한다.

1. 진입 전 같은 checkpoint/prefix에서 최대 64회 시도의 implicit pilot. 전체 최대 8회, 재실행 사이 explicit batch 4개를 둔다.
2. native `Dopri5::with_session`에서 가져온 실제 `proposed_h`를 다음 **같은 explicit 경로의 committed checkpoint**로 전달한다. implicit 상태가 커밋되면 explicit 제안을 폐기한다. 서로 다른 방법 사이에 같은 step size를 그대로 이식하지 않는다.
3. `SwitchingEvidence`와 `initial_step`, `proposed_next_step`, `comparator_work_ratio`를 기록한다. `ImplicitCheaperOnPrefix` 등은 관측된 방법 적합도이며 물리적 강성의 증명이 아니다.

이 변경은 step proposal만 이어준다. batch 사이의 FSAL, PI controller history, workspace를 보존하는 완전한 지속 switching session은 후속 작업이다. implicit driver의 다음 제안은 public 결과에 없으므로 값을 추정하여 native 값처럼 기록하지 않았다. 기존 `Default`의 동작은 유지한다.

## 직접 유도

각 방법의 비교 구간 길이를 Δt, explicit/implicit 작업 점수를 C_E, C_I라 두면 측정된 평균률은 r_E=C_E/Δt, r_I=C_I/Δt이다. 앞으로 길이 H인 구간에서 이 작업률이 유지되고 새 전환 비용이 S, 비교 pilot 비용이 P라면

\[
C_{\rm stay}=Hr_E,\qquad C_{\rm switch}=P+S+Hr_I.
\]

따라서 r_E>r_I일 때 `H>(P+S)/(r_E-r_I)`가 전환 이득의 필요충분조건이다. 유도는 두 비용의 차를 정리한 것이다. 이는 구간별 정상성 가정을 명시한 비용 정리이며 일반적인 강성 판정 정리가 아니다. 구현은 이 미래률을 인증하지 않는다. 로컬 후속 측정은 wall time을 같은 식에 대입하되 후보별 달성 정확도를 먼저 통과시켜야 한다.

최대 step 성장률이 g>1이고 작은 초기 제안 h_0에서 목표 제안 h_*까지 증가하려면, k번의 성장 후 h_k≤g^k h_0이므로 적어도 ceil(log(h_*/h_0)/log g)번의 성장 갱신이 필요하다. repeated restart는 이 갱신을 반복시킬 수 있다. 정확도·출력 제약이 변하지 않는 구간에서 이미 얻은 h 제안을 읽어 전달하는 비용은 O(1)이고 RHS를 추가 호출하지 않는다. 다만 다음 구간의 급변은 여전히 수락 검사를 거쳐야 하며, 이 식은 보편적인 runtime speedup을 보장하지 않는다.

## 원래 연구 아이디어와의 연결

진동·cofactor chart provider는 일반 DOPRI/RODAS와 같은 입력 표현을 공유하지 않는다. 잔차·재구성 오차와 원래 물리량의 오차를 평가하는 provider 계약이 있는 경우에만 자동 후보로 추가해야 한다. 작은 h를 보았다는 이유만으로 Fourier closure나 미분 불변량을 추정해 활성화하면 좌표변환 오차와 안정성 제약을 혼동한다. 구조가 제공되면 원래 문제와 transformed 문제를 동일한 관측 시각/물리 norm으로 비교하여 유리한 adapter를 선택할 수 있다.

## 다음 구현과 판별 실험

- **우선:** 로컬 raw 자료에서 batch별 initial h와 native next h, 시도/수락/거절, 방법 체류시간, 실제 RHS/JVP/linear solve 시간을 추출한다. `h0` reset의 추가 비용과 original-coordinate 안정성 제한을 분리한다.
- **지속 세션:** explicit session을 모든 checkpoint에 걸쳐 유지하고 경로가 바뀔 때만 파기한다. implicit driver도 controller proposal과 cache epoch를 내보내도록 한다. 기존 paused/unpaused bitwise 계약을 재사용한다.
- **선택 품질 시험:** 새 adapter가 고른 방법을 주시험으로 삼고, 같은 checkpoint의 안전한 소수 대안만 counterfactual로 실행한다. 정확도를 만족한 후보 중 가장 빠른 비용을 기준으로 selection regret를 계산한다. 가장 빠른 기준이 미측정이면 unknown으로 남긴다.
- **현상 변화:** 초기 transient/중간 stiff/후기 nonstiff의 전환 문제에서 pilot 간격과 최소 체류기간을 시험한다. 전체 solve당 pilot 한 번으로는 후기 변화를 학습할 수 없다.
- **비용 개선:** current score는 RHS+JVP+df/dt+n*Jacobian+linear solves로, 서로 다른 callback의 시간을 구분하지 못한다. 로컬 계측에서 얻은 고정 비용표를 별도 calibration identity로 적용하고 pilot 및 logging 비용까지 합산한다.
- **물리적 분류:** 주파수·감쇠·nonnormality·도메인 제한은 별개 축으로 기록한다. `stiff/nonstiff` 하나의 라벨로 모든 adapter 결정을 대체하지 않는다.

## 검증 해석

`evidence_switch_contracts.rs`는 제안 전달, 정확도 cap, startup 작업 감소, pilot 간격/한도/회계, derivative 권한을 시험한다. `evidence_switch_bench`는 회전, 완만한 감쇠, 강제 강성 문제에 실제 기존 solver들을 실행한다. 측정은 동일 tolerance 및 공통 endpoint cap에서의 짧은 탐색이며 달성 오차가 정확히 일치하는 최종 성능 순위가 아니다. 결과 파일의 실제 종료 상태가 검증의 근거다.

## 문헌 연결

Linda Petzold, *Automatic Selection of Methods for Solving Stiff and Nonstiff Systems of Ordinary Differential Equations*, SIAM Journal on Scientific and Statistical Computing (1983), DOI [10.1137/0904010](https://epubs.siam.org/doi/10.1137/0904010). 2026-10-25 Exa로 출판사 초록을 확인했다. 초록의 핵심은 스텝 말미에 얻는 정보를 이용하여 해당 구간에서 효율적인 method class를 선택하고 성격이 변하면 전환한다는 것이다. 여기서는 이 설계 원칙을 재사용한다. 논문 전문의 계수·알고리즘을 검증하거나 그대로 이식했다는 주장은 하지 않는다.

## 독립 검토에서 확인한 proposal 경계와 수정

초기 pilot은 각 batch의 종료점을 native session의 최종시각으로 사용했다. native DOPRI driver는 최종 accepted step 뒤에 다음 스텝 갱신을 생략하므로, 완료된 window에서 읽은 `proposed_h`는 마지막 수락 전 제안이었다. `y'=1`, `h0=0.1`에서 종료점 0.1로 만든 세션의 snapshot은 0.1이고, 전체 종료점 1 안의 중간 checkpoint 0.1에서는 실제 다음 제안 0.5가 된다.

수정본은 native session의 전체 종료점을 원래 solve의 종료점으로 유지하고, window endpoint를 등록된 중간 checkpoint로 다룬다. 전체 solve가 아직 끝나지 않았어도 해당 batch endpoint에 오류 없이 도달하면 batch 성공으로 기록한다. 전체 solve 최종점에서는 다음 제안이 계산되지 않았음을 `None`으로 남긴다. 이 반례를 직접 회귀시험에 추가했다. 최초 성능 자료는 보존하며, 수정본은 별도 `results_r1`로 측정해야 한다.

같은 수정 검토에서 quota 경계도 교정했다. native session은 중간 checkpoint를 마지막 허용 시도로 정확히 도달해도 전체 세션의 attempt budget 소진을 기록할 수 있다. 따라서 batch 성공은 요청된 endpoint에 실제 도달했고 fatal 오류가 없으며 stop reason이 없음/AttemptBudget인 경우로 정의한다. `budget_stop`은 endpoint에 못 도달했을 때만 적용한다. 전체 solve의 예산은 그대로 유지하며, `y'=1`의 한 시도 window를 전체 예산 1 및 20으로 각각 시험하여 완료된 batch를 성공으로 기록하되 전체 예산을 갱신하지 않는지 확인한다.
