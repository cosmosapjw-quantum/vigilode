# G4-S0 PHYS–MATH–CODE AUDIT

## Code-path reality

이번 checkpoint는 Python read-only analysis만 추가했다.

```text
7 frozen G3 JSON campaigns
  -> exact scientific-identity check
  -> per-candidate campaign medians
  -> protected-RODAS external error targets
  -> cross-tolerance same-error candidate selection
  -> strict/auto offline oracle
  -> feature/robustness/overhead audits
  -> plots and machine-readable artifacts
```

Candidate Rust source, adaptive controller, coefficient, tolerance, backend는 변경하지 않았다.

## Contract satisfaction

- 7 campaigns required: PASS
- source schema check: PASS
- byte-deterministic analysis rerun: PASS
- candidate algorithms modified: false
- switching activated: false
- BDF/Radau frozen hashes: PASS
- physical client bindings: absent

## Numerical honesty

- Candidate가 target을 달성하지 못하면 uncovered로 남김.
- Candidate는 자신의 tolerance ladder에서 target을 달성하는 가장 싼 run을 선택함.
- Failure 또는 missing coverage를 다른 tolerance의 extrapolation으로 채우지 않음.
- Auto oracle의 Radau 결과는 frozen reference implementation이라는 caveat와 함께 보고함.

## Regression risk

낮음. Production Rust source가 변경되지 않았다. 주 위험은 analysis semantics이며, 다음 test로 고정했다.

- canonical result contract
- byte-deterministic rerun
- campaign count/schema validation

## Ranked issues

- P0: 없음.
- P1: zero-extra-work feature의 out-of-family generalization 부족.
- P1: switch/setup cost와 accepted-step-local labels 부재.
- P2: JVP-quality probe가 일부 cell에서 5–12.5% 비용.
- P2: auto oracle의 small-dense Radau 우위가 broad performance claim으로 오독될 위험.
- P3: plot label 및 microbenchmark timing noise.
