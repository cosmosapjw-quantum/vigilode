# PHYS–MATH–CODE AUDIT — G4-S4

## Equation-to-code reality

```text
E-K prefix:
  augmented_fused_operator
  -> FusedPhiPrefixSession::begin
  -> retained Arnoldi basis/Hessenberg
  -> FusedPhiPrefixSession::finish
  -> existing residual certificate

R-JF prefix:
  initial residual/preconditioner
  -> GmresPrefixSession::begin
  -> retained basis/directions/Hessenberg/least-squares state
  -> GmresPrefixSession::finish
  -> original-system true residual
```

## Genuinely fixed

- prefix state is actual Rust solver state, not retrospective arithmetic;
- selected prefix is reused without cold rebuild;
- candidate and prefix work are counted;
- shifted gate operator is allocation-free;
- microtiming is automatically batched;
- empirical performance threshold was removed from unit tests and retained in canonical analysis only.

## Instrument defect and correction

The first canonical attempt used one-shot microtiming and is explicitly invalidated. The corrected timing amendment was committed before the seven corrected campaigns. Scientific thresholds and selected prefix remained unchanged.

## Remaining code risks

- full-MGS fused-phi prefix is restricted to one substep;
- no restart/recycle transfer across ODE steps;
- cost predictor assumes short-prefix contraction can extrapolate total dimension/work;
- active selector/controller/cache transfer is absent by design;
- memory peak and allocation counters are not yet canonical gates.

## Regression sufficiency

- E-K prefix/cold parity test;
- R-JF prefix/cold parity test;
- read-only gate contract test;
- seven campaign scientific identity;
- BDF/Radau freeze and physical-client exclusion inherited from G0.

## Verdict

- reusable kernel implementation: PASS
- timing authority: PASS after amendment
- cost-ratio predictor: FAIL
- active integration path: NOT IMPLEMENTED
