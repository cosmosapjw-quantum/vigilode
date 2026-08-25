# IMPLEMENTER PROMPT — PR #18 A1 Two-Arm Authority Receipt

Work autonomously in `~/vigilode`.

Fetch and inspect:

```text
main
research/a1-inner-tolerance-parity
handoff/a1-two-arm-authority-receipt-20260825
```

Treat the handoff branch as read-only. Do not merge it. Reuse PR #18 and leave it draft and unmerged.

## Exact starting authority

```text
main
4e3a75e5b2843dc1e135dcadba72edb1d09be94c

implementation head
7952bf96bfd9fb604e87bce41bd9b918cc9b93f4

implementation tree
dd32d7bebe50419c510f2e779b43ed6a26f29242

A1 workflow
32876594151 SUCCESS

E4 workflow
32876594326 SUCCESS
```

Before mutation, run the handoff acceptance test and independently verify remote refs, PR #18 state, the ten-path current diff, and the compile/trace GREEN baseline.

## Goal

Implement and execute the next node only:

```text
A1-TWO-ARM-AUTHORITY-RECEIPT
```

Generate a deterministic read-only receipt for:

```text
profile: EnforcedBudgetHoldout320
arms: legacy-fixed, outer-scaled-numeric-parity
families: all six G4/S5B0 families
cells: exactly 12
```

## Non-negotiable scientific boundary

The ordinary committed runtime path must remain `legacy-fixed` throughout receipt generation. The candidate may be injected only through a named receipt-only API/CLI. Equal tolerance numbers are not a proof of equal outer-error contribution.

Do not change:

- `V36_FROZEN_ZETA34_TAU = 13.39706618860016`;
- persistence thresholds or latch length;
- prefix or continuation budgets;
- GMRES restart, maxiter, convergence logic, Arnoldi, QR, Givens, or counters;
- G1/G3 policies;
- historical v3.5-v3.7 result files;
- Cargo dependencies or lockfile;
- timing protocol.

Do not perform A2/A3, ranking, speedup, active switching, merge, tag, or release.

## TDD order

Write RED tests before production changes for:

1. receipt-only candidate arm selection;
2. ordinary committed path remaining legacy-fixed;
3. exact profile and six-family domain;
4. deterministic atomic cell schema;
5. unknown arm/family/profile rejection;
6. exactly 12 unique aggregate cells;
7. missing/duplicate/extra cell rejection;
8. tau mismatch rejection;
9. recomputation of event, recommendation, unsafe, and positive-control status from atomic rows;
10. deterministic ordering and invariance to wall-time/archive metadata;
11. artifact/committed-receipt source identity binding;
12. historical-result and governed-constant immutability.

## Minimal architecture

Prefer a separate receipt-only module or named functions over modifying the ordinary committed path. Reuse the existing typed arm/lane policy and canonical trace digest.

Add:

- explicit read-only per-family/per-arm runner for `EnforcedBudgetHoldout320`;
- deterministic JSON cell output;
- focused CLI subcommand or mode that cannot be confused with production execution;
- standard-library Python aggregate validator;
- 12-cell GitHub Actions workflow;
- JSON and Markdown receipt under `research/a1_inner_tolerance_audit_20260825/`.

The aggregate must preserve atomic evidence for attempts, accepted/rejected steps, RHS/JVP/matvec work, trace digest, event keys, all finite zeta34 values and signed margins from tau, recommendation keys, unsafe recommendations, audit unsafe events, Hires positive control, hard gates, and limitations.

Do not trust derived totals from cells. Recompute them.

## Predeclared decision

- `ADMISSIBLE_AND_DISCRIMINATING`: all hard gates pass, zero unsafe recommendations, and at least one unsafe completed full-E event is correctly unrecommended.
- `ADMISSIBLE_BUT_NONDISCRIMINATING`: hard gates pass and unsafe recommendations are zero, but the positive control disappears.
- `NOT_ADMISSIBLE`: any hard safety/provenance gate fails or an unsafe recommendation appears.

Do not switch the committed arm inside the replay runner. After inspecting the complete receipt, commit an explicit decision. Switch only if the result is `ADMISSIBLE_AND_DISCRIMINATING`; otherwise retain `legacy-fixed`.

## Required verification

```bash
cargo test -p rodas5p-integrators --test a1_inner_tolerance_parity_contracts --locked
cargo test -p rodas5p-integrators --test a1_committed_trace_regression --locked
cargo test -p rodas5p-integrators --test g4_s5b0_regime_atlas_contracts --locked
cargo test -p rodas5p-cli --test frozen_full_e_shadow_cli_contracts --locked
cargo test -p rodas5p-cli --test a1_tolerance_receipt_cli_contracts --locked
python3 tools/test_a1_tolerance_receipt.py -v
cargo test --workspace --all-targets --no-run --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Run the exact-head A1, E4, and two-arm workflows. Download and independently inspect the receipt artifacts before committing the final receipt.

## Completion evidence

Return:

- base/head/tree and exact changed paths;
- RED and GREEN evidence;
- workflow run IDs;
- all 12 atomic cell identities;
- artifact manifest;
- aggregate JSON/Markdown paths;
- committed receipt paths;
- decision and the evidence supporting it;
- forbidden-diff audit;
- P0/P1/P2/P3 ledger;
- unresolved blockers;
- explicit statement that A2/A3, timing/ranking, switching, merge, tag, and release were not performed.

Stop with PR #18 OPEN / DRAFT / UNMERGED.

Do not ask the user questions. On a genuine unresolved scientific or API boundary, emit `BLOCKED_BY_UNRESOLVED_SPEC` with exact reproducer evidence and stop before guessing.
