# Migration from technique-heavy prompts

다음 기본 지시는 제거한다:

- 항상 ToT/GoT/MAPS/BoN/CoVe를 모두 실행
- 후보를 무조건 많은 수로 생성
- 항상 debate와 exhaustive search
- 항상 max reasoning
- 모든 턴에서 전체 context 요약

대신 outcome, evidence, constraints, approval boundary, completion bar, verifier, stop rule을 명시한다.
