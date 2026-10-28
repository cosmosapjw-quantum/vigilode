# A07 Robertson gate 보고 정정

## 판정

보존된 A07 캠페인을 재실행하지 않고 validator 결과와 gate 설명을 대조했다. 기존 후속 보고에서 `0.05`를 solver arm의 정확도 기준으로 적은 것은 오류였다. 이 값은 empirical reference spread 기준이다.

- 입력: Robertson `[0,40]`, `rtol=1e-8`, `atol=[1e-11,1e-11,1e-11]`, 101개 관측시각.
- Reference spread gate: 각 empirical spread `<=0.05`; 최대 spread `0.0010781557032515283`, gate 통과. 이 reference는 엄밀한 구간 인증이 아니다.
- Per-arm accuracy gate: `max_grid_wrms_distance + empirical_reference_uncertainty <= 1.0`, 모든 row 성공 조건 포함.
- Native: 최대 WRMS `0.33753007149457903`, empirical interval `[0.3364519157913275, 0.33860822719783057]`, arm gate 통과.
- Diffsol: 최대 WRMS `3.3026992500504733`, empirical interval `[3.301621094347222, 3.3037774057537246]`, arm gate 실패.
- 두 arm empirical interval은 겹치지 않아 combined accuracy comparison도 실패한다.
- 시간 권한은 `diagnostic_only`; 동일 정확도 speed 비교와 speed 우위 주장은 계속 HOLD다.

## 검증

`tools/benchmarks/diffsol/a07/src/validate_report.py`의 retained 출력과 `validated.json`을 읽어 위 기준, 값, gate boolean을 자동 대조했다. 결과 exit code 0이다. Solver arms는 다시 실행하지 않았고 허용오차나 기준을 바꾸지 않았다. 원시 추출 로그는 `verify_gate_contract.log`에 보존했다.

이 정정은 A07의 보고 정확성을 고친다. 비교 캠페인의 empirical reference 한계, Diffsol accuracy 실패, 비중첩 interval과 matched-speed HOLD는 그대로다.
