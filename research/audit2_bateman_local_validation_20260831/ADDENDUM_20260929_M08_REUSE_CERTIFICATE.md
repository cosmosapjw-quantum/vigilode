# Addendum 2026-09-29: M08 reuse certificate is computed (audit F-062)

Append-only. `MATH_BLOCKER_LEDGER.json` is unchanged.

The ledger lists M08 ("reuse certificate epsilon + delta < 1") with remaining
evidence "DeltaW proxy" and "preconditioner defect". Audit F-062 found that no
code computed either quantity: reuse was admitted by identity equality only.

`audit2_preconditioner_reuse_certificate` (feature `audit2-research`) now
computes it, and a transactional attempt reports it when
`Audit2TransactionalAttemptConfig::reuse_certificate` is set.

- **epsilon.** `epsilon = ||I - P W||_1` is evaluated column by column: one
  operator application and two preconditioner applications per column, up to
  dimension 1024. It is a floating-point evaluation, not a directed-rounding
  bound.
- **delta.** `delta = ||P||_1 ||Delta W||_1 = 0`. The cache reuses `P` only
  when the exact operator identity, the frozen-W digest and the
  preconditioner identity are all equal, so `Delta W = 0` by construction.
  Reuse across a changed W is still refused rather than certified.
- **certified.** `certified` means `epsilon + delta < 1`.
- **Work.** The certificate's work is reported in the certificate. It is
  never charged to the attempt.

The frozen Bateman six-case scenarios do not set the flag, so their configs
and receipts are unchanged. M08 therefore stays CLOSED_CONDITIONAL for those
receipts. A new run with the flag set produces the evidence the ledger asks
for, but none has been run.
