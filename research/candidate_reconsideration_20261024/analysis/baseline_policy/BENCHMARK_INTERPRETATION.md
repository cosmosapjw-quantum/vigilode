# What the new measurements establish

The primary campaign contains 205 timed complete trajectories in 41
case/method groups. The root-schedule extension contains 240 timed trajectories
in 48 groups. All timed trajectories and all warmups reach the requested
observations and satisfy the registered maximum sampled absolute-error limit
of 1e-5. These are repeat counts, not independent problem counts. The experiments
contain seven analytic systems; the expensive-RHS variant changes only callback
cost. All raw measurements, including the unhelpful comparisons, are retained.

## Bounded exploration saves failed projections but can miss useful candidates

The default switching policy produces no median-time improvement over fixed
causal forward in the seven primary cases. Fixed/adaptive median-time ratios
range from 0.731 to 0.998. The one protected calibration step adds overhead in
the favorable low-rank cases. On the two difficult spectra, long cooldowns also
skip attempts where the projected candidate would have succeeded.

For the broad diagonal spectrum, fixed causal forward makes 61 candidate
attempts, of which 50 require the protected route. The adaptive route makes
four candidate attempts, all unsuccessful, and skips 57 candidates. It
therefore invokes the protected solver on all 61 attempts. Although attempted
RHS calls fall from 3531 to 1160, JVP calls rise from 22657 to 25443. Median
time increases from 0.478 s to 0.542 s. The changing-rate model shows the same
mechanism: fixed forward has 14 successful projected attempts among 47,
whereas the adaptive route probes four times and uses the protected solver on
all 47. JVP calls increase from 11131 to 13993 and median time increases from
0.118 s to 0.161 s.

These records distinguish a bound on speculative candidate failures from a
bound on total integration cost. The cooldown provides the former. It does not
provide a regret guarantee against an algorithm whose economical domain changes
along a trajectory. The next improvement is a bounded, state-aware re-entry
test tied to a cheap structural indicator or a deliberately shorter caller
re-entry horizon. Such a policy must pay for that indicator and must be evaluated
on new trajectories, preserving these default-policy results.

The most efficient applicable protected implementation is compact-band in six
models and dense in the clock-augmented model. Its time divided by adaptive
causal time ranges from 0.000943 to 0.459. These particular structured analytic
problems do not support making the new switching route a general default.
The exact Jacobian and band callbacks are explicit comparator capabilities.
Their advantage is not evidence that a full Jacobian must always be assembled;
it identifies the value of exposing inexpensive structure when available.

## Parallel root schedules recover a concrete expensive-callback domain

With the deliberately expensive RHS and a reused two-worker pool, fixed
causal Picard is 1.250 times faster than its one-thread counterpart and 1.281
times faster than serial causal forward. Residual-first correction is 1.527
times faster than its one-thread counterpart and 1.430 times faster than
serial causal forward. These gains reopen the callback-cost regime that the
earlier cheap-RHS evidence did not cover.

Protected serial dense RODAS5P remains faster in this experiment: its time
divided by two-thread Picard time is 0.639, and divided by two-thread
residual-first time is 0.714. Cheap-RHS and Riccati cases lose performance with
two workers. Consequently the recovered use case is an explicitly requested
matrix-free/projected route with sufficiently expensive independent callbacks,
not an unconditional parallel default. The pool is constructed outside timed
solves and reused across warmups and samples; a one-shot application must also
charge pool startup. Five cyclic within-process repetitions characterize these
local measurements, without an independent-process confidence interval.

The fixed/adaptive root variants retain the same residual and physical-error
checks. Their successful analytic samples establish the tested implementations'
correctness on these inputs. Numerical stability or robustness outside these
models remains a separate qualification task.

## Counter interpretation

The existing receipt field `fallback_attempts` includes every protected-route
attempt, including deliberate calibration and cooldown. The summary calls it
`protected_route_attempts`. For complete successful groups, subtracting
`candidate_skipped` gives protected attempts following a candidate trial;
the derived field is `post_candidate_protected_attempts`. An intentional
protected step is not described as a failed candidate.
