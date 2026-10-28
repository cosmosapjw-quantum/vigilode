# Paired observation-policy benchmark

Source: `5913fe062706c820e768b9e150e5cd1c274f7bf9`.

288/288 full-state replays pass; 576/576 raw hashes match.
Dense output work/final-state invariance: 24/24 pairs.

The table chooses each policy's fastest measured configuration satisfying the same independent error cap. Missing entries mean that the sampled tolerances did not qualify both policies.

| Model | n | Error cap | Landing / dense median time | Observed min–max ratio envelope |
|---|---:|---:|---:|---:|
| decay | 1 | 1e-04 | 2.681 | 2.646–9.601 |
| decay | 1 | 1e-06 | 2.659 | 1.747–9.459 |
| decay | 1 | 1e-08 | 2.659 | 1.747–9.459 |
| logistic | 1 | 1e-04 | 2.863 | 2.714–4.515 |
| logistic | 1 | 1e-06 | 2.863 | 2.714–4.515 |
| logistic | 1 | 1e-08 | 2.621 | 2.399–4.323 |
| driven | 1 | 1e-04 | 4.516 | 3.388–5.184 |
| driven | 1 | 1e-06 | 4.158 | 2.965–4.498 |
| driven | 1 | 1e-08 | insufficient | — |
| rapid | 2 | 1e-04 | 1.479 | 1.408–2.367 |
| rapid | 2 | 1e-06 | 1.059 | 0.566–1.363 |
| rapid | 2 | 1e-08 | insufficient | — |
| damped | 2 | 1e-04 | 1.716 | 0.216–1.968 |
| damped | 2 | 1e-06 | 1.716 | 0.216–1.968 |
| damped | 2 | 1e-08 | 0.712 | 0.105–1.146 |
| nonnormal | 2 | 1e-04 | 3.914 | 2.854–7.898 |
| nonnormal | 2 | 1e-06 | 3.628 | 0.500–7.116 |
| nonnormal | 2 | 1e-08 | 2.613 | 1.368–5.564 |
| large-vector | 4096 | 1e-04 | 1.820 | 1.561–2.917 |
| large-vector | 4096 | 1e-06 | 1.662 | 1.153–2.247 |
| large-vector | 4096 | 1e-08 | 0.889 | 0.693–1.054 |
| large-vector | 16384 | 1e-04 | 2.046 | 0.861–2.779 |
| large-vector | 16384 | 1e-06 | 1.701 | 1.279–1.937 |
| large-vector | 16384 | 1e-08 | 1.058 | 0.930–1.196 |

Ratios above one favor dense observation. This compares two observation policies of the current DOPRI5 implementation on the same host. It is not a comparison to a previous source version or external solvers.

A successful solve only certifies that the adaptive driver reached its endpoint under its estimator. The reported analytic errors and strict-cap failures remain visible in SUMMARY.json.
