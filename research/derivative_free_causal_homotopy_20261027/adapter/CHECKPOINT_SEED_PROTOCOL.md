# Narrow repair registered before its execution

The original six-row `RESULTS.json` is retained. Its later same-checkpoint
entry pilots all restarted at Options.h0=1e-4 while the already observed
explicit starting checkpoint proposed a larger step. A pilot spent its finite
work allowance on repeated startup growth. This diagnoses the old B3 cold-seed
mechanism on the current code and original manufactured experiment.

Add default-false `seed_entry_pilot_from_explicit_checkpoint`. Only when an
already authorized same-checkpoint entry pilot is actually due, use the larger
of Options.h0 and the finite positive `record.initial_step`, clip to the replay
interval, and retain the existing native minimum-step proposal floor. The
native stepper still clips the attempted step to the endpoint. Do not use
`proposed_next_step`, which is measured after the explicit comparison prefix.
Do not change tolerances, pressure/entry thresholds, derivative permission,
attempt/callback budgets, the compared endpoint or acceptance.

Keep all original six arms and add one seeded selective arm at each of the
same rates 1,1000,10000. Same n16, initial state1, interval[0,1], 8 windows,
rtol1e-7, atol1e-10, h0=1e-4, max50000 attempts. Require every returned state
to satisfy the original analytic absolute-error cap1e-6; reconcile all RHS/JVP
entries including rejected and losing pilots. Preserve failures and overhead.
The two original arms must match their saved counters/states exactly.

Use targeted native tests for independent opt-in/defaults, same-checkpoint seed
identity (including first transient pilot), full accounting/cap/accuracy, and
Forbidden derivative permission. A private helper contract checks invalid
remembered proposals and independence from the endpoint's next proposal.
Perform no timing. Output the nine-arm result to REPAIRED_RESULTS.json and
its own receipt; do not overwrite the original result or command receipts.
