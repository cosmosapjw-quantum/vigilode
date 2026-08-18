# NEXT NODE — v3.5 Enforced Speculative Prefix Budget

## Objective

Turn the existing `reserve=80`/`delta=0.25` budget from a post-hoc prediction check into a transactional hard work cap, without changing the frozen zeta34 threshold.

## Contract-first requirements

1. Keep `B_abs=80` and cumulative fraction `delta=0.25`; no numerical retuning from N384.
2. At each event compute remaining budget
   `B_k=min(80, floor(0.25*committed_rjf_jvp - speculative_jvp))`.
3. Execute level1+2 through a JVP-counting budget guard that refuses JVP `B_k+1` before it occurs.
4. If the prefix cannot complete inside the budget, record `budget-exhausted`, charge all completed speculative work, emit no zeta34, and abstain to R-JF.
5. Prove R-JF state, controller, requested output, and existing work counters are unchanged.
6. First replay only consumed N96/N192/N256/N384 data for semantic regression; these are not new holdouts.
7. Do not re-label N384 as a holdout after this post-hoc semantics fix.
8. Before promotion, predeclare a new unseen budget/safety holdout profile distinct from N2048. Keep N2048 sealed as scaling holdout.

## Kill criteria

- any speculative JVP count exceeds the computed cap;
- any state/controller mutation on budget abort;
- hidden/uncharged failed work;
- material collapse of recommendation coverage on replay;
- any threshold change from tau=13.39706618860016.
