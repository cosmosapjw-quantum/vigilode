# Timing Authority Validator — PHYS-MATH / PHYS-MATH-CODE Audit

## Review verdict

`PASS_IMPLEMENTATION_REVIEW_READY`

This is the single independent checklist review for PM-3. It reviews the
validator implementation and its retained retrospective evidence; it does not
promote any timing or speedup claim.

## PHYS-MATH audit

### Definitions and dimensions

- Wall time and proposed interval are finite positive scalars.
- `Gamma = wall_seconds / proposed_interval` is checked for every arm.
- Paired denominators must be exactly equal. A mismatch is retained as the
  named `proposed-interval` whole-campaign failure.
- Arm span is `max(wall) / min(wall)` and is dimensionless.
- Order sensitivity is the absolute difference of the two alternating-order
  medians of `shadow/R-JF`; it is insensitive to whether the common ratio lies
  above or below one.
- CPU idle and steal fractions use deltas of the sealed eight `/proc/stat`
  fields. Negative deltas fail closed rather than being repaired or clipped.

### Boundary and limit behavior

- Missing exposed thermal counters are represented by an empty mapping and do
  not pass or fail a nonexistent counter.
- Recorded thermal, swap, CPU, Git, binary, contract, affinity, and thread data
  are never inferred from historical v3.6 files.
- Historical host fields remain `NOT_RECORDED`.
- Three passing campaigns are required within at most five retained attempts.
- No result authorizes speedup or active switching.

### Counterexamples exercised

- favorable ratio mutation `0.6 -> 1.4` with unchanged quality geometry;
- missing measured pair;
- paired interval mismatch;
- low idle and excess steal;
- swap-in/swap-out and exposed thermal increments;
- R-JF and shadow arm-span spikes;
- order-dependent median split;
- each cross-campaign identity dimension independently changed.

No PHYS-MATH P0/P1 finding remains.

## PHYS-MATH-CODE audit

### Equation-to-code mapping

| Contract statement | Code path | Verification |
|---|---|---|
| sealed thresholds | `load_contract` | exact-value tests |
| CPU fractions | `parse_proc_stat_cpu`, `cpu_idle_steal_fractions` | arithmetic and negative-delta tests |
| immutable attestation | `capture_attestation` | mocked capture schema test |
| all-pair retention | `_load_profile_file`, `validate_campaign` | 35/35 pass; 34 retained on one missing row |
| exact paired interval | `_validate_pair` | retained mismatch test |
| whole-campaign gates | `_profile_quality_failures`, `_attestation_quality_failures` | threshold mutation tests |
| same identity across attempts | `_identity_failure_names`, `summarize_attempts` | seven named mismatch tests |
| three within five | `summarize_attempts` | 3/4, 2/5, and 6-attempt tests |
| atomic output | `atomic_write_json` | replacement and no-partial-output tests |
| v3.6 diagnostic | `generate_v36_retrospective_diagnostic` | deterministic 35-pair test |

### Reality and noninterference

- The module imports no VigilODE solver crate or Python solver wrapper.
- It launches no measurement binary and contains no paired-run command.
- Subprocess use is limited to source/toolchain/host identity capture.
- No shell execution, `eval`, `exec`, network operation, or dynamic code load is
  present.
- Real timing attempt directories are prohibited by the selftest.
- Only PM-3 research scripts, report, result, and receipt are changed.

### Ranked residual risks

- **P2:** `/proc/stat` `iowait` can be non-monotone on some kernels. The current
  implementation deliberately fails closed on any negative sealed-field delta;
  it does not fabricate a corrected sample.
- **P3:** measurement-profile evidence requires a binary under an adjacent
  `target/measurement` path. A future copied-binary workflow would need an
  explicit signed provenance field rather than weakening this check.

These are bounded operational limitations, not blockers for the validator
implementation.

## Final claim

The implementation is ready for remote PR review as a timing-authority
validator. It has not executed a timing campaign and does not establish a
speedup.
