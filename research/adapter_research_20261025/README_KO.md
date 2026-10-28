# VigilODE: 구조 활용과 선택 정책 연구·pilot

최신 중지 자료 `d79326648a7273c0decd641eaf94d94f5bea7c63`를 기준으로 진동, 적응 스텝, 방법 선택, 병렬 실행 선택을 함께 조사했다. 실제 native 코드·작은 검증·미해결 비용을 보존한다.

- [통합 연구보고서](REPORT_KO.md): 원인 분석, 유도, 구현, 관측 결과와 남은 문제
- [기계 판독 판정](FINDINGS.json), [구체적인 다음 개발](NEXT_STEPS.json)
- [로컬 Codex 인계](LOCAL_CODEX_HANDOFF_KO.md)
- [선택 중심 벤치마크 계약](selection_benchmark/PROTOCOL_KO.md)
- [진동·Fourier·involution 연결](oscillation/THEORY_AND_PORT_KO.md)
- [adaptation·cofactor](adaptation/REPORT_KO.md)
- [switching](switching/RESEARCH_KO.md), [병렬 실행](parallel/RESEARCH_KO.md)
- [최신 게시 자료와 BASS/REI 재사용](sources/LATEST_EVIDENCE_KO.md)
- [최종 검증](VALIDATION.json), [독립 검토](reviews/INDEPENDENT_REVIEW.md)

기존 `GeneralOde::solve`는 고정 선택 API다. 새 `AutomaticOde::solve`는 RHS 우선으로 시작해 명시적으로 허용된 RODAS 경로의 관측 비용을 시험하는 opt-in API다. 구조를 선언하는 `ExponentialChirpChart`, `ExponentialCofactor`와 독립 ODE batch의 `AdaptiveBatchExecutor`는 별도의 구성요소다. 아직 임의 RHS에서 대수적 구조를 자동 발견하거나 모든 후보를 하나의 dispatcher에 결합한 버전은 아니다.

Rust 1.94.1, 기존 Cargo.lock을 사용한다. 빠른 검사 예:

```bash
cargo test --locked --offline -p vigilode \
  --test automatic_adapter_contracts --test cofactor_chart_contracts \
  --test oscillatory_chart_contracts --test evidence_switch_contracts \
  --test execution_policy_contracts --test switching_contracts --test frontier_switch_contracts
cargo test --locked --offline -p rodas5p-integrators \
  --test exponential_controller_repair --test adaptive_fused_exponential_contracts
python3 -B -m unittest discover -s research/adapter_research_20261025/selection_benchmark -p 'test_*.py'
```

원래 장기 벤치마크는 이 패키지로 자동 재개되지 않는다. 새 local 진단에서는 실제 원자료를 보존하고, 선택된 방법과 관련 대안1개를 같은 checkpoint에서 비교한다. 전체 method×thread 곱을 반복하지 않는다.
