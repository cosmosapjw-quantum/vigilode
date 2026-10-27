# Production CI 통합 정리와 측정 identity

실제 production workflow와 같은 fmt/clippy 범위를 실행하여 드러난 기존 문제를
별도 통합 작업으로 정리했다. 이 파일의 변경은 세 수치 연구 과제의 성능 결과에
새로운 가속 수치로 합산하지 않는다. 최종 gate의 실제 상태는
`evidence/VALIDATION_SUMMARY.json`과 각 명령 receipt가 결정한다.

## 형식과 정적 검사

초기 fmt 검사에는 기존 파일 21개와 새 production export 정렬 한 줄이 걸렸다.
`evidence/fmt_gate/EXACT_FORMAT_EQUIVALENCE.json`은 **당시 format-only checkpoint**의
21개 파일이 PR91 원본에 Rust1.94.1 rustfmt를 적용한 결과와 byte 단위로 같았음을
기록한다. 이어진 lint 정리로 일부 예제에 추가 변화가 생겼으므로 그 과거 receipt의
after hash를 최종 파일 hash라고 사용하지 않는다. 최종 identity는 SOURCE_MANIFEST다.

Clippy에서 실제로 지적한 항목은 다음과 같이 처리했다.

- 기존 core benchmark의 조건부 초기화를 동등한 if-expression으로 바꿨다.
- Causal budget의 `0..i` prefix accumulation은 같은 순서의 `enumerate().take(i)`로
  표현했다. 모든 계수·반올림 연산·조건·누적 순서를 유지했다.
- 기존 private Lawson 공통 구현의 여덟 인자에는 그 계약을 설명하는 국소적인
  `too_many_arguments` 예외를 두었다. API 또는 호출 비용을 바꾸지 않았다.
- 기존 public `GeneralResult`와 역사적 benchmark의 큰 enum은 inline ownership을
  유지한다. 단지 lint를 없애기 위해 Box allocation을 추가하지 않고, 해당 enum에만
  `large_enum_variant` 예외와 이유를 명시했다.
- 여러 예제·모델 단독 테스트가 각기 다른 부분집합을 사용하는 shared fixture에는
  그 사유를 적은 module-local `dead_code` 예외를 두었다. 중복된 외부 attribute는
  제거했다. Production crate 전체의 경고 수준은 낮추지 않았다.
- 기존 예제의 evenness, Option 검사와 범위 iteration을 동등한 idiom으로 정리했다.
  Checkpoint trace는 이미 보유하던 reference uncertainty와 report-only 표지를
  JSON metadata에 출력하게 했다. Solver 결과와 계산량에는 영향을 주지 않는다.
- Adapter의 두 nested conditional은 동일한 short-circuit 순서의 let-chain으로
  정리했다. 이 변경은 adapter의 **세 번째이자 마지막 correction**으로 기록했다.

초기 실패 로그와 최종 성공 로그를 함께 보존한다. `--keep-going`은 전체 target의
진단을 수집하기 위한 Cargo 옵션이며 `-D warnings`를 완화하지 않는다. Dependency
crate의 기존 nonfatal warning과 검사 대상으로 지정한 crate의 Clippy gate도 구별한다.

## 이전 측정과 최종 코드의 관계

Structural·homotopy의 수치 구현은 최종 lane 측정 후 변경하지 않았다. 해당 leaf
source hash, frozen executable hash, actual accuracy oracle과 paired profile은 각
lane의 receipt에 있다. Dependency의 뒤이은 format/lint 정리는 별도 이력이다.

Adapter의 마지막 syntax 정리는 측정 뒤 수행했다. 최종 36-arm audit에서 elapsed
time을 제외한 전체 상태·작업량·segment 결정은 기존 36개 arm과 정확히 같았다.
이 동등성은 기존 Decimal90·causal-seed 검증의 재사용 근거이며, instruction이나
시간까지 같다는 증거로 사용하지 않는다. 기존 성능 결과는 원래의 source와 binary에
그대로 결부한다. 자세한 비교와 실패한 source-overlap receipt는
`adapter/PASS3_RECEIPT.json`에 있다.

다섯 production 패키지의 확장 native gate는 고차 Fraction oracle까지 실행한 뒤,
PR91부터 존재하던 lagged JVP failure-work 집계 오류를 실제로 드러냈다. 이 실행은
1,312 passed / 1 failed / 45 ignored와 exit101로 보존한다. 아래 counter 보정 후에는
영향을 받는 lagged target들과 facade/coupled 전체 계약을 명시적으로 다시 검증한다.
전체 다섯 package sweep을 수정 후 다시 실행한 것으로 표현하지 않는다.
과거 성공 명령이 source overlap 때문에 `stale_source`로 거부된 경우도 원래 exit
code와 qualification status를 함께 남겼다.

## 실패한 물리 콜백의 과금 보정

`rodas5p_lagged.rs`의 기존 코드는 성공한 물리 JVP 수와 이미 실패까지 포함한
operator attempt 수를 같은 ledger로 취급했다. 한 번 실패한 주입 사례에서 실제
집계가 603과604로 갈렸다. 표현 가능한 counter 범위에서 진입 횟수를 A, 이미
과금된 물리 호출 수를 P라 두면 누락분은 정확히 A−P다. 두 ledger를 각각 보정하여
테스트의 원래 equality를 유지했다. Callback 실행·수치 산술·허용오차·iteration
선택은 바꾸지 않았다. 별도 production integration correction 1/3이며 세 연구
lane의 3/1/3 기록을 변경하지 않는다. 증명·PR91 source identity·실패와 성공 receipt는
`theory/FINAL_GATE_FAILURE_REVIEW.md`와 `evidence/VALIDATION_SUMMARY.json`에 연결한다.
이 private lagged 경로는 이번 adapter 벤치마크의 fast router와 다르다. 보정 후의
새로운 성능 수치를 주장하지 않고 기존 측정 identity를 그대로 보존한다.
