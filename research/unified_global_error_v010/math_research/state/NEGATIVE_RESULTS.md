# Negative results

- Equal user tolerances do not define equal external global error across method families.
- Same accepted-step count is not a cost authority.
- Linear interpolation on a common grid can cap apparent order and is excluded in P0.
- Stored trajectory bytes are not peak RSS.
- A single weighted work score can be tuned to favor a chosen solver and is excluded.
- One-shot microsecond timing was too noisy; calibrated batched timing was required.
- T1 repeated timing campaign and T4 one-pass suite are not comparable durations; combining them produced a spurious >300x ratio.
- Current every-step trajectory storage contaminates future adaptive work-precision ranking and must be separated from solver cost.
