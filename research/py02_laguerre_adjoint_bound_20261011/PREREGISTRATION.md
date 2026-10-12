# Preregistration: a sharper Laguerre recurrence-adjoint bound, two-stage (PY02)

Node PY02 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`MATHEMATICS_PORTING_KO.md` §6.3). Registered on branch `audit/rvj-reaudit-remaining-20261011` (base `0ce7c32`). It
depends on PY01 (`research/py01_laguerre_component_export_20261011`): stage 1 below reads PY01's recorded
`RESULTS.json` and nothing else, and it cannot start before that file is committed. The node also depends on AS03.

**Claim boundary.**
- This is a two-stage node. Stage 1 (admission) is fixed now. Stage 2 (implementation and fresh cases) runs only if
  stage 1 admits.
- The outcomes are ABSTAIN, PASS or FAIL. ABSTAIN is not a negative result about Laguerre in general: it records that
  the registered gate is unreachable on PY01's evidence.
- Not repeated here:
  - no broad Laguerre campaign (R-NEXT-04 and PP08 are not rerun);
  - no tuning of PP09's scalar envelope E_n = sup |L_n|: its best factor of 4.196 cannot close a gap above 1e9.
- No routing change, default promotion or wall-time claim. `admit_laguerre_total` and `laguerre_adjoint_total` stay
  unchanged; a new bound gets its own field and admission.

## Question

The Laguerre adjoint bound is `sum_j beta_j eps_(j-1)`, where beta_j >= sup_[0, L'] |z_j| comes from a depth-3
Bernstein hull (`laguerre_adjoint.rs`, `LAGUERRE_ADJOINT_DEPTH = 3`).
- If PY01 shows that this term carries the rejected totals, can a certified, sharper sup bound of the same adjoint
  polynomials z_j bring the total within budget?
- Can it do so with the same residual bounds eps_j, degree, scale, enclosure and summation?
- And can the admitted Laguerre result then cost less complete work than Chebyshev alone, under the same total budget
  and target?

## Stage 1: admission (fixed now; evaluated from PY01's `RESULTS.json` and a plan-only export)

Stage 2 is admitted only if A1, A2 and A3 all hold. The six cases are PY01's gated cases
(`n{6,12,20}-rho400-h0.1-{distinct,same}`).
- **A1 Dominance.** In every one of the six cases, the recurrence-adjoint share (sum over the columns of
  `recurrence_adjoint`, divided by the exact component sum S) is >= 0.9.
- **A2 Closability.** For at least one of the six cases, PY01's closability floor F (the non-adjoint terms plus
  sum_k |s_k| sum_j l_j eps_(j-1), with sampled l_j <= sup |z_j|) is <= the case's admission budget. If F exceeds the
  budget everywhere, the gap sits in the residual bounds eps_j or in the degree, not in beta_j. A change of the
  recurrence itself (for example a scaled recurrence, which changes eps_j) then needs its own node.
- **A3 Work feasibility.** On the stage-2 cases below, a plan-only export counts the cases whose Laguerre plan needs
  strictly fewer vector products (degree x block width) than the Chebyshev plan, both at truncation budget
  total_budget / 4. The count must be >= 8.
  - The degree does not depend on the adjoint bound. Below 8, item S4 cannot pass whatever the new bound.
  - The A3 planner is always the local plan-only helper added here, which calls `choose_transform` for both bases.
    If PY03's `plan_joint_phi` is merged, its parity with the local helper is reported only; it never decides A3.
  - The helper executes no recurrence, coefficient table or admission on a stage-2 case.
- **Recorded outcome.** ADMISSION.json records A1-A3 with their numbers and is committed before any stage-2 execution.
  - If any item fails, PY02's verdict is **ABSTAIN** with that evidence, and stage 2 is not implemented.
  - If PY01 is INVALID or FAIL on completeness, PY02 is **ABSTAIN (no component evidence)**.
  - ADMISSION.json is INVALID if the checker blob at the RUNS (PLANS) source commit differs from the checker that was
    run, or if any recorded tree status is dirty.
- **Ledger mapping.**
  - ABSTAIN from a failed A1, A2 or A3 is recorded as verdict PASS, with claim "decision ABSTAIN (A_k)" (k the first
    failed item), covering the numeric ADMISSION.json.
  - ABSTAIN for missing component evidence (PY01 INVALID or FAIL on completeness) is recorded as INCONCLUSIVE.

## Stage 2: change (only if admitted)

1. **Refined envelopes** in `crates/rodas5p-core/src/laguerre_adjoint.rs`. Certified beta_j' >= sup_[0, L'] |z_j| for
   the same stored coefficients, by adaptive subdivision of [0, L']:
   - de Casteljau halving to depth <= `LAGUERRE_ADJOINT_MAX_DEPTH` (12);
   - local interval Taylor forms in t = x - c (the method of PP09 Part B);
   - stopping when each piece's upper end is within 1.01 of the largest certified point lower bound found, or at the
     depth cap.
   The new proof version is `laguerre-adjoint-adaptive-v2`, carried in `EnvelopeKey`, so v1 and v2 entries never mix
   in the cache. The work is counted in interval operations.
   - **Beyond the DAG (disclosed).** The DAG's edit targets for PY02 are `polynomial_action.rs` and the research node.
     The edits to `laguerre_adjoint.rs` (the refined envelopes) and the new proof-version field of `EnvelopeKey` go
     beyond them. They are needed because the envelopes and their cache key live there. They are bounded by one
     requirement: the v1 key encoding and digest are bitwise unchanged.
2. **New field and admission** in `polynomial_action.rs`:
   - `laguerre_adjoint_total_refined`: the same order of upward summation as `laguerre_adjoint_total`, with beta_j' in
     place of beta_j;
   - `admit_laguerre_total_refined(budget)`, with the same guards as `admit_laguerre_total`: verified enclosure,
     certified execution, degree <= 128 and every column bounded.
3. **Unchanged:** eps_j, the degree, the scale selection, the truncation and coefficient terms, the old fields and
   `admit_laguerre_total`.

## Stage-2 cases (fresh, disjoint)

**Generator.** The R-NEXT-04 matrix generator (diagonally dominant, Gershgorin-verified), with new seeds starting at
9001. The grid has 32 cases:
- n in {7, 14};
- rho in {100, 300};
- h in {0.08, 0.16} (nominal h rho in {8, 16, 24, 48});
- distinct and same-vector inputs;
- total budget 1e-6 max(||F||, 1) and 1e-8 max(||F||, 1).

**Disjointness.** The grid shares no (n, rho, h) with R-NEXT-04 ({6, 12, 20} x {1, 50, 400} x {1e-3, 1e-2, 0.1}),
PP08 ({5, 16} x {2, 30, 200} x {1e-3, 0.03, 0.2}) or PY03.

**Reported, not gated.** The new bound on PY01's six reused cases.

**Comparator.** Chebyshev alone: `joint_phi_action` (Chebyshev, truncation total_budget / 4, no cache) admitted by
`route_admission`, on the same operator, h, input and total budget.

**Reference.** The PP08 50-digit mpmath eigendecomposition, rechecked at 70 digits.

## Commands

    PY02_PLANS=research/py02_laguerre_adjoint_bound_20261011/PLANS.json cargo test --offline --locked --release -p rodas5p-core --test laguerre_adjoint_holdout -- --ignored --nocapture plan_only
    python3 tools/py02_laguerre_admission.py --py01 research/py01_laguerre_component_export_20261011/RESULTS.json --plans research/py02_laguerre_adjoint_bound_20261011/PLANS.json --output research/py02_laguerre_adjoint_bound_20261011/ADMISSION.json
    # stage 2 only if ADMISSION.json admits:
    PY02_CASES=research/py02_laguerre_adjoint_bound_20261011/cases.json cargo test --offline --locked --release -p rodas5p-core --test laguerre_adjoint_holdout -- --ignored --nocapture
    python3 tools/py02_laguerre_adjoint_check.py --admission research/py02_laguerre_adjoint_bound_20261011/ADMISSION.json --cases research/py02_laguerre_adjoint_bound_20261011/cases.json --output research/py02_laguerre_adjoint_bound_20261011/RESULTS.json

The third command is the DAG command. Both Python tools call `tools/evidence_schema_v2.py` first (INVALID exit 2,
FAIL exit 1, PASS or ABSTAIN exit 0). Each tool is committed before its recorded input exists.

## Stage-2 gate (applies only after admission)

**Validity.** INVALID if any of the following holds:
- The AS03 rules fail.
- The case set differs from the 32 registered cases.
- The 50- and 70-digit references differ by more than 1e-40 ||F||.
- `laguerre_adjoint_total` on a case differs from a v1 recomputation, or `EnvelopeKey` does not carry v2.
- The v1 `EnvelopeKey` encoding or digest is not bitwise unchanged (checked against the pre-PY02 encoding of every v1
  key the pre-existing cache tests build).
- The checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.
- **S1 Enclosure.** Every result admitted by the refined total has bound >= its 50-digit error.
- **S2 Envelope check.** Every beta_j' is >= the sampled lower bound l_j of PY01's method (4097 points). This is a
  falsification check, not the proof.
- **S3 Admission.** At least 8 of the 32 cases have the refined total <= budget. The number 8, a quarter of the cases,
  is a design choice; the v1 total admitted 0 of R-NEXT-04's six h rho = 48 cases.
- **S4 Complete work.** In every case admitted under S3, the Laguerre arm costs less than Chebyshev alone. Two
  conditions must both hold:
  - strictly fewer vector products;
  - the Laguerre coefficient setup and envelope interval operations exceed Chebyshev's coefficient setup and
    certification operations by at most n^2 x (the saved vector products), with one dense vector product counted as
    n^2 multiply-adds. This exchange rate is a design choice.

Everything else is **FAIL**, with every number preserved.

**Reported, not gated.**
- beta_j' / beta_j, beta_j' / l_j and the refined totals on the six reused cases.
- Envelope work, and the cases admitted by Chebyshev only.

## Kill and hold

- **Envelope-only route.** The 4.196x envelope-only improvement cannot close the published > 1e9 gap (9.5e12 to
  2.1e14). Unchanged envelope tuning of E_n is out of scope.
- **Abstention.** ABSTAIN is recorded with its numbers. It is not converted into a stage-2 run by revising A1-A3.
- **No dispatch.** A stage-2 FAIL keeps Laguerre in research status with no dispatch link.

## Coordination with sibling nodes

PY01, PY02, PY03 and PY05 all edit `crates/rodas5p-core/src/polynomial_action.rs`. Their changes are merged in the
fixed order PY01 -> PY03 -> PY02 -> PY05. PY05 stays out of the shared router. PY02's v2 key must not change any v1
digest, and PY03's cache-key tests must pass unchanged after PY02 is merged.

## Prior information (disclosed)

- **R-NEXT-04 and PP09** (L-0047, L-0077): see PY01. PP09's unregistered emulation suggests the remainder dominates,
  so A1 likely holds.
- **PP08** (L-0071): in all 79 fixtures the Laguerre degree is >= the Chebyshev degree, with a vector-product ratio of
  1.0-1.57 where both admit.
- **Plan scan (before registration).** A float emulation of the two degree rules (Chebyshev tail with t = asinh(r/b),
  eta = 0; Laguerre tail e^{L/2} q^{m+1}, L in {1, 2, 4, 8, 16}) found no point where the Laguerre degree is below the
  Chebyshev degree. The grid covered h rho in [1e-4, 1e3], lambda / rho in {0, 0.1, 0.5, 0.9, 0.99} and tail
  targets 1e-4 to 1e-14. It was not native and is not evidence for A3; it shaped the prediction.
- **Nothing run.** No stage-2 case, plan or refined envelope has been computed.

## Predictions

- **A1 holds.** **A2 is uncertain:** it depends on the unmeasured beta_j / l_j and eps_j.
- **A3 fails.** At h rho = 8-48 the Chebyshev degree is well below the Laguerre degree (PP08: 30-37 against 87-115 at
  h rho = 40), so the count is predicted to be 0.
- **Most likely outcome: ABSTAIN (A3).** If admitted, S1-S2 are expected to hold, and S3/S4 are uncertain.

---

## Results

Everything above this heading is the registered text, unchanged. Executed on branch `node/py02` from `965530f`
(PY01 and PY03 merged); the registered text names `audit/rvj-reaudit-remaining-20261011` as the registration branch.

**Decision: ABSTAIN (A3)** (evidence VALID). A1 and A2 hold on PY01's evidence. A3 fails: on none of the 32
registered stage-2 cases does the Laguerre plan need fewer vector products than the Chebyshev plan (count 0, threshold
8). Stage 2 is therefore not implemented or run. **Ledger mapping (as registered): verdict PASS, claim "decision
ABSTAIN (A3)"**, covering the numeric `ADMISSION.json`. Claim boundary as registered: this ABSTAIN is not a negative
result about Laguerre in general. It records that the stage-2 gate (S4 in particular) is unreachable on these cases
whatever the adjoint bound. No bound, admission, `TotalErrorStatus`, routing or default changed; no wall-time claim.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| A: plan-only helper `plan_only_comparison`, contract tests, `plan_only` export | `935fb3e` | `polynomial_action.rs` `0f8fc10db0fd3f9ce6ee499f9eda7ac5ad44fe2b349bdb2c5d792eaccfa86a02` (was `941b9640…c33bd2c` at `965530f`); `tests/laguerre_adjoint_holdout.rs` `cf7ebdf81a84ee8ccd235f7706164b72535594361080118e450e98cee2545b3a` |
| B: checker and synthetic checker tests (before the run) | `2692d72` | `py02_laguerre_admission.py` `126c6297ee94358746cb068c9af78df66261888ed76d446089ef78ecf4e286fb`; `test_py02_laguerre_admission.py` `f60441277c994e94d16a7030b7b58dd21a91d5acddbd5d9079a08cbcf48094af` |
| C: recorded run at `2692d72` (clean tree) | `dd20b4e` | `PLANS.json` `8252bd8d7ba0b3c6e549b970b7e44f824c6aea48b245a53a79f2c3264a75e6be` (147,734 bytes, compact JSON); `ADMISSION.json` `f4f3dec5ae9a48e52424859b411ca5de7fdf900c57401992cd3b1f71ca1e61f9` (37,875 bytes) |
| Inputs, unchanged | — | PY01 `RESULTS.json` `235dcd5c5266b33c8324d33f7987ee55ac21aa6d3e589f8ffcec103765555b18` (pinned); `evidence_schema_v2.py` `1d2bfe9276252b2079914e4fa958612ed06ffb04d1fecf9016bd4f370206ff10`; `laguerre_adjoint.rs` `26b58115e8ad5fbe34ac549161661fec05006bb924ff5461efe89a31b925cb28` (not edited) |

**How the run was made.** The two stage-1 commands were run once each, in order, from a clean tree at `2692d72`
(`git status --porcelain` empty, untracked files included), with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`,
rustc/cargo 1.94.1, Python 3.11.15, mpmath 1.3.0.
- Command 1 (release): `plan_only` passed (0.04 s test time) and wrote `PLANS.json`; its recorded `source_commit` is
  `2692d72259f1f686a46744f3dad6713ab171017f` and its `tree_status` is empty.
- Command 2: exit 0 in 1.8 s wall clock. The checker recorded its HEAD `2692d72…` and an empty
  `git status --porcelain --untracked-files=no`; its blob at the plans' source commit equals the checker that ran.
- Stage-2 commands 3 and 4: not run (not admitted).

**Tests.** At A: `cargo fmt` and `cargo clippy --workspace --all-targets --locked -- -D warnings` clean;
`cargo test -p rodas5p-core --all-targets` passed (42 test-result groups ok, 0 failures), including PY01's
`laguerre_component_export` (`report_parity_on_fresh_fixtures`), PY03's `polynomial_single_backend_router` (7 tests),
the thread-transfer Laguerre adjoint and cache tests and the five new contract tests (helper = action degree, scale,
truncation bound and vector products on 14 fresh fixtures including both normalization amplitudes; helper = PY03's
`plan_joint_phi`; refusals; the generator reproduces R-NEXT-04's first two cases bit for bit; the registered grid).
At B: `python3 -m unittest discover -s tools -p test_py02_laguerre_admission.py` passed 23 tests, normally and with
`-O` (decisions ABSTAIN (A1/A2/A3), ADMIT at 8 and not at 7, ties and failed plans not counted, no component evidence
-> INCONCLUSIVE; 9 INVALID groups covering the pin, strict JSON, keys and types, the case set, input binding, budgets
and the 50/70-digit reference, plan consistency and weight factor, provenance; helpers).

No bug was found after the recorded run; there is no `*_FIRST` file and nothing was re-run.

### Validity

All conditions hold: AS03 rules (strict JSON, the PY01 pin, exact key sets, canonical hex, typed numbers); exactly
the 32 registered labels in order, (n, rho, h) disjoint from R-NEXT-04, PP08 and PY03; every A, w_k and enclosure
equal to the R-NEXT-04 generator for seeds 9001-9008 bit for bit, every enclosure Gershgorin-verified; the budgets
equal factor x max(norm_f, 1) and its quarter bit for bit; norm_f agrees with the 50-digit ||F|| to 3e-16 - 2.5e-15
relative (all ||F|| are 0.024 - 0.887, so every total budget is exactly 1e-6 or 1e-8); 50- and 70-digit references
agree far inside 1e-40 ||F||; every plan is a recurrence plan without normalization, with width 5 or 1, products =
degree x width, truncation bound <= budget, weight factor in [W, W (1 + 1e-12)]; provenance and both tree statuses
clean. PY01's evidence: verdict PASS, completeness true.

### Admission

| Item | Rule | Outcome |
|---|---|---|
| A1 Dominance | recurrence_adjoint / S >= 0.9 in all six gated cases | **holds**: lower bound of the share 0.99999999998 in every case (from 12-digit decimals, rounding margin 1e-11) |
| A2 Closability | F <= admission budget in at least one case | **holds** in 6/6: F = 1.31e-10 to 2.59e-10, F / budget(1e-6) = 1.31e-4 to 2.59e-4 |
| A3 Work feasibility | Laguerre products < Chebyshev products at total_budget / 4 in >= 8 of 32 | **fails**: count **0** |

A3 per (n, rho, h), distinct input, degrees Chebyshev / Laguerre (Laguerre scale L = 16 in all 32 cases); the
same-vector cases differ by at most 3 in the Laguerre degree and 1 in the Chebyshev degree, with width 1 instead
of 5:

| n, rho, h (h rho) | b = 1e-6 | b = 1e-8 |
|---|---|---|
| 7, 100, 0.08 (8) | 15 / 24 | 17 / 29 |
| 7, 100, 0.16 (16) | 20 / 40 | 23 / 48 |
| 7, 300, 0.08 (24) | 24 / 56 | 27 / 66 |
| 7, 300, 0.16 (48) | 32 / 100 | 37 / 119 |
| 14, 100, 0.08 (8) | 15 / 25 | 17 / 30 |
| 14, 100, 0.16 (16) | 20 / 41 | 23 / 48 |
| 14, 300, 0.08 (24) | 24 / 56 | 27 / 67 |
| 14, 300, 0.16 (48) | 33 / 101 | 37 / 120 |

Laguerre / Chebyshev vector products: 1.60 to 3.24 (rising with h rho). No Laguerre degree exceeds 128; no plan
failed.

### Reported, not gated

- **PY03 parity.** PY03's `plan_joint_phi` gives the same degree, scale, products and tail evaluations as the local
  helper in 32/32 cases (both bases).
- **Emulation.** An exact-arithmetic emulation of both degree rules at 50 digits (Chebyshev tail with
  t = asinh(r/b), eta = 0; Laguerre e^{L/2} q^{m+1} over L in {1, 2, 4, 8, 16}) reproduces every native degree and
  scale (32/32).
- With F / budget near 2e-4, A2 shows that a sharper envelope of the same z_j could in principle close PY01's total;
  A3 shows that on these fresh cases such a bound would still leave Laguerre doing 1.6x to 3.2x the vector products
  of Chebyshev, so S4 could not pass. This agrees with PY03 (on its 96 grid cases the Laguerre plan never needed
  fewer products) and PP08.

### Prediction check

| Prediction | Outcome |
|---|---|
| A1 holds | held (share 1 - O(1e-11) from the rounded decimals) |
| A2 uncertain | held in 6/6 (F / budget <= 2.6e-4) |
| A3 fails, count predicted 0 | held: count 0 |
| Most likely outcome ABSTAIN (A3) | held |
| PP08 degrees at h rho = 40 (Chebyshev 30-37 against Laguerre 87-115) | consistent: at h rho = 48, 32-37 against 98-120 |

### Interpretations and deviations

Fixed in the checker docstring at commit B, before the run:
1. **Helper.** `polynomial_action::plan_only_comparison` repeats the action's normalization, checks and weight factor
   and calls `choose_transform` for both bases (scales {1, 2, 4, 8, 16}); it is written independently of PY03's
   `plan_joint_phi`, whose parity is reported only. It executes no recurrence, table, envelope or admission and
   changes no counter.
2. **||F|| in the budget.** R-NEXT-04 took ||F|| from the executed action; the plan-only export may not execute a
   recurrence, so norm_f is the binary64 norm of the dense augmented exponential (`dense_phi_combination`), bound by
   the checker to the 50-digit ||F|| (relative 1e-10 on max(||F||, 1)). All ||F|| < 1, so the budgets are exactly
   1e-6 and 1e-8.
3. **Seeds.** R-NEXT-04 conventions with the first operator seed 9001: one seed per (n, rho, h) in grid order
   (9001-9008), distinct input from seed + 9, same vector from seed + 5 with s = (1, -0.5, 0.25, 2, -1); both budget
   factors share the inputs.
4. **A3 counting.** Both plans must exist and the Laguerre products be strictly fewer; a failed plan never counts (none
   failed). Degrees above 128 would be counted by products alone (none occurred).
5. **A1, A2 from rounded decimals.** PY01 records terms_exact, S and F as 12-digit decimals; A1 uses a lower bound of
   the share and A2 an upper bound of F (margin 1e-11). The margins are irrelevant to the outcome.
6. **PY01 inputs.** Only PY01's `RESULTS.json` is read, as registered (not `COMPONENTS.json`).
7. **Tree status.** The export records `git status --porcelain` with untracked files; the checker its own status
   without untracked files (PLANS.json is untracked when it runs). Both were empty.

Further disclosures:
- **Stage 2 not implemented.** `laguerre_adjoint.rs` is not edited: there is no `laguerre-adjoint-adaptive-v2`, no
  new `EnvelopeKey` field and no refined total or admission. The v1 key encoding and digests are therefore bitwise
  unchanged, and PY03's cache-key tests pass unchanged.
- **Beyond the DAG.** The only source edit is the plan-only helper (with its two result types) appended to
  `polynomial_action.rs`, a DAG edit target. No existing function or value changed.
- **Before the run.** Two unregistered checks ran on fresh fixtures, never on a registered case: the ignored test
  `fixture_format_plans` (n = 5, rho = 100, h = 0.08, seed 70001), written to a scratch path, to check the checker's
  key sets on the native format; and one timing call of the checker's reference and emulation on a fresh n = 14,
  rho = 300 matrix (seed 70001, emulated degrees 37 / 120). Neither changed the registered gate, which was fixed
  in the registration.
- The test file has two ignored tests (`plan_only`, `fixture_format_plans`); both write only when their path
  variable is set, so the CI run of every ignored test writes nothing. `plan_only` is matched by the registered
  filter alone (the fixture test's name does not contain it).
