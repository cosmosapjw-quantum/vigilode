# QA01: 선언된 polynomial 관계를 사용하는 선형 단계 커널

## 결론과 선택 범위

이번 변경은 알려진 낮은 차수 관계를 갖는 연산자에 한하여 선형계 `(I-B)x=b`를 Jacobian 조립, AD/JVP 추정, Newton 또는 Krylov basis 없이 계산하는 선택적 Rust 커널이다. 여기서 `B=sA`는 이미 시간 간격과 계수가 곱해진 **선언된 구조 연산자**다. 임의 nonlinear callback에서 이러한 관계를 자동 발견하거나, 일반 ODE driver를 자동 승격시키는 구현은 아니다.

세 후보 중 quotient polynomial을 구조가 확인된 경우에 채택한다. Neumann 반복은 `||B||<1`에 적합한 대조군으로 유지하고, 작은 명시적 block에서는 이미 존재하는 직접 공식이 더 적은 작업을 하므로 유지한다. 12행의 작은 결정적 실험에서 quotient는 residual 확인을 포함하여 1–2회 operator action으로 네 경우를 모두 해결했다. Neumann은 수축 대조군에서 20회, 강한 감쇠·Jordan·빠른 회전에서는 65회 후에도 실패했다. 이는 해당 사례의 **연산 횟수와 정확도** 결과이며 wall-clock speedup, 일반 Krylov 우월성 또는 전체 ODE 적분 속도 향상이 아니다.

## 이전 증명의 재사용과 이번 확장

`../../adaptive_causal_20261023/analysis/ALGEBRAIC_TRANSFER_KO.md`의 AT1을 재사용한다. 그 문서는 `../../..`의 기존 promotion theory에서 Laguerre/Chebyshev recurrence와 near-annihilator 개념을 추적한다. 이번 변경은 그 근거를 다시 대규모 재검증하는 대신, 실제 API의 scaled-operator 계약과 실패 처리, 원래 공간 residual을 추가한다. 이전 draft의 finite quotient algebra 아이디어를 옮기는 대상은 operator polynomial 계산이다. 대수기하 논문의 열거 결과나 기하학적 root 개수를 ODE 성능 수치로 옮기지 않는다.

## 명제 P1: 낮은 차수 관계가 보장하는 계산 절감

`B`는 유한차원 선형 연산자, `q(z)=sum_{k=0}^d q_k z^k`, `q_d !=0`, `d>=1`이라 하자. `q(1)!=0`이면

\[
p(z)=\frac{q(z)-q(1)}{q(1)(z-1)},\qquad
(I-B)p(B)=I-\frac{q(B)}{q(1)}.
\]

**증명.** `1-z=-(z-1)`을 곱하면 항등식이 바로 나온다. Polynomial 항등식에 `B`를 대입할 수 있다. `q(B)=0`이면 우변이 항등연산자이므로 `p(B)`가 `(I-B)`의 역연산자다. 유한차원에서는 한쪽 역연산자가 존재하면 가역이다. 대각화나 서로 다른 root 가정이 없으므로 Jordan block과 repeated root도 포함한다. □

Synthetic division을 1에서 수행하면 `t_{d-1}=q_d`, `t_{k-1}=q_k+t_k`, `q(1)=q_0+t_0`, `p_k=t_k/q(1)`이다. 준비는 `O(d)` scalar arithmetic, 후보 `p(B)b`는 Horner에 의하여 정확히 `d-1`회 operator action과 `O(d n)` vector arithmetic을 요구한다. 구현의 원래 residual 검사는 반드시 **추가 1회**의 `B` action을 사용하므로 호출당 총합은 `d`회다. Workspace는 길이 `n`인 두 벡터이고 재사용 호출 중 heap allocation은 없다. Callback 내부 allocation은 API가 보장하지 않는다.

이것은 차수 `m`의 polynomial 후보와 residual을 계산하는 경로의 `m+1`회 action에 비하여 `m+1-d`회를 줄인다. `d<m+1`이고 주어진 관계가 참일 때의 조건부 계산량 정리다. 관계를 발견하는 비용, operator provider 구성비, nonlinear remainder 처리, 전처리 또는 ODE 오차 제어 비용은 이 정리에 포함하지 않는다.

## 명제 P2: 근사 관계와 원래 residual의 연결

동일한 induced norm에서 `eta=||q(B)||/|q(1)|<1`이면

\[
\|(I-B)^{-1}\|\le
\frac{\|p(B)\|}{1-\eta}
\le \frac{\sum_{k=0}^{d-1}|p_k|\rho^k}{1-\eta},\quad \rho\ge\|B\|.
\]

**증명.** `E=q(B)/q(1)`라 두면 `(I-B)p(B)=I-E`이고 `||E||<1`이므로 `I-E`의 역은 Neumann 급수로 존재하며 norm은 `1/(1-eta)` 이하이다. 따라서 `(I-B)^{-1}=p(B)(I-E)^{-1}`이며 삼각부등식과 submultiplicativity를 적용한다. □

실제 후보 `xhat`의 원래 residual `r=b-xhat+Bxhat`에 대하여 `||x-xhat|| <= inverse_bound * ||r||`다. residual 평가의 절대 오차 상한이 `delta_r`이면 측정 norm 대신 `||r_measured||+delta_r`를 사용한다. 이것은 후보 polynomial의 반올림 오차가 별도로 완벽히 제어되지 않아도 원래 선형계에 대한 residual로 후보를 검사할 수 있다는 장점이 있다.

구현은 infinity norm을 사용한다. `relation_norm`과 `operator_norm`은 caller가 제공한 **전 공간의 수학적 상한**이며 kernel이 증명하지 않는다. 계산된 `conditional_error_estimate`의 bound arithmetic은 outward rounding이 아니므로 엄밀한 floating-point certificate로 표시하지 않는다. 실제 `residual_accepted`는 지정한 absolute residual tolerance에 대한 operational 판정이다. 이 판정을 ODE 상태 오차 수락으로 직접 바꾸지 않는다.

## 명제 P3: probe만으로 관계를 추정하면 왜 실패하는가

`B=[[-2,4],[0,-2]]`에서 `q(z)=z+2`를 택하면 `q(B)e1=0`이나 `q(B)e2=4e1`이다. 따라서 하나의 seed에서 정확한 관계를 발견해도 전 공간 관계가 아니다. 정확한 `||q(B)||inf=4`를 제공하면 `eta=4/3>=1`이므로 생성자가 거절한다. 잘못된 zero witness를 제공하면 커널이 그 거짓 자체를 알아낼 수는 없지만, `b=e2`에 대한 원래 residual은 `4/3`이므로 해당 후보를 수락하지 않는다. 반대로 `b=e1`에서 residual이 작다는 사실은 여전히 전 공간 witness를 증명하지 않는다.

정확한 degree-two 관계 `(B+2I)^2=0`를 제공하면 역연산자는 `(B+5I)/9`로 계산되고 repeated root를 회피할 필요가 없다. 구조 증명과 실제 RHS 검증은 서로 다른 역할을 맡는다.

## 안정성 설계와 코딩 결과

파일은 `crates/rodas5p-integrators/src/polynomial_resolvent.rs`다. 공개 모듈은 `DeclaredPolynomialResolvent`, `PolynomialResolventWorkspace`, typed error 및 report를 제공한다. 기존 자동 전환 경로는 변경하지 않는다.

1. 이미 scaled된 `B=sA` 관계를 받으므로 작은 `s`에서 `1/s`를 만들지 않는다. Caller는 각 새 `s`에 맞는 관계와 witness를 갱신해야 한다.
2. Coefficient 전체를 최대 절댓값으로 나눈 뒤 synthetic division을 수행한다. 이 정규화가 nonzero coefficient를 underflow시켜 degree나 관계를 바꾸면 거절한다.
3. `|q(1)|/sum|q_k|`가 caller margin 또는 길이에 따른 roundoff floor 이하이면 거절한다. Discriminant가 작다는 이유만으로 거절하지 않는다.
4. `eta>=1`, nonfinite bound, 잘못된 차원/허용오차를 거절한다. 매우 비정규적인 연산자에서는 `rho` 기반 bound가 보수적일 수 있으나 작은 residual만 보고 이를 숨기지 않는다.
5. Operator output을 NaN으로 초기화하여 callback의 부분 쓰기를 검출한다. 후보와 원래 residual을 workspace에서 계산하고, 모두 통과한 경우에만 caller output으로 복사한다.
6. 8개의 의미 있는 test가 repeated/Jordan root, nilpotency, near relation, 잘못된 probe inference, pole/cancellation, scaling, callback failure, transactional output을 검사하며 모두 통과했다.

## 수치 비교와 가지치기

`../results/polynomial_campaign.json`은 `frontier_polynomial` example의 12행이다. 비교 대상은 동일한 네 선형계와 absolute residual target `1e-11`이다. 최종 Neumann residual은 최종 반환 후보에서 다시 계산하며 이 action도 비용에 포함한다.

| 연산자 | Quotient 총 action | Neumann 총 action | Neumann 판정 | Quotient 참조값 대비 절대 오차 |
|---|---:|---:|---|---:|
| 대각 `B=-0.25 I` | 1 | 20 | 통과 | 0 |
| 대각 `B=-1000 I` | 1 | 65 | 실패 | 4.34e-19 |
| Jordan `[-2,4;0,-2]` | 2 | 65 | 실패 | 1.11e-16 |
| 감쇠·회전 `[-1,-100;100,-1]` | 2 | 65 | 실패 | 0 |

표의 참조값은 독립 형태의 2×2 역행렬 공식으로 계산한 binary64 값이다. 따라서 0이라는 항목은 실수 해와 bit-exact하다는 뜻이 아니라 두 floating-point 계산의 결과가 같다는 뜻이다. 별도의 nilpotent test는 정확히 표현 가능한 정수 답을 검사한다. 이 작은 실험을 arbitrary-precision 인증으로 표시하지 않는다.

2×2 직접 공식은 이 네 경우에서 합법적인 specialized alternative다. Operator callback 횟수는 0이지만 명시적 계수 취득 및 산술은 무료가 아니므로 이 지표만으로 직접 공식의 wall-clock 배수를 추정하지 않는다. Quotient의 장점은 일반 작은 행렬을 만드는 데 있지 않고, 전 공간의 `B` action이 싸고 관계 차수가 공간 차원보다 훨씬 작을 때 나타난다.

## 격자 PDE에 대한 적용 조건과 다음 구현

일반 sphere/2D/3D Laplacian은 격자 refinement에 따라 서로 다른 eigenvalue 수가 증가하므로 전역 최소 polynomial의 차수가 작다고 가정할 수 없다. 따라서 이 커널만으로 일반 확산 PDE를 dimension-independent하게 해결한다고 주장하지 않는다. 실제 접점은 다음과 같다.

- **Local collision block:** 동일한 유한차수 관계가 각 셀의 물리 충돌 연산자에 성립하면 direct sum에서도 같은 관계가 유지된다. `n`이 셀 수에 비례해 증가해도 `d`는 고정되어 총비용은 `O(d n)`이다. 서로 다른 block별 계수는 공통 polynomial의 차수를 증가시킬 수 있으므로 각 block별 plan을 사용한다.
- **Low-rank angular collision:** 이미 증명된 Thomson 저계수 구조를 named provider로 연결하고, 가중 물리 norm에서 witness를 제공한다. 현재 infinity-norm API에 가중 norm을 전달하는 변환 비용과 양성/보존성은 별도 검증한다.
- **Split transport/collision:** transport discretization과 collision subflow를 구분하여 이 커널은 local collision 또는 preconditioner에 사용한다. Splitting commutator 오차와 temporal order를 별도로 측정한다.

다음 단계는 (i) provider identity/epoch에 시간 간격별 관계를 묶어 stale witness 사용을 방지하고, (ii) 물리적으로 유도된 block-collision client를 causal/RODAS stage에 명시적 선택으로 연결하며, (iii) 원래 full-space stage residual과 전체 궤적 오차 기준을 유지한 채 reference solver와 비교하는 것이다. 이 세 단계가 끝나기 전 production의 일반 경로로 자동 선택하지 않는다.

## 이론/코딩 연구 루프 실행 기록

R1은 QA01의 구조와 witness 부재를 문제로 정식화했다. R2에서 기존 AT1을 재사용하고, R3에서 quotient/Neumann/direct의 세 후보를 구성했다. R4–R5에서 P1–P2의 연산량·오차 연결을 명시하고, R6에서 Jordan/probe 반례를 도출했다. R7에서 scaled chart와 pole 조건을 정리했고, R8–R9에서 실제 원래 residual/참해 오차/작업 수를 비교했으며, R10에서 일반 driver 미연결을 남은 조건으로 기록했다.

C1–C2는 별도 module과 public contract를 설계했다. C3–C4는 allocation-free reusable workspace와 transactional output을 구현했다. C5–C6은 failure/pole/nonnormal 테스트와 12행 비교를 실행했다. C7에서 Neumann 최종 후보와 residual의 시점을 일치시키고 그 마지막 callback 비용을 포함하도록 수정했다. C8은 targeted test 8/8 통과, C9는 독립 hostile review로 전달, C10은 machine-readable candidate ledger와 재현 명령을 보존했다. C9 이후 발견된 사항은 독립 감사 문서가 최종 상태를 기록한다.

재현:

```bash
cargo test --locked --offline -p rodas5p-integrators --test frontier_polynomial
cargo run --locked --offline -p rodas5p-integrators --example frontier_polynomial
```
