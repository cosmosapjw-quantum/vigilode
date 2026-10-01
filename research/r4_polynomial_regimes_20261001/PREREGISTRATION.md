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

---

## Results (appended after the run at `95cb66e`)

Output: `STUDY.json`. The operator is `diag24-wide`, with h = 0.1, 20 actions and a budget of 1e-12. Timing status is
`NOT_EVALUATED: STATISTICAL_AUTHORITY_HOLD`.

**POLY-DEV-04: PASS.**

All eight arms met the accuracy target:

| Basis | Maximum relative error | Status of all 20 actions |
|---|---|---|
| Chebyshev | 3.3e-15 | certified (certified path); estimate-only (unbounded path) |
| Laguerre | 8.3e-16 | estimate-only |

Counters reconciled in every arm:

| Basis | Summed degree = block products | Coefficient setups (cached / uncached) |
|---|---|---|
| Chebyshev | 420 (degree 21 per action) | 1 / 20 |
| Laguerre | 760 (degree 38 per action) | 1 / 20 |

In the alternating-h control, coefficients were reused only for an identical key: 2 setups, 18 reuses, 2 keys.

**STAT-DEV-05: PASS.**

- In each basis and cache mode, the unbounded-timing arm ran exactly the certified arm's degrees, products and setups.
  The timing path's work is the verification path's work.
- Cold, warm, setup and formal costs are separate fields per arm.
- Formal costs (n = 24):

| Cost | Value |
|---|---|
| Chebyshev, per action, `5 m n^2` | 60480 multiply-adds |
| Cached eigensystem, setup, `~9 n^3` | 124416 |
| Cached eigensystem, per action | 1752 |
| Amortization crossover | 3 actions |

- On this frozen symmetric operator, an amortized eigensystem is formally cheaper from the third action on. That is
  a formal count, not a measured speed. No speed claim is made.

## Corrections after the second independent review

The numbers in `STUDY.json` are unchanged. This section corrects their reading; the text above is left as first
published. Ledger rows L-0022 and L-0023 supersede L-0015 and L-0016 with input hashes at the execution commit
`95cb66e`, where `crates/rodas5p-cli/src/r4_studies.rs` is the version that ran and this file is the
preregistration before its results.

- **STAT-DEV-05 holds for polynomial work only.** The certified and unbounded arms call one routine that differs by
  a flag. Equal degrees, block products and coefficient setups therefore follow by construction, and the gate could
  hardly fail. The certified arm's enclosure and recurrence arithmetic has no work counter and is not compared. The
  wall diagnostic shows its size: the cached Chebyshev arm took 0.0211 s warm when certified against 0.00215 s
  unbounded, about 10x. "The timing path's work is the verification path's work" above should read "the timing
  path does the verification path's polynomial work; the certificate overhead is not counted."
- **Eigensystem per-action cost.** The model `2 n^2 + 25 n` (1752 at n = 24) ignored `Q^T` on each of the five
  distinct input columns. The corrected model is `6 n^2 + 25 n` = 4056. The crossover is unchanged:
  124416 / (60480 - 4056) = 2.2, so 3 actions. The study code now uses the corrected model; this `STUDY.json` was
  produced with the old one.
