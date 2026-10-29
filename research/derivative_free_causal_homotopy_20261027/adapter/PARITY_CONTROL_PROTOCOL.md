# One post-diagnosis cost-parity control

This control is selected **after** the original six-arm experiment and the
nine-arm checkpoint-seed repair. In the repaired experiment, the later JVP
pilots complete the same qualified window at work160 versus explicit192,
ratio5/6. The unchanged default entry ratio0.8 requires a20% work margin and
rejects those pilots. The original failed/completed results remain retained.

The existing validator requires `entry_cost_ratio<1`, so the additional fourth
arm uses `f64::from_bits(1.0_f64.to_bits()-1)`, exactly the largest represented
value below one (`0.9999999999999999`), and checkpoint seeding. This is the
neutral strict current-window cost criterion
`C_JVP < C_DF`, not a fitted numerical-accuracy threshold. The existing
conservative comparison establishes actual qualified same-checkpoint work
preference for the tested small integer work counts; it does not guarantee
future savings. Validation and defaults remain unchanged. Its callback ceiling also
uses this ratio, so any additional probe cost must be charged.

All equations, rates1/1000/10000, n16, y0=1, [0,1], 8windows,
rtol1e-7, atol1e-10, h0=1e-4, max50000attempts, and analytic all-returned-state
error cap1e-6 remain fixed. All old nine arms stay in the same example binary.
Compare every existing field of each saved nine-arm row after execution.

This is an explicitly post-diagnosis policy control, with no holdout or claim
of an optimal/general threshold. Default production policy remains0.8 and
checkpoint seeding remainsfalse. Run one bounded twelve-arm deterministic work
experiment, retain success/failure and whole-trajectory cost including losing
pilots, and perform no timing. Stop parameter exploration after this control.

Outputs: `PARITY_RESULTS.json`, `PARITY_SUMMARY.json`,
`PARITY_EXECUTION_RECEIPT.json`; the executed nine-arm source is
`selective_jvp_seeded_executed.rs` with the hash in `SEED_BUILD_RECEIPT.json`.
