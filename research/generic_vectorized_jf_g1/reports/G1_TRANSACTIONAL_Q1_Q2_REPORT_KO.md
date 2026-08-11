# G1 Transactional q1→q2 Report — canonical pointer

이 파일은 기존 보고서명과의 호환성을 위한 entry point다. 최종 확장 corpus와 exact 수치는
[`G1_TRANSACTIONAL_CYCLE_REPORT_KO.md`](G1_TRANSACTIONAL_CYCLE_REPORT_KO.md)를 정본으로 한다.

Canonical closeout:

```text
one-step: q1 3 / q2 4 / fallback 1, fast 0.875, fallback 0.125
adaptive: q1 0 / q2 100 / fallback 101, fast 0.4975, fallback 0.5025
median/P95 depth: 16 / 16
same-error speedup: t1 0.233, t4 0.076
explicit-J / direct factorization / fast Newton: 0 / 0 / 0
verdict: HOLD q1→q2 as generic primary solver
```
