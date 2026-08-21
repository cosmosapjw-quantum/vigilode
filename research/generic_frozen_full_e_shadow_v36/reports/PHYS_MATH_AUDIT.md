# Independent physics/mathematics audit

## Verdict

`PASS_DESCRIPTIVE_ECONOMICS`

The independent audit reran the durable runtime checker and economics analyzer against the raw
artifacts. It also confirmed byte-identical regeneration of the derived JSON/CSV outputs and all
Python verifier tests.

Supported claims:

- the frozen `k=3`, `B_abs=80`, `delta=0.25`, and
  `tau_zeta=13.39706618860016` policy was consumed without retuning;
- all 127 v3.5 prefix-policy rows and all 64 preflight recommendations match exactly;
- 64 of 64 recommended retained level-2 objects were resumed exactly once and completed;
- unsafe recommendations, budget breaches, and continuation failures are all zero;
- deterministic R-JF attempt, accepted-step, and trajectory fields match v3.5 exactly after
  excluding only wall clocks;
- the all-event ledger is prefix/continuation/total = 2,669/1,130/3,799 JVP against 388,999
  committed R-JF JVP, for a realized total fraction of 0.976609%;
- the recommended-event full-E ledger is 2,586 JVP against 13,043 matched target-attempt R-JF
  JVP, or 19.8267%.

The two denominators above must not be conflated.

The wall campaign is descriptive only. All 35 measured pairs are retained, but N=384 is
host-noise dominated: R-only wall time spans 13.825×, shadow wall time spans 59.21×, and the pair
ratio spans 0.547–4.432. No robust speedup or performance-viability inference is supported.

Forbidden promotions remain: state/controller/output parity without emitted digests, active
switching, forced-switch recovery, fresh safety, release-wide completeness, and N=2048.
