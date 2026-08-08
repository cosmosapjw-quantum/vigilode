# Decision Log

1. Preserve all fixed-step APIs and baselines.
2. Add adaptive APIs rather than silently changing fixed methods.
3. Use native RODAS embedded error for RODAS endpoints.
4. Use step doubling as the first BDF/Radau adaptive correctness lane.
5. Implement nonuniform BDF2 coefficients only inside variable/adaptive mode.
6. Defer native RADAU5 estimator and variable-order BDF3–5.
7. Compare all adaptive candidates by external global error and full work.
