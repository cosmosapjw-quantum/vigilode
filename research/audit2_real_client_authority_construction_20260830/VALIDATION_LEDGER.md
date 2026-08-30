# Validation ledger

This ledger distinguishes prior local baseline replay, candidate-free
construction checks, final host verification, and the still-pending local
six-case execution.

## Prior PR #38 local replay and stop verdict

The local replay at exact PR #38 implementation head
`a2115e2dcaeb418185a3bccda62e60b6c4ff16ab` reported:

| Check | Observation |
|---|---|
| reusable transaction suite | `13 passed, 0 failed` |
| matrix-free common-W suite | `6 passed, 0 failed` |
| original-target bridge suite | `15 passed, 0 failed` |
| readiness | Python `20 passed`; Rust `77 passed` |
| clippy `-D warnings` | exit `0` |
| formatting | exit `0` |
| `git diff --check` | exit `0` |

That run correctly stopped as `REAL_CLIENT_AUTHORITY_UNAVAILABLE` before the
six real-client cases. Its external result artifacts were reported with these
SHA-256 values:

```text
execution_manifest.json  93d8558cbbdaf20f8a0ed4ddcb59a92b96fcd9056a238ae717c024f92814e2b6
result_summary.json       6bc86bc6bc0c71d07dad77a7d1b205df1bfd0277506989b65d98b9f827131b0e
SHA256SUMS                45bc17f38fa9e0c201c6c9302e1dd64ab5302391ed7dac1855e7ed01901522f5
```

These are a stop receipt, not Bateman candidate evidence.

## Candidate-free TDD history

| Stage | Frozen test/result | Disposition |
|---|---|---|
| authority public surface RED | Missing Bateman authority symbols caused Cargo exit `101` | Expected RED before implementation |
| minimal canonical authority GREEN | Focused Rust authority contracts passed `3/3` | Superseded by expanded contract |
| checked-in manifest binding RED | Missing manifest file failed | Expected RED before artifact |
| manifest binding GREEN | Rust contracts passed `4/4` before the proof/runner contracts were added | Superseded by the final expanded `6/6` GREEN |
| exact verifier RED | Missing verifier failed Python test | Expected RED before verifier |
| exact verifier GREEN | Python authority tests first passed `3/3`; the final expanded suite passed `4/4`; direct verifier returned `AUTHORITY_CONSTRUCTION_VERIFIED` | Final candidate-free GREEN; candidate executions reported as `0` |
| dedicated feature boundary | `audit2-bateman-authority` required by the new test and example targets; CI-scope guard first failed then passed after readiness update | Final candidate-free GREEN |
| reference-uncertainty P1 RED | Killing tuple `(E_reference,B,u)=(0.9,1.0,0.2)` could not compile before new assessment symbols; Cargo exit `101` | Expected RED; no candidate called |
| reference-uncertainty P1 GREEN | Declared-upper-bound path uses `E_reference+u<=B`; estimate-only path rejects; transaction receipt separates budget and error upper bound | Final reusable-transaction replay `16/16` GREEN |
| fresh-review rounding P2 | Nearest-even `E=1`, `u=2^-54`, `B=1` can round `E+u` to `1`; nearest-even difference/L2 and budget formation also lack one-sided guarantees | Found before candidate execution |
| conservative-rounding repair | Candidate/reference component differences and L2 are outward-bounded, `+u` is outward-rounded, and the output budget is inward-rounded; the rounding counterexample rejects | Final reusable-transaction replay `16/16` GREEN |
| one-shot runner review | Cache mismatches could be labelled observed; strict fallback omitted the candidate step; cache-probe orchestration failures dropped cache/work | Two P2s and one P3 found before candidate execution |
| runner receipt repair | Added `contract-mismatch`, serialized candidate steps, required accepted-candidate/rejected-budget causality for strict fallback, and retained partial cache/work | Pre-final-review authority replay `5/5` GREEN |
| local receipt validator RED/GREEN | Missing validator first raised `FileNotFoundError`; the initial synthetic six-case contract passed `3/3`, while reorder and false self-assertion failed closed | Superseded by the final-review hardening below |
| single fresh final review P1 | The compact validator could false-pass empty `WorkCounters`, a fake committed-state digest, minimally self-asserted budget fields, and an incomplete late-failure apply ledger; runtime admission also lacked a candidate-free live W/PC rebinding contract | `ONE_FRESH_FINAL_REVIEW; ONE_P1; CANDIDATE_EXECUTIONS_0` |
| final-review RED | Five new validator regressions failed against the compact validator; the new runtime W/PC rebinding contract failed to compile because its binding surface was absent, with Cargo exit `101` | Expected candidate-free RED before the bounded repair |
| final-review bounded repair | Validator now checks exact work schema/counters, recomputed state bits/digest, full budget/subgate consistency, exact cache transitions, and late second-apply ledger; Rust admission rebinds both admitted cases to the live shifted W and PC without running a candidate | Final Python validator `12/12` and Rust authority `6/6` GREEN |

The direct exact verifier summary, retained by the final candidate-free replay,
was:

```text
status                              AUTHORITY_CONSTRUCTION_VERIFIED
candidate_executions                0
verified_operator_cases             2
execution_scenarios                 6
max_reference_l2_bound              2.075243427511439e-17
declared_reference_l2_uncertainty   1e-15
fast_exponent_exceeds_one           true
holdout_access                      NOT_OPENED_OR_EXECUTED
```

Its checked-in candidate-free receipt is
`evidence/AUTHORITY_VERIFICATION_RECEIPT.json`, SHA-256
`057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12`.
Admission also binds manifest SHA-256 `673045bf...c360` and exact-verifier
SHA-256 `542715ca...487d` before creating the opaque, consuming authority.

## Final hosted verification

Status: `HOST_CANDIDATE_FREE_FINAL_PASS`.

Complete replay evidence directory:

```text
/workspace/scratch/43ab8f4e0aeb/verification/audit2-authority-source-freeze.Ic445U
```

`tools/check-audit2-readiness.sh` exited `0`. The final replay observed:

| Class | Exact final observation |
|---|---|
| Python | `36/36 PASS` = scope `12` + output policy `8` + exact authority `4` + synthetic local-receipt validator `12` |
| Rust | `86/86 PASS` = fair-ab `8+9` + matrix-free `6` + reusable transaction `16` + structured bridge `15` + dense output `15` + homotopy `6` + Bateman authority `6` + default example tests `5` |
| clippy | `PASS` with `-D warnings` |
| formatting | `PASS` as part of the exit-0 readiness replay |
| Bateman local example | compiled by the hosted checks; never executed |
| Bateman candidate executions | `0` |

The Python local-receipt cases use frozen synthetic receipts to validate shape,
exact work/state/budget/cache fields, and fail-closed classification. They are
not executions of the Bateman client. The actual Rust uncertainty enum spelling
in JSON is uppercase `DECLARED_UPPER_BOUND`, not kebab-case.
The final replay retained the fresh-review P2 and conservative arithmetic
repair instead of treating the first formula-only repair as sufficient.

### Preserved final-attempt failure history

1. `audit2-authority-final.p9f1Kf` stopped after the first 12 Python scope
   tests because `mpmath` was absent from the active Python import path. This
   was a host dependency/import-path failure; no Rust or Bateman candidate ran.
2. `audit2-authority-final-retry.7idxY5` progressed through the candidate-free
   Python and Rust suites, then clippy rejected the large unboxed
   `Audit2BatemanPartialFailure` error variant under `result_large_err` and
   `-D warnings`. The bounded repair boxed the error value; no numerical rule,
   threshold, reference, digest, scenario, or candidate outcome changed.
3. `audit2-authority-post-review-pass.oFT5VV` was interrupted after the fresh
   reviewer caught a fixture/validator assumption that used the wrong
   kebab-case uncertainty spelling. Rust serializes the relevant enum as
   uppercase `DECLARED_UPPER_BOUND`. The partial run is not counted as final.
4. `audit2-authority-post-review-final.2rlYQV` reached clippy after the
   candidate-free suites, then failed `needless_range_loop` in the live W/PC
   rebinding check. The bounded mechanical fix used iterator `enumerate()`;
   scientific inputs and criteria were unchanged.
5. The complete replay was restarted from the beginning in
   `audit2-authority-source-freeze.Ic445U` and passed all 36 Python tests, all
   86 Rust tests, clippy, and formatting with readiness exit `0`.

### Residual compact-receipt limitations

- The compact local receipt does not retain the raw embedded-error or
  original-target vectors. Its validator therefore cannot independently
  recompute the embedded L2, original-target residual L2, or contraction
  scalar from raw vectors; it can only enforce their schema, finiteness,
  declared limits, booleans, and cross-field consistency.
- SHA-256 binding establishes byte integrity against the frozen manifest,
  verifier, proof, and state bits. A digest is not host identity, execution
  provenance, authorship, or remote attestation.

## Local scientific execution

Status: `NOT_RUN_DURING_CONSTRUCTION; HOST_CODEX_ONLY_HANDOFF_PENDING`.

No candidate receipt, selection, Bateman state hash, or Bateman `WorkCounters`
from the six-case suite exists in this node. The exact command and fail-closed
return requirements are in `CODEX_START_HERE.md`.

No tolerance, uncertainty declaration, problem size, model rate, reference,
digest, PC identity, or expected structural disposition may be changed after a
candidate observation.
