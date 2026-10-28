# 독립 결정 검토 — 2026-10-10

판정은 **ACCEPT_BOUNDED_NATIVE_RESEARCH_SEAM**이다. 현재 binary64 입력이 나타내는 실수 2×2 선형계의 연구용 인증 시제품은 후속 이식 대상으로 유지할 수 있다. production, 전체 solver 정확도, 전역 오차, 속도 및 기본 경로 승격은 모두 **HOLD**다. 검토자는 이 연구를 설계하지 않았고, native 실행이나 기존 연구 campaign을 재실행하지 않았다. 실제 모델 식별자는 UNKNOWN이다.

검토 소스는 `95d589e862c35c675fac3bee7ebdb962415a97de`, 사전등록은 `60645e213f8a5cc38c9c96063ed5ebcba89246f3`이다. interval determinant가 0을 제외할 때 interval inverse를 구성하고, 현재 W로 `b-Wx`를 재계산하여 보정량을 포함하는 구조는 타당하다. 양의 scale로 나눈 보정량의 infinity bound는 WRMS도 상계한다. 기본 연산의 round-to-nearest와 gradual underflow, 인접 endpoint widening 및 nonfinite 거절이 이 주장에 필요하다. 소유한 W/scales와 비공개 필드는 현재 API를 통한 인증 객체 위조를 막는다.

독립 `Fraction` 검사는 기존 checker를 가져오지 않고 30개 입력의 사전등록 identity와 determinant, inverse, residual, correction, weighted correction, inverse gain 및 WRMS bound를 확인했다. 모두 통과했다. 3개 discovery 입력에서는 의도한 perturbation이 binary64 표현에서 사라져 실제 오차가 0이다. 나머지 27개와 holdout 4개는 오차가 0이 아니다. 일부 bound/error는 약 1.4×10^15이므로 포함관계 통과가 유용한 인증이나 효율을 뜻하지 않는다.

첫 실행의 FAIL 보존과 fixture 수정은 정당하다. 원래 W=I, b=x, 양의 최소 subnormal scale은 유효한 입력이며 exact error는 0, exact scaled inverse gain은 1이다. 이를 invalid로 분류한 것이 오류였다. 수정된 triangular W는 scale 비율 때문에 gain이 binary64 최대값을 초과한다. 양성/holdout 결과와 구현 적대 사례의 raw payload는 두 실행 사이 동일하며, 인증 코드와 판정 기준도 바뀌지 않았다. 따라서 이를 처음부터 성공한 실행처럼 표현하면 안 된다.

predictive controller 사례는 양의 fifth power의 exact 비교로도 올바른 factor=0.2와 관측 factor=5.0의 차이가 확인된다. overflow 사례는 공개 staged solver와 실제 driver의 callback predicate를 복제한 경계의 재현이며 전체 driver/ODE 궤적 재현은 아니다. 세 개 StallAccepted/FloorAccepted 사례 및 fallback만 charge하는 코드와의 결합은 확인되지만 전역 오차 결함을 입증하지 않는다. ALG06 malformed 복사 입력 네 개의 PASS는 checker 검증 결함이며, 기존 원본 데이터가 손상됐다는 증거는 아니다.

현재 `check.py`는 형상, 개수, partition과 수치 포함관계를 확인하지만 전체 입력 manifest를 강제하지 않는다. 이번 검토에서 모든 양성 입력을 별도로 대조했으므로 현재 결과의 blocker는 아니다. 반복 사용 가능한 승격 도구로 확장할 때에는 fixture manifest, 모든 필수 negative ID, source/operator identity까지 검사해야 한다. W 형성 오차, approximate JVP, off-block coupling, 비선형 stage 전달 및 시간 누적 오차는 이번 인증 범위 밖이다.

재현 가능한 검토 기록은 `independent_checks.py`, `independent_exact_checks.json`, 판정 및 입력 hash는 `independent_decision.json`에 있다. 최종 보고서와 DAG도 이 동일한 검토에서 확인했다. 22개 node의 의존관계는 acyclic이며, 9개 상대 Markdown 링크와 finding/claim의 증거 경로·JSON pointer가 모두 유효하다. 보고서의 수치·callback 경계·최초 실패·승격 HOLD는 raw evidence와 일치한다. AS02는 기존 predictive error floor를 보존하고, homotopy contraction에는 theta와 lambda의 [0,1] 범위 및 일반화 시 |eta| 조건이 명시됐다. 사전등록의 원래 6,118바이트는 그대로 남고 결과만 뒤에 추가됐다. 최종 문서 hash는 판정 JSON에 기록했으며 추가 과학 실행 없이 **최종 artifact 검토 PASS**로 닫는다.
