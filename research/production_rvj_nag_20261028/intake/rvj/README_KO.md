# RVJ–NAG 후속 연구보고서 · 2026-10-28

주 보고서는 `RESEARCH_REPORT.json`이다. 식·가정·증명 상태·실행 결과·독립 검토·근거 경로를 연결하며, `RESEARCH_REPORT.schema.json`으로 구조를 검증한다. 사람이 읽는 유도는 각 작업의 `derivation.md`, 실행 원자료는 `results.json`에 있다.

이전 `RVJ_NAG_RESUME_20261028.zip`을 SHA-256 `18d21a8e9f7f6e5656d991a37319abd2c826884a82eb1a156f793f0a5d602d7d`로 복원하고 manifest 60개 항목을 대조했다. 원본 snapshot을 `baseline/`에 보존했다. 이전 실행 횟수는 이번 검증 횟수에 포함하지 않았다.

## 새 결과

| DAG | 이번 결과 | 상태 |
|---|---|---|
| F1 | 같은 branch를 유지한 u/v chart 전환 150회, 4 runs·288 slabs 전체 구간 인증 | 지정된 모델에서 완료 |
| F2 | 4·8·16변수 sparse cyclic closure의 O(n) 유리식 회로; 25·53·109개 연산 | 지정된 family에서 완료 |
| F3 | Native outward Hermite/Bernstein 잔차와 radius 누적; 동등 오차의 Newton·Krawczyk 비교 | 지정된 환경에서 완료 |
| F4 | A²+6B=0 extended wall의 cuspidal butterfly; edge 분수 지수와 중앙 (4,5) section | 국소 정리 완료 |
| F5 | 새로운 횡단성 인자, 길이3 국소 wall, 원문에서 빠진 수평 collision | 전역 count는 미완료 |
| F6 | 중복도를 보존하는 trace-only coupled ODE; triple-root 통과와 전체 구간 인증 | 명시한 관측량 계약에서 완료 |

F1에서는 주 결합계 N=64의 closure 전체 경로 상계가 약 3.780e-9이다. F2/F3의 세 방법은 모두 시험한 해상도 중 N=64에서 전체 변수의 1e-8 예산을 처음 충족했다(상계 약 1.422e-9). F6은 N=256에서 residual-only 전체 경로 상계 5.752e-9를 얻었으며, trace 경로는 근 열거 0회, 기준선은 1024회다.

## 전역 count 교정

기준 cubic y²=x³−x+1에서 a²=8/3, y²=37/27로 두고 x=1/3−a/2를 택한다. 두 witness `((x,y),(x+a,y))`와 `((x,−y),(x+a,−y))`는 같은 source `(1−a,0)`와 return `(1+a,0)`을 갖는다. 네 foot은 서로 다르고 reflection 계수는 −1/2,+1/2이며 normal determinant도 0이 아니다.

원래 12개 incidence 식에 역대입한 잔차는 정확히 0이고 Jacobian determinant는 −358875136/19683이다. 별도 검토자가 원래 Cramer map을 미분한 4×4 tangent determinant도 384/37로 얻었다. 따라서 이 수평 packet은 실제 reduced isolated collision이며, 원문의 수평 chart 배제에서 빠져 있었다. 두 chord 방향을 합하면 ordered 4개, witness-unordered 2개가 추가된다. 이 packet에서는 반사와 witness 교환이 같으므로 각 double-quotient orbit의 lift는 2개다.

기존 116/58을 intrinsic 전체 count로 재사용하지 않는다. 기존 b≠0 영역의 112+4개가 독립적으로 완전 검증되고 다른 누락이 없다는 전제 아래 총수는 120/60, double quotient는31이 된다. 이 조건부 합계를 이번 전역 정리로 승격하지 않았다. 전역 saturation 입력은 준비했지만 실행하지 않았고, p=0 복소 수평 chart와 전체 parameter exceptional locus가 남는다.

수직 packet과 수평 packet의 횡단성 인자는 `D=4 eta³ p+27 eta² q+27 p q`다. `(eta,p,q)=(1,−1/2,4/27)`에서는 carrier가 정칙이면서 D=0이고, 수직 quotient의 국소 길이는3이다. q를 변화시키면 `a²=−(120/13)(q−4/27)+O((q−4/27)²)`로 두 추가 실분기가 나타난다.

## 계산비용

다음 값은 N=64, 동일1e-8 예산에서 7회 반복의 전체 시간 중앙값(ms)이다. 후보 생성과 인증을 모두 포함한다. Krawczyk는 Newton 후보 뒤 실제 interval inclusion을 추가한 비교군이다.

| Closure 차원 | Compiled | Newton | Newton+Krawczyk |
|---:|---:|---:|---:|
| 4 | 1.901 | 2.130 | 3.007 |
| 8 | 3.725 | 3.829 | 8.596 |
| 16 | 7.448 | 7.624 | 19.753 |

Native 캠페인은36개 조건·252회 timed integration·15120 timed slabs다. 27개 대표 slab를 별도 Fraction 계산과 대조했다. 인증비용이 전체 시간의 대부분을 차지한다. 회로의 O(n) 구성과 반복 근찾기 제거는 검증했으며, 전체 속도의 보편적 개선은 주장하지 않는다. 전처리 code generation·compiler 시간과 개별 반복값은 원자료에 있다.

## 검증과 재실행

새 unit/contract tests는 F1 14개, F2/F3 8개, F5 12개, F6 8개다. 정확 identity 검사와 고정밀 probe는 별도 집계했다. F4는 정확검사13개와90자리 original-equation probe27개를 수행했다. F6의 저장된12개 certificate는 결정적으로 replay했다. 오류·예상된 저해상도 예산 실패·수정 이력은 `FAILURE_REGISTRY.json`과 각 작업 기록에 보존했다.

독립 검토는 `reviews/review.json`, 추가 native 검토는 `reviews/native_secondary_review.json`에 있다. 이론 R1–R10과 코딩 C1–C10은 `WORKFLOW_RECORD.json`에 실제 근거를 연결했다. 전체 DAG 상태는 F5 의무 때문에 계속 진행 중이다.

Python3.12와 `requirements.txt`의 라이브러리, native 작업에는 g++13 계열이 필요하다. 원본 출력 보존을 위해 재실행은 새 경로에서 한다.

```bash
python3 -m pip install -r requirements.txt
python3 VALIDATE.py
python3 REPLAY.py --destination /absolute/new/replay_directory --full
```

`REPLAY.py`는 지정한 새 디렉터리에 복사한 후 실행한다. Native 타이밍은 실행마다 달라질 수 있다. 재실행본의 결과는 원본 manifest와 별도의 receipt로 관리한다. 조건부 전역 saturation 스크립트는 자동 replay 대상에 포함하지 않았다.

파일 구조: `RESEARCH_REPORT.json` / `CLAIM_LEDGER.json` / `NEXT_DEVELOPMENT_DAG.json` / `EVIDENCE_INDEX.json` / `MANIFEST.json` / 여섯 DAG node를 담은 다섯 작업 디렉터리 / `reviews/` / `baseline/`.
