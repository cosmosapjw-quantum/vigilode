# IMPLEMENTER PROMPT — PR #18 Audit Full-E Evidence Closure

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
755b31750c1f0e026bbe11aca24efb71e6242624

implementation tree
abbeed3aa1e8ac5d8b00f8173d67f560a914a087

tested PR merge
31b0e52a0ebe025db99a299c38a47c88517c88c8

A1 workflow
32906175920 SUCCESS

E4 workflow
32906175923 SUCCESS

invalidated two-arm run
32906175896 STOP_INVALID_NON_AUTHORITY
```

Before mutation, run the handoff acceptance suite and independently verify remote refs, PR #18 state, current changed paths, exact workflows, and the invalidated artifact identity.

## Goal

Implement only:

```text
A1-AUDIT-FULL-E-EVIDENCE-CLOSURE
```

The old twelve-cell workflow is not authority. Do not create a receipt commit from run `32906175896` or aggregate digest `7665718c60ff9c1e0d1e86d1ff4464e8eb71d806dd0e6ce5c4f6ac0501f027a1`.

## Root-cause contract

`shadow_full_e_*` is runtime recommendation evidence. An unrecommended event normally has no shadow execution.

Independent `audit_full_e_*` evidence is read-only and must be generated for the audit-eligible event population regardless of recommendation. Never infer audit safety from absent runtime shadow execution.

## Mandatory RED tests before production changes

1. A v1 Hires cell from run 32906175896 is rejected for authority because it lacks independent audit fields.
2. `shadow_full_e_completed=false` cannot imply `audit_unsafe=false`.
3. An unrecommended event may validly have `shadow_full_e_completed=false` and `audit_full_e_completed=true`.
4. Missing or incomplete audit evidence causes `STOP_INVALID` before any predeclared scientific decision.
5. `ADMISSIBLE_BUT_NONDISCRIMINATING` requires complete audit evidence proving genuine positive-control absence.
6. Hires positive control requires an above-tau unrecommended event with completed audit full-E and audit-local inadmissibility.
7. Audit failure and ineligibility states require explicit reasons and cannot silently become safe.
8. Audit evidence is arm-specific and exactly aligned to arm/family/event identity.
9. Audit execution does not change recommendation keys, budgets, controller results, R-JF trace identity, or the committed arm.
10. Audit work is retained separately and is not charged into prefix or continuation budgets.
11. Missing, duplicate, extra, or unaudited eligible event rows are rejected.
12. Run 32906175896 and its aggregate digest are mechanically rejected as receipt authority.

Watch each RED test fail for the intended reason before implementing GREEN.

## Minimal architecture

- Reuse the existing stage-growth audit full-E computation path if its physics and event-state inputs are exactly the required ones.
- Do not duplicate or simplify the full-E solver merely for the receipt.
- Add a named receipt-only arm-specific audit function. It must not be reachable through the ordinary committed runtime path.
- Keep runtime shadow fields unchanged for provenance and compatibility.
- Add independent audit fields to every atomic event row:

```text
audit_full_e_eligible
audit_full_e_attempted
audit_full_e_completed
audit_full_e_total_error
audit_full_e_locally_admissible
audit_full_e_failure
audit_full_e_work
audit_unsafe
audit_evidence_status
```

- `audit_unsafe` must be nullable/unknown when audit evidence is incomplete. Never default unknown to false.
- Update the atomic schema version and aggregate schema version.
- Update the CLI, Python validator, adversarial tests, and workflow artifacts.
- The aggregate must validate audit completeness before computing any of the three scientific decision classes.

## Decision contract

First validate evidence completeness.

```text
incomplete or unsupported evidence
-> STOP_INVALID
-> no H_receipt
```

Only after evidence is complete:

```text
ADMISSIBLE_AND_DISCRIMINATING
- all hard gates pass
- zero unsafe recommendations
- each arm preserves at least one completed audit-unsafe Hires event that is correctly unrecommended

ADMISSIBLE_BUT_NONDISCRIMINATING
- all hard gates pass
- zero unsafe recommendations
- complete audit evidence genuinely contains no Hires positive control

NOT_ADMISSIBLE
- safety/provenance hard gate fails
- or any recommended event is audit-unsafe
```

## Scientific and mutation boundary

Do not change:

- `V36_FROZEN_ZETA34_TAU = 13.39706618860016`;
- persistence thresholds or latch length;
- prefix or continuation budgets;
- GMRES restart, maxiter, convergence logic, Arnoldi, QR, Givens, or counters;
- G1/G3 policies;
- historical v3.5-v3.7 result files;
- Cargo dependencies or lockfile;
- timing protocol;
- ordinary committed arm `legacy-fixed`.

Do not perform A2/A3, ranking, speedup, active switching, candidate activation, merge, tag, or release.

## Required execution sequence

1. Finish RED-to-GREEN source and validator closure locally.
2. Run all targeted, regression, workspace, Clippy, and formatting gates.
3. Publish a **new** scientific execution head `H_exec`; do not reuse `755b317...`.
4. Run exact-head A1 and E4.
5. Run a fresh 12-cell two-arm workflow on the new H_exec.
6. Download and independently inspect all twelve cells and the aggregate.
7. If audit evidence is incomplete, stop `STOP_INVALID` and do not create H_receipt.
8. If complete, create one descendant H_receipt containing the validated receipt and decision.
9. Run post-receipt validation externally. Do not amend the tracked receipt to insert late-bound verification identities.
10. Run fresh-context review and stop with PR #18 OPEN / DRAFT / UNMERGED.

## Required verification

```bash
cargo test -p rodas5p-integrators --test a1_two_arm_receipt_contracts --locked
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

## Completion evidence

Return:

- RED and GREEN evidence;
- new H_exec head/tree and exact changed paths;
- A1/E4/new two-arm workflow run IDs;
- all twelve atomic cell identities;
- complete audit evidence counts and failures by arm/family;
- Hires positive-control audit rows for both arms;
- artifact manifest and scientific digest;
- receipt paths only if evidence is complete;
- decision and mechanically supporting evidence;
- forbidden-diff audit;
- P0/P1/P2/P3 ledger;
- unresolved blockers;
- explicit statement that activation, A2/A3, timing/ranking, switching, merge, tag, and release were not performed.

Do not ask user questions. On a genuine unresolved scientific or API boundary, emit `BLOCKED_BY_UNRESOLVED_SPEC` with an exact reproducer and stop before guessing.
