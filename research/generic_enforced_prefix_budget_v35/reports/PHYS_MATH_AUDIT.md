# v3.5 PHYS–MATH Audit — Enforced Speculative Prefix Budget

## Exact budget invariant
Let `R_k` be committed R-JF JVP vectors before an event and `S_k` already-spent speculative prefix JVP vectors. The frozen policy defines

\[
B_k=\min\{80,\;\max(0,\lfloor 0.25R_k\rfloor-S_k)\}.
\]

The budgeted operator refuses the next JVP before the inner operator is called whenever the already-used count equals `B_k`. Hence the realized new prefix work `W_k` satisfies

\[
0\le W_k\le B_k\le80.
\]

Because `S_k` is integer-valued,

\[
S_{k+1}=S_k+W_k
\le S_k+B_k
\le \lfloor0.25R_k\rfloor
\le0.25R_k.
\]

This is a structural invariant for the prefix transaction; it is not a statistical statement and does not depend on the observed replay sample.

## Limiting cases
- `B_k=0`: no JVP is executed.
- generous cap: the budgeted level-2 prefix reproduces the unbudgeted level-2 report exactly.
- exhaustion: completed speculative work is retained, but no `zeta34` or full-E audit endpoint is produced; the committed method remains R-JF.

## Fresh holdout
On the predeclared N=320 profile, 28 non-exhausted events received full-E audit labels. One HIRES event was inadmissible (`q_E=1.1351317205`) and had `zeta34=14.3200535083 > tau=13.3970661886`, so the frozen policy abstained. Thirteen recommendations were made, with zero unsafe recommendations and all six families represented.

One semilinear event exhausted the prefix budget exactly at 80 JVP vectors and therefore produced no safety score or full-E audit endpoint.

## Scope ceiling
The theorem above covers only level1+2 prefix work. A future runtime full-E continuation would add further speculative work and needs its own complete ledger/budget treatment before activation.

## Verdict
**PASS for transactional prefix-budget semantics and the fresh discriminating empirical holdout.** This does not authorize active switching or imply a generic probabilistic safety theorem.
