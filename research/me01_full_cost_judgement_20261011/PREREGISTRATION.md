# Preregistration: full-cost judgement of accuracy-passed candidates, timing on HOLD (ME01)

Node ME01 (P2, kind `measurement`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the
October 10 re-audit. Registered on branch `audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`. It depends on
AS03 (merged).

**Claim boundary.**
- Instructions (callgrind Ir), allocation events and allocated bytes are separate units with separate authorities. No
  conversion between them, and no flop or wall-time figure derived from them.
- **Until the timing authority is released, `wall_time_speedup = null` and `status = HOLD` for every candidate** (DAG
  kill rule). Released means that `timing_authority_registry` (`crates/rodas5p-fair-ab/src/timing_authority.rs`)
  reports `Admissible` for the campaign design.
  - Today all three covering studies are `Hold`.
  - L-0007 and L-0010: the pooled-pair coverage studies FAILed.
  - L-0018/L-0025: the session-median study PASSed within its simulated domain, "held pending independent domain
    review, not timing authority".
- Only candidates whose own accuracy gate passed are entered. Historical PASS/FAIL records are not changed, and no
  published campaign is rerun as such.
- No default promotion. A judgement here is "this unit's counter supports this operation claim", nothing more.

## Question

For every candidate in this repository that has passed its accuracy gate, what does one completed trajectory cost in
each unit? The cost includes rejected attempts, setup, workspace, verification, preconditioner and fallback work. It is
measured against two references:
- the candidate's own attribution rival in the same binary;
- the strongest applicable rival for that cell.

**The concrete new gap.** Past figures are per attempt and come from different binaries. The legacy arms drifted
+0.3 % to +8.6 % between binaries (L-0085, L-0089). Setup cancelled by the 2-minus-1 protocol was never charged, and
allocations and Ir were never measured on the same cells.

## Candidates (enumerated from `research/LEDGER.jsonl` at `0ce7c32`)

**Entered.** Identity means the candidate is bitwise identical to its attribution rival, which the run re-verifies.

| id | candidate (ledger) | attribution rival | kind |
|---|---|---|---|
| E1 | SP01 `ReuseConfirmed` v2 accounting, U-form MF and K-form (L-0100) | v1 `RecomputeFinal`, same driver | identity |
| E2 | CT01 `PredictiveCapped2`, dense fast driver, 40-point grid (L-0103) | `Integral`, same driver | trajectory-changing |
| E3 | SPD01 prevalidated controller + fused landing, `-ovh` arms (L-0081) | `rodas5p-fast`, `-fast-small` | identity |
| E4 | SPD03 banded arm (L-0083; INT-03 L-0054) | dense v2, `colext64` | identity |
| E5 | SPD06 small static stages (L-0085) | `rodas5p-fast-small` | identity |
| E6 | SPD09 `colext64` (L-0086) | dense v2 | identity |
| E7 | SPD04 workspace least squares in the U-form MF driver (L-0087) | high-level least squares, same driver | identity |
| E8 | R-NEXT-02 `solve_gmres_into` in the U-form MF driver (L-0045) | `solve_gmres`, same driver | identity |

**Not entered, with reasons (closed list).**

| Candidate | Reason |
|---|---|
| SPD05 (L-0088) | no driver caller, so no trajectory |
| SPD02 (L-0082), SPD07 (L-0084), SPD08 (L-0089) | node FAIL, not re-litigated; SPD02 is subsumed by E6 |
| SP03 router (L-0101) | node FAIL (item 5); its target enters as E4 |
| ALG06 B3 (L-0096/L-0097) | robust accuracy HOLD (REVIEW_KO) |
| ALG01/03/04/05 (L-0090/0093/0092/0094/0098) | accuracy FAIL; ALG05 is superseded by E2 |
| SAFE-RECYCLE (L-0066) | no accuracy gate (counted applications only) |
| PP08 (L-0071), POLY-03 (L-0009), PP10 (L-0080), GainWitness2 (L-0099) | action-level or module-level, no trajectory; L-0009 is a timing campaign under HOLD |
| fast drivers L-0032, L-0041 | their PASS stands; they appear only as rivals |

Rows added to the ledger after `0ce7c32` are not entered here.

## Change

1. **`crates/rodas5p-cli/src/stiff_benchmark.rs`.** New `stiff-profile-run` arms:
   - `rodas5p-mf-legacy-v2` (E1);
   - `rodas5p-fast-pred2` and `rodas5p-fast-int-grid`, which use the CT01 40-point grid (E2);
   - `rodas5p-mf-ls-ws` and `rodas5p-mf-into` (E7, E8);
   - a `repetitions = 0` mode, so that the cold per-trajectory cost Ir(1) - Ir(0) includes one-time setup.

   No existing arm changes, and a CLI identity test pins this.
2. **`crates/rodas5p-cli/src/bin/allocation_audit.rs`.** A `stiff-arms` mode using the existing counting allocator.
   It records events, bytes and peak live bytes per trajectory (2-minus-1 and 1-minus-0). Allocation counting stays in
   this separate binary, so the Ir binary has no counting allocator.
3. **Profiling exporters.**
   - `tools/me01_full_cost_profile.py` wraps `tools/speed_profile.py` and runs repetitions 0, 1 and 2 plus a
     determinism repeat.
   - `tools/me01_full_cost_check.py` validates first (`evidence_schema_v2`, new kind `me01`).
4. **`docs/TIMING_DESIGN_CONTRACT.md`.** An appended section, "Operation-count judgements (ME01)", stating the unit
   separation and that wall time requires `Admissible`. Existing text is not changed.

## Cells

- **Small problems.** van-der-pol-mu1000, robertson and hires, at rtol 1e-6 and 1e-8.
- **Brusselator-1D.** n = 100, 400, 1000, at 1e-6 and 1e-8.
- **Matrix-free candidates (E1, E7, E8).** The SPD07 problem set that the CLI exposes, at 1e-6 and 1e-8.
- **E2.**
  - Seed h0 = 2e-5, a CT01 gated seed.
  - Half-decade rungs of the CT01 ladders: van der Pol and Brusselator-50 from 1e-3 to 1e-7 (9 rungs); HIRES and
    Robertson from 1e-3 to 1e-10 (15 rungs).
  - Matched-error Ir comes from CT01's frontier rule on these rungs, using the error exported by the same run.
- **Strongest applicable rival.** For each cell, the cheapest Ir per trajectory at the same rtol among the arms that
  apply to the cell's declared structure: small, dense v2, `colext64`, banded and MF. The candidate itself is excluded
  from its own strongest-rival set. It counts only when its error ratio to the candidate is in [0.5, 2.0] (design
  choice; CT01 item 4b band). Otherwise the comparison is flagged `unmatched`.

**Candidate x cell table (closed).** Problem ids are those of `crates/rodas5p-cli/src/stiff_benchmark.rs`
(`benchmark_problems`, `profile_problems`, `routing_profile_problems`). `brusselator-1d-50`, `-200` and `-500` are
n = 100, 400 and 1000. For E1, E7 and E8 the Brusselators are the JVP-only problem of
`brusselator_jvp_only_problem`, the only matrix-free problem the CLI exposes.

| id | problems | rtols |
|---|---|---|
| E1 | `brusselator-1d-50`, `brusselator-1d-160` (JVP-only) | 1e-6, 1e-8 |
| E2 | `van-der-pol-mu1000`, `brusselator-1d-50` | half-decade rungs 1e-3 to 1e-7 (9), h0 = 2e-5 |
| E2 | `hires`, `robertson` | half-decade rungs 1e-3 to 1e-10 (15), h0 = 2e-5 |
| E3 | `robertson`, `hires`, `van-der-pol-mu1000`, `brusselator-1d-200` | 1e-6, 1e-8 |
| E4 | `brusselator-1d-50`, `brusselator-1d-200`, `brusselator-1d-500` | 1e-6, 1e-8 |
| E5 | `robertson`, `hires`, `van-der-pol-mu1000` | 1e-6, 1e-8 |
| E6 | `brusselator-1d-50`, `brusselator-1d-200`, `brusselator-1d-500` | 1e-6, 1e-8 |
| E7 | `brusselator-1d-50`, `brusselator-1d-160` (JVP-only) | 1e-6, 1e-8 |
| E8 | `brusselator-1d-50`, `brusselator-1d-160` (JVP-only) | 1e-6, 1e-8 |

No other (candidate, cell) pair is run or scored.

## Commands

    CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --offline --locked --release -p rodas5p-cli
    RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 python3 tools/me01_full_cost_profile.py --rodas5p target/release/rodas5p --scratch "$SCRATCH" --output research/me01_full_cost_judgement_20261011/PROFILE.json
    RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 target/release/allocation_audit stiff-arms --output research/me01_full_cost_judgement_20261011/ALLOCATIONS.json
    python3 tools/me01_full_cost_check.py --profile research/me01_full_cost_judgement_20261011/PROFILE.json --allocations research/me01_full_cost_judgement_20261011/ALLOCATIONS.json --ledger research/LEDGER.jsonl --output research/me01_full_cost_judgement_20261011/RESULTS.json

**Binary provenance.** The binary is the literal `target/release/rodas5p`. Its SHA-256, and that of
`allocation_audit`, are written into PROFILE.json and ALLOCATIONS.json and into the ledger row's inputs. No
`RELEASE_BINARY`/`MEASUREMENT_BINARY` placeholder is used (DAG).

**Checker.** Raw files are persisted before the checker runs. The checker is committed before the recorded run.
INVALID exits 2, FAIL exits 1.

## Classification rules (fixed now; outcomes, not pass conditions)

**Per unit u.** u is Ir per trajectory (warm 2-1 and cold 1-0), allocation events, or bytes. Let R_u be the ratio of
candidate to attribution rival in that unit, on each in-scope cell. Then:
- **GAIN:** every cell <= 0.98.
- **NEUTRAL:** every cell in (0.98, 1.02].
- **LOSS:** any cell > 1.02.
- **MIXED:** anything else.

The +-2 % band is the SPD09 and SP03 overhead bound.

**Against the strongest applicable rival.** **DOMINATED** if the candidate's Ir per trajectory is > 1.00x the rival's
on any matched cell. **COMPETITIVE** otherwise.

## Gate

**Validity.** The result is INVALID if any of the following holds:
- the validator fails;
- a callgrind 1-repetition repeat differs from its run;
- a CT01 error recomputed from state bits differs from the exported error;
- the checker blob at the RUNS (PROFILE, ALLOCATIONS) source commit differs from the checker that was run, or any
  recorded tree status is dirty.

An identity break is not an INVALID condition; it is gate item 6.

**PASS** if all of the following hold.

1. **Completeness.** Every (candidate, cell, unit) in the plan has a value. Every not-entered ledger row listed above
   has its typed reason in RESULTS.
2. **Same-binary attribution.**
   - In each unit, every candidate and its attribution rival come from one binary (one SHA-256).
   - The strongest-rival comparisons also use that binary.
3. **Own counters.** Every operation claim in RESULTS cites a counter of its own unit. No ratio crosses units.
4. **Timing held.**
   - `wall_time_speedup` is null and `status` is `HOLD` for every candidate.
   - The registry source SHA-256 is recorded, and its three entries read `Hold`.
5. **Rules applied.** The checker recomputes every classification from the raw rows.
6. **Identity holds.** For every identity candidate (E1, E3-E8), the final-state SHA-256 and the counters equal its
   attribution rival's in the profiled binary, wherever identity is expected. An identity break is a FAIL of this
   item, recorded with the first differing cell; it is never INVALID.

Everything else is **FAIL**. A LOSS, DOMINATED or `unmatched` result is recorded as such and is never rerun.

## Prior information (disclosed)

Prior figures (per attempt, earlier binaries):

| Candidate | Prior figures |
|---|---|
| E1 | v2/v1 Ir 0.962-0.996 on 5 cells |
| E2 | van der Pol attempts frontier 0.801-0.863; others 0.989-1.017; no Ir |
| E3 | small driver 0.78x (van der Pol), 0.84x (Robertson), 0.93x (HIRES); dense 0.91-0.96x at n <= 8, 0.999x at n = 400 |
| E4 | 0.255x at n = 400, 0.617x at n = 100 |
| E5 | 0.838x (van der Pol), 0.848x (Robertson), 0.736x (HIRES) |
| E6 | 1.0000-1.0001x at n <= 60, 0.787x at n = 100, 0.531x at n = 400 |
| E7 | allocations 0.018-0.096x (HIRES 0.48x) |
| E8 | allocations 0.02-0.28x |

- SP03 (L-0101) measured routed/Legacy = 0.0014-0.015, so the matrix-free arms cost about 70-700x the routed (banded)
  arm on the Brusselators.
- No code of this node exists.

## Predictions

| Candidate | Unit | Prediction |
|---|---|---|
| E1 | Ir | MIXED (GAIN on van der Pol and HIRES, NEUTRAL on Brusselator-50); DOMINATED on every cell, by the banded or small driver |
| E2 | Ir | GAIN at matched error on van der Pol, NEUTRAL elsewhere, so MIXED |
| E3 | Ir | MIXED (GAIN on the small driver, NEUTRAL at n = 400) |
| E4 | Ir | GAIN |
| E5 | Ir | GAIN |
| E6 | Ir | GAIN on its n > 64 scope; NEUTRAL below |
| E7, E8 | allocations | GAIN |
| E7, E8 | Ir | NEUTRAL |

- Cold costs exceed warm costs by the most on the banded arm (band fill) and on the matrix-free workspaces.
- Overall: PASS. Every `wall_time_speedup` is null and every status is HOLD.

## Results

Appended after the recorded run. Nothing above this heading changes.
