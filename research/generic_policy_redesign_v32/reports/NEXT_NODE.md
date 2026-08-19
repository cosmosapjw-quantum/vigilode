# NEXT NODE — v3.3 Independent zeta34 Safety Calibration

## Parent

Use the final v3.2 closeout commit only.

## Goal

Independently calibrate a one-sided abstention rule using only the preselected feature form `zeta34`, without reopening feature discovery.

## Data firewall

- N=96/N=256: discovery only; forbidden for threshold tuning in v3.3.
- N=192: independent calibration; may be opened only after the v3.3 selection contract is committed.
- N=384: final safety holdout; remains sealed during calibration.
- N=2048: scaling holdout; remains sealed.

## Mandatory pre-output contract

Before N=192 execution, freeze:

1. direction: higher zeta34 is less safe; runtime E candidate may be recommended only for `zeta34 <= tau`;
2. exact candidate-threshold enumeration including all-abstain;
3. multiplicity-aware zero-unsafe calibration bound or another fully specified finite-sample conservative rule;
4. minimum nontrivial recommendation count / coverage gate;
5. groupwise family checks and fail-closed behavior;
6. any threshold shrink/safety margin;
7. tie-breaking;
8. no learned classifier, no family-specific threshold.

If the calibration rule produces all-abstain, do not spend N=384.

## Runtime restrictions

- R-JF remains committed trajectory.
- E-K endpoint is audit label only during calibration; runtime E continuation 0.
- no active switching.
- N=384 and N=2048 unopened until calibration yields a nontrivial frozen policy.

## Promotion condition

A nontrivial calibration policy must be committed with its N=192 raw-data hashes before N=384 is opened. N=384 then provides the safety holdout with no retuning.
