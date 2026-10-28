# 최신 외부 비교 자료의 독립 감사

감사 대상은 `research/nonstiff-explicit-20261023`의 게시 commit
`fbc3c272fa06b443de148631ea78bfa6a34f1b2b`다. 이 commit은 기존 비강성
DOPRI5 구현에 diffsol·Julia 비교 자료를 추가한다. 실행된 native source는
`45037fc24e8f8101605f309e1fdb3a9a4e3df56d`, 외부 adapter 실행 source는
`1cf87238ab79ee698621d80bc0a3afd903d2f6ff`다. `git diff`로 확인한 결과
45037fc2와 fbc3c272 사이 `crates/`, root Cargo.toml/Cargo.lock은 바뀌지 않았다.
따라서 기존 커널 차수 증명·계약 시험은 그대로 재사용하고, 새 비교와 그 해석을 감사했다.
이 문서는 이번 작업에서 수정할 소스의 최종 검증을 대신하지 않는다.

**판정:** 최신 게시 자료에서 새로 확인된 해시 훼손이나 수치 표의 잘못된 선택은 없다.
비강성의 명시적 선택 경로는 제한된 문제군에 사용할 근거가 있지만, 이 자료만으로 범용
production 사용을 승인할 수는 없다. 성능비는 여전히 탐색적 측정이다. 이는 기존 보고서의
결론과 일치한다. 이번 감사에서는 solver를 다시 실행하지 않았다.

## 독립 확인 범위

원자료를 읽는 별도 Python 스크립트로 다음을 확인했다. 기존 analyzer 함수를 호출하지
않고 `math` 함수로 각 fixture의 해석해를 다시 계산했다. 이 계산은 보관된 결과의
독립 재채점이며 새 수치 적분 실험이 아니다.

| 확인 항목 | 결과 |
|---|---:|
| 최종 게시 manifest에 열거된 파일 | 213/213 해시·크기 일치 |
| 새 외부 raw archive | 768/768 파일 해시·크기 일치 |
| 병합 raw archive | 1,920/1,920 파일 해시·크기 일치 |
| 새 외부 출력의 독립 해석해·오차 재채점 | 384/384 통과 |
| 제공된 해석해와 독립 계산의 최대 차이 | 1.1102230246251565e-16 |
| 기록된 최대 normalized error와 독립 재계산 차이 | 0 |
| 공통 오차 상한 아래 최속 구성 선택·비율 | 96/96 일치 |

원시 결과·검증 경로는 `evidence/LATEST_BENCHMARK_HASH_REPLAY.json`, 재현 스크립트는
`evidence/audit_latest_benchmark.py`다. 이는 전체 소프트웨어의 정상 동작, 모든 실패
분기의 안전성, 시간 측정의 통계적 유의성을 증명하지 않는다. 이미 수행된 이전 감사의
내용을 반복하여 새 성과로 계산하지 않았다.

최신 비교는 12개 case/차원 조합, 네 tolerance, 두 외부 방법에서 96개 설정·384회
실행이다. 기존 native/VODE 144개 설정·576회와 합쳐 240개 설정·960회다. 외부
96개 설정은 모두 warmup 한 번과 timed 세 번을 갖는다. 기록된 해석해와 초기값,
시간 격자, 오차 재채점과 comparison selection에서 모순을 찾지 못했다.

## 수치 결과가 실제로 말해 주는 것

`REPORT_KO.md` 54–58행의 오차는 다섯 개 저장 시각에서

\[
\max_{i,j}\frac{|y_i(t_j)-y_i^\star(t_j)|}
 {\max(1,|y_i(0)|,|y_i^\star(t_j)|)}
\]

를 계산한 값이다. 따라서 작은 진폭 성분의 상대오차, 두 관측점 사이의 최악 오차,
장시간 위상 오차를 직접 보증하지 않는다. 이 구분은 이전 보고서에도 명시되어 있다.

| 실제 반대 증거 | 개발상의 의미 |
|---|---|
| rapid에서 DOPRI5의 최선 표본 오차 약 1.971506e-8 | 등록한 grid에서는 1e-8 목표 미달이다. 더 작은 rtol에서 불가능하다는 정리는 아니다. |
| 같은 rapid에서 diffsol 약 5.873e-9, Julia 약 1.754e-9 | 정확도 개선의 현실적인 비교 기준이다. 두 외부 방법의 실패로 해석하면 안 된다. |
| 4096차원에서 Julia/DOPRI 시간비 0.2341, 0.1753 | 공통 오차 상한 1e-6/1e-8 아래 Julia가 약 4.27/5.70배 빠르게 관측됐다. |
| 16384차원에서 Julia/DOPRI 시간비 0.2052, 0.1886 | 같은 두 상한에서 약 4.87/5.30배 차이다. 보편적인 속도비나 복잡도 정리는 아니다. |
| stiff-control에서 외부 implicit 방법이 DOPRI보다 빠름 | 명시적 경로의 보편적 기본값 채택은 부적절하다. 강성 대조군은 계속 보존해야 한다. |

수치는 최신 보고서 66, 70–72, 86–91행과 병합 SUMMARY의 선택 구성에 근거한다.
타이밍은 서로 다른 시점의 cohort이며 세 번의 반복만 있으므로, 이 차이를 통계적으로
확정된 우열이라고 기술하지 않는다. 다만 다음 개발의 대상을 고르기에는 유용하다.

동일 요청 rtol=1e-7, n=4096 행에서는 DOPRI가 157 RHS, Julia가 175 RHS를 사용했지만
각 median은 0.003998260초, 0.000945500초다. 이 경우 더 많은 RHS를 쓰는 Julia가
더 빠르므로 RHS 호출 수만으로 차이를 설명할 수 없다. 상태 배열 연산·callback
경계·출력 수집을 분리해 측정할 이유가 있다. 어느 부분이 병목인지는 아직 계측하지
않았으므로 메모리 대역폭이나 BLAS를 확정 원인으로 지목하지 않는다.

실제 `WORK_PRECISION.png`도 열어 확인했다. 큰 벡터에서 Julia가 아래쪽에 놓이고,
강한 강성 대조군에서 DOPRI 비용이 오차를 줄여도 거의 평평하게 유지되는 패턴이
표의 해석과 맞는다. 일부 소차원 방법의 큰 min–max 막대와 비단조적 시간 변화는
노이즈가 크다는 경고다. 막대는 신뢰구간이 아니다.

## 코드와 비교 계약의 감사

- DOPRI facade는 `crates/vigilode/src/explicit.rs` 213–236행에서 RHS buffer를
  NaN으로 채우고 finite 여부를 검사한다. 커널의 `evaluate_rhs`도
  `crates/rodas5p-integrators/src/dopri5.rs` 327–358행에서 같은 일을 수행한다.
  이 중복은 정확한 최적화 후보다. 직접 커널을 쓰는 호출자와 공개 facade 양쪽의
  불완전 출력·nonfinite·recoverable/fatal 계약을 유지해야 한다.
- 커널의 stage 조합은 같은 파일 268–317행에서 여러 unit-stride array pass로
  구현돼 있다. 정수 차원에 비례하는 비용은 유지되지만, 상수 비용을 줄일 여지가 있다.
  fused 조합은 연산 순서를 바꿀 수 있으므로 상태·오차·accept/reject·FSAL 기록을
  함께 비교해야 한다. 단순한 speed microbenchmark만으로 채택하면 안 된다.
- diffsol은 `adapters/diffsol/src/main.rs` 197–209행에서 한 adaptive history를
  진행한 뒤 interpolation을 쓴다. Julia는 `adapters/julia/adapter.jl` 81행의
  `saveat`으로 같은 관측 시각을 저장한다. native는 관측점에 실제로 착지한다.
  출력 시각은 같지만 내부 적분 작업과 보간 오차는 다르며, 보고서 30행이 이를 공개한다.
- Julia는 accepted-state 보존용 callback을 두고 setup/copy 시간을 포함한다
  (`adapter.jl` 65–91행). 같은 요청의 JIT 준비 solve는 측정 밖이고, fresh 측정
  solve를 다시 수행한다(108–109행). 이 비용 구분은 문서와 일치한다. 실제 제품의
  cold start 지연과 steady-state solver 비용은 별도 지표로 유지해야 한다.
- 큰 벡터 fixture는 여덟 감쇠율이 반복되는 대각 문제다(diffsol 68–83행).
  차원 16384라는 사실이 물리적으로 결합된 FVM/PDE 문제의 검증을 대신하지 않는다.
- 최신 오류 검증은 configuration·source·method·output 시각·초기값을 검사하고
  strict-invalid 데이터를 비용 후보에서 제외한다(`verify_adapters.py` 60–160행,
  `analyze_external.py` 130–192행). 비교 결과를 만들기 위해 실패나 부적격 구성을
  조용히 포함한 흔적은 찾지 못했다.

## 권고하는 다음 실험과 채택 기준

1. **불필요한 RHS 전후 메모리 pass 제거.** callback을 불완전하게 쓰거나 NaN을
   반환하는 adversarial 시험을 먼저 유지한다. 같은 host·compiler·profile에서
   source를 고정한 전후 실행을 randomized paired 순서로 수행한다. 계산량·상태
   일치와 전체 setup 비용을 함께 기록한다. cheap RHS와 비싼 RHS를 분리한다.
2. **고밀도 관측과 정확도 제어.** 관측점 착지 비용을 줄이는 dense output을 도입할
   경우 endpoint 차수와 내부점 차수를 구별한다. 다섯 시각 대신 사전 지정한
   interior grid에서 해석해와 비교하고, adaptive 경로가 바뀌는지 기록한다.
   물리적 lifted error norm은 별도 계약으로 시험한다.
3. **진동 및 작은 성분 검증.** rapid의 기존 결과를 수정하지 않고, phase/amplitude
   분리·장시간 구간·vector atol·componentwise scale을 가진 새 실험을 등록한다.
   전체오차 목표를 실제로 달성한 설정끼리 비용을 비교한다.
4. **실제 결합 문제 확대.** 제조해가 있는 advection/diffusion의 semi-discrete
   문제와 비선형 client를 추가한다. 공간오차·시간오차·보존량·양성·restart 비용을
   분리한다. 대각 감쇠는 메모리 기준선으로 남긴다.
5. **출력과 실패 계약까지 포함한 공개 사용 범위.** 현 DOPRI의 forward identity-mass,
   explicit opt-in 범위를 유지한다. 자동 선택기를 추가하려면 accepted-state
   전환·probe 예산·hysteresis·restart 비용을 따로 검증한다. Jacobian이 없다는
   이유만으로 nonstiff라고 판정하지 않는다.

이 권고는 이번 최신 결과에서 관측된 차이에 연결되어 있다. 새로운 알고리즘의 도입과
검증이 끝나기 전에는 기존 broad production HOLD를 바꾸지 않는다. 기계 판독용 지적,
증거 위치와 조치는 `LATEST_BENCHMARK_AUDIT.json`에 있다.
