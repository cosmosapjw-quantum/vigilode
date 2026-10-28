# Candidate-free pre-execution verification

## Published controller

```text
base PR/head/tree       #39 / 6b00a886c4eb38d3fe199e3d77852cc1eb35eb39 / 4a9ede5c442514f1ae86d018419a2afeee5b6d01
controller C1           b5f553b5be24598c71bc7af15c97e67f503610a5
controller C1 tree      7e96da6f612eade18acc77b700b6e847d68160c9
Draft PR                https://github.com/cosmosapjw-quantum/vigilode/pull/40
runner SHA-256          f53f5bc2ea77721adc562c2640a58d24ae975f14795f7401c750c900c2980f29
adjudicator SHA-256     28697e81ea39532a2ffa86789a426c0f0a7107a1ea1a1a4269bc333cc8cf977d
candidate executions    0
canonical state root    PRESENT_WITH_PRESERVED_CANDIDATE_FREE_RUN
one-shot marker         ABSENT
```

The remote C1 tree equals the locally verified control-commit tree. C2 binds
the handoff documents only and must not change either C1 script hash.
The PR was created open, Draft, unmerged, with base branch
`research/audit2-real-client-authority-construction-20260830`.

## Candidate-free tests

The final C1 local replay used Python 3.12.13, NumPy 2.3.5, mpmath 1.3.0,
Rust/Cargo 1.94.1, and offline Cargo. Its complete output is retained outside
the repository in `final-readiness-logs.O7HC3Y/readiness-final2.log`.

```text
Python scope/policy/authority/receipt       36/36 PASS
guarded runner contracts                    25/25 PASS
independent adjudicator contracts           61/61 PASS
Python total                               122/122 PASS
fair-ab Rust contracts                      17/17 PASS
research Rust contracts                     58/58 PASS
Bateman authority Rust contracts             6/6 PASS
default-example unit contracts               5/5 PASS
Rust total                                  86/86 PASS
exact Bateman example                        COMPILED/CHECKED_ONLY
clippy -D warnings                           PASS
cargo fmt --check                            PASS
git diff --check                             PASS
```

The exact Bateman example was not run. The only executed example was the
pre-existing analytic `solve_stiff` usage contract with
`audit2_correction_used=false`; it is not Bateman scientific evidence.

## Preserved readiness-harness failure history

The first complete readiness attempt stopped inside the new fake-runner unit
suite. Readiness had supplied `CARGO_TARGET_DIR`, and the tests inherited it,
so the runner correctly treated the ambient build override as unresolved.
No Cargo/Rust tests, production runner, one-shot marker, or Bateman candidate
had started. The tests were isolated from the outer CI environment, and the
single complete retry passed. This was a test-harness isolation repair, not a
scientific criterion or threshold change.

## Preserved published-controller preflight failure

The first published controller invocation used commit
`50bb14f2f538846be26d29fe6afea1731a8fcdd1`, tree
`e745c0e65576448ddc98416d1e2aa24f4bb54ae8`, and the then-frozen exact
implementation-origin checkout. It stopped at `rust_authority_contracts`
before the one-shot boundary because commit `cac7d1b...` contains a Rust
`include_str!` contract for two research documents that are first present in
its direct documentation-only child `6b00a886...`.

```text
sealed package            /root/.local/state/vigilode/bateman-local-six-case-v1/runs/20260831T044826Z-bd0730fa
runner verdict            INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED
candidate invocations     0
validator attempts        0
attempt/launch/result      ABSENT / ABSENT / ABSENT
one-shot marker           ABSENT
execution_manifest SHA    57c63e0b3d81711926c81b152e916e5e9f111e6d86710a3480fac063d70b6520
events SHA                229db5951fecb0b59563d7c8fee82203954f32cb08c4db3b8a97c21222fc6d68
SHA256SUMS SHA            bc976e12052f2adac89a1b7eab34ef7171520e90541f1e7cab0a292e845fe9af
old adjudication verdict  INCONCLUSIVE_PACKAGE_INTEGRITY
old adjudication SHA      96dd02efae7532fc0a52c6261e4459f408187d66d7f093aa9c74cbe434464112
```

Every entry in the preserved package's `SHA256SUMS` verifies. Its immutable
historical adjudication sidecar records `INCONCLUSIVE_PACKAGE_INTEGRITY`,
exposing the old adjudicator's candidate-artifact ordering defect. No
retroactive adjudication or replacement verdict is claimed for that old-schema
package. `INCONCLUSIVE_NOT_RUN_PREFLIGHT` applies only to candidate-free failed
prefixes emitted and adjudicated under the repaired schema. Neither outcome is
a scientific rejection.

The result-independent source-binding amendment now requires execution source
head/tree/parent `6b00a886...` / `4a9ede5...` / `cac7d1b...`, separately
retains scientific implementation origin/tree/parent `cac7d1b...` /
`c23abbee...` / `f954e391...`, and adds the two compile-time document hashes.
The `cac7d1b..6b00a886` delta is exactly 13 documentation paths; `crates/`,
`tools/`, Cargo, toolchain, workflow, and frozen authority bytes are unchanged.
The one-shot key remains
`799d2f31e0fcd3e255a1be55c27d0387d798851a36bee75f04710929fa3c3852`.

## Initial bounded protocol review repair

One fresh protocol review found no P0 and four P1 classes before any candidate
observation: caller-selectable state root, validator infrastructure becoming a
scientific rejection, incomplete command/event verification, and missing
runner/binary/toolchain binding. The single bounded repair added:

- passwd-home account-global `O_EXCL` one-shot state with symlink and
  concurrent-contender tests;
- explicit validator infrastructure inconclusive outcomes;
- exact argv/cwd/environment/stream/event/source/authority verification;
- sanitized environment, executable realpath+byte identity, candidate binary
  pre/post identity, and published runner self-hash;
- eligible-only mathematical coverage and a true `sealing_started` event.

The final repaired runner passed 25 contracts, and the adjudicator passed 61,
including candidate-free integration against the actual frozen receipt
validator, a full canonical synthetic receipt, exact failed-prefix artifact
closure, trusted-host executable selection, and canonical run-specific build
target checks. The final independent protocol review found no blocker.

The final two adjudicator tests make the synthetic fixture CI-Python-version
independent while retaining the real `3.12.13` rejection gate: only the fixture
models the frozen `platform.python_version`; a direct guard test proves a
`3.13.15` runtime remains inconclusive before validator execution.

The later source-binding amendment above was triggered by a sealed
candidate-free infrastructure failure, not by a second fresh review or any
candidate result. It changes no threshold, scenario, scientific byte, or
claim.

## Frozen limitations

- The account-global marker constrains the published controller; it cannot
  prove that no out-of-band command was executed.
- The changed-W case is a cache/binding probe, not changed-W output accuracy.
- The compact receipt cannot independently reconstruct omitted embedded or
  original-target raw vectors.
- The manifest IEEE-754 bits, not ledger decimal rendering, are the admission
  authority. An `ACCEPT` is limited to six frozen contracts and is not the
  stronger `Ehat + Theta`, `sum_i a_i q_i`, per-solve-GMRES certificate.
- `M04`–`M12` and `X02` remain `NOT_EVALUATED`.
- Wolfram/xAct, SageMath/Singular, Lean/mathlib, and Rocq are outside the
  admission chain for this empirical run.
- SHA-256 binds bytes but is not host attestation.

The claim ceiling remains
`EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`.
