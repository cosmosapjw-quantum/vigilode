# PHYS–MATH–CODE AUDIT — G4-S5B1

## Code-path reality

```text
immutable G4-S1 campaigns
  + immutable G4-S5B0 campaigns
  -> common step table
  -> P0 unit-time metric correction
  -> calibration-only bounded policy grid
  -> sealed holdout replay
  -> fallback-inclusive cost ledger
```

## Genuinely fixed

- raw-wall/cost-per-unit-time ambiguity
- gate-aware parameter selection
- selected vs discarded prefix accounting
- failed E attempt accounting
- safety detector와 economic detector의 역할 분리
- 7-campaign scientific identity

## No numerical patch

```text
candidate Rust source modified   false
BDF/Radau source modified        false
active switching                 false
physical clients                 not executed
```

## Remaining implementation gap

- actual startup dual-prefix admission inside the complete-integrator path
- prefix cost envelope on quadratic vs E-friendly states
- method-specific controller/cache initialization
- forced-switch fifth-order recovery
- N=2048 sealed holdout

## Regression risk

가장 큰 위험은 `safe-e-diagnostic`의 좋은 holdout 숫자를 activation evidence로 오독하는 것이다. 같은 policy는 calibration quadratic에서 E-K를 안전하지만 비경제적인 method로 선택한다.
