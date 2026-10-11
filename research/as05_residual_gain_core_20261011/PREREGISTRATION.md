# Preregistration: porting the current-operator residual gain witness to core (AS05)

Node AS05 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(finding F105 and section 3 of `docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`; MATHEMATICS_PORTING_KO.md
sections 2, 3.1-3.4 and 5). Executed on branch `audit/rvj-reaudit-remaining-20261011`, registered on top of `0ce7c32`.
It depends on:
- AS01, the overflow-safe fallback (merged in `0283c1f`);
- AS03, the fail-closed evidence validator (merged in `d0c5722`).

**Claim boundary.**
- **What a PASS says.** A `Verified` result encloses the forward error of one linear solve `W u = b`, in a frozen
  diagonal metric, for the operator target it is bound to. Nothing more.
- **What it does not say.**
  - No stage-contamination, local or global ODE error claim, and no fifth-order claim (that is AS06/AS07).
  - No generic matrix-free claim from the 2x2 and small dense tests. A JVP-only operator without a declared structure
    and a declared assembly bound gets no `Verified` result.
  - A projected Hessenberg `nu` is never used as a full-space inverse-norm upper bound.
- **No integration.** No driver caller, no default change and no production promotion. No wall-time claim; the flop
  counts are counted operations, reported only.
- **Earlier results stand.** The re-audit's 30-cell PASS and its `FIRST_FAILURE.md` record are not edited or
  re-scored.

## Question

`research/reaudit_accuracy_speed_20261010/gain_witness.rs` (`GainWitness2`) encloses `W^-1 (b - W x)` for a
represented real 2x2 `W`, with research-local one-ulp-widened intervals. Its contract excludes:
- W formation;
- JVP/assembly uncertainty;
- off-block coupling;
- metric change;
- stale operators.

Can a core module do three things?
- Reuse the reviewed directed primitives (`directed.rs`, and `symmetric_part_upper` of `nonnormal_certificate.rs`).
- Keep the exact-binary oracle result.
- Bind `J`, `h`, `gamma`, the metric, the model epoch and a declared assembly uncertainty into the target identity,
  so that each new negative is refused rather than silently certified.

## Change

1. **`crates/rodas5p-core/src/residual_gain.rs`** (new; exported from `lib.rs`).
   - `GainEvidence {Verified(VerifiedCorrection), Estimated(EstimatedCorrection), Unavailable(WitnessRejection)}`.
   - **Authority types.** `VerifiedCorrection`, `ResidualGainWitness` and `LinearTargetId` have private fields and no
     public constructor other than the witness API. They have no `Deserialize` implementation (`Serialize` is
     allowed, for export only). `EstimatedCorrection {estimate, source: EstimateSource}` is plain data, with sources
     `ProjectedNu`, `DenseLuComparator` and `UndeclaredAssembly`.
   - **`LinearTargetId`** binds:
     - the operator kind: `Represented(W)`, the exact real matrix of the binary64 entries; or `Shifted {J, h, gamma}`,
       the exact `I - h gamma J`;
     - a SHA-256 of every entry's bits, together with the `h` and `gamma` bits;
     - the metric `s` bits (`FrozenMetric`: positive and finite);
     - `model_epoch: Option<u64>`;
     - `AssemblyUncertainty {Exact, EntrywiseAbs(delta), Undeclared}`;
     - `DeclaredStructure {Dense, IsolatedBlocks(ranges)}`.
   - **Witness routes.**
     - **(a) `represented`, dense, n = 2.** The research algorithm, ported to `directed::Interval`.
     - **(b) `IsolatedBlocks`.** Blocks of size 1 or 2. Every off-block entry must be exactly zero, otherwise
       `OffBlockCoupling`.
     - **(c) `shifted`.** `W` is enclosed outward from `J` (widened entrywise by `delta`), `h` and `gamma`. The
       residual is recomputed with the interval `W`.
     - **(d) `lognorm`, dense, n <= 8.**
       - Requirements: `a = h gamma >= 0` (enclosed); `mu_up >= mu_2(D J D^-1)` for every `J` in `J +- delta`, from
         `symmetric_part_upper`; and `1 - a mu_up > 0`, checked outward.
       - Bound: `||u* - u||_WRMS <= ||D r||_2 / (sqrt(n) (1 - a mu_up))`, with `D = diag(1/s)`.
     - The approximate-inverse defect route (`q < 1`) is not implemented in this node. It returns
       `Unavailable(RouteNotImplemented)`.
   - **`certify(&self, target, rhs, candidate)`.**
     - It never accepts a caller-supplied residual: the residual is always recomputed.
     - It returns `Unavailable` on any mismatch between the target and the witness's own binding:
       `OperatorMismatch`, `TargetMismatch` (a represented witness asked for a shifted target), `MetricMismatch` or
       `StaleEpoch`.
     - With `Undeclared` assembly it returns `Estimated`, never `Verified`.
   - **`VerifiedCorrection::transport_to(new_metric)`** is the only metric change. It multiplies the bounds by
     `kappa = max_k s_old,k / s_new,k`, computed outward, and rebinds the target.
   - **Counts.** The witness counts its directed-primitive calls as setup flops and verification flops.
2. **`directed.rs` and `nonnormal_certificate.rs`.** Only additive helpers, for example an interval dot product, or
   `metric_matrix` made `pub(crate)`. No existing function changes behaviour.
3. **`gain_witness.rs`** stays byte-identical (SHA-256 `22c97719...`). It binds the recorded `NATIVE.json`, so
   editing it would rewrite history. This deviates from the DAG's edit-target list, by design. The new test includes
   it unchanged (`#[path]`) as the parity reference.

## Cases

**A. Retained oracle.** The exact inputs of `gain_witness::study()`, through route (a):
- 24 discovery cells, 2 controls and 4 fixed holdout cells;
- the 12 invalid controls.

**B. Parity.** Core against research intervals on the 30 cells A. The intervals compared are the residual,
correction and weighted correction intervals, and the scalar gain.

**C. New negatives.** Every one must give no `Verified` result.
- **W formation (4 fixtures).**
  - The family: `J = [[lambda, K], [0, lambda]]` with `gamma` the RODAS5P `gamma` of `rodas5p_coefficients()`,
    `K in {0, 1024}` and `h` from the ordered list `0.1, 0.3, 0.7, 1.1, 1.3, 1.7, 1.9, 2.3`.
  - `lambda = fl((1 - 2^-40) / fl(h gamma))`. The candidate is the exact solution of the represented
    `fl(I - fl(h gamma) J)`, rounded.
  - The fixtures are the first 4 members (in list order, `K` inner) that discriminate. Discrimination is decided by
    exact arithmetic only: a member discriminates iff the exact shifted-W error (Fraction oracle, exact rationals of
    `J`, `h` and `gamma`) lies outside the research `GainWitness2` enclosure of the represented `W`, as computed by
    the unchanged `gain_witness.rs` (SHA-256 `22c97719...`) and exported to RUNS.json as hex endpoints. No
    floating-point comparison and no core interval enters this decision.
  - The test certifies a represented witness against the `Shifted` target, which must give `TargetMismatch`.
- **Assembly.** The 4 W-formation `J` with `Undeclared` assembly must be `Estimated`.
- **Metric change.**
  - For each of the 30 cells A, two new metrics: `(2 s0, s1/2)`, and `(next_up(s0), s1)`.
  - Certifying with a mismatched metric must give `MetricMismatch`: 60 negatives.
- **Stale epoch and operator change.**
  - A witness at `model_epoch = Some(7)`, certified against `Some(8)` and against `None`.
  - Under the same epoch, a one-ulp change of each of the 4 `J` entries, of `h` and of `gamma` on each W-formation
    fixture (`OperatorMismatch`).
  - 30 cells x 2 epoch cases (60), plus 4 fixtures x 6 changes (24).
- **Off-block coupling.**
  - 4x4 with blocks `{0,1}, {2,3}`; each of the 8 off-block positions set to `2^-1074`, `1e-300` and `1.0`: 24
    cases.
  - A dense 4x4 asked for route (a): `UndeclaredStructure`.
- **Projected nu.**
  - `W = [[1, K], [0, 1]]`, `K in {1024, 2^20}`, Arnoldi from `e1`. The `nu`-based estimate must be `Estimated`.
  - The oracle records that it underestimates the true error for `r = e2`, the F105 example.

**D. New positives** (oracle enclosure).
- The 4 W-formation fixtures through route (c) with `Exact` assembly.
- The same with `EntrywiseAbs(delta)`, `delta = 2^-60 max|J|`. This is a design choice: `delta` stays below the
  diagonal margin `2^-40`, so the interval `W` stays nonsingular. Each of the 16 sign corners `J_true = J +- delta` is
  checked. If route (c) returns `Unavailable` there, the corner check is vacuous and is reported, not passed.
- The 60 transported certificates.
- Route (b):
  - block-diagonal n = 4, 6 and 8, built from consecutive discovery cells in listed order;
  - n = 3, a 2x2 block plus a 1x1 block.
- Route (d) at n in {3, 4, 8}:
  - `J = -100 tridiag(-1, 2, -1)`, plus first-order upwind convection with Peclet in {0, 10};
  - `h gamma in {1e-3, 1e-1}`;
  - `s_k = 1 + |y_k|` with `y_k = sin(k)`;
  - the candidate is the binary64 LU solution, perturbed by `2^-30` in coordinate 1.
- Route (d) negatives: `h gamma < 0` must give `NegativeShift`; `J = 10 I` with `a = 1` must give
  `NonpositiveDenominator`.

**E. Dense LU comparator** (reported). For every case of D, faer's partial-pivot LU solution of `W c = r` gives
`||c / s||_inf` as an `Estimated` value. The comparator costs `2/3 n^3 + 2 n^2` nominal flops.

## Commands

    cargo test --offline --locked -p rodas5p-core --test residual_gain_contract
    cargo test --offline --locked -p rodas5p-core --doc residual_gain
    AS05_RUNS=research/as05_residual_gain_core_20261011/RUNS.json cargo test --offline --locked --release -p rodas5p-core --test residual_gain_contract -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/as05_residual_gain_check.py --runs research/as05_residual_gain_core_20261011/RUNS.json --retained research/reaudit_accuracy_speed_20261010/NATIVE.json --output research/as05_residual_gain_core_20261011/RESULTS.json

**RUNS.json** stores every input as hex bits, so the oracle never trusts a decimal rendering. For each case it also
stores:
- the evidence kind and the rejection class;
- every interval, as hex;
- the bounds and the flop counts;
- the research reference intervals, for B;
- for every member of the W-formation list, the research `GainWitness2` enclosure of the represented `W`, as hex.

**Checker.** `tools/as05_residual_gain_check.py`, with mutation tests in `tools/test_as05_residual_gain_check.py`, is
committed before the recorded run.
- It calls `tools/evidence_schema_v2.py` first.
- It then runs an independent Python `Fraction` oracle: exact Gaussian elimination for n <= 8, with exact rationals of
  `J`, `h`, `gamma` and `delta`. It imports nothing from the Rust code or `check.py`.
- It records the doc-test and pre-existing-suite receipts.
- Exit codes: INVALID 2 (malformed evidence), FAIL 1, PASS 0.

## Gate

**Validity.** The result is INVALID if any of the following holds:
- the AS03 validation fails;
- a case ID is missing or duplicated, or the count differs from the registration;
- an input hex does not decode to the registered input;
- a hash differs from its pin: `NATIVE.json` `b9404714...`, `gain_witness.rs` `22c97719...`, `check.py`
  `238d52a2...`, re-audit `RESULTS.json` `20a6cb32...`;
- fewer than 4 W-formation fixtures in the list discriminate under the exact rule of C (a fixture-design defect, as
  in `FIRST_FAILURE.md`);
- the tree is dirty, or the checker postdates the run;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.

1. **Retained oracle.**
   - All 30 cells A are `Verified` and pass `check.py`'s criteria, recomputed by the new oracle: component
     enclosure, `wrms_upper^2 >=` exact squared WRMS error, infinity bound, and gain `>=` exact inverse gain.
   - 11 of 12 invalid controls are `Unavailable`, each with the mapped research class.
   - The registered exception is `invalid_determinant_overlap`. Its exact determinant is `2^-52`, which the directed
     primitives compute exactly, so it is predicted `Verified`. The exception may be `Verified` (then it must enclose
     its oracle error) or `Unavailable`; both PASS.
   - Any other admit/reject difference is a FAIL.
2. **Parity.** On every cell A, every core interval is contained in the research interval, endpoint by endpoint, and
   the core `wrms_upper` is `<=` the research one.
3. **Negatives refused.** No case of C is `Verified`.
   - Each one has its registered class: `TargetMismatch`, `MetricMismatch`, `StaleEpoch`, `OperatorMismatch`,
     `OffBlockCoupling` or `UndeclaredStructure`.
   - The assembly and `nu` cases are `Estimated`.
   - The route (d) negatives of D are not `Verified`, and the `NegativeShift` and `NonpositiveDenominator` cases give
     their class.
4. **No false certificate.** Every `Verified` result of D encloses the oracle's exact error in its stated metric,
   including all 16 assembly corners and the transported metric. A single violation fails the node.
5. **Authority types.**
   - Compile-fail doc tests show that `serde_json::from_str::<VerifiedCorrection>` and a struct literal of
     `VerifiedCorrection`, `LinearTargetId` or `ResidualGainWitness` do not compile outside the module.
   - The checker finds no `Deserialize` derive or implementation and no `pub` field on these types in
     `residual_gain.rs`.
6. **Non-regression and isolation.**
   - All pre-existing `rodas5p-core` tests pass, including `pp12_lognorm_decay`, `pp12b_chain_symmetrizer`,
     `int05_*`, `rev02_*` and `safe_enclosure_*`. No pre-existing test file is modified.
   - No file outside `rodas5p-core` and its tests references `residual_gain`.

Everything else is **FAIL**, with every number preserved.

There are no usefulness thresholds, by design, as in the re-audit: bound/error ratios and costs are reported only.

**Kill and hold rules.**
- A false certificate (item 4), or a `Verified` result from `nu`, a JVP-only input or an undeclared structure, means:
  FAIL, the core API is HOLD, and AS06 stays blocked.
- Loose bounds or a cost far above LU do not fail the node. They put any speed use on HOLD, as in PP12/PP12b
  (soundness PASS, usefulness FAIL).
- The PASS scope is the registered routes and sizes only. It is not a matrix-free certificate.

**Reported, not gated:**
- bound/error per case, with exact-zero errors recorded as null;
- core/research `wrms_upper` ratios;
- `Estimated`/`Verified` ratios against the LU comparator;
- route (d) `1 - a mu_up` and `mu_source`;
- setup and verification flops against LU flops.

## Coordination with sibling nodes

AS05 and PY04 both add helpers to `crates/rodas5p-core/src/directed.rs`. The helpers are additive only (no existing
function changes behaviour) and are merged serially: AS05 first, then PY04. Before each merge a name-collision check
confirms that no new helper name duplicates or shadows an existing or sibling-added item in `directed.rs`.

## Prior information (disclosed)

- **Re-audit result.** 30/30 enclosures, 12/12 rejections (final). Bound/error is 1.414 to 1.39e15, and 3 cells are
  exact-zero.
- **First run.** 11/12, because of the fixture defect described in `FIRST_FAILURE.md`.
- **The `invalid_determinant_overlap` exception** was derived by hand before registration, from the TwoSum/FMA
  exactness of `directed.rs`. No core code has run.
- **Directed primitives.** They come from re-audit R3 and are reviewed there.
- **PP12/PP12b.** Their soundness gates PASS. Usefulness FAILs on stiff cases (L-0075, PP12b G3 16/18).

## Predictions

- Items 1-6 pass. Item 1 gives 30/30, with 11/12 rejections plus the exception `Verified`.
- Parity: core bounds equal or slightly tighter than research, with ratio >= 0.99 in every cell.
- W formation, for `K = 0`:
  - `fl(h gamma)` perturbs the `2^-40` diagonal by a relative 1e-4, so the exact-shifted error is many orders of
    magnitude above the represented bound. Route (c) encloses it.
  - With `K = 1024`, interval dependency may inflate the represented bound until it no longer discriminates. The
    selection rule then moves on to later `h`.
- Route (d) diffusion: `mu_up <= 0`, `G <= 1`, and bound/error 1 to 100. Peclet 10 is looser.
- Cost: setup flops at n = 2 are at least 10x the nominal LU flops. No speed use is expected.
- Most likely FAIL: item 2 at a subnormal or near-overflow endpoint, where the two roundings differ.
