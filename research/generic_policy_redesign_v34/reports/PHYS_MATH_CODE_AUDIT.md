# v3.4 PHYS–MATH–CODE AUDIT

## Code-path verdict

PASS for read-only safety auditing and threshold freeze; FAIL for the fixed absolute prefix-reserve contract.

## Authority path

- R-JF remains the only committed trajectory.
- k3 event logic and 25% cumulative speculative-JVP budget are inherited unchanged.
- zeta34 is computed from the existing level1+2 prefix.
- full E is computed only as the audit label; `runtime_full_e_continuations=0` in every shard.
- active switching remains false.

## Six-shard results

Five shards are `complete`; N384 semilinear is `complete-with-failures` solely because `budget_breaches=1`. All shard process return codes are 0; the failure is a scientific policy gate, not a process crash.

No Jacobian/Newton promotion is introduced.

## Exact budget failure mechanism

The pre-admission code checks whether reserving 80 future JVP vectors would fit inside 25% of cumulative committed R-JF JVP. It then executes the prefix without an actual 80-vector cap. Post hoc, it marks a breach if the realized prefix uses more than 80 or exceeds the cumulative-fraction budget.

At the failing semilinear event, the pre-admission reserve fits easily, but the realized prefix uses 109 JVP vectors. Thus “reserve=80” is currently an estimate/assertion, not an enforced transactional cap.

## Cross-profile work evidence

Observed actual level1+2 prefix JVP vectors:

- N96 (rtol 3e-6): median 19, P95 22.6, max 31,
- N192 (rtol 1.5e-5): median 19, P95 35.6, max 67,
- N256 (rtol 3e-5): median 19, P95 22.8, max 43,
- N384 (rtol 7e-6): median 19, P95 51.8, max 109.

The median is stable while the tail is not; a fixed predicted reserve is therefore not demonstrated to be scaling-safe.

## Next code requirement

Implement an actual JVP-budget interrupt around the speculative level1+2 prefix. The cap must be checked before each speculative JVP so a prefix can abort without overshooting. Aborted shadow work is charged, no state/controller mutation is allowed, and the event must abstain to R-JF.

Keep B_abs=80 and delta=0.25 initially; changing those numbers after seeing N384 would be retuning. The next node should test semantics first on consumed profiles, then use a newly sealed budget holdout rather than reusing N384 as unseen evidence.
