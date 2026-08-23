# Timing Authority Validator — PHYS-MATH / PHYS-MATH-CODE Audit

## Review verdict

`PASS_FINAL_AUTHORITY_EVIDENCE_CLOSURE_REVIEW_READY`

This review covers the PM-3 validator implementation and retained retrospective
evidence. It does not promote timing, speedup, switching, release, or merge
claims. The bounded local audit finds no remaining P0/P1 defect in the reviewed
authority path; a fresh review of the published corrective head remains required
before merge.

## PHYS-MATH audit

### Definitions and dimensions

- Wall time and proposed interval are finite positive scalars.
- `Gamma = wall_seconds / proposed_interval` is checked for every arm.
- Paired denominators must be exactly equal; a mismatch is retained as the named
  `proposed-interval` whole-campaign failure.
- Arm span is `max(wall) / min(wall)` and is dimensionless.
- Order sensitivity is the absolute difference of the alternating-order medians
  of `shadow/R-JF`; it is independent of whether the common ratio is favorable.
- CPU idle and steal fractions are re-derived from deltas of the sealed eight
  `/proc/stat` fields. Negative deltas fail closed.
- The canonical authority-evidence digest uses primary measured quantities and
  recomputed consequences. Equivalent integer/float spellings, signed zero,
  pair-row ordering, or tolerated perturbations of redundant Gamma/ratio/fraction
  fields do not create a new campaign identity.

### Boundary and limit behavior

- Missing exposed thermal counters are an empty mapping and do not pass or fail a
  nonexistent counter.
- Zero-row and one-order-only interrupted profiles emit serializable `null`
  metrics plus named failures rather than NaN or an exception.
- Missing, duplicate, and unexpected pair indices remain retained evidence and
  reject the whole campaign.
- The campaign root and profile directory have exact regular-file layouts;
  unrelated files and symlinks are rejected before hashing or promotion.
- Historical host fields remain `NOT_RECORDED` and are never inferred.
- Three passing campaigns are required within at most five retained attempts.
- Distinctness requires unique normalized paths, complete campaign-tree hashes,
  and canonical authority-evidence hashes after fresh replay.
- No result authorizes speedup or active switching.

### Counterexamples exercised

- favorable ratio mutation `0.6 -> 1.4` with unchanged quality geometry;
- missing tail and middle rows, zero rows, one row, duplicate index, and
  unexpected index;
- paired-interval mismatch, boolean repetition count, non-integer declared
  pair counts, repetition/frozen-count mismatch, raw Cargo-profile drift, and
  calibration-arm drift;
- low idle, excess steal, swap-in/out, and exposed thermal increments;
- R-JF and shadow arm-span spikes and order-dependent median split;
- every cross-campaign identity dimension independently changed;
- repeated references to one campaign and minimally fabricated PASS objects;
- complete-shaped decisions whose parsed rows are replaced by invented stubs;
- complete decisions with boolean fields substituted by equal-valued integers;
- copies changed only by unvalidated profile notes, row ordering, JSON numeric
  spelling, or redundant derived fields;
- unrelated root files and frozen-policy drift;
- identical v3.6 inputs under different checkout roots;
- CLI summary of genuine campaigns and CLI rejection of a tampered complete
  decision with no partial output.

No PHYS-MATH P0/P1 finding remains in the bounded local audit.

## PHYS-MATH-CODE audit

### Equation-to-code mapping

| Contract statement | Code path | Verification |
|---|---|---|
| sealed thresholds and frozen policy | `load_contract`, `_load_profile_file` | exact contract and zeta-tau mutation tests |
| measurement protocol identity | `_load_profile_file`, `_validate_pair`, `_validate_arm` | Cargo-profile, calibration-arm, frozen-repetition, and exact declared-pair-count type tests |
| CPU fractions | `parse_proc_stat_cpu`, `cpu_idle_steal_fractions` | arithmetic, negative-delta, and redundant-spelling tests |
| immutable attestation | `capture_attestation`, `_attestation_structural_validation` | capture schema and strict type/counter tests |
| exact custody layout | `_validate_exact_campaign_layout` | extra-root/profile and symlink-safe regular-entry logic |
| all-pair retention | `_validate_retained_pair_rows`, `_load_profile_file` | tail/middle/zero/one/duplicate/unexpected cases |
| exact paired interval and formulas | `_validate_pair`, `_validate_arm` | mismatch, finite-positive, Gamma, ratio, and repetition tests |
| whole-campaign gates | `_profile_quality_failures`, `_attestation_quality_failures` | threshold mutation tests |
| canonical evidence identity | `_campaign_authority_evidence_sha256` and adapters | notes/order/numeric/derived-field equivalence tests |
| full decision replay | `_validate_campaign_decision_for_summary`, `_canonical_json_bytes` | complete-shaped fabrication, scalar-type substitution, and tampered CLI-decision rejection |
| same identity across attempts | `_identity_failure_names`, `summarize_attempts` | seven named mismatch tests |
| three genuine distinct within five | `summarize_attempts` | 3/4, 2/5, 6-attempt, duplicate-path/tree/evidence tests |
| atomic output | `atomic_write_json` | replacement and no-partial-output tests |
| v3.6 diagnostic | `generate_v36_retrospective_diagnostic` | deterministic 35-pair and checkout-parity tests |

### Reality and noninterference

- The module imports no VigilODE solver crate or Python solver wrapper.
- It launches no measurement binary and contains no paired-run command.
- Subprocess use is limited to source/toolchain/host identity capture.
- No shell execution, `eval`, `exec`, network operation, or dynamic code load is
  present.
- Real timing-attempt directories are prohibited by the self-test.
- The bounded closure changes only the existing PM-3 validator, tests, result,
  audit, and receipt surfaces; the total PR surface remains seven files.

### Review history and final closure

The first remote review found duplicate/fabricated promotion, interrupted-layout
exceptions/NaN, and checkout-dependent retrospective paths. R4 closed the latter
two and added shallow decision validation. The second remote review showed that
complete-looking parsed-row stubs could still be trusted and that unrelated
files could perturb whole-directory identity. The final closure replaces shallow
summary validation with a complete fresh replay and introduces an exact-layout,
primary-evidence canonical digest. Additional adversarial review then removed a
last representational loophole: redundant derived Gamma/ratio/CPU-fraction
spellings are recomputed rather than treated as independent evidence.

### Ranked residual risks

- **P2:** `/proc/stat` `iowait` can be non-monotone on some kernels. The validator
  intentionally fails closed on any negative sealed-field delta.
- **P3:** measurement-profile evidence requires a binary under an adjacent
  `target/measurement` path. A copied-binary workflow would require a new
  explicit provenance field rather than weakening this check.
- **P3:** campaign authenticity is content-based, not cryptographically signed.
  This is appropriate for the current single-user scientific workflow but is
  not a multi-party custody claim.

These are bounded operational limitations, not blockers for the PM-3 validator
implementation.

## Final claim

The final authority-evidence closure is ready for remote PR review as validation
infrastructure. It has not executed a timing campaign and does not establish a
speedup.
