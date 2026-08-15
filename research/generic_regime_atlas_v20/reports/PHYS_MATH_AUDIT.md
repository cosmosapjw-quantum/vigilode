# PHYS–MATH AUDIT — G4-S5B0

## PASS

1. Both methods evaluate the same dimensionless ODE state, time, and accepted step size.
2. Cost ratios, prefix fractions, residence lengths, and speedups have consistent units.
3. R-JF is the only committed path; E-K cannot contaminate accepted state or controller history.
4. Explicit Jacobian builds, direct factorizations, and fast-path Newton iterations remain zero.
5. E-K inadmissibility is preserved as R-JF authority rather than reclassified as a cost win.
6. N=128 and N=512 show the same qualitative winner geometry.

## P1 limitations

- The six systems are controlled ramped/replicated atlas variants, not exact canonical
  Robertson/HIRES/van-der-Pol benchmarks.
- Local E-K error is not an external global-error certificate.
- Some nonautonomous `partial_t` callbacks use central time differences and require a later
  step-halving sensitivity audit.
- Positivity and family-specific invariants are not a universal atlas gate.

## Counterexamples preserved

- E-K can be faster but inadmissible; wall time alone never promotes it.
- Short E-K residences can make prefix overhead exceed 3% even when the aggregate overhead is tiny.
- Earlier oscillatory-PR and quadratic results prevent extrapolating E-first superiority to all regimes.

## Verdict

Residence-time opportunity is physically and numerically coherent as a read-only result.
Active switching and universal solver claims remain unproved.
