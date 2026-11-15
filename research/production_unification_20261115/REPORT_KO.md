# VigilODE 연구·코딩 체크포인트 — 2026-11-15

현재 production 판정은 **HOLD**다. 최신 연구 commit `bee5563c51264a7da19c519295bdda2fb317caef`에서 이어서, 실제 병렬 실행과 세션 경계 결함을 수정하고 새 병렬 적분 연구를 수행했다. 전체 solver 통합과 과학적 신규성은 아직 완료하지 않았다.

## 이번에 구현하고 실행한 부분

PPSW1-6b의 여섯 post-solve RHS 평가는 서로 독립이다. 기존 bounded Rayon 실행기를 재사용해 병렬화했으며, 순차 모드의 첫 오류 중단 동작은 유지했다. 병렬 job은 개별 work ledger를 반환하고 전부 합친 뒤 stage 순서의 첫 오류를 반환한다. 순수 nonlinear/nonautonomous callback에서 결과 비트와 모든 counter가 직렬 경로와 같음을 확인했다. 동시 실행 검사는 concurrency 증거이며 full-IVP speedup은 측정하지 않았다.

독립 감사는 다른 session의 attempt 수락, NaN 시간 수락, `t=1e16,h=0.01`에서 시간 전진 없이 성공하는 오류를 실제 실행으로 재현했다. 기존60개 테스트는 이 결함들을 놓쳤다. private session identity+generation, reject/NonFinite rejection 시 sibling invalidation, history dimension/finiteness, step/startup의 표현 가능한 시간 진행 검사로 수정했다. stage/tableau/controller/tolerance/Radau starter 공식과 원 RODAS5P는 바꾸지 않았다.

최종 동일 소스 Rust 실행은 **171 passed / 0 failed / 1 existing ignored**다: library123, 기존 Peer37, 병렬5, 경계6. 별도 baseline facade23개도 통과했다. Rust/Cargo1.94.1, locked/offline 의존성을 사용했고 테스트한 원격 파일과 제출 파일 SHA-256이 같다. 상세 RED/GREEN·명령·로그는 `evidence/peer_execution_receipt.json`에 있다.

엄격한 `clippy -D warnings`는 수정하지 않은 `source_owned_graph_schur.rs`의 기존 lint8개로 **실패**했다. 진단 clippy는 같은8개 경고로 exit0이며 수정 파일 경고는 없다. 형식 검사는 통과했다. workspace 전체·holdout timing은 이번에 실행하지 않았다.

## 8단계 병렬화 연구의 판정

원 RODAS5P의 변경 없는 black-box stage-task DAG는 depth8이다. 이것은 특수 source 구조를 이용하는 정확한 대수 변환까지 배제하는 정리가 아니다. 기존 affine/Schur/quotient/homotopy 작업을 재사용하고 work·span·통신·roundoff·원 residual을 별도로 검사한다. `(I-N)^-1=(I+N)(I+N^2)(I+N^4)`의 세 인자는 black-box operator 실행이 세 번이라는 뜻이 아니다.

새 실험은 자기수반·소산적 선형 분해를 갖는 별도 diagonal SDC 경로다. 두 번 sweep의 scalar L-stability를 유리함수로 확인했지만 stiff forcing의 오차가 충분히 줄지 않았다. 원 collocation 잔차에서 endpoint 대수오차 상계를 유도하고 독립 검토했다. 잔차 종료는 정확도를 회복했으나 kappa1000에서 평균8–12.24회, 최대13회 반복이 필요했다. 현재 실험은 depth/속도 목표를 만족하지 못했다. 결과를 숨기거나 오차 예산을 완화하지 않았다.

이 정리는 유한차원·고정 SPD metric H·constant L·HL=L^T H<=0·Lipschitz remainder·h||Q||ell<1 조건을 요구한다. 2D/3D/구면 diffusion의 weighted operator로 연결할 수 있으나 비정규 advection에는 별도 resolvent 근거가 필요하다. 이론/20개 IVP/10개 잔차 비교/실패 검사는 `stage_research/`에 있다. scalar toy를 실제 병렬 timing이나 production 증거로 사용하지 않는다.

## 회수한 계보와 기존 계획 교정

364개 origin branch, 도달 가능한2603개 commit,188개 PR metadata/본문을 확보했다.14개 계열의43개 원전 파일에 commit/blob/SHA를 연결했다. 모든 PR diff의 심층 감사는 아니다.

Library의 관련 제목 검색은 반환된 페이지 끝까지 조사했다.753개 관련 후보 중32개 자료를 읽고,21,502,249-byte RVJ archive SHA-256을 검증해4573개 member를 해시했다.82개 과학 코드 후보와21개 핵심 이식 대상을 정리했다. 현재 코드와 byte가 다른79개가 모두 미이식이라는 뜻은 아니다. raw 대화는 접근되지 않았고 private 대화/전체 archive를 공개 repo에 복사하지 않았다. `HISTORY_RECOVERY.json`과 `ARCHIVE_RECOVERY_CONTRACT.json`이 범위와 재취득 입력을 명시한다.

기존 frontend·native BDF1/2·Radau, multiplicity-preserving quotient trace, source-owned homotopy, guarded symbolic controller, affine/Schur 구조가 이미 있다. 새 경쟁 frontend를 만드는 계획을 수정했다. 기존 sphere Layout3(q,mu,phi)는 energy-angle 공간이며 일반 Cartesian3D가 아니다.

## 다음 턴과 machine 계약

기존 production programme를 유지하면서11개 구체 U 작업을 T 목표에 연결했다. task별 식/입력/경로/검사/실패/완료 조건, capability와 startup/fallback 권한, 동일 달성오차 비교, anti-drift 검사 및 매 턴 push를 명시했다. 과거 docs-only/반복 승인 조건은 현재 코드 작업 승인에 맞게 교정했다. official G0와 기본 routing은 그대로다.

우선 U03 독립 Peer qualification을 진행한다. step ratio sigma와 approximate-J theta는 별도 축이다. 실제 linear solve 실패 시 기존 six-solve precharge도 먼저 검사·교정해야 한다. U06 capability registry와 U08 구조 이식은 독립 진행 가능하다. 이후 common-W matrix-free, 기존 frontend의 명시적 method adapter, geometry, error-transfer/novelty, matched-error holdout으로 이어간다.

신규성은 미확립이다. 후보 기여는 source-owned 구조 인증·잔차/전역오차 전달·권한을 보존하는 병렬 portfolio와 실제 비용 우위의 결합이다. Peer-W/SDC/scan/구면 기법 자체는 선행 연구다. 현재 scope의 구현 검증과 전체 production 자격을 구별한다.

## 2026-11-15 후속: U03 PPSW1 독립 qualification (PARTIAL)

상세는 `peer_qualification/QUALIFICATION_KO.md`, 원시 근거는 `peer_qualification/logs/u03_green.log`와 `RESULTS.json`이다. production은 계속 **HOLD**다.

- 계수: 논문 식 (14)/(15)/(23)/(4)를 직접 푸는 독립 경로 R1(mpmath 60자리)로 `ppsw1::tableau`를 sigma 1e-9..1.1의 15점에서 비교했다. 최대 차이 1.4e-16 scale-eps다. 기존 계약 테스트의 oracle은 구현과 같은 닫힌형이어서 독립 비교가 아니었다.
- 실행: 다항식 5차 정확성, Prothero–Robinson lambda -1..-1e6(균일 보폭 관측 차수 약 6), LIN3 비정규(독립 numpy와 0.988–1.0003배 일치), sigma 0.5..1.1 패턴, final clipping 1e-12까지 오차 증가 없음, 시작 h0, 추정기, 적응 driver(tol 1e-3..1e-12, 전체 작업 counter)를 실제 구현으로 실행했다.
- theta 축(독립 backend): theta≠1은 L-안정성을 잃는다. 비정규 stiff 문제에서 theta 0.8/0.85 발산, 0.9 비단조, 0.95..1.1과 비가환 perturbation 1e-2 이하만 수렴했다.
- 수정 C1: `solve_stages`가 실패 가능한 LU solve 전에 6회를 선부과하던 회계를 실행 기준으로 고쳤다(RED 재현 후 GREEN, 기존 171개 통과).
- 열린 결함: F1 h0 규칙이 tolerance를 무시해 starter가 구간 전체를 덮을 수 있음(tol 1e-9에서 74배 초과), F2 cap+hysteresis 성장 제한으로 tol과 무관한 step 수, F3 추정기 잡음 바닥이 경고 모델보다 커서 tol 1e-12에서 h가 1e-19로 붕괴 후 typed 오류. 비선형·matched-error·시간/RSS는 NOT_RUN이다.
- capability: per-step Newton-free지만 Jacobian-free/derivative-free/matrix-free가 아니다. startup은 simplified Newton과 dense 5n×5n LU를 쓴다. fallback은 없다.
