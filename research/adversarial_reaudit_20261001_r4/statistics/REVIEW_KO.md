# R4 통계·timing·campaign 재감사

감사 대상은 `1c54194123ee6abc6daa512e8574922f510b4e2c`, 실행 전 등록 commit은 `b2914f3e3c03eda60a8547619db6284aac8250a2`이다. 생산 소스는 변경하지 않았다. 판정은 **REWORK**이며 신규 P2 두 건을 확인했다. R3의 원래 다섯 raw-case 반례는 이번 실행에서 모두 거절되므로 그 수정은 인정한다. 새 문제는 receipt를 합치기 전의 세션별 검증과, 실패한 coverage 권위의 소비자 전달에 있다.

증거 상태를 구분한다. `native.stdout.jsonl`은 이 감사에서 실제 실행한 Rust 결과 21행이다. `INHERITED_EVIDENCE.json`의 timing과 coverage 숫자는 저장소에 게시된 기존 결과를 읽은 것이며 새 timing 측정이 아니다. CLI의 `WallCriterion::Passed` 연결은 소스 경로로 확인했고 전체 CLI 속도 campaign을 새로 실행하지 않았다.

## 1. 기존 수정의 인정

`PairedTimingCase::admit`는 빈 order, 잘못된 order, 잘못된 batch, 음수 warmup, NaN warmup을 모두 `INVALID_RAW_TIMING_PROTOCOL`로 거절한다. 원 R3-STAT-01 반례는 닫혔다. 독립 세션을 실제 subprocess로 실행하고 session record, 실패, arm 및 executable identity를 보존하는 receipt 계층도 추가되었다. 원 캠페인의 raw record 8개 arm을 현 public verifier에 다시 넣었고 저장된 결정과 모두 일치했다.

이번 전체 runtime 담당의 관련 회귀 검사도 통과했다: paired timing 13, R2 authority 5, R3 A/A batch 1, R3 Monte-Carlo 2, R3 raw admission 3, R3 timing design 4, CLI campaign contract 1. 원 로그는 `../evidence/runtime/native/suite_summary.json`에 있다. 이 숫자와 본 native probe 21행은 서로 다른 검증이며 합쳐서 단일 unit-test 수처럼 표현하지 않는다.

R3의 Hoeffding gate는 유한 bootstrap 횟수 B에 대한 조건부 Monte-Carlo 오차를 제어한다. population coverage와 다른 문제라는 구분은 현재 문서가 적절히 유지한다. B=10000에서 엄격한 MC gate가 결정을 상당수 보류한다는 점 역시 공개되어 있다. 실제 coverage 실패를 숨기거나 성공으로 재해석하지 않은 것은 이 업데이트의 장점이다.

## 2. R4-STAT-01 — coverage HOLD가 기계 판정에 반영되지 않는다

`docs/REAUDIT_R3_CLOSURE_20261001.md`는 두 coverage study의 FAIL 때문에 POLY03 Promote를 포함한 모든 paired timing 결정에 `STATISTICAL_AUTHORITY_HOLD`가 걸린다고 명시한다. 그러나 `paired_receipt.rs:533–550`의 `verified_decision`은 raw receipt 일관성, 재계산 일치, 실패 유무만 확인하고 numeric decision을 그대로 반환한다. `main.rs:1240–1275`의 wall 소비자는 그 Promote를 곧바로 Passed로 매핑한다. coverage authority를 나타내는 입력이나 조건은 없다.

실제 native 결과는 다음과 같다.

| 입력 | 현 public verifier |
|---|---|
| 6개 독립 session label, 한 case, 일정한 1.3 비율, A/A=1의 유효 synthetic receipt | `Ok(Promote)` |
| 게시된 POLY03 warm Chebyshev receipt | `Ok(Promote)` |

여기서 새 finding은 coverage 실패 자체가 아니라 문서의 HOLD와 machine-consumable authority가 분리되어 있다는 점이다. 이전 작성자가 production speed를 부당하게 주장했다는 비판도 아니다. 기존 보고서는 숫자의 한계를 분명히 공개한다. 다만 JSON과 CLI를 소비하는 다음 단계에는 그 prose가 자동 전달되지 않는다.

수정은 numeric decision을 지우는 방식이 적절하지 않다. `diagnostic_decision`, `receipt_integrity`, `coverage_authority`, `admissible_decision`을 분리해야 한다. 마지막 필드만 scientific wall gate가 사용하게 하고, 실패하거나 없는 coverage authority는 `NotEvaluated/STATISTICAL_AUTHORITY_HOLD`로 내려보낸다. study hash, estimand/design version, 허용 domain을 함께 묶어 다른 모형의 PASS를 재사용하지 못하게 해야 한다. 단순히 receipt의 boolean을 신뢰하는 방식은 해결이 아니다.

## 3. R4-STAT-02 — 이후 세션의 불완전한 raw cell이 Promote된다

receipt 병합은 첫 세션의 warmup만 merged case에 남긴다(`paired_receipt.rs:238–269`). 세션별 검사는 labels와 ABBA order, warmup **값**의 finiteness를 보지만, warmup **개수**와 candidate/reference sample 개수를 확인하지 않는다(`453–509`). 이후 merged case에 대한 검사는 전체 길이만 본다.

| 원자료 변형 | 결과 |
|---|---|
| 두 번째 세션 warmup을 전부 제거 | `Ok(Promote)` |
| 두 번째 세션 warmup을 한 개만 남김 | `Ok(Promote)` |
| 세션 1의 candidate sample 한 개를 세션 2로 이동: 29/31개 | `Ok(Promote)` |
| 같은 29/31 변형을 reference sample에 적용 | `Ok(Promote)` |
| finished timestamp를 started 이전으로 변경 | `Ok(Promote)` |
| 한 세션의 batch 변경 | 거절 |
| 한 세션의 case를 제거 | `Ok(Inconclusive)` |

29/31 반례에서는 sample 값이 같아서 병합한 flat vector를 원본과 동일하게 유지할 수 있다. 따라서 digest와 재계산이 일치한다는 사실은 raw cell의 유효성을 증명하지 않는다. 이는 새 provenance를 가짜로 만드는 암호학적 문제와 별개인 내부 구조 검증 결함이다. 실제 게시된 campaign에 이런 훼손이 있었다는 증거는 없으며, 반례는 synthetic record로 구성했다.

각 raw session cell에서 `candidate.len == reference.len == labels.len == order.len == protocol.pairs`와 warmup 최소 개수를 병합 전에 검증해야 한다. timestamp도 유한성과 순서를 검사한다. 다만 모든 이후 세션의 batch를 해당 세션 warmup으로 재교정하라고 요구하면 안 된다. 현 campaign은 첫 세션의 batch를 의도적으로 고정한다. 첫 calibration record의 identity와 batch를 명시적으로 묶고 후속 세션이 그것을 따라야 한다는 별도의 검증으로 구현해야 한다.

## 4. 게시된 숫자를 어디까지 받아들일 수 있는가

coverage v1과 v2는 각각 36개 primary scenario 중 18개가 실패했다. v2의 single-case coverage는 0.7825–0.9255이고 five-case coverage는 0.9935–1.0이다. 이 결과는 새로운 audit finding이 아니라 공개된 부정 결과이며 보존해야 한다.

five-case의 보수성은 해당 모형에 관한 관측이다. simulator는 replication마다 median-centered case effect를 새로 뽑으므로 특정 고정 corpus에 대한 균일한 보장을 제공하지 않는다. 특히 case 다섯 개가 같은 세션 응답을 갖는 극한에서는 case resampling이 아무 값도 바꾸지 않아 정확한 bootstrap 법칙이 single-case와 같아진다. case 수가 5라는 조건만으로 authority를 부여해서는 안 된다. 새 validation에는 case effect가 없는 경우, 완전히 상관된 case 응답, 고정된 case-effect 벡터를 포함해야 한다.

| 게시된 arm | point speedup | 구간 | 재계산한 numeric decision |
|---|---:|---|---|
| Chebyshev cold | 0.5651 | [0.0181, 1.9895] | Inconclusive |
| Chebyshev warm | 3.1657 | [1.1672, 13.4140] | Promote, authority HOLD |
| Laguerre cold | 0.1224 | [0.00142, 0.3170] | Block |
| Laguerre warm | 1.1796 | [0.1957, 3.1194] | Inconclusive |
| Homotopy p1/p2/p4/p8 | 0.1735 / 0.0718 / 0.0481 / 0.0369 | 원 CAMPAIGN 참조 | 모두 Block |

POLY03은 `joint_phi_action_unbounded`의 kernel 시간이다. h=1, 고정 symmetric operator, 같은 다섯 입력 vector, 별도 정확도 검증이라는 좁은 의미에서 비교가 구성되었다. 시간 구간 안에 roundoff certificate를 포함했다고 주장하지 않는다. warm coefficient reuse가 유리했다는 관측을 adaptive h나 operator 변경, certified production path로 확장하면 안 된다. 다음 campaign은 setup·steady reuse·cache 검증·certificate 비용을 분리하고, frozen symmetric 문제에서 cached eigensystem 같은 비교군의 amortization도 사전에 정의하는 편이 낫다.

HOM06의 0.17 이하 speedup은 실제 부정 결과다. 약 5배 RHS, 6–8배 JVP와 매 step의 thread-pool 생성은 그 연구 설계가 아직 성능 개선에 이르지 못했음을 보여 준다. 이 결과를 더 많은 thread로 재시도하기보다 thread pool 재사용 및 연산량 감소부터 수행해야 한다. 새 source/새 cost model 없이 같은 timing만 반복할 근거는 없다.

추가 확인 과제로, release는 fat LTO이고 timed closure는 결과를 `map(|_| ())`로 버린다. 오류 분기와 finiteness 검사가 계산을 일부 강제하므로 전체 계산이 제거됐다고 입증한 것은 아니다. 다음 benchmark에는 output/work를 `black_box` 또는 검증된 sink로 소비하여 최적화에 대한 측정 의미를 명시하는 것이 좋다. 현재 이 항목은 **미검증 risk**이며 확정 finding으로 세지 않는다.

## 5. 새 연구 결과 — exact session median 구간

실제 개선 후보로 bootstrap resample 수를 계속 늘리는 대신 독립 세션에 대한 유한표본 순서통계량 구간을 구현했다. 이 후보의 estimand는 고정 corpus에서 각 case의 **population session-cell median**을 구한 뒤 다시 case median을 취한 것이다. 기존 pooled-pair median과 일반적으로 다르며, 기존 대칭 additive model에서는 일치한다. 자세한 유도는 `DERIVATION_KO.md`에 있다.

C개 case, S개 iid session vector, 전체 오류예산 alpha에 대해 가장 큰 k를

`2 * sum_{j=0}^{k-1} binom(S,j) / 2^S <= alpha/C`

로 선택한다. 각 case의 `[Z_(k), Z_(S-k+1)]`를 만든 뒤 endpoint별 case median을 취하면, union bound와 median의 좌표별 단조성으로 전체 coverage가 최소 1-alpha이다. case 사이의 독립성은 필요 없다. 만족하는 k가 없으면 무한 구간을 반환한다. 누락 셀, 중복 session identity, 비유한값, 실패 세션은 거절한다.

alpha=0.05의 예:

| S | C | 구간 | 검증된 coverage 하한 |
|---:|---:|---|---:|
| 6 | 1 | case min/max | 31/32 = 0.96875 |
| 6 | 5 | 무한, Inconclusive | 1 |
| 8 | 5 | 각 case min/max의 median | 123/128 = 0.9609375 |

이 연구의 Python 실행은 exit 0이다. exact Fraction/binomial 계산으로 32개 design, 6개 전수 sign enumeration, 12개 decision control, 4개 잘못된 입력 거절을 확인했다. 이는 실제 timing 측정이나 생산 통계 승격이 아니다. iid session, complete fixed corpus, non-informative inclusion, 고정 종료라는 전제가 핵심이며 OS process가 다르다는 사실만으로 전제가 증명되지 않는다. f64 log와 endpoint rounding까지 enclosure했다는 주장도 하지 않는다.

구체적 다음 순서와 acceptance/kill criteria는 `NEXT_STEPS.json`의 5개 node에 고정했다. 먼저 authority 전달과 raw session admission을 수리하고, estimand를 versioning한 다음 exact interval 후보를 Rust로 이식하여 새 사전등록 coverage/power 연구를 실행하는 순서다. 독립 reviewer의 최종 판정은 전체 보고서의 decision receipt가 소유한다.
