# A1 Two-Arm Authority Receipt

- Decision: `ADMISSIBLE_AND_DISCRIMINATING`
- Profile: `enforced-budget-holdout-320`
- Scientific execution head: `8f2eac6a770af3a8898176dbf9fb2c0d15f6580c`
- Scientific execution tree: `17e9cdfd54998a201105f6f03a5a38883073e48c`
- Tested execution merge: `8e2485323b0a86745dd8eb438cecacf63f914202`
- Tested execution merge tree: `17e9cdfd54998a201105f6f03a5a38883073e48c`
- Execution workflow: `32914955031` attempt `1`
- Scientific digest: `407596e088b89d243280225e01667b8cee559c7015b4777f0bce441764eb7fc6`

The ordinary committed arm remains `legacy-fixed`. This receipt does not activate the candidate and makes no timing, ranking, speedup, or equal-error-contribution claim.

## Arm totals

| Arm | Attempts | Accepted | Rejected | RHS | JVP | Linear matvecs | Hires positive control |
|---|---:|---:|---:|---:|---:|---:|---|
| `legacy-fixed` | 435 | 416 | 19 | 3480 | 82996 | 76471 | true |
| `outer-scaled-numeric-parity` | 435 | 416 | 19 | 3480 | 68989 | 62464 | true |

## Safety and provenance

- Complete cells: 12
- Unsafe recommendations: 0
- Audit unsafe events: 2
- Artifact manifest entries: 12
- Receipt commit/tree and post-receipt workflow IDs are intentionally external late-bound evidence.

## Limitations

- Continuation work is charged to the total speculative ledger but never feeds the prefix-only budget ledger or later caps.
- R-JF parity excludes only attempt and accepted-step wall-clock fields; no stronger state/output parity claim is made without explicit digests.
- The committed protected R-JF trajectory and controller remain authoritative; the retained full-E result is read-only shadow evidence.
- The frozen k=3, B_abs=80, delta=0.25, and zeta34 threshold are consumed without retuning.
- The outer-scaled arm matches preserved phi tolerance numbers only; this receipt makes no equal-error-contribution claim.
- These five profiles are consumed economics evidence, not a fresh safety holdout; active switching and N=2048 remain sealed.
- Wall time, ranking, speedup, active switching, and candidate activation are outside this receipt.
