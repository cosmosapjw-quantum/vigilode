# First integrated comparison: what improves and what still costs work

The useful result is selective. Adaptive renewal recovers from a lifetime probe
limit on the two late-transition fixtures, while reusing a successful implicit
pilot's step scale removes a repeated startup ramp on stiff forced cosine.
The same changes do not justify replacing the native policy universally.
Case5 exposes a delayed-entry regression; relaxation pays for stronger return
evidence; oscillatory fixtures continue to favor a persistent explicit route.

All 25 paired cells completed. Their 33 distinct arm records were deterministic
in sampled solves and identical across 17 repeated appearances in different
pairs. Aggregate attempts, accepted/rejected work, RHS, Jacobian, JVP, explicit
time derivatives and linear solves all reconcile with segment sums. Disposition
RHS and attempt totals also reconcile. This audit compares this binary's
records; it does not assert identity with an older commit.

Only four paired cells satisfy both the registered timing conditions and the
physical endpoint cap. None meets the registered criterion for a clear timing
improvement. Early/late drift and block variation invalidate most elapsed-time
comparisons, including several apparently large gains. The deterministic work
changes below remain informative. The raw timing ranges remain visible.

## Step reuse has one clear constructive mechanism and one avoidable defect

| Fixture | RHS without → with entry-step reuse | Attempts without → with | Interpretation |
|---|---:|---:|---|
| Stiff forced cosine | 456 → 394 | 66 → 58 | First implicit tenure drops from 12 to 6 attempts |
| Case5 | 2032 → 2040 | 331 → 332 | Reused mean step is smaller than the original proposal |
| Relaxation | 723 → 722 | 101 → 101 | One fewer RHS; one additional rejection |
| Transition 0.39 | 30515 → 30515 | 4942 → 4942 | No work advantage |
| Transition 0.79 | 29335 → 29327 | 4845 → 4844 | One implicit attempt saved; no clear timing advantage |

On forced cosine, the original next-tenure proposal was `1e-7`; the completed
winning pilot supplies a mean accepted step of about `1.02475e-3`. The native
implicit acceptance test remains responsible for every subsequent step. The
result is 62 fewer RHS calls and 56 fewer linear solves, with the same endpoint
error of about `7.33e-10`. The pilot state is not substituted for the explicit
physical state. This is a measured reduction of a restart ramp, not a new
order-of-accuracy theorem.

Case5 demonstrates why unconditional replacement is inappropriate. Its pilot
mean step is about `5.30051e-5`, below the existing `1e-4` proposal. Using that
smaller mean adds an implicit attempt and eight RHS calls. A growth-only reuse
rule directly addresses this observed defect. It is a revision informed by
these measurements and requires a fresh comparison; R1 cannot validate it.

## The case5 regression is mostly in committed explicit work

The native evidence policy uses 1456 RHS calls, compared with 2032 for adaptive
renewal without step reuse and 2040 with reuse. Both policies perform one losing
entry pilot and one winning entry pilot. The adaptive policy enters the implicit
route later, at approximately `t=0.000835`, instead of `t=0.000524`. It therefore
spends 96 additional committed explicit attempts, or 576 additional explicit
RHS calls, before entry. Changing the backend or counting fewer discarded
pilots does not explain or solve this scheduling defect.

The initial losing pilot has five rejected attempts and zero accepted steps.
Consequently it supplies no accepted-progress estimate for its proposed regime.
A bounded one-shot retry on the next eligible prefix is a concrete candidate:
it can reconsider this unresolved startup comparison without restoring a pilot
at every explicit batch or losing later transition sentinels. This condition
must be tested separately from the growth-only step seed.

## Renewal is useful after the old lifetime limit expires

At transition position 0.39, charged RHS work falls from 184724 to 30515; at
0.79, it falls from 68323 to 29327. The legacy policy uses up its eight permitted
losing pilots before an implicit method becomes competitive and never switches.
The adaptive policy subsequently performs a ninth entry pilot, which wins. Its
endpoint errors are about `2.19e-10` and `2.46e-9`, respectively, both below the
common `1e-5` cap. The corresponding deterministic work proxy, including JVPs
and linear solves, falls from 185949 to 48386 and from 69543 to 34934.

These gains come primarily from renewing selection evidence, not from the step
seed: its ablation saves zero and eight RHS calls. Positions 0.39 and 0.79 are
new parameter holdouts within the same previously studied tanh-transition
family. The results support keeping this adaptation for changing regimes.

## Stronger return evidence has a visible cost

On relaxation, the legacy policy uses 675 RHS calls and the integrated policy
uses 722. An actual implicit solve on the proposed return window costs 48 RHS
calls; step reuse recovers one call elsewhere. Both methods make two switches.
The explicit return is accepted after a same-checkpoint comparison, but this
fixture does not exhibit a wrong return under the old rule. The additional work
therefore supports a stronger decision contract, without proving an observed
robustness or speed advantage on this case. Keep it available where avoiding a
stale-window decision matters; do not call this a free optimization.

## Direct controls identify the remaining selection and backend costs

Rotation has identical native/adaptive work: 5371 RHS calls, versus 5251 for
direct DOPRI. Tight gyromotion changes from 24555 to 24554 RHS calls, but its
broader work proxy grows from 25313 to 25323 because JVP/linear work increases.
That single-call reduction is not a meaningful economy. Decay also has identical
policy work. Persistent explicit execution remains the appropriate low-cost
option when the caller does not authorize competitive implicit work.

The case5 direct matrix-free and dense controls both use 307 RHS calls. The
matrix-free control additionally evaluates 1206 JVP vectors; the dense control
constructs 34 Jacobians and uses the corresponding direct factors. Their time
ratio appears large, but drift prevents assigning a defensible speedup factor
from R1. The work records establish why comparing a matrix-free selector solely
against a dense direct control confounds two different design choices. Any
small-dimension dense specialization must remain explicit about its matrix
permission and should be validated independently of the selector.

The selector has eight landing windows and records accepted batch endpoints,
whereas direct performance controls request only span endpoints. Those landing
obligations belong to the measured selection service. They prevent interpreting
the direct comparison as an equal-output-contract kernel test. The separate
RHS-only availability check instead uses matching observation times and passes
all native counter/state equality checks.

Default-tolerance gyromotion completes numerically but has endpoint error near
`0.0316`, exceeding the `1e-4` cap by more than 300 times. It remains a reported
failure cohort. The tight configuration reaches approximately `3.9e-5` and
passes. No default-tolerance timing is eligible as evidence of useful speed.

## What to change next

1. Restrict winning-pilot step reuse to an increase over the original proposal;
   preserve the ordinary implicit acceptance and rejection tests.
2. Test one bounded retry after a zero-accepted-progress entry pilot, with an
   independent case5 ablation. Keep evidence renewal available later in the
   trajectory.
3. Preserve all R1 data. Repeat the changed comparisons with longer timed
   batches under unchanged drift, work and accuracy conditions. Do not relax
   the accuracy cap or timing criteria to obtain an improvement label.
4. Retain transition renewal as a supported adaptive candidate, step reuse as
   a startup optimization, and actual-window return comparison as a stronger
   but paid decision option. Defer general performance promotion until the
   corrected scheduling and timing evidence justify it.

Five generated PNG figures were visually inspected: labels and empirical ranges
are readable, no clipping was found, and timing that fails qualification is
marked with crosses. R1 figure bytes are preserved. The plotting source now
uses more precise titles for future results: RHS evaluations are named as such,
and the endpoint-error title does not imply every tolerance cohort succeeds.
