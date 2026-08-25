# Invalidated A1 Two-Arm Execution — Run 32906175896

## Authority status

```text
STOP_INVALID_NON_AUTHORITY
```

The workflow transport and twelve-cell matrix completed successfully, but the evidence schema was insufficient for the predeclared scientific classification. Preserve this run as diagnostic evidence; never use it to authorize a receipt, candidate activation, merge, timing claim, or ranking claim.

## Exact identity

```text
scientific execution head
755b31750c1f0e026bbe11aca24efb71e6242624

scientific execution tree
abbeed3aa1e8ac5d8b00f8173d67f560a914a087

tested execution merge
31b0e52a0ebe025db99a299c38a47c88517c88c8

tested execution merge tree
abbeed3aa1e8ac5d8b00f8173d67f560a914a087

workflow run / attempt
32906175896 / 1

workflow artifact
9584976503

artifact transport digest
sha256:2dfa08350497a544d48833c697a6a148abcb954dc30dc7adbb7aefcc24a8b644

aggregate scientific digest
7665718c60ff9c1e0d1e86d1ff4464e8eb71d806dd0e6ce5c4f6ac0501f027a1
```

## Root cause

The atomic cell generator used runtime shadow fields:

```text
shadow_full_e_completed
shadow_full_e_locally_admissible
```

and derived:

```text
audit_unsafe = shadow_full_e_completed && !shadow_full_e_locally_admissible
```

The runtime shadow is executed only when the policy recommends the event. The Hires above-threshold positive-control events are intentionally unrecommended, so their runtime shadow fields are false even though an independent audit full-E execution is required to determine whether they are unsafe.

Therefore:

```text
shadow_full_e_completed = false
```

means **not executed by the runtime recommendation path**. It does not mean safe, and it does not establish positive-control disappearance.

## Hires evidence showing the gap

Both arms retained an above-threshold unrecommended Hires event:

```text
legacy-fixed
zeta34 = 14.320053508327359
margin = +0.9229873197271985
recommended = false
shadow_full_e_completed = false

outer-scaled-numeric-parity
zeta34 = 14.252647475840892
margin = +0.8555812872407316
recommended = false
shadow_full_e_completed = false
```

No separate `audit_full_e_completed`, audit admissibility, audit failure, or audit work evidence was present. The reported `ADMISSIBLE_BUT_NONDISCRIMINATING` result is therefore unsupported and withdrawn.

## Required invalidation consequence

Because the runner schema and aggregation semantics must change, this scientific execution head and its artifacts cannot be patched into authority after the fact. The next implementation must:

1. add independent arm-specific audit full-E evidence;
2. publish a new scientific execution head;
3. rerun all twelve cells;
4. validate the new artifacts before creating any receipt commit.

## Supersession rule

Where this file or the updated `CURRENT_STATE.json` / `IMPLEMENTER_PROMPT.md`
conflicts with older two-arm handoff prose, this repair is authoritative. The
older plan, threat catalogue, invariant matrix, and review prompt remain useful
only for constraints that do not conflict with the audit-full-E evidence
closure.
