# Preregistration: GCRO-DR with the Arnoldi vectors reorthogonalized against the recycle images (review DAG node REV-01b)

## Question

L-0057 (REV-01) refuted its own diagnosis: with the first Arnoldi vector orthogonal to `C`, `max |C^T V|` in the
failing solves was still 0.71 to 0.999. The hypothesis tested here (H2) concerns the Arnoldi step. Each new vector
`next = M^-1 A v_j` is orthogonalized against `C` in one classical pass, then against `V` with two passes. When
`||next||` after projection is much smaller than `||M^-1 A v_j||` (near an invariant subspace), the single pass leaves
a component of order `eps ||M^-1 A v_j||` along `C`, and normalization makes it O(1). H2 predicts that a second pass
against `C` keeps `[C V]` orthonormal to working precision and removes the recycle-induced failures.

## Changes (opt-in; defaults unchanged)

`rodas5p-krylov`: `GcrodrSolveOptions` gains `reorthogonalize_recycle: bool`. When it is set, the projection of each
new Arnoldi vector against `C` is applied twice. Both passes compute the coupling coefficients, and both
contributions are added to the coupling matrix `B = C^T A V`, so the relation `A V_p = C B + V_{p+1} H` stays exact.
The extra dot products and updates are charged. With the flag false (default), the solve is unchanged bit for bit.

The REV-01 test files construct `GcrodrSolveOptions` by literal. They gain `..GcrodrSolveOptions::default()`, with no
other change; their recorded run is the one at `e0882f8`.

## Controls

B cold GCRO-DR, C recycled, **G recycled with `reorthogonalize_recycle`**, FG with the REV-01 start projection as
well, and EG with INT-01's 1e-8 reuse check as well. A (cold GMRES) is the false-convergence reference. Settings are
as in L-0057.

## Systems

1. **Fresh Brusselator-120** (n = 240), primary. Same construction as REV-01's Brusselator-80. Calibration
   (`tests/rev01b_calibration.rs`, control C only, before this commit): 3 failures in the 320 trajectory solves and 9
   in the 16 one-step solves; generating run 92 attempts, success. No other control was run on it.
2. Brusselator-50 and Brusselator-80 sets of L-0057 (seen; reported, not gated).
3. INT-01's CDR family (no-harm check).

## Commands

`REV01B_OUTPUT=research/rev01b_gcrodr_recycle_reorthogonalization_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test rev01b_gcrodr_recycle_reorthogonalization -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-krylov --locked --test rev01b_gcrodr_reorthogonalization_contracts`

## Gate

**PASS** if all hold:

1. **Defaults unchanged.** With `reorthogonalize_recycle = false`, `solve_gcrodr_with_options` is bit for bit what it
   was at `e0882f8` (contract test: a run against `solve_gcrodr_with_policy`). The REV-01, INT-01 and R-NEXT-03
   contract tests pass.
2. **Mechanism (H2).** In every completed cycle of control G on set 1, `max |C^T V| <= 1e-8`.
3. **Failures removed.** Control G has no failure on set 1 where cold GCRO-DR (B) succeeds.
4. **No false convergence, full accounting** (L-0057 rules; the extra passes count no operator products), and **no
   harm** on set 3 (G fails on no more solves than C).

Otherwise **FAIL**. If item 2 holds and item 3 fails, H2 explains the orthogonality loss but not the failures. If
item 2 fails, H2 is refuted. Reported, not gated: every control on sets 1 to 3, and operator products against B and
C.

## Amendment (before any recorded run, while writing the contract tests)

Gate items 2 and 3 named control G, which reorthogonalizes the Arnoldi vectors but not the first one. Without the
REV-01 start projection, `v_1` is the recomputed residual and has a component along `C` whenever the carried pair has
any defect. So `max |C^T V| <= 1e-8` cannot hold for G whatever H2 says about the later vectors. A contract system
showed this: G alone gave `max |C^T V|` above 1e-8; G with the start projection gave 7e-15 to 2.6e-14.

Items 2 and 3 therefore apply to **control FG** (start projection plus reorthogonalization). That is the control
that isolates H2. Control G is reported. Nothing else changes, and no study code had run when this was written.

## Prior information

L-0046, L-0052 and L-0057 (all on the Brusselator-50 sets, plus the Brusselator-80 set in L-0057) and the calibration
above. No code of the option exists before this commit.

---

## Results (appended after the run at `a06bc40`)

Output: `RESULTS.json`. Ledger row L-0058. Contract tests: `rev01b_gcrodr_reorthogonalization_contracts` 2/2,
`rev01_gcrodr_options_contracts` 2/2, `int01_gcrodr_policy_contracts` 5/5, `rnext03_gcrodr_trace_contracts` 2/2.

**Gate: FAIL. H2 is refuted** (items 2 and 3 fail; items 1 and 4 hold).

Failures per control (operator products in parentheses where useful):

| Set | B cold | C recycled | G reorth. | FG start + reorth. | EG check + reorth. |
|---|---|---|---|---|---|
| **Brusselator-120 trajectory (320), primary** | 0 | 3 | 10 | 28 | 5 |
| **Brusselator-120 one-step (16), primary** | 0 | 9 | 3 | 0 | 0 |
| Brusselator-50 trajectory (320) | 0 | 13 | 23 | 10 | 0 |
| Brusselator-50 one-step (16) | 0 | 8 | 1 | 0 | 0 |
| Brusselator-80 trajectory (320) | 0 | 30 | 32 | 19 | 10 |
| Brusselator-80 one-step (16) | 0 | 6 | 6 | 11 | 0 |
| CDR (288) | 0 | 0 | 0 | 0 | 0 |

- **Item 2 fails.** In FG's completed cycles on the primary set, `max |C^T V|` reaches 0.9999. On the healthy contract
  system it was 2.6e-14, and on the CDR family 2.8e-11. A second projection pass against `C`, with the first vector
  projected as well, does not keep `[C V]` orthonormal on the Brusselator systems. So the loss is not the
  single-pass rounding H2 described.
- **Item 3 fails.** FG has 28 recycle-induced failures on the primary set, more than C's 12. On the primary
  trajectory set every option is worse than plain recycling (C: 3). On one-step sets FG and EG remove the failures.
- **Item 4 holds:** no false convergence, full accounting, and no harm on the CDR family.

**What remains open.** A component of norm about 1 along `C` that survives two projections can only enter where a
vector is normalized after almost everything has been projected away. That makes the next candidate an undetected
near-breakdown. The Arnoldi step tests breakdown against the full projection norm, and with a stiff `W`
(`||W||` up to 1e4 here) a vector that is rounding noise relative to `||M^-1 A v||` can pass that test and enter `V`.
This is an inference that no node has tested.

**Decision for the GCRO-DR line.** Three preregistered repairs have now failed to remove recycle-induced failures on
fresh sets: the reuse check (L-0052), the start projection (L-0057) and the reorthogonalization (this node). On these
Brusselator systems recycling also saves no operator products against cold GCRO-DR. Recycled GCRO-DR stays opt-in
and is not recommended for such systems. Cold GCRO-DR and GMRES had no failure on any set. Further repair attempts
need a preregistered breakdown diagnostic first. Claim ceiling: these frozen systems; no timing.
