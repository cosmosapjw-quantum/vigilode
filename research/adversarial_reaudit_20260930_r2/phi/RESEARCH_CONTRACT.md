# R2 φ-action 재감사 계약

- SSOT: repository commit `7708ef90554fc3986478d4602de6a01c7266b14f`; 핵심 수정 commit `1299b62`, 비교 대상 integration `ff84ed6`.
- 목적: PHI-P1/P2/P3의 실제 closure를 독립 scalar series로 확인하고 normalized representation의 계산 가능 범위를 적대적으로 검증한다.
- 보호 범위: 기존 production source, tests, Cargo.lock은 읽기 전용이다. standalone client/prototype만 이 폴더에 둔다. 원격 mutation은 root owner만 수행한다.
- 적용 하네스: GPT-6 Astra research v4.0.0 START_HERE / PROJECT_INSTRUCTIONS / RESEARCH_STATE, coding AGENTS / SCIENTIFIC_CONTRACT. 기본 state는 NOT_RUN 템플릿으로 읽었고 실행 증거로 사용하지 않았다.
- oracle: |h|≤0.1 scalar φ_k의 defining Taylor series 120항; extreme scale tests는 A=0이므로 φ_4(0)=1/24의 닫힌 식. production Padé와 독립이다.
- baseline tolerance: relative 1e-12, absolute 0. 기존 재현의 1e-14 absolute allowance에 의존하지 않는다.
- candidate: 가중치 `h^k b`의 곱셈 순서 변경 및 dense combination의 common amplitude normalization. 이들은 수학적으로 동치인 제한된 standalone 후보다.
- claim ceiling: native probe/targeted tests 통과는 해당 fixture와 API 경로에 국한한다. ResidualEstimate는 엄밀한 forward error certificate가 아니다. General nonnormal backward/forward stability, production 적용 및 전체 solver speedup은 주장하지 않는다.
- stop condition: 이전 세 반례와 현재 두 구체적 수치 위험에 대한 실제 native 실행·결과 분류·개발 acceptance를 확보하면 이 범위 감사를 종결한다. 독립 reviewer의 연구 후보 승격은 root 통합 단계에서 수행한다.
