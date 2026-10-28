# Second revision: startup evidence and initial-step reuse

The first benchmark revision identified two different losses. A completed entry pilot could have a mean accepted step below the user's original proposal; replacing the proposal with this smaller value added work on the fast-transient fixture. Separately, an initial implicit pilot that accepted no steps was treated as ordinary negative economic evidence, delaying the next useful comparison after the explicit method had already advanced past the difficult startup region. The measured first-revision source and tests are preserved under `research/adapter_integration_20261027/revisions/r1/`; the enclosing benchmark packet retains its outcomes.

The second revision applies two bounded changes.

## Growth-only proposal reuse

Let \(h_0\) be the user's initial proposal and \(\bar h=(t_b-t_a)/n_a\) the completed economical pilot's mean accepted step. The next implicit run receives

\[
h_{\mathrm{proposal}}=\max(h_0,\bar h)
\]

when the pilot is successful, nonfatal, economical, and \(\bar h\) is finite and positive. Otherwise it receives \(h_0\). The configured value is clipped within native input bounds and the driver clips the actual step to its endpoint. This changes only the initial proposal: the accepted explicit checkpoint, derivative-cache invalidation, and native acceptance tests are unchanged.

In particular, a pilot mean smaller than \(h_0\) no longer introduces an extra startup reduction. This is a scheduling property, not a guarantee that the larger proposal will be accepted or be fastest; the driver can reject it normally. `reuse_entry_pilot_step=false` remains the direct ablation.

## One unresolved-entry retry per explicit tenure

An incomplete nonfatal pilot with zero accepted steps provides no successful implicit prefix on which to compare the methods. Under `retry_unresolved_entry_once`, the first such loss requests a new trial on the next eligible explicit prefix. The new trial starts from an already accepted later physical checkpoint. The typed reason is `UnresolvedEntryRetry`.

The request is sticky across ineligible batches. A separate boolean records that this explicit tenure has already allocated its one immediate retry. Actual comparison consumes the pending request; another incomplete pilot cannot rearm it. Ordinary finite-age sentinels and adaptive backoff remain available. A completed but uneconomical pilot does not qualify for this exception. Fatal errors still terminate the solve.

The special retry continues to require explicit work pressure, a positive accepted prefix, remaining global attempts, the normal callback-work ceiling, and the same-checkpoint comparison. It does not itself select an implicit route.

The count bound is direct: the tenure starts with `zero_progress_retry_used=false`; allocating a request changes it to true; no loss, elapsed timer, or scheduling observation changes it back. Thus at most one comparison in that explicit tenure can receive this immediate-bypass reason. Its attempt count is bounded by the smaller of the configured pilot budget and the remaining global attempt budget. This is a bound on the special bypass count, not a claim that total trajectory work differs by at most one pilot, because an earlier route decision may change all later work.

The default and legacy evidence factories leave this option off. The adaptive factory enables it; `retry_unresolved_entry_once=false` provides its separate ablation.

## Verification contracts

The new integration fixture supplies a recoverably failing Jacobian so that every implicit entry pilot has zero accepted steps. It checks exactly one immediate retry, the later accepted explicit start checkpoint, equality of the complete accepted explicit trajectory with the retry-disabled run, an exhausted global attempt budget, and all recorded-work totals. A private state-machine fixture repeats the zero-progress comparison 100 times and verifies that the immediate retry cannot rearm. Another private test covers growth-only seeding at smaller, equal, larger, zero-accepted, and unsuccessful pilot boundaries.

The existing fast-transient relaxation test now explicitly disables this new startup exception so that it continues to exercise the independent relaxation-memory mechanism. Its accuracy, next-prefix identity, work-reduction, and accounting assertions remain. Default-policy performance and correctness are evaluated separately by the enclosing benchmark.

These are the source changes and intended contracts. The enclosing validation and benchmark receipts determine which measured cases justify retaining the candidate.
