# Preregistration: a one-sided acceptance rule for the residual-to-output budget on fresh step sizes (review DAG node REV-03)

## Question

R-NEXT-01 (L-0049) failed its item 3 on one case. At Robertson h = 1e-2, a *rejection* was right but not resolved by
the budget (error norm 1e6, budget 2e122). Critical review C7 notes that an unresolved rejection cannot admit a bad
step. A guard needs only the one-sided rule: every acceptance (`err <= 1`) must satisfy `err + B <= 1`. L-0049 tested
the budget at h in {1e-4, 1e-2}, the same values as L-0038. Does the one-sided rule hold at step sizes not used
before, for both drivers, all three Krylov methods and all problems?

## Method (no driver or budget changes)

- `tests/rev03_export.rs`: the R-NEXT-01 exporter (same problems, drivers, Krylov settings and tolerances), with the
  step sizes **h in {3e-4, 3e-3, 3e-2}** in place of {1e-4, 1e-2}.
- `tools/rnext01_residual_output.py` (unchanged) computes the budgets at 60 digits.
- `tools/rev03_one_sided.py` evaluates the gate below from that output.

## Commands

1. `REV03_EXPORT=research/rev03_one_sided_acceptance_20261003/stages.json cargo test --release -p rodas5p-integrators --locked --test rev03_export -- --ignored --nocapture --test-threads=1`
2. `python3 tools/rnext01_residual_output.py --stages research/rev03_one_sided_acceptance_20261003/stages.json --output research/rev03_one_sided_acceptance_20261003/BUDGETS.json`
3. `python3 tools/rev03_one_sided.py --budgets research/rev03_one_sided_acceptance_20261003/BUDGETS.json --output research/rev03_one_sided_acceptance_20261003/RESULTS.json`

## Gate

**PASS** if all hold:

1. **Budget validity.** For every completed case and both drivers, the error-norm deviation from the exact step is
   within `B` and `y_new`'s deviation within its bound (the tool's `valid`).
2. **One-sided rule.** Every driver result with `err <= 1` is a resolved acceptance (`err + B <= 1`). At least 10
   driver results must be acceptances, or the item is not demonstrated and fails.
3. **Bounds available.** Every budget is finite.

Otherwise **FAIL**. Reported, not gated (P1): resolved and unresolved rejections, budget tightness
(`d_(s-1)` over the actual stage deviation), solver failures, and the L-0038 item-3 relative criterion on these
cases.

## Prior information

L-0049 and the critical review C7. Only the exporter's step list changes; the budget tool is the one recorded at
L-0049. No run at these step sizes has been made before this commit.

---

## Results (appended after the run at `892ffc1`)

Outputs: `stages.json` (native export), `BUDGETS.json` (60-digit budgets from the unchanged R-NEXT-01 tool) and
`RESULTS.json` (this node's gate). Ledger row L-0062. Disclosure: `stages.json` went into the REV-04 results commit
`5beab96` by mistake; its content is the export of the command above and has not changed since.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Budget validity | **holds** on all 52 completed cases (54 exported; 2 are GCRO-DR solver failures on the Brusselator, at h = 3e-4 for both drivers and at h = 3e-3 for the K driver) |
| 2. One-sided rule | **holds**: all 68 acceptances (`err <= 1`) are resolved (`err + B <= 1`) |
| 3. Bounds available | **holds**: every budget is finite |

- **Rejections.** 30 rejections are resolved and 6 are not. All 6 are Robertson at h = 3e-2 (both drivers, all three
  Krylov methods). The error norm there is 1e6, and the budget is 1.8e246 to 2.5e246, for the same reason as
  L-0049's h = 1e-2 case: the exact stages explode. These are the safe side, since an unresolved rejection rejects.
- **L-0038's relative criterion** flags 27 of the cases as discrepancies between the drivers. All 27 lie inside the
  budgets, as at L-0049.
- **The guard this supports:** accept a step only if `err + B <= 1`. It resolved every acceptance on these fresh
  step sizes. That is evidence on six problems at three step sizes, not a proof for others. The budget remains far
  from tight on stiff blow-ups. Claim ceiling: one-step comparisons; no driver change; no timing.
