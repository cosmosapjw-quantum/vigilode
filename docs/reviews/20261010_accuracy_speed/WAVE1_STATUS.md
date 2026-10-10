# Wave 1 status: AS01, AS02, AS03, SP01, SP03

This is the execution record of the recommended first wave of
`research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`.

- **Branch:** `audit/rvj-accuracy-speed-wave1-20261010`.
- **Base:** `4c64317`, which is the October 10 re-audit (`REVIEW_KO.md`, `MATHEMATICS_PORTING_KO.md`).

**Claim ceiling.**
- No node in this wave promotes a default.
- No node makes a global-accuracy claim.
- No node makes a wall-time claim. The timing authority stays on HOLD, so every speed figure below is a count of work
  or same-binary callgrind instructions (Ir).

## Summary

| Node | Finding / question | Kind | Outcome | Evidence |
|---|---|---|---|---|
| AS03 | F104: checkers PASS on malformed evidence | software fix | closed | `tools/evidence_schema_v2.py`, wired into the ALG04/05/06 checkers; historical RESULTS reproduce byte for byte |
| AS01 | F102: the production fallback accepts through `Inf <= Inf` | software fix | closed | `ProductionFallbackRule`, `tests/residual_acceptance_contract.rs`; ALG06 RUNS byte-identical |
| AS02 | F101: the predictive factor underflows | software fix | closed | `predictive_term` (log domain on non-normal intermediates), `tests/predictive_extreme_contract.rs`; ALG02 and ALG05 RUNS byte-identical |
| SP01 | Remove the duplicate final GMRES residual | preregistered | **PASS**; decision rule "versioned" | L-0100; `research/sp01_dupfix_adoption_20261010/` |
| SP03 | Route by declared, validated structure | preregistered | **FAIL** (gate item 5) | L-0101; `research/sp03_declared_structure_routing_20261010/` |

## Safety fixes (AS01-AS03)

**AS03 (F104).** The common validator fails closed. It requires:
- strict JSON;
- base files pinned by SHA-256;
- exact key, row and arm sets;
- unique raw keys;
- state-vector lengths equal to the problem dimension;
- finite, typed numbers;
- twin and reference identity bound before any gate.

A violation is **INVALID** (exit 2), which is distinct from FAIL. The ALG04, ALG05 and ALG06 checkers call the
validator first and reproduce their recorded RESULTS byte for byte. The four F104 mutations and a bool/int swap all give
INVALID, as the independent review re-checked.

**AS01 (F102).** `ProductionFallbackRule` checks `||b||` and the threshold before the fallback is built.
- When the literal threshold is finite, the rule is the old comparison.
- When it overflows, the rule compares under exact power-of-two scaling.
- A non-finite right-hand side, residual or candidate rejects.

Old and new decisions agree on all 338k comparable cases among 400k random ones. One behaviour change applies only to
overflow: an overflowing physical candidate now fails the stage as a linear-solve failure. Before, it was accepted and
then surfaced as `NonFinite`.

**AS02 (F101).** `predictive_term` keeps the old expression bit for bit when every intermediate is a normal number.
Otherwise it evaluates `log(safety) + log(h/h_acc) + (log err_acc - 2 log err)/k` and clamps in the log domain.
- The registered case (h = 1e-100, err = 1e-200) now gives 0.2, where it gave 5.0 before.
- `h = +Inf` is now an error.
- The documentation separates the "factor <= 1 after rejection" policy from "no increase after the actual trial".

## SP01: duplicate-residual removal alone

Both GMRES kernels have `ResidualAccounting::{RecomputeFinal (v1), ReuseConfirmed (v2)}`. Under v2, the final
residual is skipped only after at least one restart cycle that stopped on a true residual of the current iterate.

| Gate item | Result |
|---|---|
| 1. Parity | 44/44 cells (U-form into 14, U-form default 14, K-form integrate 8, K-form step 8) and 10/10 kernel boundary cases bitwise identical |
| 2. Exact accounting | JVP(v1) - JVP(v2) equals the independently observed number of confirmed exits in 44/44 cells (83 to 7,572 per cell); no other counter differs |
| 3. Ir | v2/v1 = 0.9725 (van der Pol), 0.9620 (HIRES) and 0.9962 (Brusselator-50) on the U-form into path; 0.9890 and 0.9933 on HIRES, K-form |
| 4. Contract | every pre-existing test passes under the default the decision rule chose; no pre-existing test file was modified |

**Decision rule: versioned.** With v2 as the default, 9 pre-existing tests fail, and every one of them pins v1
diagnostic counters or a recorded export (`DEFAULT_TRIAL*.json`). So the default stays v1, and v2 is an explicit
option. JVP ratios v2/v1 are 0.75-0.98 on the U-form and 0.83-0.98 on the K-form. These ratios are reported, not
gated.

## SP03: routing by declared structure

`ProblemStructure` is attached through a validated builder, and `integrate_rodas5p_routed_observed` is a new opt-in
entry point. Band verification compares the band with the JVP, or with the dense Jacobian when there is no JVP. It uses
k = 2 seeded vectors, a relative tolerance of 1e-12, and charges its own work.

| Gate item | Result |
|---|---|
| 1. Validation | 10/10 malformed declarations rejected before any step |
| 2. Parity | 15/15 cells bitwise identical to the direct target; banded equals dense on all 8 Brusselator cells |
| 3. Strongest arm | routed / min(dense, dense-colext64) = 0.487 / 0.485 at n = 400; 0.790 at n = 100; 0.542 / 0.540 at n = 320; routed / Legacy = 0.0014-0.015 |
| 4. Slope | 1.057 / 1.031 from n = 400 to 1000 (bound 1.15) |
| 5. Overhead | **1.0738 / 1.0347 at n = 1000** (bound 1.02); 1.005-1.017 for n <= 400 |

**Why item 5 failed.** The CLI Brusselators have no JVP, so verification builds the dense Jacobian. That is a one-time
O(n^2) cost: memset of the 8 MB matrix plus two dense products, 16.5 M Ir at n = 1000. A verification that compares
with JVPs, or a band-only verification, is a candidate for a new preregistered node. It is not a reinterpretation of
this one.

## Independent review

An adversarial read-only review found no blocking issue. It recomputed the following from the committed evidence:
- SP01 parity and accounting;
- the SP03 verdict;
- the ledger hashes (L-0100, L-0101);
- the AS03 mutations.

**Should-fix S1 (fixed in this wave).** SP03 band verification used plain sums of squares. For products beyond about
1e154, or below about 1e-154, both norms overflowed (or underflowed), and a band that dropped the subdiagonal was
accepted. This is the F102 pattern again.
- **Fix:** scaled norms (`safe_l2`); reject a reference norm that is not representable; treat a nonzero difference
  against a zero reference as a mismatch.
- **Test:** the new test `tests/band_verification_range.rs` fails 2 of 3 on the old code.
- **Recorded data:** unaffected. The recorded mismatches are exactly 0, and the run is pinned at `f3a5a8a`.

**Minor findings, left open and recorded here.**
- **M1.** Verification is pointwise at (t0, y0), with a whole-vector relative tolerance. Fill that depends on the
  state, or a weak off-band coupling in a stiff row, can pass. A per-row check would be stronger. This is the
  registered design, not a rule breach.
- **M2.** The SP03 checker subtracts the router's self-reported verification charge without recomputing it. The
  committed data is consistent.
- **M3.** The SP01 checker takes the Ir figures from PROFILE.json as written. The reviewer recomputed all 10 profiled
  entries: 2-minus-1 values and determinism hold.
- **M4.** The AS03 DAG step "execution receipts" is not done. ALG05 item 6 still accepts test names found in an
  unpinned Rust file. Closing this without changing a historical gate needs a new versioned checker.
- **M5.** In ALG04 and ALG06, a `ValidationError` raised inside a metric helper would exit 1 with a traceback instead
  of writing INVALID. After validation passes this cannot happen.
- **M6.** SP01's registered decision rule could have promoted a default, which conflicts with the DAG invariant "no
  production default promotion". It did not trigger. Later registrations should not include such a branch.
- **M7.** SP01 and SP03 ran their item-4 suites on pre-merge trees. The merged-tree evidence is the validation matrix
  below.
- **M8.** Under AS01, the classification of overflow failures changes from non-finite to linear-solve.

## Validation matrix on the merged head

**Full matrix on `4b01b63`.** This is the head with all five nodes and both ledger rows, before the S1 fix. All 13
steps passed:
- `cargo fmt --check`;
- clippy `-D warnings` on the workspace and on the integrators with `audit2-research`, `audit2-bateman-authority`
  and `audit2-stage-certificate`;
- workspace tests: 1007 passed, 0 failed, 76 ignored;
- integrators with `audit2-research`: 630 passed, 0 failed;
- the audit2 readiness check;
- `check-research-node.py` against the re-audit base;
- `check-authority-refs.py`;
- the ignored-in-CI check;
- every `tools/test_*.py`;
- every ignored test in the `measurement` profile: 76 passed, 0 failed.

**Re-run after the S1 fix.** The S1 fix touches only `routing.rs`, adds one test file and adds this document. After it
was merged, these checks were re-run on the merged head:
- fmt;
- workspace clippy, and clippy with `audit2-research`;
- the `rodas5p-integrators` tests, with and without `audit2-research`;
- the node and reference checks.

All passed; see the commit that records this section.

## Next nodes unblocked

Per the DAG, AS03 unblocks the following:
- SP02, SP04;
- PY01, PY03, PY04, PY05;
- EX01, ME01, CT01 (CT01 also needs AS02), PC01 (also needs SP03).

AS04 needs AS01. AS05 needs AS01 and AS03. AS06 needs AS04 and AS05.
