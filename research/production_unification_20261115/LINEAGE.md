# VigilODE 연구 계보 재조사 — 2026-11-15

현재 연구 기준은 `bee5563c51264a7da19c519295bdda2fb317caef`이다. 기본 브랜치 `main`의 `8d0c79184e09efb5bdadc24a6315c60a71a44264`만 읽으면 이후 수개월의 코드가 누락된다. 현재 연구 기준은 PR188의 `ef04bfab6926fcd4ba5f5fa2bbd8696f5e69d011`을 포함한다. 구현·근거·권한은 기본 브랜치 이름이나 가장 최근 날짜만으로 결정하지 않는다.

`git_inventory.json`은 광고된 origin 브랜치 **364개**, 그 로컬 clone에서 도달 가능한 커밋 **2,603개**와 부모 관계를 기록한다. `pr_inventory.json`은 GitHub API의 `state=all&per_page=100` 두 페이지에서 **188개 PR**의 공개 본문과 정확한 head/base SHA를 회수했다. 158개가 open, 24개가 merged, 6개가 closed-unmerged다. 모든 PR diff·브랜치 tree를 심층 검증한 것은 아니다. 삭제된 ref, unreachable object, 비공개 대화 전체는 이 조사 범위 밖이다.

`lineage.json`에는 14개 계열과 43개 source binding의 commit/blob/SHA-256을 넣었다. 각 상태는 소스 검사와 역사 기록을 뜻하며, 이 조사 작업자가 native 검사를 새로 실행했다는 뜻이 아니다. 문서의 과거 PASS와 현재 재현 PASS는 구분한다.

## 재사용할 실질적인 수학과 코드

| 계열 | 회수한 실제 산출물 | 적용 한계와 다음 이식 |
|---|---|---|
| 기존 frontend | `crates/vigilode`의 `Problem/Options/Solution`, RODAS5P protected routing | 새 frontend 타입을 복제하지 않고 method adapter/capability/history를 확장한다. |
| 기존 타 적분기 | `bdf.rs`, `radau.rs`의 public fixed/adaptive integrator와 DOPRI5/exponential | BDF/Radau는 계획에만 있는 후보가 아니다. 허용된 fallback과 독립 comparator로 연결하되 자체 정확도 검증이 필요하다. |
| common-W/JVP | PR74–77의 coupled solver, shared Krylov, residual-first, causal transaction | JVP는 derivative를 사용한다. small projected LU와 전체 state matrix 생성을 구별한다. |
| RHS-only homotopy | PR91–92,117–119,150–156,177의 value-map, residual-dominance, source monotone inverse | invariant domain과 contraction witness가 적용되는 범위에서 사용한다. BE root를 원 RODAS raw-U stage로 바꾸어 부르지 않는다. |
| 대수적 resolvent | PR93,184 및 `polynomial_resolvent.rs`, `conditioned_polynomial.rs` | `(I-B)p(B)=I-q(B)/q(1)`의 full-space witness와 original residual을 유지한다. 유한 probe로 polynomial identity를 인증하지 않는다. |
| 특이점/대수기하 | PR97–98 `trace_quotient.rs`: monic quartic quotient의 multiplicity-preserving trace | `a=b=3/4` triple root에서도 다중도를 보존한다. 모든 복소근의 합이며 selected-real branch의 대체물이 아니다. |
| algebraic closure | `cyclic_closure.rs`의 원 방정식과 전체 slab certificate, Bernstein bound | 고정 모델·선택된 branch·incoming radius·시계 연속성을 보존해야 한다. |
| 동역학 | signed logarithmic norm, affine error transport, Gray–Scott lognorm | 감쇠 대각 부호를 지우지 않는다. 비정규계에서 eigenvalue-only stiffness 판별은 불충분하다. |
| symbolic 실행 | `SwitchState`의 legal outcome word에 대한 ownership-transfer refinement | 실제 buffer swap이 구현되어 protected reference 갱신마다 n 좌표 복사를 제거한다. ODE 위상공간의 일반 symbolic abstraction은 별도 문제다. |
| causal packet | PR135–147의 damped convolution, rank-two Volterra, represented-clock 및 누적 floating error | source closure와 초기·이전 window error를 포함해야 한다. 일반 nonlinear stage independence는 따라오지 않는다. |
| graph-Schur/scan | PR185–187의 source-positive cubic, feedback core <=3, path forest와 original-W residual | 외부 8단계는 유지된다. 역사적 full-IVP median wall ratio 2.242032452의 손해를 negative evidence로 보존한다. |
| Peer-W | PR188의 PTSW2B 및 PPSW1-6b | 6개의 동일 W solve가 독립인 별도 방법이다. 현재 dense Jacobian/LU와 Radau/Newton startup을 전체 Newton-free로 표현하지 않는다. |
| 2D/구면 | Gray–Scott periodic 2D와 photon `(q,mu,phi)` finite-volume adapter | 후자는 energy-angle phase-space다. 일반 Cartesian 3D 공간 backend를 이미 구현했다고 해석하지 않는다. |
| algebraic topology | 그래프 cycle-core/path 구조는 존재 | 일반 chain complex/homology backend는 확인되지 않았다. oriented mesh incidence와 boundary-of-boundary identity부터 구체적으로 연결한다. |

## 현재 기준에 포함되지 않은 유용한 분기

`research/vigilode-dyn-symbolic-c8-20261102`의 `3336d682139fb2096316e301dd635c6b91b0935e`에는 네 개의 설계 문서가 있고 C8은 **PENDING**이다. 중요한 미실행 항목은 다음과 같다.

1. 기존 Hillis–Steele `O(n log n)` scan과, 순서를 보존하는 Blelloch `O(n)` work scan 및 chain 간 병렬화의 실제 비교.
2. 원래 W를 유지하는 stage backend 전환과 전체 IVP method 전환의 분리.
3. `alpha(J)<0`지만 `mu_2(J)>0`인 nonnormal transient 판별 및 detector 비용 회계.
4. hysteresis/dwell-time으로 router chatter를 막되, 안정성은 실제 error-transfer map의 Lyapunov/제품 norm bound로 검증.

이 분기는 현재 기준에 merge되지 않았다. 이식할 때 문서 전체를 무비판적으로 merge하기보다 위 대상과 정확한 source SHA를 활성 DAG 노드에 연결한다.

## anti-drift 계약에 반영할 교정

- `jacobian-free`, `derivative-free`, `matrix-free`, `Newton-free`를 독립 capability로 기록한다. JVP/FD/AD, projected dense algebra, startup 및 fallback의 capability를 전체 실행의 합으로 판정한다.
- numerical method, linear stage backend, source-specific algebraic provider, physical client, routing policy를 별도 identity로 유지한다.
- 정확한 quotient trace와 일반 selected-root closure를 합치지 않는다. 다중도 제거는 표현 최적화가 아니라 목표 변경이다.
- exact affine scan은 nonlinear stage graph에 대한 일반 병렬화 증명이 아니다. scan의 rounding reassociation과 residual 검사를 유지한다.
- 명시적 boundary map의 기하학적 항등식은 공간 operator 검증이다. 시간 적분의 stiff stability나 속도를 대신하지 않는다.
- test 파일 존재, 역사적 test receipt, 이번 실행 PASS, production qualification을 별도 필드로 관리한다.

## 다음 이식의 최소 판별 실험

`lineage.json`의 `recommended_ports`는 aggregate/selected-branch semantic guard, guarded method-history transition, work-efficient affine scan, geometry incidence의 네 계약을 제공한다. 각각 원래 목표와 오차를 유지하는지 확인하는 작은 반례/불변량 검사를 먼저 수행하고, 통과한 경로만 matched achieved-error whole-IVP 비용 비교로 진행한다. 이는 기존 연구를 재사용하는 구체적 작업이며 새로운 범용 solver 성과를 미리 주장하는 계획이 아니다.
