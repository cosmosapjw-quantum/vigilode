# Switching integration audit

This note records the integration of `e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824` with `9b062e812b821a20dfcb40dfa0340a9df41e835e`. It describes source-level contracts. Execution results are maintained in the enclosing audit packet.

## Resolution of the two implementations

| Concern | Integrated behavior |
| --- | --- |
| Entry trial cost | Preserve the native callback-work ceiling. A completed trial still must satisfy the whole recorded-work comparison. |
| Stable negative entry evidence | `adaptive_evidence` uses typed adaptive memory and finite-age retry sentinels. It does not additionally apply the legacy entry cooldown or rate rearming. |
| Legacy policy | `evidence_driven` retains the native exponentially increasing cooldown, rate rearming, and lifetime trial count. |
| Regime relaxation | A substantial fall in work/time combined with growth of the next step schedules a new comparison on the next eligible prefix, rather than replaying the preceding mixed transient. The request survives intervening ineligible batches. |
| Return to explicit integration | Under the adaptive policy, the previous implicit rate is only a trigger for a bounded explicit trial. A complete explicit trial must then beat an actual complete implicit trial starting at the same committed checkpoint and ending at the same window endpoint. |
| Productive implicit comparison | When the return comparison favors implicit integration, its accepted endpoint is committed directly. The window counters, mean accepted step seed, and reported observations are updated; no duplicate solve is required. |
| Implicit runs | Preserve native coalescing to the next scheduled recheck and exact intermediate window landings. Preserve the mean accepted step as a later run's initial proposal. |
| First implicit run after entry | The adaptive candidate additionally enables `reuse_entry_pilot_step`: a completed, economical, nonfatal pilot supplies its finite mean accepted step only when larger than the user's `h0`. The baseline leaves this option off. The pilot's state and caches remain discarded. |
| Unresolved startup trial | The adaptive candidate permits one next-eligible retry per explicit tenure after the first incomplete, nonfatal pilot with zero accepted steps. A separate used flag prevents repeated immediate retries. |
| Statistics | Preserve the newer copyable native snapshots, static disposition strings, public `windows` count, and full charging of every discarded trial. |
| Very short tail | Clip configured proposals within native step bounds for explicit runs, implicit runs, and comparison replays. The driver performs the final exact endpoint clipping. |
| Extreme work rate | Saturate the return ceiling's rounding allowance as well as its float-to-integer conversion, avoiding an integer overflow at an infinite or exceptionally large observed work rate. |

The two entry scheduling rules serve the same purpose. Multiplying their cooldowns would give an undocumented product of waiting times and could defeat the adaptive sentinel. The integrated code selects one rule according to the explicit policy opt-in.

## Mathematical meaning of a successful comparison

Let \(W_E\) and \(W_I\) denote the nonnegative recorded work of the two trials on a common accepted physical interval, and let \(0<\rho<1\). For a complete admissible implicit candidate, an entry decision requires

\[
W_I\leq \rho W_E.
\]

For an adaptive return, both trials complete the same requested window and the decision requires

\[
W_E\leq \rho W_I.
\]

These are observed work inequalities. They do not assert equality of global error, future work, or elapsed time. The acceptance tests and tolerance vectors of the native methods remain in effect. The shared qualified-cost routine first checks evidence identity, cost units, and admissibility; the switching caller constructs both records from one saved physical checkpoint.

For entry trials, the explicit batch may include rejected attempts beyond its last accepted time. Their cost remains charged to the explicit side. Consequently, even the same-prefix comparison is a practical routing estimate, rather than an equivalence theorem between two globally accurate integrations. Endpoint accuracy is checked separately in the audit benchmark.

The callback work of an implicit trial is a lower bound on its full recorded work because the latter also includes linear solves. If accumulated callback work already exceeds \(\rho W_E\), completing that same run cannot establish the entry inequality. Stopping the run at that point is therefore a safe rejection of that particular candidate. One callback may cross the bound; the callback's weight bounds the overshoot, with a separate allowance for the existing declared-band verification. A DOPRI return trial is bounded after an attempted step and can overshoot by that step's callback cost. Neither bound is a prepaid bound on the entire integration.

## Error and state preservation

Every trial reduces the remaining global attempt budget. A discarded explicit return state is not published after a fatal implicit comparator. An incomplete same-window comparison cannot establish a return to explicit integration. A successful implicit comparator that loses the comparison remains charged; one that wins is useful accepted progress. Fatal callback classification takes precedence over work-ceiling classification in the native protected driver.

Coalescing preserves the native controller within each run, but restarting a later run from its average accepted step does not transfer the complete controller or linear workspace. This limits the scope of any performance claim about persistence.

## Contracts added for the integration

1. Change legacy entry backoff/rearming parameters by extreme factors while keeping adaptive entry enabled on stable rotation. The complete statistics and state must remain identical; this detects accidental stacking of the two schedulers.
2. Integrate a representable \(10^{-15}\) terminal interval with the legacy explicit batch path. It must reach the exact endpoint, preserve the constant-RHS solution, charge one attempt, and invoke no derivative callback.
3. Inject a fatal Jacobian failure into an adaptively bounded entry pilot. The solve must stop with complete accounting, no route switch, and an uncommitted failed implicit trial.
4. Test return-ceiling saturation for ordinary, overflowing, and infinite observed rates.
5. Seed the implicit routing state and integrate a representable \(10^{-15}\) terminal interval. The regression first reproduced a rejected initial proposal and zero progress; the corrected proposal reaches the endpoint with a successful committed implicit record.
6. Compare entry-step reuse enabled and disabled on a stiff forced problem. The first entry pilot must be identical, the next implicit proposal must equal the completed pilot's mean accepted step only when enabled, the physical endpoint must remain accurate, and every counter must equal the sum of recorded segments.
7. Compare entry-step reuse enabled and disabled on rotation where pilots lose. The complete statistics and accepted state must remain identical.

The prior native economy tests and joint adaptive tests are also retained. Their accuracy, state, and accounting assertions are not weakened. The stable-rotation comparison required a specific reassessment: the imported joint test claimed strictly fewer trials than its older baseline, but the newer native baseline already has exponential backoff and callback ceilings. A diagnostic test measured exactly three pilots, 886 attempts, and 5,338 RHS calls for both the newer native policy and adaptive policy. That assertion is now noninferiority, with identical accepted states, the original endpoint accuracy cap, typed negative-evidence observation, and full accounting still required. Stable rotation therefore establishes parity in this fixture, not a new speedup. The first failed test remains in the enclosing validation receipt.

The source integration's focused rerun passed ten adaptive contracts, seven evidence-switch contracts, and three new integration contracts. After adding the implicit tiny-tail repair, the four private return/comparison contracts passed. The final combined validation and timing receipts are owned by the enclosing audit packet; these focused runs are not substituted for them.

## Remaining decisions for measured evaluation

The adaptive policy is an opt-in candidate. It should be selected for problems where renewed negative evidence and stricter return comparisons offset their extra trial cost. Stable nonstiff problems can remain on the persistent RHS-only route; known stiff problems can use a supplied efficient implicit provider directly. An observed-work advantage alone does not justify selecting a slower wall-clock implementation. The enclosing benchmark compares actual wrapper-inclusive time, achieved endpoint error, attempts, callback counts, and productive versus discarded comparison work.

The first new optimization candidate is entry-step reuse. The entry replay already pays to discover accepted step sizes, but the unseeded implementation discarded this information and restarted the next implicit run at the user's original `h0`. The initial revision reused the mean accepted step; measured results then motivated restricting reuse to growth above `h0` and adding one bounded unresolved-startup retry. [REVISION_2.md](REVISION_2.md) gives the precise changes and their count bounds. These options leave method coefficients, error tests, physical state, and derivative-cache invalidation unchanged. Their effectiveness is empirical, so the benchmark includes separate ablations. The factory choice remains opt-in at the level of `adaptive_evidence`; default and legacy evidence configurations leave both options off.

The next substantial implementation target is a fully resumable implicit session. It requires an explicit contract for controller and workspace invalidation after a route change, provider change, or state update. The present integration deliberately preserves the measured coalescing optimization while keeping that larger change separate.
