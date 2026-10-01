# Preregistration: frozen-operator polynomial regimes and cost separation (R4-POLY-DEV-04, R4-STAT-DEV-05)

Written and committed before the run. Results are appended below the line at the end after the run.

## Questions

1. **POLY-DEV-04.** Do the cached and uncached, certified and unbounded-timing arms, in both bases, meet one accuracy
   target? Is coefficient reuse confined to identical cache keys?
2. **STAT-DEV-05.** Are cold, warm, setup and certified costs separately machine-readable, and do the work counters
   reconcile with the work done? How do the formal costs compare with an amortized cached eigensystem?

## Source and command

- Source commit: `96639dbf9b6be7c288028534570674d4decfcfcb`, rustc 1.94.1.
- Command:
  `target/release/rodas5p r4-study --study polynomial-regimes --actions 20 --output research/r4_polynomial_regimes_20261001/STUDY.json`

## Design

- One frozen operator, `diag24-wide` (verified enclosure), with h = 0.1 and a truncation budget of 1e-12.
- Twenty actions with distinct input vectors.
- Eight arms: basis {Chebyshev, Laguerre} x path {certified enclosures, unbounded timing} x cache {cached, uncached}.
- Cache-identity control: twenty cached Chebyshev actions alternating h in {0.1, 0.2}.
- Formal cost model: dense symmetric eigensystem at about 9 n^3 once, then 2 n^2 + 25 n per action, against
  5 m n^2 per polynomial action.

## Gates

- **POLY-DEV-04 PASS** if every arm meets all of the following. Otherwise **FAIL**.
  - Maximum relative error is at most 1e-10 against the scalar reference.
  - Every action has the expected total-error status: certified for certified Chebyshev, estimate-only otherwise.
  - The counters reconcile: block products equal the summed degree, setups plus reuses equal 20, and reuses equal 19
    when cached and 0 when uncached.
  - In the control, reuse happens only for identical h: 2 setups, 18 reuses, 2 keys.
- **STAT-DEV-05 PASS** if every arm's counters reconcile and, for each basis and cache mode, the unbounded-timing arm
  ran the same summed degree, block products and coefficient setups as the certified arm. In other words, the timing
  arm's work is the verification path's work. Otherwise **FAIL**. Cold, warm, setup and formal costs are reported per
  arm.

Timing status is `NOT_EVALUATED: STATISTICAL_AUTHORITY_HOLD`. Wall seconds are diagnostics, and no speed claim is
made.

## Prior information

The R3 POLY-03 kernel campaign (L-0009) timed the unbounded path. This node does not reuse its timing.
