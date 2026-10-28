# Adapter exact-return research protocol

Baseline: PR91 `206298b4068d923bb47c7160b111b7439098660c`.
Task scope: exact observed-work return decisions and practical probe overhead.
The existing strict-entry, checkpoint seed, fatal priority, original solver
tolerances and charged partial work remain dependencies.

## Alternatives fixed before the first native experiment

1. **Exact return with ordinary probes:** preserve existing scheduling and
   historical-rate trigger, use ratio one only for that trigger, then require
   a real same-window comparison. With explicit cost E, stop the implicit
   comparator after its callback prefix exceeds E. Compare checked integers.
2. **Exact productive sentinel:** the incumbent implicit advance completes
   first, providing work I for this window. The explicit challenger is bounded
   by I−1 and is accepted iff E<I. The productive incumbent remains available
   if the challenger loses. Existing sentinel interval one realizes this mode.
3. **Transfer the winning explicit probe's native session:** would preserve
   FSAL/controller/workspace after the return, but the current callback-scoped
   `Dopri5::with_session` cannot escape its lifetime. This requires a separate
   continuation API rather than copying a checkpoint that lacks FSAL identity.
   We will not represent state/step-only transfer as session reuse.

No fitted cost margin is introduced. Entry/return scheduling controls remain
explicit operational heuristics; callback work is an accounting objective, not
a universal CPU-time model. The productive-sentinel bound is proved separately.

## Frozen experiments

Use dimension16, span[0,1], rtol1e-7, atol1e-10, h0=1e-4,
max_steps100000 and eight observation windows. Cases: nonlinear cubic decay
(rate1 and1000), smooth stiff-to-easy nonlinear relaxation, and smooth
stiff-to-easy-to-stiff nonlinear relaxation. All are autonomous clock-augmented
when needed, supply only a JVP, and have independently evaluated exact states.
Policies: persistent RHS-only, PR91 seeded strict-entry, that policy plus exact
ordinary returns, PR91 plus productive sentinel, and exact productive sentinel.
All returned states are checked (absolute cap1e-6 on cubic cases,1e-5 on the
existing oscillatory-return family); every native and callback counter agrees.

Record raw successful and failed arms. Performance measurements are secondary
to the fixed accuracy/accounting contract. Short paired timing uses one binary,
same inputs and ABBA/BAAB order; profile selected actual trajectories with
Callgrind. Interpret instructions and wall time separately.

## Iteration budget

Initial implementation and initial fixed experiment are pass0. At most three
diagnosis-driven correction/re-experiment passes may follow; each records the
specific cause and changed dependency. No unreported parameter sweep is allowed.
Changes that only document or format already-run code do not consume a numerical
correction pass. Global production qualification remains a separate scope.
For the final CI closure, however, the two `collapsible_if` repairs below are
conservatively charged as correction3. This exhausts the task's three-pass
limit even though it changes neither policy nor numerical operations.

## Correction1, after the frozen20-arm experiment

All20 arms passed fixed accuracy/accounting but all selective arms made zero
switches. In the relaxing case, a completed pilot cost204 versus192 explicit
work; its four accepted steps averaged0.0025203, but later pilots restarted
from explicit proposals near0.00032. The mean of that already accepted pilot
prefix was discarded solely because it did not establish cheapness.

Add opt-in `reuse_observed_entry_step`: transport the latest nonfatal pilot's
finite positive mean accepted step to a later pilot as a proposal. The later
checkpoint state and native accuracy tests are unchanged; entry still needs
fresh strict work evidence. Incomplete zero-accepted/fatal pilots supply no
seed. No multiplier or predictive cost claim is introduced. Add16 diagnostic
arms (four cases×four selectivepolicies), preserve the original20 and original
two branch-coverage test failures. This correction addresses the observed
startup issue rather than adjusting accuracy or comparison thresholds.

## Correction2, private fixture initialization

The existing private telemetry fixture failed to compile after the new
`observed_entry_h` field was added. Initialize that field to `None`; no runtime
algorithm, tolerance, comparison or fixture trajectory changes. The 35 distinct
native contracts pass after this correction.

## Correction3, final CI lint closure

Final production CI identified two `collapsible_if` sites in `switching.rs`:
observed-step proposal storage and adaptive-entry configuration validation.
Collapse each nested conditional into a let-chain and rustfmt that file only.
Preserve the original left-to-right short-circuit evaluation, helper invocation
count, predicate order, mutation body and error text. Change no policy,
constant, fixture or acceptance criterion.

Re-run the same 35 native contracts and execute all 36 benchmark arms once
under the shared build/measurement lock. Compare every numerical state, time,
counter, segment decision and fixed option against the existing frozen results;
the missing default-false `reuse_observed_entry_step` field in the original
20-arm serialization is the sole permitted schema normalization. Elapsed wall
time is excluded from this correctness comparison. Reuse the existing Decimal90
oracle only after exact equality of its input trajectories is established.
Keep the old paired timing/profiles, source hash and measured executable hash
as historical pre-style-cleanup evidence; do not infer new timings from
numerically equivalent output.

Outcome: the repeated35 native contracts pass, and all36 final arms exactly
match every prior non-timing output field, including17,440 returned components.
`FINAL_AUDIT.json` records the comparison and oracle evidence reuse.
Two commands overlapped unrelated inherited-example CI edits; their actual
exit0/wrapper stale-source2 receipts are retained in `PASS3_RECEIPT.json`.
Their source qualification was repeated after the parent's freeze, with no
additional numerical correction. Final source and executable hashes differ
from the retained historical profiling identities and are reported separately.
