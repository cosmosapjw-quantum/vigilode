# Loop12 새 확장 정리 독립 판정

2026-10-11. 독립 reviewer: `/root/loop12_extension_review`. 후보 생성과 검증 설계에는 참여하지 않았으며, 두 저자의 새 증명·claim manifest·작은 exact 검산 코드와 기록을 읽고 판정했다. 기존 공개 정리와 campaign은 재검증하지 않았다. 하네스의 상태 템플릿을 실행 상태로 상속하지 않았다. 모델 신원은 UNKNOWN, 성능 비교는 NOT_EVALUATED다.

**판정: DW-A–F와 R1–R6의 12개 조건부 이론 정리를 PROMOTE한다.** 이는 보고서와 구체적 코드 이식 설계에 적용되는 판정이다. 현재 native production, 실제 multiwindow 전역 궤적 인증, 실측 속도는 HOLD다. 새로운 학술적 발견이라는 신규성 판정이나 형식증명 도구의 검증을 뜻하지 않는다.

## 주장별 판정

| 주장 | 판정 | 직접 검토한 핵심 | 필수 적용 조건·한계 |
|---|---|---|---|
| DW-A | PROMOTE | 공통 감쇠를 제거하면 cubic 항에 정확히 e^{-2κt}가 남는다. π 가중 에너지 항등식과 초기 fibre 보존을 확인했다. | 공통 κ, 양의 정규화 π, 실수 χ인 명시적 모델. |
| DW-B | PROMOTE | 실수/formal λ 계수 Banach algebra에서 self-map·수축·tail을 확인했다. exp-polynomial 차수 귀납도 성립한다. | DW6의 엄격 여유. 비정칙 cubic의 복소시간 holomorphy를 사용하지 않는다. p,Q의 κ 독립성과 bit cost를 구분. |
| DW-C | PROMOTE | φ 유한합, 양의 beta 적분, repeated-node 정규화 및 a→0 극한이 일치한다. | 임의 차수 outward evaluator는 미구현. 상쇄 때문에 precision 비용이 커질 수 있다. |
| DW-D | PROMOTE | 단조 H의 weighted Gronwall과 각 mode Duhamel 분해에서 초기오차·tail·잔차를 보존한다. | 전체 구간 true/candidate tube 및 exact-descriptor residual. arithmetic와 phase를 중복 없이 포함. |
| DW-E | PROMOTE | 두 exact flow의 차이에서 rank-one majorant를 얻고 P²=P로 창간 곱을 닫는다. | local η에는 inherited error를 다시 넣지 않는다. 고정 π/model과 실제 grid의 witness 필요. |
| DW-F | PROMOTE | 동일 homotopy 계수 맵의 선형 집계와 strict cost gate가 성립한다. 모든 출력의 공유 연산 및 p=0 IO를 수정했다. | 실제 complete cost 상·하계. 이미 집계된 최선 baseline에 대한 새 이득은 아니다. |
| R1 | PROMOTE | 변분상수법의 양방향 대응과 Markov closure의 kernel-invariance 조건을 확인했다. | 알려진 D,U,V와 직접 rank-r 평가. 독립 초기 fibre를 보존. |
| R2 | PROMOTE | 실수 detuning의 factorial tail, matrix moment, nilpotent packet realization이 정확하다. | 실제 rank·폭·군집을 유지. 작은-r만으로 전체 비용의 n 독립성을 주장하지 않는다. |
| R3 | PROMOTE | 원래 kernel의 B와 full-channel gain으로 reduced defect를 모든 원 mode 오차에 전달한다. | continuous residual와 독립적으로 공급된 공통 domain; component scale와 arithmetic를 보존. |
| R4 | PROMOTE | 단체 적분의 t^s/s!가 수렴과 선형화 역함수 bound를 준다. BLT<1은 필요 없다. | 전역 Lipschitz 또는 별도 존재/tube 조건. 임의 고정 projection의 오차가 저절로 작아지는 것은 아니다. |
| R5 | PROMOTE | 동일 Π를 free profile와 공유 적분에 적용하는 수정 후 same-map 항등식이 성립한다. 명시된 예의 16904<49152 계산을 확인했다. | 남은 비공통 비용 32248 미만인 unit-cost scalar work 조건. Bit identity/실측 속도 정리가 아니다. |
| R6 | PROMOTE | full-space 소멸다항식으로 rd차원 closure, 불변 ideal 항등식으로 quotient dynamics를 얻는다. | 전체 초기 moment 보존. 작은 최소다항식·싼 Gröbner 탐색·전역 매끄러운 quotient는 별도 문제. |

## 실제로 수정된 문제

1. **R5 projection 혼동:** Π가 상수항만 남기는 경우 constant forcing의 unprojected 출력은 t에 선형이지만 projected 출력은 초기상수다. 최초의 무조건 동일성 주장은 거절했다. 최종 증명은 같은 Π를 적용한 출력과 R3의 unprojected 원모델 출력을 구분한다.
2. **R5 비용 단위:** 적분 weight의 준비만 공통으로 상쇄한다. weight 적용은 센다. 곱셈 1은 comparator 하계에서 제거했고 덧셈도 포함한다.
3. **DW-F 다중 출력 비용:** 각 출력시각마다 C_p 계수를 다시 조립하는 ordinary scalar algebra를 명시적으로 추가했다. 특수함수 비용에 숨기지 않았다.
4. **DW-F 영차 경계:** p=0에도 입력과 전체 출력은 O(n)이다. 최종 복잡도는 n(p+1)을 사용한다.

## 근거의 범위

일반 정리의 판정 근거는 문서의 유도다. 작은 Fraction 프로그램과 기록은 계수 항등식·상계 숫자의 진단이며 native solver 검증이나 물리 궤적 인증이 아니다. Reviewer는 프로그램과 기록을 읽었고 기존 또는 새 campaign을 다시 실행하지 않았다. 최종 theorem 승격에는 위 제한과 미구현 상태가 함께 붙는다. 실제 wall time, precision에 따른 bit complexity, 새로운 path의 production 품질을 이 증명으로 대신 승인하지 않았다.

## 판정한 파일 identity

| 파일 | SHA-256 |
|---|---|
| `DAMPING_WINDOWS_KO.md` | `aec53bf2b66b9ba6172bbc99c82f40f740d661e899a370fbe83db43b52484230` |
| `CLAIMS_DAMPING.json` | `5daf8982b74cc429b63419a0d4d3615f408ebbae7a46324bb0c3f269d065991d` |
| `check_damping_exact.py` | `f6deb13389869054f28860f29d688c8e6bb0ee62ccdaa992a1a6e03812c44128` |
| `DAMPING_EXACT_CHECKS.json` | `bb715ec55ab065d7d7610eecbf9c270242d1548d0bd607a5f9b589195f18a0de` |
| `RANK_GEOMETRY_KO.md` | `d4811f0f40f1c00041de793dc49aa23f1568e2eed9c2820bef88d80c529a12a7` |
| `CLAIMS_RANK.json` | `8cc6d8af15f890ea66cd2248707b85e6abd2af1c546cf94c682fb793aa5c5820` |
| `check_rank_exact.py` | `0fb7148594561ac3e81d8abaabd07f6c33450c9db8594224305a01c7d185e017` |
| `RANK_EXACT_CHECKS.json` | `3e5da3edd57b9699c89b8154d5bcd57781b075bba9155729f09beefe72d6afcd` |

최종 파일의 material 오류는 닫혔다. 이 판정 뒤에는 추가 감사의 감사를 반복할 필요가 없으며, 위 조건을 구현하는 단계로 넘길 수 있다.
