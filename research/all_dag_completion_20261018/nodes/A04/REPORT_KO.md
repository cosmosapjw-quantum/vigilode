# A04 — 검증된 Leja fused 출력의 예산별 소비 계약

현재 산술 증명은 `research/cloud_completion_20261018/LEJA.md`의 L1–L3를
재사용한다. 이번 변경은 그 계산을 반복하지 않고, 검증 결과를 실제 selector나
integrator가 안전하게 소비할 수 있도록 **출력·전체 오차 상계·수용 예산을 하나의
불변 borrowed token으로 연결**한다. 실제 domain은 저장된 dense binary64
실대칭 행렬, Gershgorin으로 검증된 비양수 spectrum, 유한 입력과 기존 directed
arithmetic 계약이다.

## 현재 코드와 후속 로컬 결과

조사 기준 HEAD는 `40a36d922e915e1d1c296adc136b7e5b7676eb67`이다. 기존
`leja_phi_action_enclosed_with_workspace`는 전체 fused Euclidean 오차를
`Certified { bound }`로 반환하며, 차수 cap 때문에 원래 예산을 달성하지 못해도
그 유한 상계 자체는 유효하다. 따라서 `Certified`만 보는 소비자는 충분한
정확도를 달성했다고 잘못 판단할 수 있다. 공개 report는 불변이지만, 이 두
상태를 합쳐 확인하는 소비 함수가 없었다.

`LOCAL_REPORT_KO.md`와
`evidence/local_codex_rerun_20261018/LOCAL_RERUN_20261018.json`을 읽었다.
그 실행 source는 `412856893c3451917ea19c38ec2965f8790d3a1c`이고,
Leja의 기존 enclosed/total/setup tests 및 Fraction oracle가 성공했다.
이 기록을 현재 변경 이후 실행이라고 표현하지 않는다. 또한 그 기록에서
복구된 allocator positive control은 collective 비교 경로에 관한 것이므로,
새로운 Leja 전체 실행의 실측 allocation 주장으로 옮기지 않는다.

## 구현 계약과 짧은 증명

`CertifiedLejaReport::admit_enclosed_fused(epsilon)`는 다음을 확인한다.

1. `epsilon`은 유한한 비음수 absolute Euclidean 예산이다.
2. report schema는 명시적 enclosed API의 schema이고, operator evidence는
   Gershgorin이다. Legacy candidate와 workspace 초기 placeholder는 거절된다.
3. `total_error`는 유한 비음수 `Certified { bound: B }`이고 report의
   `total_bound`와 동일하다.
4. `B <= epsilon`이다.

통과하면 private constructor를 가진 `AdmittedLejaFused<'a>`가 report를
borrow하여 실제 `fused()` slice, `bound()`, `budget()`를 제공한다.
별도 벡터 복사나 산술 재실행은 없다. `admit_requested_budget()`는 원래
`total_budget`을 그대로 넣어 호출한다.

L1–L2가 보장하는 정확한 represented target을 F, report가 소유한 출력 벡터를
Fhat이라 하면 `||F-Fhat||_2 <= B`이다. 위 gate의 마지막 부등식과 결합하면
`||F-Fhat||_2 <= B <= epsilon`이다. Token의 private fields와 lifetime은
검사한 bound와 실제 반환 벡터의 연결을 유지한다. 이 과정은 O(1) 시간·추가
공간이며 새로운 allocation을 요청하는 연산이 없다. 이 소스 수준의 비용
성질은 전체 action의 실측 allocation·시간과 구별한다.

원래 예산을 못 맞춘 report를 **다른 독립 호출자가 명시한** 더 큰 예산에서
재사용하는 것은 이 부등식으로 정당화된다. 원래 `converged`와
`total_budget`은 변경하지 않는다. 기존 요청의 성공 여부는
`admit_requested_budget()`로만 판단하면 된다. 특정 사용자 tolerance를
수정하여 기존 실패를 성공으로 재분류하는 정책은 추가하지 않았다.

Coefficient cache의 key는 `(c.to_bits(), gamma.to_bits(), degree_cap)`이다.
고정된 내부 Leja points에서 계수와 그 enclosure는 이 값들만으로 정해지므로
같은 key를 갖는 서로 다른 operator에 계수표를 재사용할 수 있다. 실제
matvec/recurrence/fused output은 매번 새 operator로 재계산된다. 이번 검사는
동일 spectrum과 다른 off-diagonal 부호를 가진 2×2 행렬 두 개를 사용해
warm cache의 계수 setup 0회와 출력의 올바른 부호 변화를 함께 확인한다.

## 새 판별 검사

`crates/rodas5p-core/tests/leja_enclosed_contract.rs`에 다음 5개를 추가했다.

- 유효하지만 원래 예산을 넘는 capped bound: 원래 예산에서 거절하고 bound와
  정확히 같은 새 예산에서 수용한다. borrowed slice의 pointer identity와
  기존 convergence/budget 보존을 확인한다. 한 ULP 작은 예산은 거절한다.
- 수렴한 legacy report, 초기 workspace placeholder, NaN·무한·음수 예산의
  거절을 확인한다.
- 동일 coefficient key를 가진 서로 다른 operator의 cache reuse가 실제
  결과를 섞지 않는지 검사한다.
- output overflow를 일으킨 시도가 마지막 성공 report와 그 admission을
  바꾸지 않는지 검사한다.
- 정확한 zero fused action을 zero budget에서 수용한다.

Private token의 외부 위조는 새 compile-fail doctest로 검사한다.
실제 실행 결과와 code identity는 `RESULT.json` 및 `evidence/`를 참조한다.

## A08 연결과 남은 실행

A08 selector는 enclosed report의 `admit_requested_budget()` 성공 token을
받아 출력에 접근할 수 있다. 성공/거절 여부를 `converged`나 schema 문자열만으로
재구현할 필요가 없다. `LejaAdmissionError`는 `InvalidBudget`, `Uncertified`,
`BoundExceedsBudget`으로 구분되며 모두 기존 report를 보존한다.

전역 ODE 오차, sparse·비대칭·임의 callback operator, 개별 phi column의
오차는 기존 domain과 같다. 동일 정확도와 setup 경계에서의 elapsed speed는
A07의 실측 실행으로 판정한다. 기존 suffix O(m²), dense interval replay
O(m n²)의 비용은 그대로 남는다.

## Integrated execution update

Root executed all 7 leja_enclosed_contract tests and the admission compile-fail doctest successfully. Runtime restoration is complete; independent review remains pending.
