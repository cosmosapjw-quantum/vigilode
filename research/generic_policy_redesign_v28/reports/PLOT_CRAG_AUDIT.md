# PLOT-DRIVEN CRAG AUDIT — v2.8

## Generated figures

1. `LEVEL2_AMPLIFICATION_VS_FULL_E_ERROR.png`
2. `HIRES_LEVEL2_STAGE_GROWTH.png`
3. `LEVEL2_CUMULATIVE_JVP_COST_CDF.png`
4. `LEVEL2_AMPLIFICATION_FAMILY_RANGES.png`

## Direct visual reading

### Amplification vs full-E error

The two historically inadmissible points lie above both the full-E error boundary and every safe event in \(\rho_4/\rho_3\). The separation is visually large compared with the within-safe scatter. However, the two unsafe points are essentially a duplicated HIRES geometry across dimensions, so the plot is evidence of a structural signal, not sample-size evidence for a classifier.

### HIRES stage growth

The unsafe HIRES trajectory begins at an unexceptional \(\rho_2\approx2.27\), but its growth from c3 to c4 is visibly steeper than the safe lower-amplitude neighbor. The safe high-amplitude HIRES trajectory demonstrates why absolute \(\rho_i\) cannot be the authority: it is much larger at every stage and still historically admissible.

### Cost CDF

Most level1+2 prefixes cost around 12–16% of the corresponding R-JF JVP-vector count, but both dimensions have a visible right tail near 24–25%. This rules out describing level 2 as a negligible routine probe.

### Family ranges

The small-h baseline 3.24 runs through several family ranges rather than separating them. HIRES contains the only observed unsafe high-amplification excursion. Van der Pol has safe values near 3.61, proving that a threshold only modestly above 3.24 would create false rejections.

## CRAG

### Correctness

The stage-growth interpretation agrees with the frozen equations and with the small-h expansion \(\rho_4/\rho_3\to(c_4/c_3)^2=3.24\) under smooth nondegenerate assumptions. The figure therefore has a mathematical baseline rather than being a purely fitted visualization.

### Retrieval

The original parallel pexprb54s4 construction has two independent internal stages after D2 and a three-sequential-evaluation dependency structure. The general exponential Rosenbrock framework linearizes the flow at each step and treats nonlinear remainder terms explicitly. These external facts are consistent with interpreting D3/D4 as genuinely later nonlinear information, but they provide no theorem that the empirical ratio is an admissibility certificate.

### Augmented adversarial mutations

- **Dimension mutation:** the same unsafe HIRES geometry reproduces at N=128 and N=512 with nearly identical ratios.
- **Absolute-magnitude mutation:** safe HIRES has both much larger and smaller \(\rho_2\) than the unsafe event; absolute defect magnitude fails.
- **Baseline mutation:** safe events cross the 3.24 asymptotic baseline on both sides; baseline itself cannot be a threshold.
- **Family mutation:** safe Van der Pol reaches 3.61, narrowing the observed safety gap to 0.668.
- **Cost mutation:** semilinear is safe but produces the largest level-2 JVP tail, so a safety-improving feature can still fail economic gates.
- **Post-hoc raw-ratio mutation:** \(\eta_4/\eta_3\) is close to its 1.8 asymptotic baseline for safe events and high for the unsafe HIRES event, but it was not predeclared and is retained only as corroboration.

### Generation

The surviving hypothesis is not “threshold \(\rho_4/\rho_3\) at the observed gap.” It is:

> finite-step departure of the normalized stage-growth profile from its small-h baseline is a candidate safety witness that requires independent calibration and a separate speculative-cost gate.

## Final claim status

- “Level 2 adds information absent at level 1 on this corpus”: **SURVIVES**.
- “The observed HIRES failure has anomalous later-stage growth”: **SURVIVES**.
- “\(\rho_4/\rho_3\) is a generic safety certificate”: **REJECTED / unsupported**.
- “Level 2 is cheap enough to run routinely”: **REJECTED**.
- “Proceed to active switching”: **REJECTED**.
