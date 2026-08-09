# Matrix-Free Common-W Block Gate v0.14 — 먼저 읽기

## 목적

이 cycle은 RODAS5P의 공통 shifted operator

\[
W=M-h\gamma J_n
\]

를 명시적 Jacobian 행렬이나 factorization 없이

\[
Wv=Mv-h\gamma(J_nv)
\]

로 적용하고, 한 timestep의 8개 stage RHS를 동일 tolerance와 동일 preconditioner 아래에서 다음 네 방식으로 비교한다.

1. 독립 GMRES — serial RHS;
2. 독립 GMRES — local Rayon RHS concurrency;
3. dynamic rank deflation을 갖는 block GMRES;
4. seed basis projection 뒤 독립 true-residual refinement를 수행하는 seeded shared GMRES.

이번 cycle은 q=0/1/2 homotopy 적분기를 아직 연결하지 않는다. Stage-Batch Feasibility Gate에서 확인한 common-W 기회를 strict matrix-free 환경에서 검증하고, 다음 homotopy integration branch에 편입할 linear backend를 고르는 단계다.

## Canonical 결과

- cases: `432`
- solver rows: `1728`
- successful rows: `1692`
- failed rows: `36`
- explicit Jacobian builds: `0`
- factorization builds: `0`
- scientific checksum: `098384ae7c2ee20a1ac68d30646bb378ec64807e0ebc332638d16fa88d780bdf`

| 경로 | 성공률 | median speedup | median operator-vector ratio | 판정 |
|---|---:|---:|---:|---|
| independent GMRES serial | 1.000 | 1.000 | 1.000 | protected baseline |
| independent GMRES Rayon | 1.000 | 1.828 | 1.000 | robust concurrent fallback |
| block GMRES | 0.917 | 5.018 | 0.331 | high-reward research backend; default HOLD |
| seeded shared GMRES | 1.000 | 1.030 | 0.974 | rank-1 specialized path |

Block GMRES는 성공한 396개 row 중 353개에서 `1.15x` speedup gate를 넘었지만, 큰 dimension·높은 RHS rank·강한 stiffness/nonnormality에서 36개 hard failure가 발생했다. 실패한 work는 장부에 보존했다.

## 최종 판정

```text
STRICT MATRIX-FREE COMMON-W OPERATOR GATE: PASS
INDEPENDENT RAYON MULTI-RHS GATE: PASS
BLOCK-GMRES UNIVERSAL DEFAULT GATE: HOLD
SEEDED-SHARED UNIVERSAL DEFAULT GATE: HOLD
INTEGRATED HOMOTOPY SPEEDUP GATE: NOT YET TESTED
```

다음 단계는 synthetic exact-rank만 보고 dispatch rule을 고정하는 것이 아니라, 실제 q=0/1/2 homotopy 및 SABR round에서 8개 RHS의 numerical rank·singular-value decay·round-to-round angle을 측정하고, block/seeded/independent backend를 transactionally 선택하는 것이다.
