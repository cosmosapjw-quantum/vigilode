# Revised adapter: measured improvements and remaining tradeoffs

The second revision produces reproducible work reductions and qualified elapsed
time improvements in two concrete situations. Reusing a successful pilot's
larger step scale reduces the repeated implicit startup ramp. A single retry
after an entry pilot makes no accepted progress prevents an unnecessarily long
explicit startup. Both improvements operate through observable numerical
conditions, rather than hard-coded problem names.

All 26 paired comparisons completed, giving 520 timed batches. Eleven comparisons
pass the registered accuracy, deterministic-record, batch-duration and drift
conditions. Seven favor the second arm, two favor the first, and two show no
clear difference; some of these are deliberately included direct controls.
The remaining comparisons retain their work/error results but do not support
quantified speed claims. No acceptance threshold was relaxed after execution.

## The two changes have separately measured effects

| Comparison | RHS calls | Attempts | Median paired time ratio | Observed five-block range |
|---|---:|---:|---:|---:|
| Forced cosine: evidence → revised adaptive | 456 → 394 | 66 → 58 | 0.898 | 0.860–0.936 |
| Forced cosine: unseeded → growth-only seed | 456 → 394 | 66 → 58 | 0.872 | 0.850–0.897 |
| Case5: retry disabled → retry enabled | 2032 → 680 | 331 → 106 | 0.796 | 0.772–0.909 |

The ratios are second-arm time divided by first-arm time. They are medians of
five paired block ratios, not ratios of separately calculated marginal median
times. The ranges are observed extrema, not confidence intervals. All three
comparisons satisfy the preregistered requirement that every paired ratio be
below 0.95.

On forced cosine, the complete revised selector is 10.2% faster than the native
evidence policy in this experiment. The isolated step-seeding ablation shows
12.8% lower time. These separate paired experiments need not yield identical
percentages. The first implicit tenure falls from 12 to six attempts; total RHS
calls fall by 13.6%. The endpoint error remains approximately `7.33e-10` against
a `1e-5` cap. A mean accepted pilot step is reused only when larger than the
original initial proposal. The solver still applies its ordinary local error
test to the next attempt.

The case5 retry ablation shows 20.4% lower elapsed time, 66.5% fewer RHS calls and
225 fewer attempts. The initial implicit entry pilot makes five rejected
attempts and zero accepted progress. The revised scheduler permits exactly one
additional eligible-prefix comparison instead of treating that unresolved
startup as durable negative evidence. The retry wins in one implicit attempt.
The physical state remains the accepted explicit state; implicit entry occurs
at `t≈0.000113635`, compared with `t≈0.000835028` when the retry is disabled.

Against the native evidence policy, case5 work falls from 1456 to 680 RHS calls
and from 235 to 106 attempts. That particular paired timing cell fails the
registered drift condition, so its apparent time reduction is not promoted into
a speed claim. The independently qualified retry ablation is the elapsed-time
evidence for the new scheduling mechanism.

The R1 shrinking-seed defect is removed. With the new growth-only guard, case5
has exactly 680 RHS calls whether entry-step seeding is enabled or disabled.
The corresponding timing range crosses one. The seed therefore makes no
supported performance contribution to case5, while it retains the measured
forced-cosine benefit.

## Direct controls prevent overstating the selector's value

| Qualified direct comparison | Direct / adaptive time ratio | Interpretation |
|---|---:|---|
| Case5, matrix-free RODAS | 1.245; range 1.207–1.281 | Adaptive is about 19.7% faster than this same-backend direct control |
| Case5, dense RODAS | 0.0952; range 0.0917–0.0978 | Dense direct is about 10.5 times faster than the matrix-free adaptive route |
| Forced cosine, dense RODAS | 0.269; range 0.260–0.302 | Direct implicit execution remains substantially cheaper |
| Decay, DOPRI | 0.533; range 0.482–0.555 | Avoiding unnecessary selection is faster for this simple explicit case |

Case5 illustrates why RHS calls alone cannot rank runtime. The adaptive route
uses 680 RHS calls versus 307 for direct matrix-free RODAS, but only 796 JVP
vectors versus 1206, and 201 linear solves versus 312. Its explicit startup can
be cheaper than resolving that startup entirely with the matrix-free implicit
backend. The full observed runtime, including all probes and setup, confirms
an advantage against that direct control at the common error cap.

The dense backend is a stronger alternative on this two-dimensional fixture.
Its direct matrix-free/dense comparison has a dense-to-matrix-free time ratio of
`0.0747`, with observed range `0.0710–0.0807`; both use 307 RHS calls and attain
the same reported endpoint error near `1.04e-7`. This approximately 13.4-fold
backend difference is distinct from the selector improvement. It supports a
caller-authorized small-dimension dense specialization. It does not justify
silently introducing matrices into a request that requires matrix-free work.

These controls request only span endpoints, whereas the selector retains eight
uniform landing windows plus accepted batch endpoints. Their comparison measures
the complete selection service, including its output obligations. The separate
RHS-only availability check uses matching observation times and verifies exact
native work/state equality.

## Some mechanisms remain conditional or costly

Transition renewal preserves the large work reductions found in R1:
`184724→30515` RHS calls at transition position 0.39 and `68323→29327` at 0.79.
The old lifetime limit prevents a ninth entry comparison; adaptive renewal
performs it and switches successfully. Step seeding contributes zero and eight
RHS calls of those savings. The transition timing cells remain affected by
drift, so this report retains the deterministic work result without assigning
a validated speedup factor. These are parameter holdouts within the same
tanh-transition family.

On relaxation, the stronger same-checkpoint return comparison costs 48 RHS
calls; one call saved elsewhere gives a net change of `675→722`. Its complete
elapsed time is **12.2% higher** than the native evidence policy, with paired
range `1.087–1.159`. Both methods make the same two route changes on this fixture.
The stronger decision contract is useful when a stale preceding window is a
concern, but this fixture demonstrates its price rather than a measured recovery
from an incorrect switch. It should remain an explicit reliability option,
not be advertised as a universal acceleration.

Rotation and decay have no native/adaptive work difference. Tight gyromotion
saves one RHS call while adding enough JVP/linear work to increase the broader
work proxy. There is no supported optimization there. The persistent explicit
route remains the economical choice when no implicit comparison is required.

## Accuracy and accounting audit

There are 34 distinct case/arm records. Thirty-one pass their full-state endpoint
cap. The three failures are precisely the registered default-tolerance
gyromotion arms: adaptive and evidence have error `0.0315989394`, and direct
DOPRI has `0.0316814163`, against `1e-4`. All methods reach the endpoint, so
successful completion alone would have concealed this accuracy failure.
Tight gyromotion reaches error approximately `3.9e-5` and passes.

All aggregate selector work fields reconcile with segment sums. Disposition
RHS/attempt totals reconcile as well. Eighteen duplicated full records in
different paired cells agree exactly. Ratios were independently recomputed
from batch elapsed times and repetition counts for all 26 comparisons. All nine
native evidence records are identical between R1 and R2, showing that these
revisions leave that policy unchanged on the registered fixtures. This does not
claim full parity with an earlier repository commit.

The five PNG figures were inspected at their rendered size. Labels and ranges
are readable, no clipping was found, and unqualified timing is visibly marked
with crosses. The startup-retry ablation appears in the numerical comparison
table; its effect should not be inferred from the unqualified primary case5
timing point.

## Development choices supported by these results

Retain growth-only pilot step reuse and the bounded unresolved-startup retry as
conditional optimizations. Retain adaptive renewal for changing regimes. Keep
actual-window return confirmation available with its measured cost, and avoid
enabling competitive implicit probing universally on stable explicit problems.
Treat small-dimension dense execution as a separate, explicitly authorized
specialization. Do not qualify default-tolerance gyromotion for speed ranking.

The measured gains justify these specific improvements. They do not establish
that a single automatic configuration is the best production choice across all
stiff, oscillatory and nonstiff problems. The next useful experiments should
target difficult startup histories, changing regimes and misleading return
windows, while comparing the selected method with a relevant direct control
under the same physical error requirement.
