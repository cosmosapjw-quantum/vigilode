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
