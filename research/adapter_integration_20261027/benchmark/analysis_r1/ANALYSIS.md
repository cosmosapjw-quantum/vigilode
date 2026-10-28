# Adapter comparisons

B/A below one favors B; times include solver setup and all trial work.

| Case | A → B | RHS A → B | B/A time | Range | Qualified | Endpoint error A → B |
|---|---|---:|---:|---|---|---:|
| rotation | evidence → adaptive | 5371 → 5371 | 0.996 | 0.785–1.060 | True | 9.26e-07 → 9.26e-07 |
| rotation | adaptive → dopri | 5371 → 5251 | 0.542 | 0.501–0.597 | False | 9.26e-07 → 9.28e-07 |
| decay | evidence → adaptive | 103 → 103 | 0.982 | 0.864–1.053 | True | 3.39e-09 → 3.39e-09 |
| decay | adaptive → dopri | 103 → 97 | 0.500 | 0.376–0.560 | False | 3.39e-09 → 8.04e-09 |
| case5 | evidence → adaptive | 1456 → 2040 | 0.939 | 0.683–1.362 | False | 5.48e-08 → 5.37e-08 |
| case5 | adaptive → rodas | 2040 → 307 | 0.969 | 0.902–1.105 | True | 5.37e-08 → 1.04e-07 |
| case5 | adaptive-unseeded → adaptive | 2032 → 2040 | 1.029 | 0.986–1.178 | False | 5.53e-08 → 5.37e-08 |
| case5 | adaptive → rodas-dense | 2040 → 307 | 0.086 | 0.062–0.105 | False | 5.37e-08 → 1.04e-07 |
| case5 | rodas → rodas-dense | 307 → 307 | 0.072 | 0.065–0.086 | False | 1.04e-07 → 1.04e-07 |
| stiff-forced | evidence → adaptive | 456 → 394 | 0.895 | 0.855–1.146 | False | 7.33e-10 → 7.33e-10 |
| stiff-forced | adaptive → rodas | 394 → 112 | 0.256 | 0.244–0.312 | False | 7.33e-10 → 8.15e-10 |
| stiff-forced | adaptive-unseeded → adaptive | 456 → 394 | 0.900 | 0.660–0.916 | False | 7.33e-10 → 7.33e-10 |
| relaxation | evidence → adaptive | 675 → 722 | 1.188 | 1.080–1.409 | False | 4.81e-09 → 4.77e-09 |
| relaxation | adaptive → rodas | 722 → 462 | 0.580 | 0.515–0.849 | False | 4.77e-09 → 9.17e-09 |
| relaxation | adaptive-unseeded → adaptive | 723 → 722 | 0.946 | 0.768–1.248 | False | 4.81e-09 → 4.77e-09 |
| transition-39 | evidence → adaptive | 184724 → 30515 | 0.541 | 0.335–0.843 | False | 1.8e-08 → 2.19e-10 |
| transition-39 | adaptive → rodas | 30515 → 8853 | 1.662 | 1.293–1.939 | False | 2.19e-10 → 2.7e-10 |
| transition-39 | adaptive-unseeded → adaptive | 30515 → 30515 | 1.012 | 0.713–1.538 | False | 2.19e-10 → 2.19e-10 |
| transition-79 | evidence → adaptive | 68323 → 29327 | 0.647 | 0.599–0.904 | False | 1.85e-08 → 2.46e-09 |
| transition-79 | adaptive → rodas | 29327 → 8446 | 3.098 | 2.475–4.034 | False | 2.46e-09 → 8.44e-10 |
| transition-79 | adaptive-unseeded → adaptive | 29335 → 29327 | 1.030 | 0.888–1.204 | True | 4.05e-10 → 2.46e-09 |
| gyro-tight | evidence → adaptive | 24555 → 24554 | 1.064 | 0.606–1.153 | False | 3.9e-05 → 3.9e-05 |
| gyro-tight | adaptive → dopri | 24554 → 24403 | 0.775 | 0.700–0.870 | False | 3.9e-05 → 3.9e-05 |
| gyro-default | evidence → adaptive | 6769 → 6745 | 0.930 | 0.742–1.272 | False | 0.0316 → 0.0316 |
| gyro-default | adaptive → dopri | 6745 → 6631 | 0.647 | 0.621–0.717 | False | 0.0316 → 0.0317 |

Unqualified comparisons:
- rotation adaptive/dopri: batch shorter than 20ms.
- decay adaptive/dopri: early/late drift >15%, block range >30%.
- case5 evidence/adaptive: early/late drift >15%, block range >30%.
- case5 adaptive-unseeded/adaptive: early/late drift >15%, block range >30%.
- case5 adaptive/rodas-dense: block range >30%.
- case5 rodas/rodas-dense: early/late drift >15%, block range >30%.
- stiff-forced evidence/adaptive: block range >30%.
- stiff-forced adaptive/rodas: early/late drift >15%, block range >30%.
- stiff-forced adaptive-unseeded/adaptive: block range >30%.
- relaxation evidence/adaptive: early/late drift >15%, block range >30%.
- relaxation adaptive/rodas: early/late drift >15%, block range >30%, batch shorter than 20ms.
- relaxation adaptive-unseeded/adaptive: early/late drift >15%, block range >30%, batch shorter than 20ms.
- transition-39 evidence/adaptive: early/late drift >15%, block range >30%.
- transition-39 adaptive/rodas: early/late drift >15%, block range >30%.
- transition-39 adaptive-unseeded/adaptive: early/late drift >15%, block range >30%.
- transition-79 evidence/adaptive: block range >30%.
- transition-79 adaptive/rodas: early/late drift >15%, block range >30%.
- gyro-tight evidence/adaptive: early/late drift >15%, block range >30%.
- gyro-tight adaptive/dopri: early/late drift >15%, block range >30%.
- gyro-default evidence/adaptive: early/late drift >15%, block range >30%, at least one endpoint cap failure, sampled timed solve cap failure, registered report-only tolerance cohort.
- gyro-default adaptive/dopri: at least one endpoint cap failure, sampled timed solve cap failure, registered report-only tolerance cohort.

Failed cells: 0. See ANALYSIS.json for complete records.

The two transition positions are new parameter holdouts within an already studied function family.
The case5 matrix-free/dense pair isolates backend cost from policy overhead; neither route is silently substituted.
