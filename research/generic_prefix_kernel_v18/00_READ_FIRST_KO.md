# G4-S4 Actual Reusable Prefix Kernel — Read First

이번 node는 G4-S3의 retrospective prefix model을 실제 Rust Krylov state로 교체했다.

## 완료

- E-K fused-phi Arnoldi prefix basis/Hessenberg 재사용
- R-JF GMRES prefix basis/directions/Hessenberg/least-squares state 재사용
- cold path와 prefix-resume path의 output relative defect 정확히 0
- selected two-vector prefix cost gate 통과
- seven CPU-pinned corrected campaign의 scientific identity 통과

## 실패

Two-vector prefix의 E-K/R-JF wall-cost ratio point prediction은 sealed 19% gate를 통과하지 못했다.

```text
calibration P95-higher  0.257
holdout P95-higher      0.289
campaign P95 range      0.218–0.400
campaigns passing       0/7
```

## 판정

```text
actual reusable prefix kernels  PROMOTE
exact prefix-resume parity      PROMOTE
two-vector point predictor      HOLD
active switching                HOLD
```

다음 node는 point predictor를 post-hoc tuning하지 않고, expanded sealed telemetry에서 conservative cost-ratio interval과 abstention policy를 검증한다.
