# ORIGINAL_TARGET_BRIDGE verification record

## Source and host import

- Inspected delivery: `e77ec86376ca89850e18e99963992aeeb01055c2`
- Inspected tree: `44e5d68dcb91c8a167c987fa9c92a8134f866f87`
- Isolated branch: `research/audit2-original-target-bridge-20260830`
- Original checkout and other worktrees: preserved
- Newer source-branch delta: none at fetch time; no provenance rebind required

The required first host import/usage run was:

```text
cargo run --locked -p rodas5p-integrators --no-default-features \
  --example solve_stiff > solution.json
exit 0
```

Parsed result: `success=true`, `complete_output=true`, 93 internal steps, zero
output-clipped steps, 106 output states, maximum absolute error
`2.4396448194963227e-9`, and `audit2_correction_used=false`. The JSON SHA-256
is `ba335358ceac9d1c8f23e4507f613af99c471a6646b10f4600ce3cf4deee60ed`.
This is a narrow import/usage check, not a production or general accuracy claim.

## Red-green and bounded review evidence

- The sign contract first failed to compile because
  `audit2_original_residual_bridge` did not exist; it passed after the pure
  bridge helper was implemented.
- The whole-bridge contract first failed to compile because the opt-in entry
  and result types did not exist; it passed after the research entry was wired.
- An original-action injection exposed a signed-zero fixture mismatch. The
  test reconstruction was changed to reproduce the actual strict-lower
  stage-mix loop exactly; production code and criteria were not altered.
- The one fresh review initially returned P1=1 and P2=2. Its three findings and
  the single bounded repair are recorded in
  `original_target_bridge_fresh_review_disposition.md`.

## Final commands and exits

```text
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts -- --nocapture --test-threads=1
exit 0; 15 passed, 0 failed
post-repair log SHA-256:
528857339e94a81e5c6a2a9706be40befe777dbe2d7d884dbf42b74f7cc3f923

bash tools/check-audit2-readiness.sh
exit 0
20 Python tests; 54 Rust tests; feature-off example; format and affected clippy
readiness log SHA-256:
24b39d35d655b6f1508c937decab6bad1ea85677796914a73774037a8890acfe

jq -e . original_target_bridge_results.json
exit 0
jq -e . original_target_bridge_raw_sample.json
exit 0

post-repair log versus 12 compact rows, fixed raw sample, and four work profiles
exit 0; all exact comparisons true after ignoring only zero-valued compact counters
```

The readiness set consists of 12 CI-scope Python tests, 8 research-contract
Python tests, 8 global-error Rust tests, 9 accuracy Rust tests, 15 bridge Rust
tests, 15 dense-output Rust tests, 6 homotopy Rust tests, and 5 usage Rust tests.
No full-workspace run or historical campaign is claimed.

The initial readiness attempt is retained as a real failure: affected clippy
reported a large enum variant, needless borrows, test type complexity, and a
range-loop lint. Those issues were fixed before the one fresh review; no test
threshold or scientific criterion was changed.

## Result-independent rules and results

The case grid, mass/nonnormal fixture, `64*eps` structural rule, bridge sign,
`4096*eps` backward-error ceiling, `8192*eps*condition_f` state-agreement
ceiling, and raw sample coordinate `(n=4,h=0.01)` were fixed independently of
outcomes. No value was fitted or widened after inspection.

Across the 12 inherited grid cases and one inherited mass/nonnormal case:

- maximum bridge identity L2 error: `0`;
- wrong-sign mutant gap: `26.907248094147423`;
- maximum common-W original-target backward error:
  `2.0497995971367258e-13`, below fixed `4096*eps` =
  `9.094947017729282e-13`;
- maximum original-target condition estimate: `2686.797650783813`;
- maximum condition-aware same-target relative correction difference:
  `2.7388464792698878e-11`; every row passed its fixed
  `8192*eps*condition_f` ceiling;
- maximum original/projected residual difference L2:
  `5.839915487267741e-16`;
- maximum output projection absolute difference L2:
  `2.715936613215248e-16`;
- maximum embedded projection absolute difference L2:
  `7.99715048305193e-17`.

The last two values are observations only and are not acceptance criteria.
All 13 rows are `BudgetNotSpecified`; estimate-only uncertainty cannot produce
categorical accuracy admission.

## Unchanged inputs and validity

The protected paths `fixtures/`, `research/scientific_validity_v2_20260829/`,
`crates/rodas5p-core/`, and `Cargo.lock` have no diff from the inspected
delivery. The 54 historical rows and their checksums were neither changed nor
regenerated.

- `RESULT_VALIDITY`: limited to the 13 declared small explicit systems and the
  specified linearized diagnostics.
- `PROVENANCE_VALIDITY`: bound to the inspected delivery and eventual stacked
  PR diff; the publication comment supplies the actual head/tree.
- `PACKAGING_VALIDITY`: remains pending until the branch is pushed, a draft
  stacked PR is open, and its live checks/synchronization state is recorded.

Claim ceiling: `EXPLORATORY_NONAUTHORITATIVE` original-target compatibility
diagnostic only. No nonlinear/output accuracy, production activation,
scalable backend, timing, ranking, speedup, holdout, freeze, PM-7, K0, tag, or
release claim is authorized.
