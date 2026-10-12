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

## Results

Everything above this heading is the registered text, unchanged. Executed on branch `node/as05` from `8ce9bda` (the
registration commit); the registered text names `audit/rvj-reaudit-remaining-20261011` on top of `0ce7c32`. Only the
branch name differs; `8ce9bda` is the registration commit on top of `0ce7c32`.

**Verdict: PASS** (evidence VALID; checker exit 0). Items 1-6 all hold. This is a soundness result for the
registered routes and sizes only: one linear solve, a frozen diagonal metric, the bound target. It is not a stage,
local or global ODE error claim, not a matrix-free certificate, and there is no driver caller, default change or
wall-time claim. The re-audit's 30-cell PASS and `FIRST_FAILURE.md` are not edited or re-scored.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| Registration (text above) | `8ce9bda` | — |
| A: `residual_gain.rs`, `lib.rs` export, contract test + export | `b128350` | `residual_gain.rs` `c8d08e64a758630e3da7ea8e053739d9b589bb025eab8e1f59975b75c45539f6`; `lib.rs` `86ecf16460114a517f97e68b08c458e39f15cce467fe2d9b630034bfb9828298` |
| B: checker, receipts tool, synthetic checker tests (before any recorded run) | `e474409` | `as05_residual_gain_check.py` `432eaba932e122dc0ef549ca29ed218157021a4506c2e9cc7d47bc0df1a6a794`; `test_as05_residual_gain_check.py` `5baec32eca875f5e6ba3f37376ed1fb092d60e51e0c87a2e406aea4f341beae6`; `as05_receipts.py` `952d80efbc2c041d9bf8faecec67fcdac3ae1585fbed6723d68ed032604a530e` |
| First recorded attempt at `e474409`, preserved | `0e4ab5d` | `RESULTS_FIRST.json` `b50d9eba57cc710ae8c029fa1522d7c64109faa7814b66311c75de79b6b73472` (INVALID: no RUNS.json); `RECEIPTS_FIRST.json` `5e78d5695f7e969dd4bac486b3ffd0f313bd04ed394f64eb1bb9ca44b959484c` |
| Export path fix (test file only) | `6cbca8f` | `residual_gain_contract.rs` `fbf992b0ecebb27f3f844dc9dba9ec373a9894838685449aa1d5610dace4e3e7` |
| C: recorded run at `6cbca8f` (clean tree) | `e998826` | `RUNS.json` `9217decce7e42c0b37de33e0b51335de204e5c7aac79977b54f4a5611756339a` (422,588 bytes, compact JSON); `RECEIPTS.json` `aa01945caae741ca1f5ee55a087ce492ed2b4fb77a52879408bd2b822e13efee`; `RESULTS.json` `3a1d0408185b8d6ae7f021e666e48177a50bae41e9ba2199f0190e3aa9ba863c` |
| Inputs, unchanged (pins) | — | `NATIVE.json` `b9404714…bd364`; `gain_witness.rs` `22c97719…e5c89`; `check.py` `238d52a2…b89`; re-audit `RESULTS.json` `20a6cb32…0e21`; `evidence_schema_v2.py` `1d2bfe92…06ff10` |

`directed.rs` and `nonnormal_certificate.rs` are byte-identical to `8ce9bda`: no helper was needed
(`symmetric_part_upper` is already `pub`), so there is nothing for PY04 to merge against in those files.

**First attempt (preserved, not a scientific result).** At `e474409` the registered export command
(`AS05_RUNS=research/.../RUNS.json`, a path relative to the repository root) failed: cargo runs the test binary in the
package directory, so `std::fs::write` found no such directory and no `RUNS.json` was written. The checker, run next,
returned INVALID because `RUNS.json` was missing (`RESULTS_FIRST.json`); the receipts of that commit (`RECEIPTS_FIRST.json`)
are all exit 0 on a clean tree. Commit `6cbca8f` changes only the export's path handling (a relative path is joined to
the repository root). No case, input, witness, checker or gate changed. The run was then repeated once at `6cbca8f`.

**How the run was made** (rustc/cargo 1.94.1, Python 3.11.15, `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, clean
tree at `6cbca8f`):
1. `python3 -B tools/as05_receipts.py --output <scratch>/RECEIPTS.json` ran registered commands 1 and 2 and
   `cargo test --offline --locked -p rodas5p-core --all-targets`, all exit 0, clean tree before and after (I10).
2. Registered command 3 (release) wrote `RUNS.json`: 307 cases, `source.tree_status` empty.
3. The receipts were copied next to `RUNS.json`; registered command 4 (the checker) printed PASS and exited 0.

Checker limitation (found while writing these results, not changed): the suite table in `RESULTS.json` is keyed by
target name, so the two groups named `unittests` (`src/lib.rs` and the `shared_shift_jet_study` example) appear as one
row (37 names for 38 groups). The receipt itself has 38 `test result: ok` lines with 0 failed, and the checker also
requires every command's exit code to be 0, so no failure could be hidden.

A second export from the same commit to a scratch path is identical to `RUNS.json` except `source.tree_status`, which
lists the then-untracked outputs.

**Tests at commit A.** `cargo fmt`; `cargo clippy --workspace --all-targets --locked -- -D warnings` clean;
`cargo test -p rodas5p-core --all-targets` passed (38 test-result groups, 0 failures, including `residual_gain_contract`
6 passed, 1 ignored); `cargo test -p rodas5p-core --doc` passed (5 `residual_gain` doc tests: 1 run, 4 compile-fail).
**Tests at commit B.** `python3 -m unittest discover -s tools -p test_as05_residual_gain_check.py` passed 39 tests,
normally and with `-O`: 1 valid PASS fixture, 1 immutable-output test, 19 INVALID fixtures, 18 FAIL fixtures (every
item 1-6, plus the overlap-exception-Unavailable pass case).

### Validity

All registered conditions hold: the AS03 rules pass (strict JSON; NATIVE pin); 307 of 307 case IDs, no duplicate,
every group count as registered; every input hex equals the input regenerated by the checker (NATIVE.json, the
snapshot gamma `0.21193756319429014`, the registration; route (d) `s_k` within one faithful rounding of `sin k`); all
four pins match; the research reference intervals of the 30 cells equal NATIVE.json bit for bit; the checker blob at
`6cbca8f` is the checker that ran; RUNS and RECEIPTS name the same commit and every recorded tree status is empty.
W-formation: **16 of 16** members discriminate under the exact rule (the export's flags agree); the fixtures are the
first four, `wf_h0.1_k0`, `wf_h0.1_k1024`, `wf_h0.3_k0`, `wf_h0.3_k1024`.

### Gate

| Item | Rule | Measured | Result |
|---|---|---|---|
| 1 | 30 cells A Verified and enclosing (components, WRMS, infinity, gain); 11/12 invalid Unavailable with the mapped class; overlap exception Verified-and-enclosing or Unavailable | 30/30 Verified and enclosing; 11/11 classes as mapped; `invalid_determinant_overlap` Verified (`wrms_upper` 2, exact WRMS error 0.7071, components, infinity and gain enclose) | holds |
| 2 | every core residual/correction/weighted interval inside the research interval; core `wrms_upper` <= research; core gain <= research | 30/30; core/research `wrms_upper` from 0 (3 exact-zero cells) to 0.99999952 | holds |
| 3 | no C case Verified, each with its class; assembly and nu Estimated; route (d) negatives | TargetMismatch 4/4, MetricMismatch 60/60, StaleEpoch 60/60, OperatorMismatch 24/24, OffBlockCoupling 24/24, UndeclaredStructure 1/1, Estimated(UndeclaredAssembly) 4/4, Estimated(ProjectedNu) 2/2, NegativeShift 1/1, NonpositiveDenominator 1/1 | holds |
| 4 | every Verified result of D encloses the exact error in its stated metric (16 corners for delta) and is bound to its target | 81 Verified checked: route (c) exact 4/4, route (c) delta 2 cases x 16 corners = 32 corner checks, transported 60/60, route (b) 4/4, route (d) 11/11; 0 violations | holds |
| 5 | compile-fail doc tests present and passing; no Deserialize, no pub field on the authority types | 4/4 present and passed (`from_str::<VerifiedCorrection>`, three struct literals); 0 Deserialize, 0 pub fields | holds |
| 6 | pre-existing suite passes; no pre-existing test file modified; no reference outside rodas5p-core | 38 test-result groups ok, 0 failed, exit 0 (pp12, pp12b, int05_*, rev02_*, safe_enclosure_* present); test dir diff = `A residual_gain_contract.rs` only; `residual_gain` (whole word) in `.rs`/`Cargo.toml` only under `crates/rodas5p-core` | holds |

### Reported, not gated

- **Bound/error, cells A:** 1.414 in 26 cells, `control_symmetric` 1.4143, `control_near_singular` 3.80e8; 3 exact-zero
  errors (null). The research prototype ranged 1.414 to 1.39e15 on the same cells.
- **Core/research `wrms_upper`:** 14 cells in [0.99927, 0.99999952]; 16 cells below 0.99, down to 1.0e-15 and three
  exact zeros (`grid_k1048576_s{0,1,2}_p0`, where the perturbation rounds away and the directed residual is exactly 0).
  The research prototype's unconditional one-ulp widening of every operation is what the core removes.
- **W-formation (route (c), Exact):** bound/error 4.18, 5.91, 6.80, 9.61 on the 4 fixtures (exact shifted errors
  6.4e7, 2.2e21, 4.0e7, 4.0e21 in WRMS). The represented research enclosures are about 1e-4 wide for `K = 0` and they
  miss these errors by 11 or more orders of magnitude.
- **Route (c), EntrywiseAbs(`delta = 2^-60 max|J|`):** `K = 0` fixtures Verified, all 16 corners enclosed, bound/worst
  corner error 6.09 and 9.71. The `K = 1024` fixtures give `Unavailable(DeterminantContainsZero)`: the interval
  `W_01 W_10` term (`|h gamma 1024| x h gamma delta`, about 1e-16) exceeds `det W` (about 2^-80). Their corner checks are
  **vacuous** (reported, not passed).
- **Route (b):** bound/error 1.414 (n = 4), 2.449 (n = 6), 2.828 (n = 8), 1.732 (n = 3).
- **Route (d):** all 11 Verified use `interval-cholesky`. `mu_up` is -48.7 (n = 3) and -23.9 (n = 4) for both Peclet
  numbers, -0.038 (n = 8, Pe 0) and +40.6 (n = 8, Pe 10). `1 - a mu_up` lower bound is 0.959 to 9.56. Bound/error is
  1.15 to 48.1 (Pe 10 looser). `lognorm_n8_pe10_a1e-1` is `Unavailable(NonpositiveDenominator)`: `1 - 0.1 x 40.6 < 0`
  in the metric `s_k = 1 + |sin k|`. No certificate; reported.
- **Projected nu (F105):** both estimates are 0.7071 (`nu = 1`, breakdown after one Arnoldi step), against exact WRMS
  errors 724.1 (`K = 1024`) and 741455 (`K = 2^20`): underestimates by about 1e3 and 1e6. Both are `Estimated`.
- **LU comparator (`||c/s||_inf` of faer's LU on `fl(W)`):** it equals the Verified infinity bound on route (b) (ratio
  1), is 0.02 to 0.87 of it on route (d), and is **0** on all 8 W-formation cases: the candidate is the rounded exact
  solution of the represented `fl(W)`, so the comparator residual vanishes and it cannot see the W-formation error that
  route (c) encloses.
- **Counted flops:** setup / nominal LU (`2/3 n^3 + 2 n^2`) at n = 2 is 9.15 (route (a), 122 calls) to 13.35 (route
  (c) with delta, 178 calls); verification 92 calls at n = 2. At n = 8, route (b) 488 setup / 368 verification, route
  (d) 1681 / 725 (excluding the `symmetric_part_upper` search) against 469 nominal LU flops. No speed use is implied.
- **Transport:** 60/60 transported certificates enclose; the transported gain (reported) encloses the exact gain in the
  new metric in every case.

### Prediction check

- Items 1-6 pass; 30/30; 11/12 rejections plus the exception Verified: **as predicted**.
- Parity "equal or slightly tighter, ratio >= 0.99 in every cell": **not borne out**. Containment holds everywhere,
  but 16 of 30 cells are much tighter (down to 1e-15, three exact zeros).
- W formation: `K = 0` as predicted (route (c) encloses; the represented bound misses by many orders). The `K = 1024`
  members **do** discriminate (16/16), contrary to the hedge that interval dependency might stop them; instead the
  `K = 1024` delta enclosure is the one that fails to be nonsingular (vacuous corners).
- Route (d) diffusion: `mu_up <= 0` and `1/(1 - a mu_up) <= 1` as predicted; bound/error 1.15 to 23.0 for Pe 0, inside
  1 to 100. Peclet 10 is looser as predicted, and at n = 8 its `mu_up` is positive (one registered positive is refused),
  which was not predicted.
- Cost "setup at n = 2 at least 10x LU": **partly**. Route (c) is 12.75x and 13.35x, route (a) 9.15x.
- The most likely FAIL (item 2 at a subnormal or near-overflow endpoint) did not occur.

### Interpretations and deviations

All were fixed in the checker docstring (I1-I17) at commit B, before the recorded run:
- I1-I7 fix the unspecified inputs: b = (1, 1), unit metric and epoch 7 for the W-formation family; the weighted
  research enclosure decides discrimination; next_up for one-ulp changes; off-block and route (b) bases from the first
  discovery cells; route (d) with `h = a`, `gamma = 1`, the upwind form, b = ones, `k = 1..n`, first-coordinate additive
  perturbation; nu with r = e2, start e1.
- I8-I9: the target constructors are part of the witness API; the approximate-inverse route is a function returning
  `Unavailable(RouteNotImplemented)`; `directed.rs` and `nonnormal_certificate.rs` are unchanged.
- I10-I11: the receipts tool; item 6 scope is Rust sources and Cargo manifests (whole word `residual_gain`).
- I12-I16: gain added to item 2; binding checked in items 1 and 4; vacuous corners and refused route (d) positives
  reported; overlap exception may be Unavailable with any class; flop-count convention.
- I17 (disclosure): before commit B, a development export (scratch path, from the tree that became commit A) and
  development receipts were used to debug the export and the checker's parsing. They are not recorded evidence, and no
  case, input, threshold or gate was changed after them. The only checker change was the whole-word match in I11.
- The DAG lists `gain_witness.rs` as an edit target; as registered it stays byte-identical and is included by path.
- The first attempt failed on the export path (above); `*_FIRST.json` are kept.

**Kill and hold.** No false certificate, no Verified result from nu, an undeclared assembly or an undeclared
structure: the core API is not put on HOLD by a kill rule, and AS06 is not blocked by this node. Usefulness is not
gated. The counted setup cost is 9x to 13x nominal LU at n = 2, and route (d) refuses one registered stiff convective
positive. So any speed use stays on HOLD, as in PP12/PP12b. There is no default promotion.
