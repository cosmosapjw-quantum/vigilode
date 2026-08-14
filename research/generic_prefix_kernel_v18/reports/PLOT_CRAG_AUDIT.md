# Plot-based CRAG Audit — G4-S4

## Correctness

`G4_S4_PREDICTED_VS_ACTUAL_COST_RATIO.png`에서 points는 대체로 ideal line의 주변에 있지만, calibration/holdout 일부가 19% relative envelope를 벗어난다. Aggregate median만 보고 selector를 승격할 수 없다.

## Retrieval

`G4_S4_PREDICTION_ERROR_BY_CASE.png`는 failure가 dimension 하나에 단조롭게 묶이지 않음을 보인다. `N=32,S=1000`과 `N=512,S=1000`이 gate를 깨지만 `N=128,S=1000`은 잘 맞는다. 단순 dimension threshold는 원인을 설명하지 못한다.

## Augmented

`G4_S4_PREFIX_COST_INFLATION.png`와 `G4_S4_REUSE_TOTAL_OVER_COLD.png`는 prefix 자체와 continuation economics는 안정적임을 보인다. 따라서 predictor failure를 prefix-reuse failure로 확대해서는 안 된다.

## Generation

향후 predictor는 single point가 아니라 uncertainty interval을 출력해야 한다. Exponential projected-space cost와 GMRES least-squares/orthogonalization cost를 method-specific하게 분리하지 않으면 severe nonnormal regime에서 point prediction tail이 반복될 것으로 예상한다.

## Adversarial mutation

- one-vector prefix: prediction failure가 훨씬 크다;
- campaign change: seven campaign 모두 19% P95 gate 실패;
- severe nonnormal regime: 최대 campaign-median miss를 만든다;
- timing batching removal: scheduler noise가 scientific conclusion을 오염한다.

## Claim classification

- surviving: actual prefix reuse is exact and cheap;
- narrowed: two-vector prefix is informative but not a 19%-accurate wall-ratio estimator;
- rejected: active switching authorization from G4-S4.
