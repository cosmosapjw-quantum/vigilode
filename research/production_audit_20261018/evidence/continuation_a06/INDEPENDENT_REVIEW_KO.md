# A06 단계 RHS 범위 독립 재검토

판정: 첫 stage RHS 대응 `SCOPED_PASS` 유지. 전체 A06 `HOLD`.

기존 post-EXEC 검사 6/6 exit 0은 실행 commit `f5de5b2cea2fdfa04bcac3bd093456c866dccee9`와 결과 해시 `bfb1c94128f8b4aee612a0fe231d91b4e8d0c1859eff9f3c9dfab7d7ef27e591` 아래 재사용 가능하다. 현재 audit HEAD `d087dab9a8dacff0b195566c4a649273acdb476c` 이후 관련 소스, 회귀, 이론, tableau, Cargo.lock은 변경되지 않았다.

검증된 주장: identity mass에서 `A=I-hγJ`, `s=hγ`, `Ax=r0`; 고정 fixture의 `r0=h f(y0)=(-1/8,0)`가 solve_into에 전달되는 첫-stage 대응. 추가 `s`를 곱한 negative control은 `(1-s)r0` shifted residual을 만든다. 테스트는 일반 stage RHS buffer 포착이나 API의 자동 오류 거부를 주장하지 않는다.

남은 항목: 비선형 전체 stage RHS와 coupling 연결, generated model/backend dispatch 결속, setup+모든 stage+재구성+residual witness의 비용, 전체 경로 zero-allocation, trajectory/output/reference/비교. 현재 sequential 경로의 stage별 Vec 생성·복사 때문에 전체 경로 zero-allocation은 승격 불가다.
