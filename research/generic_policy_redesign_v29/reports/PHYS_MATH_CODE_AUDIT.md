# PHYS–MATH–CODE AUDIT — v2.9

## Verdict

**MECHANISM PASS / SAFETY POLICY FAIL.** The code path obeyed the read-only, budgeted calibration contract; the scalar witness itself failed independent calibration.

## Code-path facts

- Parent v2.8 HEAD: `9e1da52afd27ddf80b6dad213489a32703e4ea26`.
- Contract was committed before solver output: `a5492398e66c52d9544bbecc9d97ea51355f9c6f`.
- Implementation/selector were frozen before calibration output: `ea7c76663223108425385a50f00e52cf7d33d326`.
- Calibration used 12 family-sharded runs, two fresh profiles.
- R-JF remained the committed trajectory.
- E-K full endpoint was computed only as an explicit audit label; `runtime_full_e_continuations=0` in every shard.
- `switching_active=false` in every shard.
- `budget_breaches=0` in every shard.
- All trajectory summaries report explicit Jacobian builds=0, direct factorizations=0, Newton iterations=0.

## Budget integrity

Every event passed the predeclared prospective reserve check before level1+2 was purchased. Every actual prefix stayed below the fixed reserve of 80 JVP vectors. No post-hoc budget relaxation occurred.

## Selection integrity

The threshold selector was written and synthetic-TDD tested before calibration. It scanned the complete predeclared finite threshold family, enforced pooled/groupwise zero unsafe, the multiplicity-aware upper-bound gate and minimum recommendation count, applied the predeclared 0.95 shrink, and returned `all-abstain`.

No Rust path or selector logic was changed after seeing calibration output.

## Why the N=384 holdout remains sealed

The contract requires freezing a safety rule before the final holdout. The only rule produced is the trivial all-abstain policy. Running N=384 now cannot validate a nontrivial A34 witness and would consume a useful unseen corpus. Therefore the correct failure-preserving action is to close v2.9 as negative and retain N=384 for a later, separately predeclared feature-validation node.

## Regression scope

Focused v2.9 TDD, existing level-2 parity tests, formatting and targeted Clippy were closed before calibration. A fresh closeout validation is required on the final negative-result commit; workspace-wide release PASS must not be claimed unless a complete exit code is obtained.
