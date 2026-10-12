# Preregistration: windowed homotopy parallel native pilot on the HT01 client (HT02)

Node HT02 (P3, kind `experiment`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the
October 10 re-audit (`MATHEMATICS_PORTING_KO.md`, sections 7.2-7.5). Registered on branch
`audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`. Dependencies: HT01 (client selection, registered alongside)
and AS03 (merged). This node is **conditional**: it runs only if HT01's committed `RESULTS.json` says
`SELECTED(client)`.

**Claim boundary.**
- Counted work and a counted critical-path model (span) only. No wall time: the timing authority is on HOLD
  (`timing_authority_registry`: L-0007, L-0010, L-0025 all `Hold`). "No speed claim from lane count" (DAG).
- The output authority is the unchanged lambda = 1 endpoint certificate. Intermediate lambda, predictor agreement or a
  truncated q-level residual is never an error bound.
- Kill rule (DAG): more continuation plus certification than the saved sequential work, or a failed tube, stops the
  node.
- No default promotion. No rerun of HOM-06, R-NEXT-06 or any diagonal / depth-7 study.

## Question

On the real client selected by HT01, does a windowed homotopy with deterministic workers and round barriers do three
things?
- Keep enclosure parity with the sequential arm, with every accepted output certified at lambda = 1.
- Give a lower counted critical path than the strongest sequential comparator, once setup, synchronization,
  certificate, rejected candidates and fallback are charged.
- Record its full work.

## Branch A: HT01 decided ABSTAIN (registered outcome; predicted)

Nothing is built or run. The checker reads HT01 `RESULTS.json` (pinned by SHA-256) and writes `RESULTS.json` with:
- `decision: "ABSTAIN_NOT_RUN"`;
- the HT01 decision and failing criteria per candidate;
- `runs: 0` and `wall_time_speedup: null`.

The ledger verdict is **INCONCLUSIVE** ("ABSTAIN, not run; no evidence about homotopy parallelism"). It is not PASS,
because nothing was tested. Under `tools/check-research-node.py`'s stall rule this is a node without a numeric result;
the order of later nodes must respect that rule.

**Ledger order.** HT02's first ledger row must come immediately after a node whose ledger row is a numeric PASS or
FAIL, never after another INCONCLUSIVE node. The stall rule (rule (d) of `tools/check-research-node.py`) allows at
most two consecutive nodes without a numeric result; ledgering HT02 right after a numeric node keeps the next slot
free.

    python3 tools/ht02_parallel_check.py --ht01 research/ht01_homotopy_client_contract_20261011/RESULTS.json --abstain-only --output research/ht02_homotopy_parallel_pilot_20261011/RESULTS.json

## Branch B: HT01 decided SELECTED(client)

**Supplementary registration first.** The client is unknown at this registration, so the concrete stage-1 cells, the
reference rule and the atol scale are not fixed here. Before any Branch-B code is written, a supplementary
registration HT02b (`research/ht02_homotopy_parallel_pilot_20261011/PREREGISTRATION_HT02B.md`) must be committed. It
fixes the cells (declared cases, split, rtols), the references (rule, precision, reference-limited threshold) and the
atol scale. HT02b may not change any gate of this document. Without HT02b, Branch B is not run.

### Stage 0: feasibility (what must exist before any measurement)

Every item is a committed contract test with a recorded outcome.

- **F1. Endpoint certificate.** `NativeTargetCertificate` or `PreparedStructuredCertificate` for the client, with
  `ModelBinding::GeneratedFromModel`, decided by `capability()` before speculative work.
- **F2. Verified tube.** On a fixed tube `T`: `q_lambda >= ||D^-1|| (eta ||C|| + |lambda h| L_N)`, `q_lambda < 1`,
  and `delta_lambda + q_lambda r <= r` with the ball inside `T`, each enclosed with outward rounding (section 7.3).
  The inverse defect and the rounding of inverse application are included.
- **F3. Deterministic parallel rounds.** Workers read only the previous round's frozen K. The output is bitwise
  identical for threads in {1, 2, 4, 8} and for two task partitions: output authority is independent of the
  partition.
- **F4. Complete ledger.** `HomotopyWorkLedger` gains:
  - max task work per round;
  - barrier count;
  - rejected-candidate work;
  - fallback work (including the 8-stage sequential fallback and retries);
  - setup;
  - peak bytes, from a counting allocator in the test.
- **F5. Comparator.** HT01's S3 comparator is runnable in the same test binary.

**Stage 0 outcome.** If F1-F5 all pass, the outcome is FEASIBLE and stage 1 runs. Otherwise it is **HOLD**, with the
failing items. A HOLD gives a stage-0 ledger row: PASS of the stage-0 audit (every item answered with committed
evidence) with "decision: HOLD", or FAIL if an item is unanswered.

### Stage 1: measurement

**Cells** (made concrete by HT02b).
- The client's declared cases are split by HT01's `RESULTS.json`. Stage-0 engineering uses only the first declared
  case. All other declared cases are held out.
- rtol in {1e-6, 1e-8}, with atol = rtol x the client's declared atol scale.
- References are computed per the client's reference rule. Reference-limited cells (error < 100 u, as in CT01) are
  excluded and counted.

**Arms.**
- `seq`: the strongest sequential comparator.
- `par-P`, for P in {1, 2, 4, 8}: windowed homotopy with deterministic workers.
- `fallback-only`: a control that always takes the fallback. It is reported only.

**Units.**
- Work: RHS and JVP evaluations, W factorizations and solve vectors, certificate directed operations, and
  same-binary callgrind Ir per trajectory.
- Span: `T_par,model` in solve units `c_solve`, as in R-NEXT-06, with the client's declared callback cost converted
  through its HT01 declaration. One unit is charged per barrier when P > 1 (design choice; R-NEXT-06 charged one
  dispatch unit per batch).
- Peak bytes.

**Commands.**

    HT02_RUNS=research/ht02_homotopy_parallel_pilot_20261011/RUNS.json cargo test --offline --locked --release -p rodas5p-integrators --test homotopy_client_parallel -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --offline --locked -p rodas5p-integrators --test homotopy_client_parallel
    python3 tools/ht02_parallel_check.py --ht01 research/ht01_homotopy_client_contract_20261011/RESULTS.json --runs research/ht02_homotopy_parallel_pilot_20261011/RUNS.json --output research/ht02_homotopy_parallel_pilot_20261011/RESULTS.json

Both commands run with `RAYON_NUM_THREADS` unset (the test owns its pools) and `OPENBLAS_NUM_THREADS=1`. `RUNS.json`
stores every state as hex bits, the counters, the ledger and the certificates, and is persisted before any gate. The
checker calls `evidence_schema_v2` first: INVALID exits 2, FAIL exits 1. The checker is committed before the recorded
run.

## Gate (stage 1)

**Validity.** INVALID if any of the following holds:
- the AS03 rules fail, or `RUNS.json` does not match its schema;
- the (cell, arm) row set differs from the cells of HT02b times the registered arms, or a row is duplicated;
- a recomputed quantity differs from the exported one: a `W_total` or `T_par,model` from its recorded components, a
  reference error from the state bits, or a certificate check;
- HT02b was committed after any Branch-B source change;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold on every held-out cell.

1. **Enclosure parity.**
   - Every accepted `par-P` output is certified at lambda = 1 by the unchanged certificate code.
   - The certified bound is at least the reference error at every output point, for `seq` (where it certifies) and
     for `par-P`.
   - `par-P` outputs are bitwise identical across P.
   - `par-8` error is at most 1.5x the `seq` error at equal rtol (design choice; ALG-series twin bound).
2. **Critical path.** `T_par,model(P = 8) / T_seq <= 0.87` (= 1/1.15), with setup, barriers, certificate, rejected
   candidates and fallback on the critical path.
3. **Kill rule not triggered.**
   - Per cell: the continuation, certification, rejected and fallback span is below the saved span, `T_seq` minus the
     idealized task span. The idealized task span is the sum over rounds of the maximum task work at P = 8, with zero
     setup, barrier and certificate cost.
   - No tube verification (F2) fails on a held-out step. A step that falls back after a failed tube counts as a
     failed tube.
4. **Full work recorded.**
   - `W_total` components sum exactly to the counters.
   - `W_total(par-8) / W_seq` is reported, not bounded.
   - `wall_time_speedup` is null.

Everything else is **FAIL**, with every number preserved. A kill-rule trigger is FAIL with "stop". No parameter is
retuned after the run.

## Prior information (disclosed)

- HOM-06 (L-0008): every certified transactional arm was Blocked, at 0.17 (1 thread) and 0.04-0.07 (2-8 threads),
  with about 5x RHS.
- R-NEXT-06 (L-0050): depth 7. Margins were negative for n = 1-8 and about +0.043 solve units per attempt at n = 16.
- L-0034 to L-0036: radius and path certificates on the R4 fixtures only.
- No code of this node exists.

## Predictions

- **Branch A (ABSTAIN_NOT_RUN): expected, with high confidence.** HT01 is predicted to ABSTAIN: no declared expensive
  client, and the largest measured callback share is 0.108, below 0.348.
- **If Branch B runs:**
  - Stage 0: F2 and F4 are the likely HOLD points for any client outside the declared quadratic family, because the
    default witnesses are diagonal or n <= 2 only.
  - Stage 1: gate item 2 is expected to FAIL unless the callback share is far above the HT01 floor, given the
    measured 5x RHS multiplier and the near-zero R-NEXT-06 margins.

## Results

Appended after the recorded run. Nothing above this heading changes.

Everything above this heading is the registered text, unchanged. Executed on branch `node/ht01`, after HT01's
recorded run; the registered text names `audit/rvj-reaudit-remaining-20261011` as the branch.

**Outcome: Branch A, decision `ABSTAIN_NOT_RUN`.** Ledger verdict: **INCONCLUSIVE** ("ABSTAIN, not run; no evidence
about homotopy parallelism"). HT01 (`f372026`; its `RESULTS.json` verdict is PASS, decision ABSTAIN, no lead) selected
no client, so nothing was built or run.
- There is no HT02b, no Branch-B code, no Rust change and no cargo command.
- `runs = 0` and `wall_time_speedup = null`.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| Registration (text above) | `8ce9bda` | — |
| HT01 input: `research/ht01_homotopy_client_contract_20261011/RESULTS.json` (recorded at `d443431`) | `d443431` | `255670b231af2db9649935d8cbb2dddc6dfdcec6004d03b0695da79a06f64434` (pinned in the checker) |
| Checker and synthetic tests (before the run) | `52a5205` | `ht02_parallel_check.py` `9b570c52a7a72acdbf0c15b57d0046aa6d9af2975076aa07526c4da57f98f373`; `test_ht02_parallel_check.py` `832be15c015303b281cfe9db5f495eae797ee8f67f73fcf6675ab87c88cd1f66` |
| Recorded run at `52a5205` (clean tree) | this commit | `RESULTS.json` `c76d621663c949e4fb0929d4e5ff5d54b1729fde1f012e9128e91cecc37746d6` (4,057 bytes) |

The registered Branch-A command was run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1` and Python 3.11.15, and
exited 0:

    python3 tools/ht02_parallel_check.py --ht01 research/ht01_homotopy_client_contract_20261011/RESULTS.json --abstain-only --output research/ht02_homotopy_parallel_pilot_20261011/RESULTS.json

`RESULTS.json` records:
- the source commit `52a52057138b8f1bf2e6902b5d9f96582e05d8fc`, an empty tree status, and the checker blob `f2b8f24d`;
- HT01's verdict, decision, leads and source commit, and the criteria each candidate failed.

The synthetic tests ran 14 of 14 OK at `52a5205`.

### HT01 decision and failing criteria

| candidate | criteria not met (HT01) |
|---|---|
| K1 Fourier | S1, S2 (PP07 `q1_solves = 0`), S3 (no applicable arm), S4 |
| K2 quadratic family | S1 (no declaration), S3 (no measured direct arm; L-0008 comparator JF-GMRES only) |
| K3 Bateman | S1, S2, S3, S4 |
| K4 Darboux chart | S1, S2, S3, S4 |
| K5 robertson/hires/vdp | S1 (no declaration; f = 0.040 < 0.348), S4 |
| K6 brusselator-1d-N | S1 (no declaration; f = 0.071 < 0.348), S4 |
| K7 corpus v2 | S1, S2, S3, S4 |
| K8 `problems.rs` fixtures | S1, S2, S3, S4 |

### Prediction check

Branch A (`ABSTAIN_NOT_RUN`) was predicted with high confidence, and that is the outcome. The stage-0 and stage-1
predictions of Branch B are not tested.

### Interpretations and deviations

These were fixed before the run, in the checker's docstring.
- **Exit code.** The registration lists exit codes for INVALID (2), FAIL (1) and PASS (0) only. A Branch-A run on valid
  evidence has no gate to fail, so it exits 0. It records and prints the verdict INCONCLUSIVE, which is not PASS.
- **Branch B.** Branch B is not implemented. Invoking the checker without `--abstain-only` is refused as INVALID: HT02b
  must exist before any Branch-B code (registered).
- **Validity rules.** The checker applies the AS03 primitives to the HT01 fields it reads. It also requires a PASS
  verdict and an ABSTAIN decision from HT01, the pinned SHA-256, a clean tree, and the checker committed unchanged at
  HEAD.

**Ledger order (registered).** HT02's first ledger row must come immediately after a node whose ledger row is a
numeric PASS or FAIL, never after another INCONCLUSIVE row (rule (d) of `tools/check-research-node.py`).
