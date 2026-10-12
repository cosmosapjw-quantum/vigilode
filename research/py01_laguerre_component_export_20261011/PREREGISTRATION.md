# Preregistration: per-term export of the Laguerre total bound (PY01)

Node PY01 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`docs/reviews/20261010_accuracy_speed/REVIEW_KO.md` §5, `MATHEMATICS_PORTING_KO.md` §6.3). Registered on branch
`audit/rvj-reaudit-remaining-20261011` (base `0ce7c32`). It depends on AS03, the fail-closed evidence validator
(`tools/evidence_schema_v2.py`, merged). PY02 depends on this node's recorded result.

**Claim boundary.**
- Attribution of an existing total only. No bound is changed or tightened, and no admission changes:
  `admit_laguerre_total`, `laguerre_adjoint_total` and `TotalErrorStatus` keep their current behaviour.
- No broad Laguerre campaign. R-NEXT-04 (L-0047) and PP08 (L-0071) are not rerun. Only the nine cases listed below are
  re-executed with instrumentation; the concrete new gap is PP09's G1 failure (L-0077: no component fields published).
- No routing, wall-time or default-promotion claim. The PP09 FAIL and the R-NEXT-04 PASS stand unedited; the PP09
  directory (a DAG edit target) receives at most an append-only cross-reference note after this node's recorded run.

## Question

R-NEXT-04 rejected six cases (h rho_enc = 48, degree 111-114, scale 16, L' = 16) whose Laguerre totals are 9.48e6 to
2.09e8 against budgets of 1e-6: a total/budget gap of 9.5e12 to 2.1e14. Its `cases.json` publishes only the total.
PP09's unregistered emulation put truncation + coefficient at 1e-18 to 1e-17 of each total and left the remainder
unsplit between recurrence adjoint, column summation and fused summation. Which term carries each total, when every
term is read from the native report of the same execution and none is inferred by subtraction?

## Change

1. **Entry point** in `crates/rodas5p-core/src/polynomial_action.rs`: an opt-in
   `joint_phi_action_laguerre_components(op, h, input, budget, work) -> CoreResult<(JointPhiReport,
   LaguerreComponentExport)>` on the existing `joint_phi_action_impl` / `joint_phi_action_core` path. Its report must be
   bitwise identical to `joint_phi_action(op, h, input, PolynomialBasis::Laguerre, budget, None, work)`, counters
   included. No existing function changes behaviour.
2. **`LaguerreComponentExport`**, all binary64 values as hex:
   - **Identity:** basis, branch, execution, input kind, degree, `laguerre_scale`, `beta`, `laguerre_extent` (L'),
     h, truncation budget, enclosure (lambda, rho, evidence), operator fingerprint, normalization shift, adjoint proof
     version (`LAGUERRE_ADJOINT_PROOF_VERSION`) and depth (`LAGUERRE_ADJOINT_DEPTH`), and per column the
     `EnvelopeKey` digest. `identity_sha256` is the SHA-256 of a canonical encoding of these fields, and every term
     block below is stamped with it.
   - **Terms per column k = 0..4:** the report's `ErrorComponents` fields `truncation`, `coefficient`,
     `recurrence_adjoint`, `summation` and `normalization`. `recurrence_majorant` is exported as a diagnostic only; it
     is not part of the total.
   - **Assembly:** `fused_summation` and the native `laguerre_adjoint_total` (`total_upper`).
   - **Adjoint sub-decomposition per column:** the stored coefficients `c_{n,k}` (n = 0..m, the midpoints the
     recurrence used), the envelopes `beta_j` from `laguerre_adjoint_envelopes`, the local residual bounds `eps_j`
     of the recurrence column used (`local[column]`), and `|s_k|`.
   - Every value is copied from the variable the action used; none is recomputed in the export.
3. **Component map, fixed now** (the DAG terms):
   - truncation = `truncation`; coefficient = `coefficient`; recurrence-adjoint = `recurrence_adjoint`;
   - summation = `summation` (the coefficient-weighted sum and scaling, and out of window the column rescaling
     rounding);
   - assembly = `fused_summation` plus `normalization` (input bits lost by the power-of-two normalization).

## Cases

The cases come from `research/rnext04_laguerre_admission_20261003/cases.json` (SHA-256 `700726f0…a01b7d`, pinned in
the checker). Each is rebuilt from the published bits of A, h and w:
- Same-vector inputs use v = w_0 and the scales [1, -0.5, 0.25, 2, -1] of the R-NEXT-04 source.
- The operator is `SymmetricNonpositiveOperator::new(A, 0, 1.2 rho (1 + 1e-12), ..)`, the R-NEXT-04 rule, with the
  nominal rho of the label, or rho = 50 for the two scale cases (from the R-NEXT-04 source).
- The truncation budget is 1e-10 and the admission budget the published `budget`.

**Gated:** the six PP09 cases `n{6,12,20}-rho400-h0.1-{distinct,same}`.
**Controls** (also gated, for the identity and assembly paths):
- `n6-rho400-h0.01-distinct`: admitted, degree 22.
- `subnormal-1e-310` and `large-1e300`: shifts -1029 and 997, the only published cases out of the window.

Nine executions in all; no other case is executed.

## Commands

    cargo test --offline --locked -p rodas5p-core --test laguerre_component_export
    PY01_COMPONENTS=research/py01_laguerre_component_export_20261011/COMPONENTS.json cargo test --offline --locked --release -p rodas5p-core --test laguerre_component_export -- --ignored --nocapture --test-threads=1 export_components
    python3 tools/py01_laguerre_component_check.py --published research/rnext04_laguerre_admission_20261003/cases.json --pp09 research/pp09_laguerre_envelope_20261004/RESULTS.json --components research/py01_laguerre_component_export_20261011/COMPONENTS.json --output research/py01_laguerre_component_export_20261011/RESULTS.json

**Contract tests.** The first command is the DAG command. Its contract tests:
- report parity with `joint_phi_action` on two fresh small fixtures;
- identity stamping;
- refusal, with a typed error and no export, of any case without a complete adjoint total: a declared (unverified)
  enclosure, or degree > 128.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first. Malformed or unbound evidence is INVALID (exit 2), a
gate failure is FAIL (exit 1), and a pass is PASS (exit 0). The checker and its self-test (synthetic records only) are
committed before the recorded run. The export and the check each run once.

## Gate

**Validity.** INVALID if any of the following holds:
- The AS03 rules fail: strict JSON, exact key sets, hex decoding, unique labels, or not exactly the nine cases.
- The binding fails. The published A, h and w bits must be equal. The run's degree, `laguerre_scale`,
  `normalization_shift`, fused bits and `laguerre_adjoint_total` bits must equal the published `degree`,
  `laguerre_scale`, `normalization_shift`, `fused` and `coverage.total`. Otherwise the decomposed total is not the
  published one.
- A term block's stamp differs from the `identity_sha256` that the checker recomputes from the identity fields, or two
  cases share an identity.
- The checker blob at the RUNS (COMPONENTS) source commit differs from the checker that was run, or any recorded tree
  status is dirty.

**PASS** if all of the following hold in all nine cases.
1. **Completeness.** All five terms are present for each of the five columns, plus `fused_summation` and the adjoint
   sub-decomposition: m + 1 coefficients, m + 1 envelopes and m residual bounds, with m the degree. Every term is
   finite and >= 0. No term is null or derived by subtraction.
2. **Enclosure.** The checker forms the upward sum in the action's order: `fused_summation`; per column truncation,
   coefficient and summation; every `recurrence_adjoint`; every `normalization`. It emulates the `directed.rs` rules
   in exact rational arithmetic, including the unconditional step below 1e-289.
   - The sum must be >= the native total.
   - In window (shift 0) it must equal the total bit for bit.
   - Out of window it must be at most total (1 + 1e-12) + (5 sqrt n + 64) 2^-1074, the R-NEXT-04 amended rule.
3. **Source binding of the adjoint.** For each column, the checker sums mul_up(beta_j, eps_(j-1)) upward over
   j = 1..m, applies mul_up(|s_k|, .) and, out of window, the exact 2^shift scaling rounded up. The result must equal
   the exported `recurrence_adjoint` bit for bit, and the `EnvelopeKey` digest must match the coefficients, extent
   and depth.

Everything else is **FAIL**, with every number preserved.

**Reported, not gated.**
- **Shares and bottleneck.** Per case, the share term / S of each component, where S is the exact rational sum of the
  terms, and the bottleneck. Also a comparison with PP09's derived truncation and coefficient values.
- **Per column:** max_j beta_j, the index j of the largest beta_j eps_(j-1), and the sampled lower bounds
  l_j <= sup_[0, L'] |z_j|.
  - l_j is the largest lower end of the `mpmath.iv` enclosure of |z_j(x)| (60 digits) over 4097 equispaced points of
    [0, L'], with the backward recurrence of `laguerre_adjoint.rs` on the exact stored coefficients.
  - The overestimate beta_j / l_j is reported.
- **Closability floor.** F = N + sum_k |s_k| sum_j l_j eps_(j-1), where N is the exact sum of the non-adjoint terms.
  - With the same residual bounds, no envelope of the same z_j gives a total below F, up to rounding. F is not a
    bound.
  - F is reported against the budget. It is an input to PY02's registered admission rule.

## Kill and hold

- **Missing components halt attribution.** An item-1 FAIL or an INVALID binding halts attribution. No Laguerre
  campaign, no PY02 stage 2 and no bound change proceeds on inferred components.
- **No re-scoring.** No threshold or component map is revised after the run, and a FAIL is not re-scored.

## Coordination with sibling nodes

PY01, PY02, PY03 and PY05 all edit `crates/rodas5p-core/src/polynomial_action.rs`. Their changes are merged in the
fixed order PY01 -> PY03 -> PY02 -> PY05, so this node's opt-in entry point lands first and the later nodes keep its
parity test. PY05 stays out of PY03's shared router. PY02's v2 envelope key must not change any v1 digest (including
the per-column `EnvelopeKey` digests this node exports) or PY03's cache-key tests.

## Prior information (disclosed)

- **R-NEXT-04.** The published totals and budgets.
- **PP09.** G2 envelope: max E_n = 710.47, factor 4.196. Derived emulation (not registered):
  - truncation 8.0e-11 to 9.4e-11 and coefficient 5.1e-11 to 1.6e-10;
  - the remainder equals the total to 12 digits;
  - normalization is 0 in the six cases.
- **Source state.** `polynomial_action.rs` has SHA-256 `a4ee9654…` as at L-0071; after L-0047 only the PP08 router
  module was added. `laguerre_adjoint.rs` has `5a4ddcb5…` as at L-0047. `directed.rs` changed since then only in a
  comment (`1cc041d`).
- **Nothing run.** No code of this node exists before this commit. No beta_j, eps_j or l_j has been computed.

## Predictions

- **Verdict:** the binding holds (unchanged Laguerre path), and items 1-3 hold: PASS.
- **Terms:**
  - Truncation and coefficient equal PP09's derived values to the printed digits.
  - Summation plus `fused_summation` is below 1e-12 of S.
  - `recurrence_adjoint` is >= 0.999999 of S in all six gated cases.
- **Not predicted.** The adjoint sum is carried by large depth-3 Bernstein envelopes beta_j. The size of beta_j / l_j,
  and whether F falls below the 1e-6 budget in any gated case, are not predicted: they are what PY02 needs.

## Results

Everything above this heading is the registered text, unchanged. Executed on branch `node/py01` from `8ce9bda` (the
registration commit); the registered text names `audit/rvj-reaudit-remaining-20261011` as the registration branch.
Only the branch name differs.

**Verdict: PASS** (evidence VALID). In all nine cases every term of the native Laguerre total is exported from the
variables the action used, the upward re-sum reproduces the total (bit for bit in the window), and every
`recurrence_adjoint` is reproduced bit for bit from its exported envelopes, residual bounds and `|s_k|`. Claim
boundary as registered: attribution only. No bound, admission, `TotalErrorStatus` or default changed; the R-NEXT-04
PASS and the PP09 FAIL stand unedited; no wall-time claim.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| Registration (text above) | `8ce9bda` | — |
| A: entry point, export types, contract and export tests | `0903621` | full digests below |
| B: checker and synthetic checker tests (before the run) | `f32ba9c` | `py01_laguerre_component_check.py` `7354101236af3db009b07dfb10e2b9155261df090d0641f5abba598e68e05829`; `test_py01_laguerre_component_check.py` `7b6cdcf802f0a1c2ca190aa3cdca2d7fb738afc7711c961937d383a76e4ccd12` |
| C: recorded run at `f32ba9c` (clean tree) | `0229b6f` | `COMPONENTS.json` `51e42d645224eb3c5d52ad1f844e4c7b24404fb4c6f6c935a55baa5b2824ad1f` (303,225 bytes, compact JSON); `RESULTS.json` `235dcd5c5266b33c8324d33f7987ee55ac21aa6d3e589f8ffcec103765555b18` (366,869 bytes) |
| Inputs, unchanged | — | R-NEXT-04 `cases.json` `700726f0bd2d58d84fbd1e42bf4daa66efd80312057021bc1e69515f28a01b7d`; PP09 `RESULTS.json` `c50789bc3fab65a4f600a329723d48e422c60f8162a9ff39cec0dbd63e318beb`; `evidence_schema_v2.py` `1d2bfe9276252b2079914e4fa958612ed06ffb04d1fecf9016bd4f370206ff10` |

Full digests at commits A/B (unchanged since):
- `crates/rodas5p-core/src/polynomial_action.rs`: `deddb9fe26e0d2a622721c65eae16e9c29353ef7e9d7fca841c93587131d2039`
  (was `a4ee9654…41caf9` at `8ce9bda`, as disclosed)
- `crates/rodas5p-core/src/laguerre_adjoint.rs`: `26b58115e8ad5fbe34ac549161661fec05006bb924ff5461efe89a31b925cb28`
  (was `5a4ddcb5…2ac6c`)
- `crates/rodas5p-core/tests/laguerre_component_export.rs`: `0485dafd971a998caf5aebf2c594b192c4d63cde75e27537ebfe50502a05281c`
- `crates/rodas5p-core/src/directed.rs`: unchanged, `0227b1c605b1ad399d8a9675d3bab7c665b25d3de9c9da8577a537e57183b045`

**How the run was made.** The three registered commands were run once each, in order, from a clean tree at
`f32ba9c`, with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, rustc/cargo 1.94.1, Python 3.11.15, mpmath 1.3.0.
- Command 1 (debug): 3 passed, 2 ignored, exit 0.
- Command 2 (release): `export_components` passed in 15.0 s of test time and wrote `COMPONENTS.json`; its recorded
  `source_commit` is `f32ba9c7e391848a008b9da0084538f4b7644fa8` and its `tree_status` (`git status --porcelain`,
  untracked files included) is empty.
- Command 3: exit 0 in 445 s wall clock (4 worker processes for the l_j). The checker recorded its HEAD
  `f32ba9c…` and an empty `git status --porcelain --untracked-files=no`; its blob at the source commit equals the
  checker that ran.

**Tests.** At A: `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -D warnings` clean;
`cargo test -p rodas5p-core --all-targets` passed (every test-result group ok, 0 failures, including the R-NEXT-04
`laguerre_admission_cases` and the thread-transfer Laguerre adjoint and cache tests). At B:
`python3 -m unittest discover -s tools -p test_py01_laguerre_component_check.py` passed 17 tests, normally and with
`-O` (a PASS fixture; 10 INVALID tests covering strict JSON, the pin, keys, duplicates and missing cases, hex,
seven binding mutations, the registered construction, stamps, shared identity and provenance; 4 FAIL tests covering
each item; the directed-rounding emulation and the l_j sampler).

No bug was found after the recorded run; there is no `*_FIRST` file and nothing was re-run.

### Validity

All conditions hold. AS03 rules pass (strict JSON, both pins, exact key sets, canonical hex, nine unique labels).
Binding: in all nine cases the A, h and w bits equal the published ones, and the run's degree, scale (16),
normalization shift, fused bits and `laguerre_adjoint_total` bits equal the published `degree`, `laguerre_scale`,
`normalization_shift`, `fused` and `coverage.total`. The identity follows the registered construction (including the
operator fingerprint recomputed from the published A and `rho = 1.2 rho_nominal (1 + 1e-12)`). Every term-block stamp
equals the recomputed `identity_sha256`; the nine identities are distinct. Provenance and both tree statuses are clean.
The export's parity record shows report and counters bitwise equal to `joint_phi_action` in all nine cases (reported).

### Gate

| Case | m | shift | 1 completeness | 2 enclosure | 3 source binding |
|---|---|---|---|---|---|
| n6-rho400-h0.1-distinct | 112 | 0 | holds | re-sum = total bit for bit (2.3116136212520286e7) | 5/5 columns bitwise, keys match |
| n6-rho400-h0.1-same | 111 | 0 | holds | bitwise (9.48444338140252e6) | 5/5 |
| n12-rho400-h0.1-distinct | 113 | 0 | holds | bitwise (2.917961222782702e7) | 5/5 |
| n12-rho400-h0.1-same | 112 | 0 | holds | bitwise (2.7944478761394933e7) | 5/5 |
| n20-rho400-h0.1-distinct | 114 | 0 | holds | bitwise (2.088653365857714e8) | 5/5 |
| n20-rho400-h0.1-same | 113 | 0 | holds | bitwise (1.0784292469364363e8) | 5/5 |
| n6-rho400-h0.01-distinct (control) | 22 | 0 | holds | bitwise (4.8584865445265526e-11) | 5/5 |
| subnormal-1e-310 (control) | 9 | -1029 | holds | re-sum 5.914e-321 = total 5.746e-321 + 34 x 2^-1074, allowance 64 + 5 sqrt 6 = 76.2 (+ total 1e-12) | 5/5 (2^shift rule) |
| large-1e300 (control) | 9 | 997 | holds | re-sum = total (5.725862808866298e289) | 5/5 |

### Attribution (reported, not gated)

Shares of S, the exact sum of the five DAG terms (assembly = `fused_summation` + sum of `normalization`):

| Case | truncation | coefficient | recurrence_adjoint | summation | assembly | bottleneck |
|---|---|---|---|---|---|---|
| n6-h0.1-distinct | 3.71e-18 | 3.46e-18 | 1.0 | 7.1e-22 | 1.7e-24 | recurrence_adjoint |
| n6-h0.1-same | 8.43e-18 | 5.33e-18 | 1.0 | 1.3e-21 | 7.8e-25 | recurrence_adjoint |
| n12-h0.1-distinct | 2.86e-18 | 4.30e-18 | 1.0 | 4.2e-22 | 6.7e-25 | recurrence_adjoint |
| n12-h0.1-same | 3.37e-18 | 2.84e-18 | 1.0 | 4.8e-22 | 5.7e-25 | recurrence_adjoint |
| n20-h0.1-distinct | 4.41e-19 | 7.83e-19 | 1.0 | 7.6e-23 | 3.3e-25 | recurrence_adjoint |
| n20-h0.1-same | 8.23e-19 | 9.25e-19 | 1.0 | 1.2e-22 | 1.8e-25 | recurrence_adjoint |
| n6-h0.01-distinct | 0.548 | 0.452 | 2.5e-4 | 7.2e-5 | 6.7e-6 | truncation |
| subnormal-1e-310 | 0.764 | 0.212 | 4.2e-3 | 0.017 | 3.3e-3 | truncation |
| large-1e300 | 0.784 | 0.216 | 8.9e-6 | 6.8e-5 | 1.6e-5 | truncation |

In the six gated cases the absolute truncation is 7.99e-11 to 9.42e-11 and the coefficient term 5.05e-11 to
1.64e-10, both equal to PP09's derived values to all 8 printed digits; `recurrence_adjoint` is 9.48e6 to 2.09e8.

**Where the adjoint sum sits.** In every column of the six gated cases the largest product beta_j eps_(j-1) is at
j = 1 (46 % to 60 % of the column's sum). There beta_1 = 4.8e20 to 3.1e21 (column 0), while the sampled lower bound
of sup_[0, 16] |z_1| is l_1 = 1.3863 (all six cases); eps_0 = 9.3e-15 to 4.1e-14. The overestimate beta_j / l_j at
the dominant j ranges from 3.1e16 (column 4) to 2.2e21 (column 0), and per column `recurrence_adjoint` exceeds its
sampled floor |s_k| sum_j l_j eps_(j-1) by 4.4e15 to 8.2e19. beta_j / l_j falls to 1.0 at the top index (z_m = c_m).
In the three controls (m = 9, 22) beta_j / l_j = 1.0 for every j: the depth-3 Bernstein envelope is sharp there.

**Closability floor** F = N + sum_k |s_k| sum_j l_j eps_(j-1) (not a bound):

| Case | F | F / budget (1e-6) | total / F |
|---|---|---|---|
| n6-h0.1-distinct | 1.670e-10 | 1.67e-4 | 1.38e17 |
| n6-h0.1-same | 1.314e-10 | 1.31e-4 | 7.22e16 |
| n12-h0.1-distinct | 2.099e-10 | 2.10e-4 | 1.39e17 |
| n12-h0.1-same | 1.748e-10 | 1.75e-4 | 1.60e17 |
| n20-h0.1-distinct | 2.589e-10 | 2.59e-4 | 8.07e17 |
| n20-h0.1-same | 1.911e-10 | 1.91e-4 | 5.64e17 |

F is below the 1e-6 budget in all six gated cases. With the same residual bounds eps_j, degree and scale, the
attribution therefore places the R-NEXT-04 rejections in the envelopes beta_j of the adjoint polynomials z_j, not in
eps_j, the degree or the e^{L'/2} terms. This is the input PY02's A1 and A2 read; it is not a bound and admits
nothing. (For subnormal-1e-310, F exceeds the native total, total / F = 0.976: N sums the scaled-back terms, each
rounded up separately into the subnormal range. F is reported only.)

### Prediction check

| Prediction | Outcome |
|---|---|
| Verdict PASS (binding holds, items 1-3 hold) | held |
| Truncation and coefficient equal PP09's derived values to the printed digits | held, 6/6 cases, both terms |
| Summation + `fused_summation` below 1e-12 of S | held (at most 1.3e-21 of S) |
| `recurrence_adjoint` >= 0.999999 of S in the six gated cases | held (share 1 - O(1e-17)) |
| Not predicted: beta_j / l_j and F against the budget | beta_j / l_j up to 2.2e21 at j = 1; F / budget 1.3e-4 to 2.6e-4 |

### Interpretations and deviations

Fixed in the checker docstring at commit B, before the run:
1. **Truncation budget.** The two scale controls ran at 1e-10 x amplitude (1e-320 and 1e290), the R-NEXT-04 source
   rule; the seven other cases at 1e-10. The literal 1e-10 would have executed those controls at another degree than
   the published one, which the registered binding rejects. The binding (degree, fused, total bits) held in both.
2. Item 2's 1e-12 is the exact decimal 10^-12 and the bound is evaluated exactly.
3. The "EnvelopeKey digest" is the key's `coefficients_sha256`, checked with its degree, extent bits, depth, proof
   version and limits.
4. Out of the window, item 3 emulates the action's `ExpBound::exact(v).scaled_pow2(shift).to_f64_up()` exactly.
5. A null, negative or miscounted term is an item-1 FAIL; malformed hex is INVALID.
6. The export records `git status --porcelain` with untracked files; the checker records its own status without
   untracked files (COMPONENTS.json is untracked when checked). Both were empty.
7. Shares use the five DAG terms; the PP09 comparison uses column sums at 8 significant digits.
8. l_j uses x_i = i L' / 4096, i = 0..4096, as point intervals; F is exact from the 60-digit l_j.
9. The export's parity record is reported, not gated (the contract tests gate parity).

Further disclosures:
- `joint_phi_action_core` now calls `laguerre_adjoint_recurrence_bound_with_envelopes`, which runs the same checks and
  the same upward sum as `laguerre_adjoint_recurrence_bound` (unchanged) and also returns the envelopes. The bound is
  bitwise the same; the full core test suite passes, and the six R-NEXT-04 totals were reproduced bit for bit.
- The test file has a second ignored test, `export_fresh_fixture_records`, which is not registered. Before the run, it
  was executed once on fresh fixtures (new matrices and vectors under the nine labels, h = 0.01, written to a scratch
  path) to check that the checker accepts the export format. No registered case was executed before command 2.
  Both ignored tests write only when their path variable is set, so the CI run of every ignored test writes nothing.
- The PP09 directory is not edited by this node; the optional cross-reference note is left to the integrator.
