# 누적 검증 보고서 (2026-10-28)

최종 소스는 commit `5c57bf146f2f5faf69f711f133a83fcb265cee90`, tree `9633bde7dda1e9dd4748d1aecc8f22af68b5f5ac`이다. 23개 노드, 10개 상호작용, 8개 suite, 7개 Q gate를 분리해 기록했다. S-PRS부터 P-PUSH까지는 source-locked replay/inherited evidence이며 current-native PASS가 아니다.

## 실행 결과

- facade-contracts, coupled-contracts, all-targets는 PASS.
- release 첫 suite는 PASS(21 ignored). integrators는 최초 1800초 제한에서 917 pass 후 timeout 되었고, 연장 243초 실행은 1196 pass/54 ignored였다.
- docs PASS. smoke는 full-state 4행 PASS, 최대 오차 `3.3718316472253207e-9`.
- local-benchmarks 8개 명령과 local-profiles 2개 명령은 모두 exit 0.
- Q-BENCH adapter 7 PASS. Q-GIVENS와 Q-FAIR-AB는 범위 지정 독립 검토 PASS. Q-LINT는 Clippy exit 0, focused 202 pass/1 ignored.

## 음성 증거와 보류

Q-NEGATIVE는 `REPRODUCED_SCIENTIFIC_FAIL`이다. 실제 실행은 exit 101이며 `15/7 > 1.3`, Off interior `311.467`을 보존한다. Q-LEDGER는 current authority 49개 누락, repair candidate rejected, PR94 CI 53개 누락으로 HOLD이다. Q-API도 HOLD이다. I-STATE, I-EPOCH, I-WORK, I-BUDGET, I-POLY, I-ROOT, I-STRUCTURE, I-NEGATIVE, I-PROFILE, I-CLIENT와 PR-N01..N06은 직접 실행 증거가 없어 NOT_RUN이다.

## 성능 해석

rapid benchmark는 초기 `rtol=1e-6`에서 최대 오차 `1.0473e-4`로 cap을 실패했다. `rtol=1e-8` 재실행은 최대 오차 `9.4603e-7`, phase `1.4996e-7`, amplitude `9.8307e-7`이며 work는 동일했다. 중앙값은 switch 5.701655 ms, dopri 5.591458 ms이다. Callgrind의 관측 격자 16→32 차분은 추가 관측점당 switch 14849.375, DOPRI 14827.4375 guest instruction이었다. 종료 시각은 같지만 수락 스텝/RHS 작업도 달라 전체 solve 비용이나 동일 작업 반복으로 해석하지 않는다. timing authority는 `diagnostic_only`다. bench driver와 ledger repair candidate는 NOT_ACCEPTED이고 광범위한 production 승격은 하지 않는다.

standalone vendor validator는 필수 `vendor_dir` 인자를 주지 않은 호출로 exit 2였으며 vendor graph 검증은 NOT_VALIDATED이다. locked offline metadata/build와 E4 synthetic mode 검사는 별도로 통과했다. 최종 및 delivery cargo fmt는 inherited baseline과 동일한 두 coupled 파일에서 모두 exit 1이었다. correction budget은 R-STRUCT 3/3, R-HOMO 1/3, R-ADAPT 3/3, I-PROD 1/3이며 Q-LEDGER의 delivery 이후 거절된 수정 1회가 추가되어 누적 1/3이며, 그 외 계획 노드는 0/3이다. local benchmark/profile exit 0은 qualification이 아니다. 이 보고서는 full authority, 모든 ignored test, 실제 client, 독립 외부 solver 성능을 주장하지 않는다.

사전등록-only commit `12057f5fee622c1804d0e5db8f371ca5938cc336`은 실행 전 원격에 도달했다. 다만 실제 코드 실행 commit `5c57bf146f2f5faf69f711f133a83fcb265cee90`은 측정 당시 로컬에만 있었으므로, 실행 전 원격 도달이라는 절차 조건은 충족되지 않았다. 원시 영수증과 해시로 결과를 보존하되 이 이탈을 소급 PASS로 바꾸지 않는다.

재현 명령과 원시 증거 위치는 `REPRODUCE.md`, 파일별 digest는 `EVIDENCE_MANIFEST.json`을 참조한다.
