# Next node — P1-01 read-only event-conditioned E-K re-entry shadow

## Proposed branch

```text
generic-read-only-event-reentry-shadow-v2.5
```

Create it only from the final committed P1-00 closeout.

## Central distinction

`rho_c2_wrms` is available only **after** the E-K dependency-level-1 prefix has
been executed. It can therefore classify/abort a launched E-K shadow trial, but
it cannot be treated as a zero-cost R-JF-side trigger for deciding whether to
launch that trial.

P1-01 must separately define:

1. an event trigger using only already-available protected R-JF state/telemetry;
2. residence, review interval and cooldown semantics;
3. the speculative E-K level-1 prefix work ledger;
4. the tolerance-scaled early-defect decision surface;
5. full E-K completion and failed-shadow/fallback accounting.

## Required architecture

- Protected matrix-free RODAS5P remains the **only committed trajectory**.
- E-K state is shadow-only; no requested output or controller state is mutated.
- Routine dual-prefix acquisition remains forbidden.
- Event candidates must be detectable before E-K work, using R-JF-side
  quantities only.
- Once an event launches E-K, the level-1 prefix is charged even if `rho` stops
  the shadow before the full step.
- Every discarded/full/failed E-K shadow cost remains in the ledger.

## Calibration discipline

Do not import the failed G4-S5B1 policy thresholds as authoritative. They may be
historical comparison points only. Seal a bounded event family before output,
then use the existing calibration/holdout partition without physical-client
retuning.

At minimum sweep, without activating:

- residence/review/cooldown candidates;
- R-JF-side drift/event candidates;
- complete observed `rho_c2_wrms` boundaries on calibration only;
- raw `eta_c2` control;
- static protected R-JF control.

Freeze any candidate policy before its holdout replay. If no candidate survives
without favorable threshold cherry-picking, retain static R-JF.

## Hard gates

- committed R-JF state/output/controller: exact identity;
- unsafe shadow recommendation: 0;
- failed/discarded prefix work retained: 100%;
- no hidden RHS/JVP/phi/Jacobian/Newton work;
- median holdout regret <= 1.15;
- P95 holdout regret <= 1.5;
- median trial overhead <= 3%;
- P95 trial overhead <= 5%;
- no active switching;
- no physical-client threshold tuning.

The overhead gates apply to **actual speculative prefix/full-shadow work**, not
just the cheap scalar WRMS reduction measured in P1-00.

## Required plots

- event locations over R-JF residence;
- full vs early-aborted E-K shadow work;
- regret distribution vs static R-JF;
- overhead CDF including failed/discarded trials;
- raw-vs-scaled early-defect outcomes at real event windows;
- calibration-vs-holdout policy stability.

## Stop rule

If the event trigger cannot avoid routine E-K prefix acquisition, if tail
regret/overhead gates fail after frozen calibration, or if committed R-JF
identity is broken, stop P1-01. Do not proceed to forced active switches.

Only a passing read-only shadow opens P1-02 forced-switch fifth-order recovery.
