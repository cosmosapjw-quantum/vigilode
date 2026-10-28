# What the reconsideration measurements establish

The final primary campaign contains 275 timed complete trajectories in 55
case/method groups. The final root-schedule extension contains 240 timed
trajectories in 48 groups. All timed trajectories and warmups reach their
requested observations and satisfy the registered maximum sampled absolute-error
limit of 1e-5. The largest observed errors are 1.705e-7 in the primary campaign
and 6.597e-10 in the root extension. All primary work counters repeat identically
within their groups. These are repetition counts, not independent problem
counts: six analytic ODE families yield seven registered workload cases because
one repeats the few-mode model with injected callback cost. An eighth
configuration restricts the basis of the unchanged few-mode model.

The first complete campaign remains under `analysis/baseline_policy/` with its
205 primary and 240 root-variant timed trajectories. Raw measurements and source
identities distinguish both campaigns. The final primary processes interleave
the original switching policy and its corrected version in the same binary, so
the correction can be compared without relying solely on between-process times.

## A diagnosed cost failure and its bounded correction

The original exponentially increasing cooldown suppressed failed projections
but missed useful candidates later in a trajectory. On the broad-spectrum
problem it evaluated only four candidates, all unsuccessful, while fixed
causal forward obtained 11 successful projected roots in 61 attempts. The
changing-rate problem had the same pattern: four probes missed 14 successful
projected roots found by fixed causal forward. Avoiding a failed cheap projection
does not compensate for missing an expensive protected solve that a later
successful projection could replace.

The final policy therefore prices a failed probe using its measured work C and
protected-route work P. With declared overhead fraction epsilon=0.25, its
unclipped cooldown is ceil(C/(epsilon P))-1 accepted protected steps, bounded
below by one and above by the configured maximum. The implementation preserves
the exponential option as `exploration_overhead_fraction=None`; both policies
appear in the final same-process experiment. The bound concerns a declared work
proxy under the stated reference assumptions. It does not provide a universal
elapsed-time or changing-regime regret guarantee.

| Final experiment | Fixed causal | Original switching | Priced switching |
|---|---:|---:|---:|
| Broad: candidate attempts / protected-route attempts | 61 / 50 | 4 / 61 | 30 / 58 |
| Broad: JVP calls | 22657 | 25443 | 25012 |
| Broad: median trajectory time, s | 0.5201 | 0.5877 | 0.5578 |
| Changing rate: candidate attempts / protected-route attempts | 47 / 33 | 4 / 47 | 28 / 37 |
| Changing rate: JVP calls | 11131 | 13993 | 11730 |
| Changing rate: median trajectory time, s | 0.1318 | 0.1819 | 0.1431 |

The correction recovers three successful projected roots in the broad case and
ten in the changing-rate case. Relative to original switching, median speed
ratios are 1.054 and 1.271. The broad-case timing ranges overlap substantially;
the deterministic work change is firmer evidence than that modest median gain.
Fixed causal forward remains faster in both cases. The correction reduces an
identified cost failure without establishing a globally best policy.

The additional rank-starved test restricts initial and maximum basis dimension
to one and permits no expansions on the unchanged three-rate few-mode ODE.
All 30 fixed candidate attempts fail their numerical screen. Priced switching
makes 15 failed candidate attempts and skips 15, reducing RHS calls from 960 to
720 and JVP calls from 1920 to 1785. Its median time is 1.115 times faster than
fixed causal forward. Exponential cooldown makes only three probes and is
1.173 times faster than the priced policy in this persistent-failure setting.
Both retain the protected solver and produce the same sampled error.

These two domains expose the trade-off: rapid re-entry helps when a successful
candidate domain emerges, while long cooldown helps when the configured basis
remains inadequate. The policy is an explicit opt-in choice. The appropriate
next extension is a cheap, source-bound structural indicator for persistent
rank inadequacy or a caller-specified re-entry horizon, with its own costs and
validation, rather than a case-name rule.

In the other five primary configurations, the priced and original policies
follow the same counted work and routing. Differences in their short-run
timings are timing controls; they are not attributed to the policy correction.
This distinction is machine-readable as
`priced_policy_changed_routing_or_counted_work` in `CAUSAL_BENCHMARK.json`.

## Strong protected controls still determine applicability

The fastest applicable protected implementation is compact-band in seven
configurations and dense in the clock-augmented model. Its time divided by
priced causal time ranges from 0.00104 to 0.483. These structured analytic
problems therefore support an optional projected route, not a general default.
The dense and compact-band controls receive explicit analytic Jacobian
capabilities; the projected and matrix-free controls receive an analytic JVP.
Their different resource contracts are part of the comparison.

In particular, the broad and changing-rate results make preconditioning and
inexpensive declared structure high-priority development targets. The expensive
protected raw-U GMRES route dominates those cases. Reducing unsuccessful
projection work alone cannot remove that underlying linear-solve cost.

## Parallel root schedules recover an expensive-callback domain

In the final injected-cost experiment, a reused two-worker pool makes fixed
causal Picard 1.237 times faster than its one-thread counterpart and 1.250 times
faster than serial causal forward. Residual-first root correction is 1.436
times faster than its one-thread counterpart and 1.318 times faster than serial
causal forward. Thus a concrete callback-cost domain survives reconsideration
of the earlier cheap-RHS exclusion.

Protected serial dense RODAS5P remains faster: its time divided by two-thread
Picard time is 0.604, and divided by two-thread residual-first time is 0.637.
Cheap-RHS and Riccati cases lose performance with two workers. The supported
use case is therefore an explicitly requested projected/matrix-free route with
expensive independent callbacks. The injected 2048 sine evaluations per RHS
call are a synthetic cost control, not a measured application speedup.

Pool construction occurs outside timed solves and the caller-owned pool is
reused. A one-shot application must additionally charge startup. The measured
residual-first path has a continuation-capable implementation, but this
experiment does not establish a speed benefit specifically caused by nonlinear
continuation subdivisions. Root correction and continuation are distinguished.

Five cyclic within-process repetitions provide local descriptive timings,
without an independent-process confidence interval. Reference errors are checked
at 17 common physical observations; they do not certify the intervals between
observations. All external VODE/DVODE, diffsol and Julia comparisons remain
outside this small experiment.

## Counter interpretation

The existing receipt field `fallback_attempts` counts every protected-route
attempt, including deliberate calibration and cooldown. The summary calls it
`protected_route_attempts`. For complete successful groups, subtracting
`candidate_skipped` gives protected attempts following a candidate trial;
the derived field is `post_candidate_protected_attempts`. An intentional
protected step is not described as a failed candidate.
