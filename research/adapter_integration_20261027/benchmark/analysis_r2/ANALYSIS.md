# Adapter comparisons

B/A below one favors B; times include solver setup and all trial work.

| Case | A → B | RHS A → B | B/A time | Range | Qualified | Endpoint error A → B |
|---|---|---:|---:|---|---|---:|
| rotation | evidence → adaptive | 5371 → 5371 | 0.980 | 0.848–1.258 | False | 9.26e-07 → 9.26e-07 |
| rotation | adaptive → dopri | 5371 → 5251 | 0.581 | 0.480–0.602 | False | 9.26e-07 → 9.28e-07 |
| decay | evidence → adaptive | 103 → 103 | 1.016 | 0.998–1.445 | False | 3.39e-09 → 3.39e-09 |
| decay | adaptive → dopri | 103 → 97 | 0.533 | 0.482–0.555 | True | 3.39e-09 → 8.04e-09 |
| case5 | evidence → adaptive | 1456 → 680 | 0.878 | 0.792–1.242 | False | 5.48e-08 → 5.48e-08 |
| case5 | adaptive → rodas | 680 → 307 | 1.245 | 1.207–1.281 | True | 5.48e-08 → 1.04e-07 |
| case5 | adaptive-unseeded → adaptive | 680 → 680 | 1.038 | 0.940–1.136 | True | 5.48e-08 → 5.48e-08 |
| case5 | adaptive → rodas-dense | 680 → 307 | 0.095 | 0.092–0.098 | True | 5.48e-08 → 1.04e-07 |
| case5 | rodas → rodas-dense | 307 → 307 | 0.075 | 0.071–0.081 | True | 1.04e-07 → 1.04e-07 |
| case5 | adaptive-no-startup-retry → adaptive | 2032 → 680 | 0.796 | 0.772–0.909 | True | 5.53e-08 → 5.48e-08 |
| stiff-forced | evidence → adaptive | 456 → 394 | 0.898 | 0.860–0.936 | True | 7.33e-10 → 7.33e-10 |
| stiff-forced | adaptive → rodas | 394 → 112 | 0.269 | 0.260–0.302 | True | 7.33e-10 → 8.15e-10 |
| stiff-forced | adaptive-unseeded → adaptive | 456 → 394 | 0.872 | 0.850–0.897 | True | 7.33e-10 → 7.33e-10 |
| relaxation | evidence → adaptive | 675 → 722 | 1.122 | 1.087–1.159 | True | 4.81e-09 → 4.77e-09 |
| relaxation | adaptive → rodas | 722 → 462 | 0.565 | 0.499–0.650 | False | 4.77e-09 → 9.17e-09 |
| relaxation | adaptive-unseeded → adaptive | 723 → 722 | 0.991 | 0.984–1.046 | True | 4.81e-09 → 4.77e-09 |
| transition-39 | evidence → adaptive | 184724 → 30515 | 0.468 | 0.451–0.538 | False | 1.8e-08 → 2.19e-10 |
| transition-39 | adaptive → rodas | 30515 → 8853 | 1.778 | 1.601–3.052 | False | 2.19e-10 → 2.7e-10 |
| transition-39 | adaptive-unseeded → adaptive | 30515 → 30515 | 1.048 | 0.474–1.298 | False | 2.19e-10 → 2.19e-10 |
| transition-79 | evidence → adaptive | 68323 → 29327 | 0.666 | 0.580–0.942 | False | 1.85e-08 → 2.46e-09 |
| transition-79 | adaptive → rodas | 29327 → 8446 | 2.668 | 2.164–3.149 | False | 2.46e-09 → 8.44e-10 |
| transition-79 | adaptive-unseeded → adaptive | 29335 → 29327 | 1.089 | 0.568–1.129 | False | 4.05e-10 → 2.46e-09 |
| gyro-tight | evidence → adaptive | 24555 → 24554 | 0.888 | 0.848–1.574 | False | 3.9e-05 → 3.9e-05 |
| gyro-tight | adaptive → dopri | 24554 → 24403 | 0.707 | 0.501–0.897 | False | 3.9e-05 → 3.9e-05 |
| gyro-default | evidence → adaptive | 6769 → 6745 | 1.088 | 0.824–2.116 | False | 0.0316 → 0.0316 |
| gyro-default | adaptive → dopri | 6745 → 6631 | 0.611 | 0.481–0.738 | False | 0.0316 → 0.0317 |

Unqualified comparisons:
- rotation evidence/adaptive: early/late drift >15%, block range >30%, batch shorter than 80ms.
- rotation adaptive/dopri: early/late drift >15%.
- decay evidence/adaptive: early/late drift >15%, block range >30%.
- case5 evidence/adaptive: early/late drift >15%, block range >30%.
- relaxation adaptive/rodas: batch shorter than 80ms.
- transition-39 evidence/adaptive: early/late drift >15%, block range >30%.
- transition-39 adaptive/rodas: block range >30%.
- transition-39 adaptive-unseeded/adaptive: early/late drift >15%, block range >30%.
- transition-79 evidence/adaptive: early/late drift >15%, block range >30%.
- transition-79 adaptive/rodas: early/late drift >15%, block range >30%.
- transition-79 adaptive-unseeded/adaptive: early/late drift >15%, block range >30%.
- gyro-tight evidence/adaptive: early/late drift >15%, block range >30%.
- gyro-tight adaptive/dopri: early/late drift >15%, block range >30%.
- gyro-default evidence/adaptive: early/late drift >15%, block range >30%, at least one endpoint cap failure, sampled timed solve cap failure, registered report-only tolerance cohort.
- gyro-default adaptive/dopri: early/late drift >15%, block range >30%, at least one endpoint cap failure, sampled timed solve cap failure, registered report-only tolerance cohort.

Failed cells: 0. See ANALYSIS.json for complete records.

The two transition positions are new parameter holdouts within an already studied function family.
The case5 matrix-free/dense pair isolates backend cost from policy overhead; neither route is silently substituted.
