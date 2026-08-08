# 먼저 읽을 것 — RODAS5P Rust pre-homotopy baseline

이 branch는 `rust-parity-v0.1.0-alpha1` 위에서 R10-A allocation/kernel audit까지 반영한
**homotopy 이전 Rust 기준선**이다. Homotopy/continuation 알고리즘은 production source에 포함하지 않았다.

## 최종 판정

- allocation-free shifted operator와 reusable Krylov workspace: **PROMOTE**
- 동일 Rust binary/kernel 아래의 GMRES·LGMRES·GCRO-DR 비교 기반: **PROMOTE**
- 252개 paired cell에서 baseline/current operator work: 전부 동일
- GMRES/LGMRES wall 개선: 중앙값 약 1% 수준
- GCRO-DR wall 개선: 없음; projected algebra allocation이 남음
- default solver 및 unconditional recycling 변경: **HOLD**
- `(m,k)` tuning, adaptive admission/reset, nonlinear frozen trace 확대: **DEFER**
- 다음 active branch: lower-triangular nonlinear stage coupling을 겨냥한 homotopy-based method 연구

상세 문서:

1. `research/reports/R10A_ALLOCATION_KERNEL_AUDIT_KO.md`
2. `research/closeout/PRE_HOMOTOPY_BASELINE_CLOSEOUT_KO.md`
3. `research/results/R10A_KERNEL_AUDIT_SUMMARY.json`
4. `research/HOMOTOPY_BRANCH_HANDOFF_KO.md`
5. `research/receipts/R10A_FINAL_VALIDATION_WORKTREE.log`
