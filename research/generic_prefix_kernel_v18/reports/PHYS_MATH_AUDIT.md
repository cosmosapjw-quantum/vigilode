# PHYS–MATH AUDIT — G4-S4

## Definitions and dimensions

- Krylov dimension, GMRES iteration count, prediction error, prefix inflation, wall ratio는 모두 무차원이다.
- E-K와 R-JF는 동일 state dimension과 matrix-free operator contract에서 비교된다.
- shifted operator sign은 기존 protected RODAS convention을 유지한다.

## Correctness

- cold path와 prefix-resume path의 output relative defect가 모두 정확히 0이다.
- R-JF 최종 authority는 preconditioned residual이 아니라 original-system true residual이다.
- E-K final authority는 기존 fused-phi residual certificate다.
- active method switching이 없으므로 switching-induced order loss는 이번 node에서 발생하지 않는다.

## Numerical limitations

1. Nonnormal Krylov convergence는 짧은 prefix 뒤 늦게 변할 수 있다.
2. Projected exponential cost는 Krylov dimension에 선형이 아니다.
3. One-substep full-MGS result를 restart/IOP/rational Krylov로 일반화할 수 없다.
4. Synthetic structured operators이며 complete adaptive ODE trajectories가 아니다.

## Counterexample retained

One-vector prefix는 residual slope가 없어 prediction이 크게 실패한다. Two-vector도 severe `N=512` regime에서 sealed 19% envelope를 초과한다. 따라서 exact basis reuse가 곧 reliable cost prediction을 뜻하지 않는다.

## Verdict

- mathematical reuse contract: PASS
- output parity: PASS
- two-vector wall-cost predictor: FAIL
- active switching: NOT AUTHORIZED
