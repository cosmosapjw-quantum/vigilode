# Plot-based CRAG Audit — G4-S5B1

## Correctness

`G4_S5B1_HOLDOUT_REGRET.png`는 두 contract policy의 서로 다른 failure mode를 분리한다.

- R-default: 낮은 overhead, 큰 missed-opportunity tail
- E-first: 낮은 median regret, fallback/review overhead tail

`safe-e-diagnostic`은 holdout에서 oracle에 가깝지만 activation policy가 아니다.

## Retrieval

`G4_S5B1_METRIC_CORRECTION.png`에서 24/258 hard rows가 raw-wall winner와 unit-time winner 사이에서 바뀐다. 따라서 P0 correction은 선택적 미세조정이 아니라 authority correction이다.

## Augmented

7개 campaign 모두에서:

- R-default P95 regret가 1.5를 크게 넘음
- E-first P95 regret가 1.5 근처지만 안정적으로 초과
- safe-E holdout P95는 1.3 아래
- safe-E calibration P95는 약 2 이상

이 결론은 one-shot timing 하나에 의존하지 않는다.

## Generation

Plots가 예측하는 다음 결과는 다음이다.

- 더 복잡한 continuous selector보다 startup economic admission이 정보효율이 높다.
- quadratic-like states를 구별하지 못하면 E-first는 generic policy가 될 수 없다.
- 6-step residence를 모든 startup에 강제하면 짧은 고이득 trajectory를 놓친다.

## Adversarial checks

1. Residence를 줄이면 R-default의 advection miss는 줄지만 probe overhead/thrashing 위험이 증가한다.
2. Economic margin을 완화하면 E-first review가 줄지만 quadratic-like misselection은 해결되지 않는다.
3. Prefix 비용을 0으로 둬도 R-default의 short-trajectory regret는 남는다.
4. Failed E work를 삭제하면 E-first overhead가 거짓으로 PASS한다.

## Claim status

- **Surviving:** E-K/R-JF asymmetric portfolio has substantial opportunity.
- **Narrowed:** safety telemetry is sufficient for robustness, not for economics.
- **Rejected:** either current asymmetric policy is ready for active switching.
