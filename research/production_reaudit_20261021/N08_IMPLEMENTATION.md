# N08: 실행 영수증과 승격 판정의 실제 분리

`tools/production_promotion_gate.py`의 source/PLAN/build/runtime/raw 결속과 범위 제한 판정을 구현하고 마지막 두 결속 결함까지 수정했다. 결과는 **구현 완료, targeted 계약 테스트 13 PASS**이다. 이 결과가 실제 solver의 production 적격성을 뜻하지는 않는다. 이 환경에서는 native bench 전체 통합이나 NCP scientific campaign을 실행하지 않았다.

## 구현된 판정

Gate는 독립적으로 전달된 committed config와 저장된 PLAN을 비교한다. 실행 identity, source commit/tree/closure, 필요한 validation 명령과 모든 benchmark case를 고정 계획에서 유도한다. Summary가 스스로 선언한 축소된 required 목록만으로 통과하지 않는다. Corpus 파일의 SHA-256도 frozen config의 `locked_inputs[corpus]`와 일치해야 한다.

Validation receipt의 build artifact SHA-256 및 ldd runtime library manifest를 각 실행 receipt의 executable binding과 비교한다. 비교 실험에서는 receipt argv, frozen case의 suite/case/arm/thread, binding이 가리키는 실제 executable path까지 일치해야 한다. Raw file hash는 그 실행 자체의 artifact manifest에 있어야 한다. Source 경계, 실행 종료, timeout, RSS, affinity와 OOM 관측도 확인한다. 잘못된 raw receipt는 case HOLD와 전체 receipt integrity HOLD에 모두 반영된다.

Work–precision 결과에는 N02의 전체 RADAU5/CVODE ladder를 요구한다. PASS 문자열만으로 승인하지 않는다. 각 level의 rtol/atol, 양쪽 status와 grid 길이, 유한한 cross-family 차이, 두 번째 level부터의 양쪽 refinement 차이, finest RADAU5 baseline을 검사한다. 원래 tolerance에서 candidate의 global grid error가 1 이하이고 모든 반복이 성공해야 `EMPIRICAL_GRID_PASS`이다. 전역 연속 궤적의 정확도나 속도 승격은 여기서 추론하지 않는다.

다른 실험은 각 계약이 실제 제공하는 범위를 유지한다.

| 결과 | 인정되는 범위 |
| --- | --- |
| polynomial / homotopy | 요청된 ODE endpoint의 bound |
| actual-k | stage functional bound |
| bound-client diagonal | 요청된 ODE endpoint의 bound |
| bound-client global | 해당 bound model의 full-curve 계약 |
| bound-client allocation | 대수적 allocation 계약 |

이들 값이 original budget을 만족하면 `SCOPED_CERTIFICATE_PASS`일 수 있다. 그것을 일반 solver의 모든 모델에 관한 production 자격으로 확대하지 않는다. `broad_production`은 자동으로 승격되지 않고 **HOLD**이다. Gate의 exit 0은 분석 절차가 완료되었다는 뜻이다.

## 마지막 두 실제 결함과 회귀 검증

1. 이전 경로는 caller가 제공한 corpus와 raw JSON이 서로 일치하면, 그 corpus가 frozen PLAN의 hash와 다른 사실을 놓칠 수 있었다. 최종 수정은 supplied corpus SHA를 config lock과 대조한다.
2. 이전 경로는 비교 command가 올바르고 executable binding이 유효한 build artifact를 가리키면, 둘이 서로 다른 실행 파일이라는 사실을 놓칠 수 있었다. 최종 수정은 command의 executable path와 binding path가 같은지 대조한다.

새 regression fixture는 synthetic PLAN, source identity, 11개 validation command receipt, build artifact 및 library manifest, comparison raw, N02 WP raw를 연결한 완전한 작은 receipt chain이다. 정상 chain은 scoped PASS만 허용하고 broad production은 HOLD로 남는다. 공격 fixture는 각각 corpus/raw/receipt를 내부적으로 일관되게 교체하거나, 다른 유효한 build executable로 binding을 바꾼다. 최종 코드에서는 둘 다 HOLD이다.

두 guard만 제거한 controlled mutation replay에서는 두 regression이 모두 실패했다(`PASS != HOLD`). 이는 역사적 solver 실패 로그가 아니라, 실제 새 테스트가 결함을 잡는지 확인하기 위한 명시적인 synthetic mutation이다. 원래 관측했던 missing-reference-level false PASS도 별도 JSON으로 보존했다.

| 증거 | 결과 |
| --- | --- |
| `evidence/n08/promotion_final_contracts.log` | targeted unittest 13 PASS |
| `evidence/n08/final_check_mutation_replay.log/json` | guard 두 개 제거 시 예상한 regression 2 FAIL, 오류 0 |
| `evidence/n02/promotion_initial_missing_levels.json` | 초기 gate가 level 없는 PASS label을 EMPIRICAL_GRID_PASS로 처리한 실제 재현 |
| runtime agent의 NCP harness 계약 테스트 | 별도 보고서/최종 matrix에 기록; 여기서 native 실측으로 재해석하지 않음 |

## 인계

Local Codex 6 Luna low의 역할은 전체 native 컴파일·작은 통합 수정·계약 테스트 확인이다. NCP에서는 frozen config와 source-clean checkout으로 runner를 실행한다. 결과는 다음 consumer로 판정한다.

```sh
python3 tools/production_promotion_gate.py \
  --summary /absolute/ncp-output/SUMMARY.json \
  --corpus crates/vigilode-bench/protocols/n02_independent_reference_v1.json \
  --config research/production_reaudit_20261021/NCP_CONFIG.json \
  --expected-commit <published-commit> \
  --out /absolute/ncp-output/PROMOTION_GATE.json
```

HOLD 원인은 raw receipt와 함께 보존한다. Failed rows를 버리거나 tolerance를 사후 완화하거나, 같은 published 실험을 변경 이유 없이 다시 실행하는 절차는 없다. Source/receipt 경계의 무결성 확인은 악의적인 host에 대한 원격 attestation이나 수학적 proof checker의 대체물이 아니다.
