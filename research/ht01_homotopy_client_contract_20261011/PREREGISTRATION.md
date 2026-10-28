# Preregistration: real-client selection for homotopy parallelism (HT01)

Node HT01 (P2, kind `design`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October
10 re-audit (`docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`, section 5; `MATHEMATICS_PORTING_KO.md`, section 7).
Registered on branch `audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`. DAG dependencies: none. The AS03
validator (`tools/evidence_schema_v2.py`, merged) is used by the checker. HT02 depends on this node.

**Claim boundary.**
- Design review and client selection only. No homotopy run, no timing, no speed or span claim. The DAG says the
  numerical command "must be registered after real client selection"; that registration is HT02, gated on this node.
- DAG kill rules, applied as written: no shifted solves in the Fourier client => preserve PP07 ABSTAIN (L-0074), and no
  manufactured many-shift target; do not rerun the cheap diagonal / depth-7 studies (L-0008, L-0050) without a new
  cost mechanism.
- A SELECTED client is a client worth piloting, not evidence that parallel homotopy helps it. No default promotion.

## Question

Is there, among the clients that exist in this repository at `0ce7c32`, one that meets all four preconditions under
which windowed homotopy parallelism could pay: a declared expensive RHS/JVP, shifted solves in its step, a strong
sequential comparator, and complete work/span accounting with a lambda = 1 endpoint certificate? The outcome is
`SELECTED(client)` or `ABSTAIN`, with the per-candidate evidence either way.

## Change

1. **`docs/HOMOTOPY_EXPENSIVE_CLIENT_CONTRACT.md` (new): the expensive-client contract.**
   - Frozen target: the original lambda = 1 stage target `(D - C) K - b0 - h N(K) = 0`; the interval tube and the
     endpoint certificate (`NativeTargetCertificate` or `PreparedStructuredCertificate`; `OperationalDiagnostic`
     never admits) are fixed before any candidate is built.
   - Parallel scope: independent windows / coefficient tasks inside one round; rounds, lambda continuation and
     recurrence levels stay dependent. No claim that workers remove sequential depth.
   - Ledger: `W_total` and `T_par,model` as in MATHEMATICS_PORTING section 7.4 (setup, max task per round, barriers,
     certificate, rejected candidates, fallback, dispatch), plus peak bytes.
   - Cost declaration (`ExpensiveClientDeclaration`): callback identities, declared Ir per RHS and per JVP evaluation
     with its provenance, whether a `BatchRhsFn` is provided. A declaration is a claim that HT02 must verify.
2. **`tools/ht01_client_screen.py` (new):** writes `SCREEN.json` (raw, per candidate and criterion) from static scans
   and committed files only, each input pinned by SHA-256. No cargo, no new run of any solver.
3. **`tools/ht01_selection_check.py` (new):** validates `SCREEN.json` with `evidence_schema_v2` primitives
   (`loads_strict`, exact key sets, finite typed numbers, `emit_invalid`) and applies the decision rule.
4. **No change** to `chart_transport.rs` or `shared_shift_jet.rs`. The contract cites their stated limits (the chart
   module is called by no integrator; the shared jet is one dissipative J, executes no shift in parallel). Source
   changes belong to HT02.

## Candidates (closed list, fixed now)

| id | client | source |
|---|---|---|
| K1 | Fourier path client (PP05/PP06) | `fourier_path_candidate.rs`, `fourier_path_certificate.rs` |
| K2 | R4 / HOM-06 / R-NEXT-06 quadratic family, n in {1, 2, 4, 8, 16} | `transactional_q1_q2.rs` (`QuadraticModel`) |
| K3 | Bateman two-timescale real client (feature `audit2-bateman-authority`) | `audit2_bateman_real_client_research.rs` |
| K4 | Darboux chart client y = x^2 (w + 1/kappa) (L-0043, L-0048) | `chart_transport.rs::chart_integrate` |
| K5 | robertson, hires, van-der-pol-mu1000 | `rodas5p-cli/src/stiff_benchmark.rs` |
| K6 | brusselator-1d-N, N in {30, 40, 50, 160, 200, 500} | `stiff_benchmark.rs` |
| K7 | scientific corpus v2 calibration: 6 families x n in {96, 384, 1536} | `scientific_corpus_v2.rs` |
| K8 | `problems.rs` fixtures (scalar linear, Prothero-Robinson, manufactured, mass, complex Dahlquist, semilinear) | `problems.rs` |

Not screened: the corpus v2 holdouts (Oregonator, Pollution, Medical Akzo, Brusselator-2D n = 512), by
`docs/HOLDOUT_HYGIENE.md` rules 2 and 4. No candidate is added or removed after this commit; a new client needs a new
registration.

## Criteria (all four required; every value or typed reason is recorded per candidate)

- **S1, declared expensive RHS/JVP.**
  - (a) The client carries an `ExpensiveClientDeclaration`. The screen checks this statically.
  - (b) Measured callback share `f`: the Ir share of the `problem-user-code` category in the strongest sequential
    arm's committed callgrind profile (`research/*/PROFILE*.json`), at every profiled rtol. `f >= f_min = 0.348`.
  - Derivation of `f_min` (necessary, overhead-free): the ideal span ratio is `1 - f (1 - r/P)`, with `r = 5` (RHS
    multiplier of the certified transactional arms, L-0008) and `P = 8` (the largest HOM-06 thread count). It must
    reach `1/1.15` (`PAIRED_TIMING_REQUIRED_SPEEDUP`), so `f >= (1 - 1/1.15) / (1 - 5/8) = 0.348`.
  - A candidate without a committed profile has `f = unmeasured`; S1 then rests on (a) alone.
- **S2, shifted solves present.** The strongest sequential arm's counters show W = I - h gamma J solves per attempt
  > 0, with >= 2 independent stage-solve tasks in a round. K1 is decided by PP07 `RESULTS.json` (`q1_solves = 0`).
- **S3, strong sequential comparator.** The cheapest arm by committed same-binary Ir per trajectory among the
  applicable native arms (small, dense v2, colext64, banded, matrix-free U-form), with committed accuracy evidence and
  its ledger row. A comparator that is only JF-GMRES, where a direct arm applies, fails S3.
  - **Applicable arms, closed per candidate** (fixed now; `SCREEN.json` carries this list per candidate, and the
    checker refuses any other arm as INVALID). An arm in the list without a committed profile or accuracy evidence is
    recorded as `unmeasured`; it is never replaced by an arm outside the list.

    | id | applicable native arms |
    |---|---|
    | K1 | none: the client makes no solver call (L-0074); S3 records the typed reason `no_applicable_arm` |
    | K2 | small (n <= 8), dense v2, matrix-free U-form |
    | K3 | small (if its dimension is <= 8), dense v2 |
    | K4 | small (n = 1), dense v2 |
    | K5 | small, dense v2, matrix-free U-form |
    | K6 | dense v2, colext64 (n > 64), banded, matrix-free U-form |
    | K7 | dense v2, colext64, banded (families with a declared band only), matrix-free U-form |
    | K8 | small (fixtures with n <= 8), dense v2 |
- **S4, work/span accounting and endpoint certificate.**
  - (a) `Q2CertificateSource::capability` is `Available` for the client's dimension and structure, with
    `ModelBinding::GeneratedFromModel`. `SampledAgreement` fails, because it is "never a proof" (R4-HOM-DEV-03).
  - (b) `HomotopyWorkLedger` can carry every `W_total` and `T_par,model` term. Missing fields (today: max task per
    round, barriers, rejected-candidate work, fallback work, peak bytes) are listed as HT02 obligations, not failures.

**Decision rule.** `SELECTED(k)` if exactly one candidate meets S1-S4. If several do, the one with the largest
minimum `f` (design choice); an unmeasured `f` ranks below any measured `f`. Otherwise `ABSTAIN`. A candidate with
`f >= 0.348` but no declaration is recorded as a lead for a new registration; it is not selected.

## Commands

    python3 tools/ht01_client_screen.py --output research/ht01_homotopy_client_contract_20261011/SCREEN.json
    python3 tools/ht01_selection_check.py --screen research/ht01_homotopy_client_contract_20261011/SCREEN.json --output research/ht01_homotopy_client_contract_20261011/RESULTS.json
    python3 tools/test_ht01_selection_check.py

Both tools and the contract document are committed before the recorded run. The checker exits 2 with `INVALID` on
malformed or unpinned evidence, 1 on `FAIL`, and 0 on `PASS`.

## Gate (procedure; the decision itself is an outcome, not a pass condition)

**Validity.** INVALID (exit 2) if any of the following holds:
- `SCREEN.json` fails the `evidence_schema_v2` rules, or an input is not pinned by its SHA-256;
- a candidate's arm list differs from the closed list above;
- the checker blob at the RUNS (`SCREEN.json`) source commit differs from the checker that was run, or any recorded
  tree status is dirty.

**PASS** if all of the following hold.
1. Every candidate K1-K8 has a row with S1-S4 values or typed reasons, and no other candidate appears.
2. Every number is recomputed by the checker from pinned committed files: profiles, PP07 `RESULTS.json`, the L-0008
   campaign and the ledger.
3. K1's row cites `q1_solves = 0` and records "PP07 ABSTAIN preserved". No artificial many-shift workload, and no rerun
   of a diagonal or depth-7 study, appears anywhere in the node.
4. The decision equals the rule applied to the rows. The contract document exists at the run commit.

Everything else is **FAIL**. Ledger: PASS with "decision: SELECTED(k)" or "decision: ABSTAIN", as in L-0074.

## Kill and hold

- ABSTAIN stops HT02, which records ABSTAIN without running (its own registration).
- Thresholds are never revised after `SCREEN.json` exists. A later client needs HT01b.

## Prior information (disclosed)

- **Committed `problem-user-code` shares, computed before registration** from SP03, SPD01, SPD02, SPD03, SPD06 and
  SPD09 profiles:
  - small driver: 0.025-0.065 (van der Pol, Robertson, HIRES);
  - banded arm: 0.094-0.098 (Brusselator n = 100 to 1000);
  - dense v2: 0.011-0.059;
  - matrix-free Legacy: 0.086-0.108.
- **K2.** The certified q1/q2 arms spent about 5x the RHS and were Blocked at 0.04-0.17 (L-0008). R-NEXT-06 margins
  were negative for n = 1-8, and about +0.043 solve units per attempt at n = 16 after correction (L-0050).
- **K1.** No solver call of any kind (L-0074).

## Predictions

- **Decision: ABSTAIN** (high confidence).
  - S1(a) fails for every candidate: no client carries a cost declaration.
  - S1(b) fails everywhere measured: the largest share, 0.108, is below 0.348. Even with `r = 1` the ideal ratio is
    `1 - 0.875 x 0.108 = 0.905`, which is a 1.105x model gain, below 1.15.
  - K2 is the only S4 pass (GeneratedFromModel). It fails S1, and the DAG forbids rerunning it.
  - K1 fails S2. K4 is a scalar chart with no linear solve, so it fails S2 and S4.
- **Gate:** PASS. **Leads:** none expected.

## Results

Executed 2026-10-17 under commit `05d6c7cc7b77815970f76c4a57c83b816fd587fd` (ledger `L-0113`).

### 1. Screening Execution and Gate Status
- **Commands:**
  - `python3 tools/ht01_client_screen.py --output research/ht01_homotopy_client_contract_20261011/SCREEN.json`
  - `python3 tools/ht01_selection_check.py --screen research/ht01_homotopy_client_contract_20261011/SCREEN.json --output research/ht01_homotopy_client_contract_20261011/RESULTS.json`
  - `python3 tools/test_ht01_selection_check.py` (13/13 unit and mutation tests passed)
- **Gate:** PASS.
- **Decision:** ABSTAIN.
- **Selected candidate:** None.
- **Leads:** None.

### 2. Candidate Evaluation Summary
- **K1 (Fourier path client):** Fails S1 (no declaration), S2 (`q1_solves = 0`, cites PP07 `RESULTS.json`, PP07 ABSTAIN preserved), S3 (`no_applicable_arm`), S4 (no `GeneratedFromModel` certificate).
- **K2 (Quadratic family n in {1, 2, 4, 8, 16}):** Meets S2, S3 (`dense_v2`, `L-0008`), S4 (`GeneratedFromModel` Available for n <= 2/diagonal; S4(b) `PASS_WITH_HT02_OBLIGATIONS`). Fails S1 (no declaration).
- **K3 (Bateman two-timescale):** Meets S2, S3 (`small`, `L-0032`). Fails S1 (no declaration), S4 (unverified).
- **K4 (Darboux chart client):** Fails S1 (no declaration), S2 (scalar chart, no linear solve), S3 (`no_applicable_arm`), S4 (unverified).
- **K5 (Robertson, HIRES, Van der Pol):** Meets S2, S3 (`small`, `L-0085`). Fails S1 (no declaration, measured callback share min $f = 0.0198 < 0.348$, max $f = 0.0651 < 0.348$), S4 (unverified).
- **K6 (Brusselator-1D N=30..500):** Meets S2, S3 (`banded`, `L-0083`). Fails S1 (no declaration, measured callback share min $f = 0.0577 < 0.348$, max $f = 0.0984 < 0.348$), S4 (unverified).
- **K7 (Scientific corpus v2):** Meets S2, S3 (`colext64`, `L-0086`). Fails S1 (no declaration), S4 (unverified).
- **K8 (problems.rs fixtures):** Meets S2, S3 (`small`, `L-0041`). Fails S1 (no declaration), S4 (unverified).

### 3. Impact on HT02
In accordance with Branch A of `research/ht02_homotopy_parallel_pilot_20261011/PREREGISTRATION.md`, the outcome of HT01 being ABSTAIN directs HT02 to record `ABSTAIN_NOT_RUN` with verdict `INCONCLUSIVE` without executing any parallel homotopy runs.
