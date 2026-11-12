# VigilODE 현행 코드 및 Production-readiness 비판 감사

기준은 2026-11-12 현재 확인된 PR #188의 draft HEAD `ef04bfab6926fcd4ba5f5fa2bbd8696f5e69d011`이다. `main`은 2026-08-26 커밋 `8d0c7918...` 이후 오래되어 현재 연구 코드를 대표하지 않는다. PR #188은 **Production HOLD / G0 미충족 / 릴리스 아님**이라고 명시한다. 공식 G0는 23 nodes/34 edges, 진행 7/23, successor 0/12이며, 목표 집합 전체를 입증하지 못했다. 원 G0는 수정하지 않는다.

## A. Production solver 판정

**판정: NOT QUALIFIED.** 이유는 기능 부족보다 전체 solver qualification의 부재다. 매칭된 전역 출력오차의 full-IVP 비교(PW06), 독립 리뷰(T09), strong-stiff global order, W-tolerance window, 전체 워크스페이스 테스트, 모든 fallback의 비용/권한 회계가 완결되지 않았다. 현재 green contract tests와 synthetic e2e 결과는 구현 계약의 일부를 확인하는 증거이지 production performance나 일반 정확도 인증이 아니다.

## B. RODAS5P 8-stage 병렬화

고정된 coefficient snapshot과 변환 코드에 따라 `Gamma=(I/gamma-C)^(-1)`, `alpha=A Gamma`를 계산했다. 표현된 모든 8단계 strict-lower predecessor가 비영이고 각 stage i의 predecessor 수는 [0,1,2,3,4,5,6,7]이다. dependency edges=28, critical depth=8, same-step antichain width=1. 따라서 **일반 비선형 IVP에서 원래 tableau와 stage equation을 그대로 두고 8 stage task를 정확히 동시에 실행할 수 없다.** 뒤 stage RHS에 앞 stage 상태/미분값이 실제로 필요하다.

다른 병렬화 축은 남는다: vectorized RHS, JVP/operator actions, Krylov matvec/reduction, preconditioner, 여러 독립 IVP, spatial MOL. Affine prefix scan은 affine recurrence에만 exact이며, dense matrix composition의 추가 work/통신을 함께 측정해야 한다. Predictor-corrector, stale stage, graph-Schur, finite homotopy correction은 별도 변형법으로 기록한다. 기존 PR187 native graph-Schur는 원본 8-stage full IVP 12개 사례에서 fast-direct보다 느렸고 median wall ratio 2.242032452였다. 이 실패는 제거하지 말고 negative control로 보존한다.

## C. 진짜 stage-parallel 경로: Peer-W

PR188에는 `PPSW1-6b`가 opt-in으로 들어왔다. 6-stage, singly implicit, variable-step order-5 Peer-W; 한 step의 6개 shifted linear system은 독립이며 common W factorization 1회 후 6개 RHS solve가 가능하다. 그러나 time-step history는 여전히 순차적이다. 현재 코드는 dense Jacobian/dense W/LU를 쓰므로 matrix-free 또는 derivative-free가 아니다. 각 step의 Peer linear solve는 Newton iteration을 사용하지 않지만 startup에서 Radau IIA/Newton을 쓰므로 전체 알고리즘을 Newton-free라고 부를 수 없다. 현재 solve는 parallel map이지만 RHS 조립과 새 stage f 평가 루프는 순차다.

추정기(5개 이전 stage의 degree-4 predictor 차이)와 controller는 일부 논문의 산문을 해석한 owner choice/derived choice이지, 독립 인증된 전역오차 bound가 아니다. estimator 가중치의 큰 합으로 인한 반올림/노이즈도 평가해야 한다. 강성 전역 차수, W tolerance 0.8-1.0, startup의 transient resolution, last-step clipping, events, matrix-free, mass matrix, frontend, matched-error full-IVP qualification이 남아 있다.

## D. 계보/수학 이식 지도

선별한 주요 PR 계보: PR74/75/77 coupled stage homotopy; PR90/91/92/93 performance, error/work, fused polynomial actions; PR105 residual GMRES prefix; PR117 nonlinear RK stage root; PR123 raw-U vs derivative-free BE stage; PR125 achieved-work precision; PR135 causal convolution/affine prefix; PR136 rank-2 Volterra; PR147 original RODAS source guard; PR150 RHS-only homotopy; PR163 common achieved-error; PR176 G0 reconstruction/method inventory; PR177 monotone algebraic homotopy; PR179 causal frontier/depth-work tradeoff; PR184 Cayley-Hamilton W; PR185 graph cycle-core Schur; PR186 affine path scan; PR187 native graph-Schur negative benchmark; PR188 Peer-W opt-in modules.

수학 성숙도:
- Algebraic/polynomial/resolvent/conditioned chart: 부분 구현, 상당 부분 source-owned.
- ODE dynamical systems: log-norm/nonnormality/stability 분석 일부.
- Symbolic dynamics: 일반 dynamical system 모듈로 확인되지 않음. finite-state controller는 symbolic dynamics가 아님.
- Algebraic topology: 일반 chain complex/homology/cohomology 통합 모듈 확인되지 않음. graph cycle-core와 동일시하지 말 것.
- Homotopy: RHS-only/JVP/source-specific stage correction 경로 존재하지만 generic original-residual-to-LTE theorem은 미완.
- Spatial clients: Gray-Scott 2D MOL 존재. Sphere P06은 제한된 photon finite-volume client로 일반 spherical ODE/PDE solver가 아님. 일반 3D MOL backend는 식별되지 않음.

검색된 history matrix metadata에는 187 PR과 361 branch refs가 있으나 모든 PR diff와 모든 branch를 개별 심층 리뷰한 것은 아니다. 선별 lineage와 source 파일을 추적했다.

## E. Provenance 문제

1. `OWNER_DECISIONS_20261110.json`의 OD-23..27 날짜가 2026-11-13으로 기록되어 2026-11-12 감사 시점보다 미래다. 오너 재확인 없이 승인 근거로 쓰지 않는다.
2. `PW04_PPSW1_6BC_VERIFICATION.json`은 bare `NaN`/`Infinity`가 있어 strict RFC8259 JSON이 아니다. 과거 증거를 조용히 덮어쓰지 않는다.
3. `METHOD_PORTFOLIO_20261109.json`은 PPSW1 고차 Peer-W Rust 구현 전으로 작성된 stale inventory다.
4. 기존 PR188 receipt는 같은 agent lineage 의존성이 있으며 independent review와 full-IVP evidence를 대체하지 못한다.

## F. Novelty/significance

Parallel Peer-W, Peer-W global error control, JFNK/JVP, prefix scans, spherical DFS, surface RBF MOL은 모두 prior art가 있다. 단순 통합이나 재패키징은 수치해석 novelty가 아니다. 후보는 (i) original-residual safeguard를 갖는 matrix-free common-W block-Krylov Peer-W, (ii) RHS-only implicit Peer와 residual-to-local-error theorem, (iii) nonnormal/stiff MOL에서 method-history/error-transfer certificate와 permission-safe transactional fallback, (iv) 구조-aware portfolio가 matched achieved-error whole-IVP speedup을 보이는 새 problem class다. 각 후보는 구체적 이론/알고리즘 차이와 독립 benchmark를 확보해야만 novelty 주장을 허용한다.

## G. 다음 단계

6-turn plan은 `TURN_PLAN.json`에 있다. 다음은 코드 추가가 아니라 T01 fair-ab reference replay 및 matched achieved-error authority 확립이다. 이후 PPSW1 qualification, Peer stage/block Krylov, 통합 frontend, 2D/3D/spherical MOL, independent novelty/production review 순서다. 매 턴 별도 branch, immutable backup, remote readback을 수행한다.
