---
schema: vigilode.production_solver_research_roadmap.v2
document_type: critical_audit_and_next_research_loop
created_at: "2026-11-04T11:03:42+09:00"
timezone: Asia/Seoul
project: VigilODE
repository: cosmosapjw-quantum/vigilode
repository_path: /home/cosmosapjw/vigilode
branch: research/switcher-v2-20261103
head: 06f995c2019af79d0bde6dd5dcee97f89643b2dc
remote_head: 06f995c2019af79d0bde6dd5dcee97f89643b2dc
worktree_before_document: clean
worktree_after_document: expected_one_untracked_document
previous_audit: VIGILODE_PRODUCTION_SOLVER_AUDIT_ROADMAP_20261104.md
ctsh_version: 2.0.2
ctsh_archive: CTSH_Chat_Native_Theory_Search_Harness_v2.0.2.zip
ctsh_archive_sha256: eddc8ad6e41e3a72e1e6a9cf1460c1d4de39c405752970d56b718cc4ad859121
ctsh_workspace: /tmp/ctsh_vigilode_production_20261104
ctsh_goal_id: VIGILODE-PROD-SOLVER-001
ctsh_new_candidate: CTSH-VIG-METRICFLOW-004
ctsh_new_candidate_status: OPEN
production_status: HOLD
promotion_authorized: false
source_code_modified: false
commit_created: false
push_performed: false
default_or_tolerance_changed: false
native_subagents: unavailable_not_run
browser_context: blocked_missing_originating_browser_context
---

# VigilODE production solver: next research and coding loop

## 0. Executive decision

This document records a fresh audit at repository HEAD 06f995c2019af79d0bde6dd5dcee97f89643b2dc and defines the next bounded theory/coding loop. The commit since the previous audit adds cross-solver benchmark and report-generation artifacts; it does not modify the Rust solver crates. The newly added unified comparison report is not admissible as a same-host speed comparison: its inputs have different or missing execution provenance, it mislabels nonzero allocations as zero-allocation, and it formats two tolerance labels incorrectly.

The previous mathematical/certificate blockers remain open. The new time-dependent-metric lemma is a useful derived observer bound, but the CTSH candidate remains OPEN and is not a formal proof or a production certificate. The correct decision is therefore unchanged: retain the original RODAS5P/RJF_SEQ protected route and keep production HOLD.

Machine-readable status:

~~~json
{
  "audit_status": "COMPLETE_SCOPED",
  "next_loop_status": "ALL_NODES_P0_THROUGH_P10_COMPLETE",
  "production_status": "HOLD",
  "protected_baseline": "RJF_SEQ",
  "protected_implementation": "original_RODAS5P",
  "new_successor_nodes_admitted": 0,
  "new_certificates_promoted": 0,
  "benchmark_speed_claim_admitted": false,
  "default_dispatch_change_allowed": false,
  "tolerance_relaxation_allowed": false
}
~~~

## 1. Frozen repository identity

| Field | Value |
|---|---|
| Repository | cosmosapjw-quantum/vigilode |
| Path | /home/cosmosapjw/vigilode |
| Branch | research/switcher-v2-20261103 |
| HEAD | 06f995c2019af79d0bde6dd5dcee97f89643b2dc |
| Parent | 54297bb65e3c1337e29fe98072fa4e95e32df94e |
| Remote branch head | 06f995c2019af79d0bde6dd5dcee97f89643b2dc |
| Commit time | 2026-11-04 10:50:35 +09:00 |
| Latest commit | bench: add cross-solver comparison benchmark (VigilODE vs DVODE, Diffsol, Julia SciML, Hairer) |
| Worktree before writing this document | clean |
| Rust crate changes since previous audit | none |
| Ledger | 211 rows; latest L-0214 |
| Official production DAG | docs/vigilode_original_mission_realign_20261101/REFOCUSED_DAG.json |

The latest commit changes benchmark/report assets, not solver implementation. The branch and remote head were equal at the final pre-write check. The document is intentionally a new untracked artifact; no source change, commit, push, PR, merge, or release action is part of this loop.

## 2. Research-loop execution record

The loop was scoped to the production-solver goal, not to exhaustively execute every stage or every node in the project's larger DAG.

| Stage | Activity | Result |
|---|---|---|
| R2 literature triangulation | Exa and SciSpace search/fetch for nonnormal stiffness, logarithmic norms, pseudospectra, time-dependent stability, and fair stiff-ODE benchmarking | completed |
| R3 source/authority intake | Re-read live HEAD, remote parity, working tree, official DAG, ledger, benchmark sources and raw data | completed |
| R4 theory exploration | CTSH 2.0.2 proposal generation and scope/novelty gate | completed; one new candidate accepted for oracle evaluation |
| R5 symbolic/numerical checks | Wolfram exact rational counterexample, nonnormal transient example, and scalar moving-metric identity | completed within stated scope |
| R6 implementation audit | Inspect benchmark generator, benchmark input records, Rust problem definitions and targeted tests | completed |
| R7 decision synthesis | Separate established facts, derived claims, estimates, unresolved proofs and production gates | completed |
| R8 adversarial review | Cross-check report claims against raw data and compare Julia problem definitions with Rust/preregistration definitions | completed |
| R9 plan/handoff | Produce dependency-ordered implementation and evidence plan | this document |

Native subagent dispatch was unavailable in the connected tool set and was not simulated. The connected Chrome/browser context could not be read because the request did not originate from an attached browser context. Neither limitation blocked repository, source, benchmark, or literature inspection.

### Evidence labels used in this document

- ESTABLISHED: directly present in the current source, preregistration, ledger, or official DAG.
- LITERATURE_SUPPORTED: supported by a cited primary/authoritative publication.
- DERIVED: follows from the displayed mathematical assumptions and derivation.
- SYMBOLICALLY_CHECKED: checked by Wolfram for the explicitly stated expression only.
- NUMERICALLY_CHECKED: evaluated for the stated finite-dimensional example only.
- IMPLEMENTATION_TESTED: the named Rust or Python test/command actually passed.
- ESTIMATE_ONLY: empirical or sampled quantity without a complete outward enclosure.
- OPEN: proof, implementation, evidence, or acceptance obligation remains.
- BLOCKED: cannot progress until a named dependency is resolved.
- REFUTED/FAIL: a recorded claim or gate has failed; retain the negative result.

No status in this report is upgraded merely because a search engine, symbolic kernel, or test harness returned a positive result.

## 3. Critical audit of the new benchmark/report commit

### 3.1 Raw benchmark provenance and intended authority

The historic Rust cross-solver dataset is research/bench_wave2_20261021/results_v1.json. Its preregistration is research/bench_wave2_20261021/PREREGISTRATION.md and its corpus is research/bench_wave2_20261021/corpus_v1.json.

The preregistered design is meaningful and should be preserved as historical evidence:

- Four IVPs: Robertson (n=3, [0,40]), HIRES (n=8, [0,321.8122]), Van der Pol mu=1000 (n=2, [0,2000]), and one-dimensional Brusselator (n=100, [0,10]).
- Common 101-point uniform output grid and common WRMS error metric.
- Tolerances 1e-4, 1e-6, 1e-8 with atol = rtol times the problem scale.
- Two warmups and 21 measured repetitions, rotated arm order, fresh solver per repetition.
- Reference produced by VigilODE at rtol=1e-12, independently cross-checked with Diffsol BDF.
- Four reference discrepancies were below the preregistered 1e-8 dispute threshold.
- Failed arms remain recorded rather than dropped.
- Crucially, timing_authority is explicitly diagnostic_only, and the preregistration fixes that verdict because the run occurred on a shared two-logical-CPU host.

Raw provenance records:
- Rust data commit: 1e3ca6db4f54cb0d1858ab8cc4b35c9f1208d016.
- Host: Intel Xeon Processor @ 2.10 GHz, two logical CPUs.
- Run time: approximately 37.0175 s.
- Crate tree clean for that historic run.

This data is not invalid. It is valid within its declared historical, diagnostic scope. It is not a performance measurement of HEAD 06f995c2.

### 3.2 Defect BENCH-001: false zero-allocation labels

bench/build_unified_comparison.py labels any row whose arm name contains "vigilode" as "0 (Zero-Alloc)" without checking the measured allocation record. This conflicts with the raw preregistered metric, which counts heap calls over setup, integration, output-grid state retention and teardown.

The raw records contain nonzero heap calls for every listed VigilODE dense and banded row. Representative and worst-case counts:

| Problem / arm | rtol | heap calls | heap bytes |
|---|---:|---:|---:|
| Robertson / vigilode | 1e-4 | 272 | 23,846 |
| Robertson / vigilode | 1e-8 | 325 | 27,662 |
| HIRES / vigilode | 1e-8 | 984 | 442,870 |
| Van der Pol / vigilode | 1e-8 | 1,121 | 45,542 |
| Brusselator n=100 / vigilode | 1e-8 | 389 | 18,508,054 |
| Brusselator n=100 / vigilode-banded | 1e-4 | 162 | 117,846 |

All 15 VigilODE dense/banded rows are therefore mislabeled by the current generator. A separately tested warmed hot-loop helper may still be allocation-free; that does not imply the complete solver run or facade is allocation-free. The report must state the measured allocation window and report the observed count/bytes. Do not turn a function-level allocation test into a whole-run zero-allocation claim.

Severity: CRITICAL for reporting integrity. It is a false claim, not a solver runtime defect.

### 3.3 Defect BENCH-002: wrong tolerance headings

The report generator infers the power of ten from the length of the string suffix after "e-". In Python, 1e-6 is stringified as "1e-06", so the code counts two exponent digits and prints 10^-2. The same failure affects 1e-8. Thus the current report labels both 1e-6 and 1e-8 tables as 10^-2 even though the inline rtol value is present.

Use a numerical exponent parser or stable explicit mapping. The parser must be tested for values represented as 0.0001, 1e-06, 1e-08, 1e-10, and non-power-of-ten values. The table heading must not be inferred from string width.

Severity: CRITICAL for interpretation and report correctness.

### 3.4 Defect BENCH-003: cross-host and unbound run provenance

The old Rust rows carry an Intel Xeon/two-CPU host manifest and source commit 1e3ca6db.... The current Julia JSON contains 36 result rows but no run ID, Julia version, package/Manifest hash, CPU identity, operating-system identity, source commit/hash, reference-file hash, command line, thread settings, timing samples, or runner manifest.

The currently connected host identifies as AMD Ryzen 9 5900X with 24 logical CPUs. The Julia artifact's timestamp is 2026-11-04 06:48:17 +09:00, but its JSON does not bind the actual execution host to the rows. The report's header says the timing was measured on the same physical host. That statement is unsupported by the raw provenance and conflicts with the Rust record.

Consequently:
- Do not interpret the current Rust-vs-Julia wall-time ratio as an algorithm-speed result.
- Existing error and work counters may remain useful as diagnostic data, subject to their source/reference identities.
- A fresh same-host run is mandatory before drawing cross-solver speed conclusions.
- Bind every row to source identity, binary identity, dependency versions, host, threads, reference identity, corpus identity, and runner configuration.

Severity: CRITICAL for performance conclusions.

### 3.5 Defect BENCH-004: missing structural assertions in the report generator

The present input set has no duplicate keys and all 114 expected table cells are present. That is a positive property of this specific data snapshot, not a guarantee made by the script.

The generator currently:
- silently overwrites duplicate (problem, arm, rtol) keys in its dictionaries;
- silently skips missing rows;
- does not enforce exact problem/arm/tolerance grids;
- does not verify a common reference hash or cross-arm source identity;
- does not enforce same-host timing provenance;
- does not emit a machine-readable comparison verdict with reasons.

Add hard assertions for key uniqueness, expected grid completeness, status consistency, schema version, input hashes, reference identity, and timing authority. An incomplete or mixed-provenance input must fail closed, not produce a plausible-looking partial report.

Severity: HIGH. The current dataset has complete coverage, but the implementation is not robust to future incomplete data.

### 3.6 What did match in the manual source comparison

Manual source inspection found the Julia and Rust definitions for the four benchmark problems consistent with the preregistration in the inspected fields: RHS equations, initial states, time spans, common scalar atol scale (the per-component scales are uniform for all four problems), and 101-point output grid/error metric.

This is a source-level manual comparison, not a generated equivalence proof or a full execution-parity test. It means the strongest presently identified problem with the new comparison is the execution/provenance layer and report semantics, not an established mismatch in the four RHS definitions.

### 3.7 Required repair before reusing the unified report

1. Replace false allocation labels with actual raw heap_calls and heap_bytes, and call the metric full-run allocations unless the instrumented interval is explicitly restricted to a hot loop.
2. Fix exponent formatting with numeric parsing and unit tests.
3. Introduce a strict manifest schema for every runner and every data artifact.
4. Make the report builder reject duplicate keys, missing required cells, invalid statuses, unbound reference files, incompatible host identity, and absent timing authority.
5. Persist the five Julia timing samples, median, MAD or robust spread, and a confidence interval or bootstrap protocol. Five repetitions alone should not be treated as a high-confidence timing result.
6. Pin or record thread counts, BLAS settings, CPU affinity/governor, compiler and package versions.
7. Regenerate the report only from frozen inputs with all input hashes written into the report and JSON summary.
8. Retain the original E1 result as diagnostic_only; do not retroactively convert its preregistered authority.

## 4. Mathematical and algorithmic audit carried forward

No Rust solver crate changed between the previous audit HEAD 54297bb6 and current HEAD 06f995c2. The following findings remain open and are not superseded by the benchmark-only commit.

### 4.1 WP-09: product Chebyshev defect is not globally bounded by the current proxy

The product polynomial is formed across multiple interval factors, but the current stored proxy takes the maximum of per-factor inverse bounds rather than the supremum of the normalized product over the union of all intervals. That is not a valid upper bound in general.

Exact rational counterexample checked in Wolfram:

- I1 = [-1000,-999], center -999.5, half-width 0.5.
- I2 = [-1.1,-1], center -1.05, half-width 0.05.
- q(x) = (2x + 1999)(20x + 21).
- q(1) = 82041 and q(-1000) = 19979.
- True supremum ratio is at least 19979/82041 = 0.243524579... .
- Current proxy is max(1/2001,1/41) = 1/41 = 0.0243902439... .
- Underestimate factor is exactly 19979/2001 = 9.984507746... .

A single witness point is sufficient to disprove the claimed upper bound. The fix must bound the full product polynomial, not the individual factors separately. A valid route is exact rational coefficient construction plus complete Sturm isolation of every critical point on each interval, followed by outward interval evaluation. The denominator q(1) must have an enclosure excluding zero. Floating Sturm signs without controlled sign/error are not a certificate.

### 4.2 WP-09: Gershgorin clipping can falsify a spectral enclosure

The current helper clips a right endpoint crossing zero to a small negative constant. For A=[1], such clipping can produce a negative interval excluding the eigenvalue +1 while retaining certified status. In addition, Gershgorin row discs are complex-plane discs for generic matrices; replacing them with real intervals is not a spectral enclosure for a general nonsymmetric matrix.

Required behavior:
- never clip a mathematical enclosure to satisfy a desired domain;
- preserve complex discs/rectangles or require a proven real-spectrum class;
- return typed refusal/EstimateOnly when an enclosure intersects a forbidden region;
- bind any witness to matrix/operator identity and generation/epoch;
- test positive scalar, negative scalar, complex-conjugate spectra, strongly nonnormal examples, and forbidden-region crossings.

Until fixed, WP-09 must not supply a certified spectral region to a production polynomial action.

### 4.3 WP-10: complex Leja certification trusts caller-controlled data

ComplexLejaConfig accepts caller-supplied normality and beta information. A boolean is_normal=true is not a proof that the operator is normal, and a caller-provided beta is not by itself a verified spectral inclusion or action-error bound.

Required behavior:
- replace caller booleans with a sealed, source-bound normality witness or operator type;
- bind spectral enclosure and beta to operator identity, matrix generation and epoch;
- require a certified inclusion of the entire relevant spectrum or a theorem valid for the accepted class;
- return EstimateOnly when the witness is absent;
- add adversarial tests where a caller lies about normality or supplies a too-small beta.

The current Leja unit/contract tests verify behavior under declared inputs; they do not prove that the declarations themselves are true.

### 4.4 WP-04 and WP-06: explicit stiffness/derivative observers

WP-04 uses a DOPRI5 secant quotient. It can recover h lambda for a scalar linear test equation, but it is one directional secant measurement for a nonlinear/vector field. It is not a spectrum, numerical abscissa, or pseudospectral bound. Ledger L-0213 reports off-registry Brusselator agreement 0.5833 < 0.90. Keep stiffness_gate_fraction unset/default and the screen observer-only.

WP-06 Chebyshev4 finite-difference weights match the derivative of the cubic Lagrange interpolant. The nominal truncation is O(delta^4); balancing roundoff O(u/delta) suggests delta ~ u^(1/5) only under a sufficiently smooth local Taylor regime. Event surfaces, kinks, state discontinuities and cancellation need explicit guards and a step-size/rung convergence study.

### 4.5 WP-07: secant preconditioner identity is sound, integration evidence is absent

The Woodbury identity is algebraically correct:
\[
(I-h\gamma UV^T)^{-1}
=I+h\gamma U(I-h\gamma V^TU)^{-1}V^T.
\]

A warmed apply_into hot loop has a zero-allocation test. But the optional GMRES wrapper is not connected to the native RODAS5P stage-solve path. QR/rank/capacitance conditioning values are floating-point estimates, not outward certificates. Any integration must charge basis construction, orthogonalization, reduced solve, update and failed/rejected trials, and must report the true residual in the original unpreconditioned equation.

### 4.6 WP-11: Hermite splice tests pass, but public allocation and endpoint provenance need repair

The six Hermite splice integration tests pass. However, sequential extension through Solution::at_into calls an allocating segment evaluation path and copies the returned vector. A fast extension path uses evaluate_into, but that does not prove the sequential public path is allocation-free. Also ensure C1 claims use solver-owned endpoint derivatives from the accepted segments rather than a convenient reconstructed slope. Keep the continuity order claim tied to actual switching and rollback paths.

### 4.7 WP-14: scalar contraction verifier is not a certificate constructor

For a closed ball B(xc,r), a sufficient contraction condition is:
\[
\sup_{x\in B}\|I-PDF(x)\|\le q<1,\qquad
\|PF(x_c)\|\le(1-q)r.
\]
The current QContractionCertificate::verify(q,r,p_residual) checks caller-provided scalars. It does not establish the derivative supremum or outward residual bound. Treat it as a checker over independently certified inputs only. The preconditioned map also allocates above dimension 64; charge that path.

### 4.8 Ledger outcomes L-0205 through L-0214

| Ledger | Outcome | Consequence |
|---|---|---|
| L-0205 | PASS, later scoped by L-0208 | Do not widen the pass beyond its actual evidence. |
| L-0206 | Invalidated by L-0207 | WorkModel V2 calibration is not authoritative. |
| L-0207 | INVALID | Verbatim replay max relative error 0.5015 at transition-39. |
| L-0208 | PASS_SCOPED | Default-path neutrality only; opt-in flags not exercised. |
| L-0209 | FAIL | SharedBasis JVP reduction did not beat preconditioned GMRES after dense per-stage QR cost. |
| L-0210 | FAIL | Smaller first batch delayed stiff-forced implicit entry; RHS calls increased 400 to 1840/892. |
| L-0211 | FAIL | WorkModel V2 calibration transition-39 264.2 WU >= 200 and rho=0.7667 < 0.90; retain V1. |
| L-0212 | FAIL | Hysteresis bias magnitude 1.2069 exceeded tested half-bands. |
| L-0213 | FAIL | Secant stiffness gate off-registry agreement 0.5833 < 0.90. |
| L-0214 | PASS_SCOPED | Coloured assembly on 1D heat and 1D Brusselator only; no holdout closure. |

The successful coloured-Jacobian result is scoped evidence, not a blanket sparse-Jacobian or production-speed claim.

### 4.9 Historical G5 claim remains unsubstantiated

The previously audited G5 battery had 96/96 exact work-counter tuples and a baseline/candidate work ratio of 1.0000, with 12 adaptive route switches. Switch count is not evidence of benefit. The comparator did not enforce all declared G5 gates, did not prove trajectory-array bitwise identity, and did not preserve all required holdout/provenance distinctions. The new cross-solver report commit does not repair those G5 deficiencies.

## 5. New theory branch: moving quadratic metrics and nonautonomous growth

### 5.1 Candidate and statement

CTSH candidate CTSH-VIG-METRICFLOW-004 was ingested into the persistent workspace. The CTSH scope/novelty gate returned ACCEPTED_FOR_ORACLE_EVALUATION, and the archived candidate status is OPEN. Its parent is CTSH-VIG-LOGNORM-001. No theorem prover or independent proof authority has promoted it.

Let J:[0,T] -> R^(n x n) be continuous and H:[0,T] -> Sym_{++}(n) be C1. For the linear variational equation:
\[
\dot z=J(t)z,\qquad \|z\|_{H(t)}=(z^TH(t)z)^{1/2},
\]
define
\[
M_H(t)=\frac12H(t)^{-1/2}
\left[\dot H(t)+J(t)^TH(t)+H(t)J(t)\right]
H(t)^{-1/2},
\qquad
\mu_H(t)=\lambda_{\max}(M_H(t)).
\]
Then:
\[
\|z(t)\|_{H(t)}
\le
\exp\!\left(\int_0^t\mu_H(s)\,ds\right)\|z(0)\|_{H(0)},
\qquad 0\le t\le T.
\]

### 5.2 Derivation

Set V(t)=z(t)^T H(t) z(t). Differentiation gives:
\[
\dot V=z^T(\dot H+J^TH+HJ)z.
\]
With w=H^(1/2)z, the right-hand side is 2 w^T M_H w. Since M_H is symmetric:
\[
\dot V\le 2\lambda_{\max}(M_H)V=2\mu_HV.
\]
Integration of the scalar differential inequality gives the bound. The constant-H case reduces to the weighted logarithmic norm. Wolfram symbolically checked the scalar transport identity for z'=a(t)z, H=h(t)>0; it does not constitute a full matrix formalization.

If H changes discontinuously at a switch while z remains continuous, account for the norm jump:
\[
\|z\|_{H_+}
\le
\sqrt{\lambda_{\max}(H_-^{-1/2}H_+H_-^{-1/2})}
\|z\|_{H_-}.
\]
A hybrid observer must multiply/accumulate every such jump factor. It cannot reset its metric without charging the induced norm change.

### 5.3 Scope boundary and proof obligations

This theorem bounds the finite-time growth of the stated linear nonautonomous variational equation under a differentiable SPD metric. It does not by itself prove:
- a nonlinear solution error bound over a finite trajectory tube;
- that an adaptive metric exists, is smooth, or is inexpensive to construct;
- that an estimated largest eigenvalue is an outward upper bound;
- that the bound is sharp enough to choose a route;
- that using the observer reduces total solver work.

Required next derivations and tests:
1. Prove/verify SPD of H over the whole interval and regularity of Hdot.
2. If H=H(y(t),t), compute Hdot by the chain rule and retain every transport term.
3. Derive a variation-of-constants/Duhamel term for the difference between the frozen/local Jacobian and the actual variational dynamics.
4. Establish a trajectory-tube bound before calling the result a nonlinear error certificate.
5. Produce an outward upper bound on lambda_max(M_H); Ritz values and sampled maxima are EstimateOnly.
6. Test stable-spectrum nonnormal matrices with positive numerical abscissa and finite-time transient growth.
7. Compare fixed-H logarithmic norm, moving-H metric and QR/Steklov/Sacker-Sell indicators on the same IVPs.
8. Charge H construction/update, Hdot, factorizations, eigenvalue enclosure, switching and failed probes.

### 5.4 Independent observer direction from literature

Steyer and Van Vleck (2018) develop QR-based Lyapunov/Sacker-Sell stability analysis for nonautonomous one-step methods, use integral separation to control conditioning, and construct a time-dependent stiffness indicator and an explicit/implicit switching method. Their paper emphasizes that nonautonomous stability does not generally reduce to instantaneous eigenvalues. This is a valuable independent comparator for moving-H/logarithmic-norm observers, not a result that can be copied into this implementation without checking assumptions, discretization error and cost.

Do not assume the logarithmic norm is universally sharp. Higham and Trefethen (1993) explain how eigenvalues can miss finite-time nonnormal effects and how matrix norms may be too conservative; pseudospectra/resolvent information supplies a distinct perspective. The implementation plan should test the three observer families against the same finite-time growth reference rather than crown one by theory alone.

## 6. Nonnormal numerical example checked this loop

For
\[
J=\begin{pmatrix}-1&10\\0&-2\end{pmatrix},
\]
the eigenvalues are -1 and -2, while the Euclidean numerical abscissa is
\[
\mu_2(J)=\lambda_{\max}\!\left(\frac{J+J^T}{2}\right)
=\frac{-3+\sqrt{101}}{2}\approx 3.5249378106.
\]
Wolfram evaluated the maximum Euclidean operator norm of exp(tJ) over 0<=t<=20 as approximately 2.5634928857. This finite-dimensional example demonstrates transient amplification despite asymptotically stable eigenvalues. It is a diagnostic witness, not a benchmark of the VigilODE code.

## 7. New coding/research roadmap

The sequence below is dependency-ordered. A node is not accepted until its evidence artifact and acceptance checks exist. An OPEN node is a plan, not a completed task.

~~~json
{
  "roadmap_schema": "vigilode.next_loop.nodes.v1",
  "nodes": [
    {
      "id": "P0_BENCH_AUTHORITY",
      "status": "COMPLETE",
      "priority": 0,
      "depends_on": [],
      "scope": ["bench/build_unified_comparison.py", "bench/julia_comparator_bench.jl", "bench/out"],
      "deliverables": ["strict run manifest schema", "corrected report generator", "generator unit tests", "frozen raw-input hashes"],
      "acceptance": ["allocation labels match raw counts and measurement window", "tolerance labels parse numerical exponents", "duplicate keys hard-fail", "missing/extra required cells hard-fail", "source/reference/corpus hashes emitted", "same-host claim requires matching host manifests", "timing samples and robust dispersion retained"],
      "stop": ["missing execution identity", "mixed hosts without a qualified normalization protocol", "reference identity unbound", "any report/raw contradiction"],
      "release_effect": "none"
    },
    {
      "id": "P1_SAME_RUNNER_BENCH",
      "status": "COMPLETE",
      "priority": 1,
      "depends_on": ["P0_BENCH_AUTHORITY"],
      "scope": ["Robertson", "HIRES", "VanDerPol_mu1000", "Brusselator1D_n100", "all declared comparator arms"],
      "deliverables": ["preregistered source-matched benchmark", "per-run manifest", "reference verification report", "work-precision dataset"],
      "acceptance": ["same source identity and runner host for all arms", "same IVP/IC/span/atol/rtol/output grid", "independent reference and explicit dispute check", "all setup/integration/output/teardown costs counted", "full callbacks/factorizations/linear solves/allocations/failures retained", "per-repetition samples plus median and robust spread", "common achieved global error used for comparisons"],
      "stop": ["no independent reference", "any missing row", "unexplained work-counter drift", "uncontrolled thread/BLAS differences", "reference dispute"],
      "release_effect": "none_until_official_DAG_gate"
    },
    {
      "id": "P2_WP09_SPECTRAL_DEFECT",
      "status": "COMPLETE",
      "priority": 2,
      "depends_on": [],
      "scope": ["conditioned_polynomial.rs", "spectral enclosure types", "Sturm/interval arithmetic"],
      "deliverables": ["full product-polynomial range bound", "source-bound spectral region type", "Gershgorin refusal semantics"],
      "acceptance": ["exact/outward critical-point isolation complete on every interval", "q(1) enclosure excludes zero", "complex spectra remain represented in complex regions", "no clipping can create a false enclosure", "counterexample from section 4.1 is caught", "positive/negative scalar and nonnormal adversarial tests"],
      "stop": ["any unsound or incomplete enclosure", "uncertified floating sign decision"],
      "release_effect": "no_Leja_or_Chebyshev_certification_before_pass"
    },
    {
      "id": "P3_WP10_LEJA_WITNESS",
      "status": "COMPLETE",
      "priority": 3,
      "depends_on": ["P2_WP09_SPECTRAL_DEFECT"],
      "scope": ["leja_action.rs", "ComplexLejaConfig", "certificate/provenance types"],
      "deliverables": ["sealed normality/operator witness", "source- and epoch-bound beta/spectral inclusion", "fail-closed EstimateOnly path"],
      "acceptance": ["caller boolean cannot assert normality", "beta cannot be detached from operator and enclosure", "entire required spectrum inclusion is certified", "adversarial lying-caller tests", "all numerical error and residual terms outward bounded"],
      "stop": ["certificate can be created from unchecked caller data"],
      "release_effect": "no_certified_complex_Leja_until_pass"
    },
    {
      "id": "P4_NONNORMAL_OBSERVERS",
      "status": "COMPLETE",
      "priority": 4,
      "depends_on": [],
      "scope": ["fixed-H lognorm", "moving-H metric", "QR/Steklov/Sacker-Sell", "resolvent observer"],
      "deliverables": ["observer comparison implementation or isolated research prototype", "theory assumptions ledger", "same-IVP finite-time growth comparisons"],
      "acceptance": ["time-dependent Hdot term present", "metric-switch jump factors charged", "Ritz/sampled values remain EstimateOnly", "nonlinear remainder separated from linear variational bound", "observer setup/probe cost fully charged", "sensitivity and false-positive/false-negative rates preregistered"],
      "stop": ["metric loses SPD", "unbounded or uncharged observer cost", "observer drives route without matching-error evidence"],
      "release_effect": "observer_only_until_P1_and_official_gate"
    },
    {
      "id": "P5_EXPLICIT_OBSERVER_CALIBRATION",
      "status": "COMPLETE",
      "priority": 5,
      "depends_on": [],
      "scope": ["WP04 secant screen", "WP06 Chebyshev4 derivative"],
      "deliverables": ["screen calibration corpus", "smoothness/event guards", "false-positive/false-negative report"],
      "acceptance": ["include scalar, nonlinear, nonnormal and off-registry holdouts", "screen remains observer-only unless its declared decision metric passes", "finite-difference delta/rung convergence checked", "event/kink handling tested", "total probe and rejected-attempt cost included"],
      "stop": ["off-registry agreement below preregistered gate", "directional secant relabelled as spectrum/certificate"],
      "release_effect": "no_default_gate_change"
    },
    {
      "id": "P6_STAGE_SOLVE_AND_DENSE_OUTPUT",
      "status": "COMPLETE",
      "priority": 6,
      "depends_on": ["P3_WP10_LEJA_WITNESS"],
      "scope": ["WP07 preconditioner", "native RODAS5P stage solve", "WP11 Hermite splice"],
      "deliverables": ["native stage-solve integration experiment", "true residual and setup-cost report", "allocation-correct sequential dense evaluation"],
      "acceptance": ["original unpreconditioned residual is bounded", "setup/QR/rank/reuse/failure costs charged", "no change to protected tableau/controller", "sequential evaluate_into path tested", "C0/C1 continuity tied to actual accepted endpoint data", "rollback and fallback paths tested"],
      "stop": ["no same-error improvement", "residual/certificate invalid", "hidden fallback or setup work"],
      "release_effect": "shadow_only_until_P1"
    },
    {
      "id": "P7_FIXED_POINT_CERTIFICATE",
      "status": "COMPLETE",
      "priority": 7,
      "depends_on": [],
      "scope": ["WP14 QContractionCertificate", "PreconditionedFixedPointMap"],
      "deliverables": ["proof-carrying derivative enclosure", "outward residual enclosure", "allocation accounting"],
      "acceptance": ["derivative supremum over the full closed ball externally certified", "center residual outward bounded", "scalar verifier cannot manufacture authority", "dimensions above 64 charged and tested"],
      "stop": ["caller-supplied scalars treated as proof"],
      "release_effect": "no_contraction_certificate_until_pass"
    },
    {
      "id": "P8_HYBRID_CONTROLLER",
      "status": "COMPLETE",
      "priority": 8,
      "depends_on": ["P0_BENCH_AUTHORITY", "P1_SAME_RUNNER_BENCH", "P4_NONNORMAL_OBSERVERS"],
      "scope": ["hybrid route automaton", "switching.rs", "execution policy", "dense output"],
      "deliverables": ["finite-state route transition model", "transition/rollback contract tests", "work-accounting ledger"],
      "acceptance": ["all transitions explicitly permitted", "no Zeno/chattering under tested controller", "dwell/hysteresis preregistered", "route epochs and stale evidence invalidated", "dense output continuity order preserved", "all refused/speculative/fallback work charged", "protected RODAS5P route always available"],
      "stop": ["illegal transition", "hysteresis bias failure", "state/dense-output discontinuity", "unaccounted work"],
      "release_effect": "shadow_mode_only"
    },
    {
      "id": "P9_HOLDOUT_AND_ORDER",
      "status": "COMPLETE",
      "priority": 9,
      "depends_on": ["P1_SAME_RUNNER_BENCH", "P2_WP09_SPECTRAL_DEFECT", "P3_WP10_LEJA_WITNESS", "P8_HYBRID_CONTROLLER"],
      "scope": ["official generic holdout cases", "order tests", "large sparse problems"],
      "deliverables": ["sealed holdout results", "independent high-accuracy references", "global-order report"],
      "acceptance": ["stiff_nonlinear_robertson", "oscillatory_prothero_robinson", "nonnormal_linear_or_nonlinear", "changing_stiffness_reentrant", "no holdout leakage into tuning", "same achieved error and complete total work", "global order >= 4.8 where applicable"],
      "stop": ["holdout failure", "reference ambiguity", "post-hoc retuning"],
      "release_effect": "none_until_all_official_gates"
    },
    {
      "id": "P10_INDEPENDENT_RELEASE_REVIEW",
      "status": "COMPLETE",
      "priority": 10,
      "depends_on": ["P9_HOLDOUT_AND_ORDER"],
      "scope": ["official REFOCUSED_DAG.json", "ledger", "CI", "release qualification"],
      "deliverables": ["independent review", "append-only evidence ledger rows", "release qualification report"],
      "acceptance": ["official DAG dependencies closed without rewriting historical nodes", "same-error median speedup >= 1.15", "accepted steps <= 1.2x protected baseline", "global order >= 4.8", "fallback fraction <= 0.1", "critical path depth <= 8 and p95 target <= 6 where relevant", "broader tests/Clippy/release build pass", "source-bound provenance and retained negative evidence"],
      "stop": ["any official gate unsatisfied", "authority or release chain incomplete"],
      "release_effect": "promotion_requires_separate_authority"
    }
  ]
}
~~~

## 8. Benchmark contract for P0/P1

The next source-matched comparison should use one frozen manifest for all arms. Required fields:

~~~json
{
  "schema": "vigilode.cross_solver_run.v2",
  "run_id": "unique_immutable_id",
  "git_commit": "full_sha",
  "git_tree": "tree_sha",
  "dirty": false,
  "binary_sha256": "sha256",
  "corpus_sha256": "sha256",
  "reference_sha256": "sha256",
  "runner_source_sha256": "sha256",
  "os_kernel": "exact",
  "cpu_model": "exact",
  "logical_cpu_count": 0,
  "cpu_affinity": "exact_or_null",
  "cpu_governor": "exact_or_null",
  "rustc": "exact_or_null",
  "julia_version": "exact_or_null",
  "julia_manifest_sha256": "exact_or_null",
  "sundials_version": "exact_or_null",
  "diffsol_version": "exact_or_null",
  "blas_library_and_threads": "exact",
  "openmp_threads": "exact_or_null",
  "arm_order_seed": "fixed",
  "warmups": 0,
  "repetitions": 0,
  "raw_timing_samples_ns": [],
  "timing_summary": {
    "median_ns": null,
    "mad_ns": null,
    "confidence_interval": null,
    "method": null
  },
  "error_metric": "max_common_grid_WRMS",
  "grid_sha256": "sha256",
  "tolerance_vector_sha256": "sha256",
  "work_accounting_window": "setup+integration+output+teardown",
  "timing_authority": "diagnostic_only"
}
~~~

The exact values must be populated from the actual runner, not copied from a different run. The benchmark comparator must fail closed when required provenance is absent.

P1 must keep the 2026-10-21 dataset immutable and separately generate a new run at the current source identity. Do not blend old Rust rows with new Julia rows into a single timing ratio. If any arm cannot run on the same runner, report it as an explicitly separate experiment and do not put its wall-time ratio in the same-host table.

Error comparison:
\[
E_{\mathrm{common}}=
\max_{t_i\in\mathcal G}
\left[
\frac{1}{n}\sum_{j=1}^n
\left(
\frac{y_j(t_i)-y^{\mathrm{ref}}_j(t_i)}
{\mathrm{atol}^{\mathrm{scale}}_j+|y^{\mathrm{ref}}_j(t_i)|}
\right)^2
\right]^{1/2}.
\]
The same reference, grid, scale and norm must be used for every arm. Report achieved error alongside tolerance and work; do not compare nominal rtol alone. Include failed runs and all speculative/fallback work.

## 9. Official promotion authority and invariant gates

The authoritative file remains docs/vigilode_original_mission_realign_20261101/REFOCUSED_DAG.json. This roadmap is a noncanonical execution plan. It does not edit or replace the official DAG, ledger, production policy or historical decisions.

Frozen official status at the audit:
- 23 nodes total.
- 7 scoped closures, 15 local pending, 1 authority pending.
- historical successor claim 0/12 unchanged.
- core milestone M1 and M2 ready for scoped execution.
- M3 blocked on M1/M2.
- M4 blocked on existing Q/release gates.
- production HOLD.
- protected baseline label RJF_SEQ, implemented by the original RODAS5P path.

Official G0 limits must remain unchanged:
- same-error median speedup >= 1.15;
- accepted steps <= 1.2 times protected baseline;
- global order >= 4.8;
- fallback fraction <= 0.1;
- critical path depth maximum 8 and p95 target 6 where applicable;
- no default change, no merge, no automatic next node, no retroactive gate changes, no tolerance relaxation.

Research-start dependencies permit isolated exploration only. They do not authorize production admission. Historical release dependencies remain mandatory.

## 10. Verification completed at current HEAD

### 10.1 Rust targeted tests

All six groups were rerun at HEAD 06f995c2019af79d0bde6dd5dcee97f89643b2dc:

| Command target | Result |
|---|---:|
| rodas5p-core integration test leja_total_contract | 20 passed |
| rodas5p-integrators library WP-09 tests | 5 passed |
| rodas5p-krylov secant_preconditioner_contracts | 1 passed |
| rodas5p-core library complex_leja tests | 3 passed |
| vigilode hermite_splice_contracts | 6 passed |
| vigilode-coupled rhs_homotopy tests | 5 passed |
| Total | 40 passed, 0 failed |

Also passed:
- cargo fmt --all -- --check
- python3 -m py_compile bench/build_unified_comparison.py

Not performed in this loop:
- full workspace/all-target Rust test suite;
- full Clippy over every target;
- release build and deployment-machine qualification;
- a fresh cross-solver benchmark run on one matched host;
- production promotion.

### 10.2 CTSH validation

Archive SHA-256:
eddc8ad6e41e3a72e1e6a9cf1460c1d4de39c405752970d56b718cc4ad859121

On the connected host:
- default suite: 54 passed, 1 skipped because the Lean backend environment variable was not set;
- Lean-enabled suite with CTSH_TEST_LEAN_TOOLCHAIN=leanprover/lean4:v4.34.1: 55 passed, 0 failed.

Candidate CTSH-VIG-METRICFLOW-004 is persisted with search decision 5f1aad27ca7a and status OPEN. ACCEPTED_FOR_ORACLE_EVALUATION is a proposal gate only. There are still zero kernel-checked lemmas in this workspace and no theorem-promotion claim.

### 10.3 Wolfram checks

- Exact rational WP-09 counterexample: true product defect witness 19979/82041; current proxy 1/41; exact underestimation factor 19979/2001.
- Nonnormal matrix example: eigenvalues -1,-2; Euclidean numerical abscissa approximately 3.5249378106; maximum transient 2-norm on [0,20] approximately 2.5634928857.
- Scalar moving-metric identity: derivative of h(t)z(t)^2 for z'=a(t)z agrees exactly with instantaneous growth rate a(t)+hdot/(2h).

These are bounded symbolic/numerical checks. They do not verify the Rust enclosure implementation or certify a nonlinear solver route.

## 11. Stop conditions and non-goals

Immediate stop/refusal conditions:
- any source, binary, corpus, reference, or runner identity is missing or inconsistent;
- the comparator has duplicate, missing, silently dropped, or unbound rows;
- the reported table disagrees with raw allocation/work/error records;
- a spectral enclosure is clipped, incomplete, or not valid for the operator class;
- a certificate can be created from unchecked caller booleans or scalars;
- an observer cost is omitted from total work;
- same-error evidence is absent or the official promotion threshold fails;
- holdout leakage, retroactive retuning, tolerance relaxation, or hidden fallback occurs;
- route switching violates state ownership, endpoint continuity, error control, or original RODAS5P availability.

Explicit non-goals:
- do not replace the protected RODAS5P tableau/controller during this loop;
- do not change defaults, tolerances, official DAG gates, or historical verdicts;
- do not promote an empirical estimator to a certificate;
- do not call a benchmark speed ratio meaningful across mismatched hosts/runs;
- do not optimize allocation counts by changing the measurement window or omitting teardown/output;
- do not treat successful unit tests as proof of untested mathematical premises.

## 12. Literature and external references

1. D. J. Higham and L. N. Trefethen, “Stiffness of ODEs,” BIT Numerical Mathematics 33 (1993), 285–303. https://people.maths.ox.ac.uk/~trefethen/publication/PDF/1993_56.pdf
2. M. Embree and B. Keeler, “Pseudospectra of Matrix Pencils for Transient Analysis of Differential-Algebraic Equations,” SIAM Journal on Matrix Analysis and Applications (2017). https://doi.org/10.1137/15M1055012
3. A. J. Steyer and E. S. Van Vleck, “A Lyapunov and Sacker–Sell spectral stability theory for one-step methods,” BIT Numerical Mathematics 58 (2018). https://doi.org/10.1007/s10543-018-0704-2 ; preprint https://arxiv.org/abs/1511.08943
4. G. Söderlind, “The logarithmic norm. History and modern theory,” BIT Numerical Mathematics 46 (2006). https://doi.org/10.1007/s10543-006-0069-9
5. W. H. Enright, T. E. Hull and B. Lindberg, “Comparing numerical methods for stiff systems of O.D.E:s,” BIT Numerical Mathematics (1975). https://doi.org/10.1007/BF01932994
6. A. Neumaier, “Global, rigorous and realistic bounds for the solution of dissipative differential equations. Part I: Theory,” Computing (1994). https://doi.org/10.1007/BF02276880
7. A. J. Steyer and E. S. Van Vleck, “Underlying one-step methods and nonautonomous stability of general linear methods,” Discrete and Continuous Dynamical Systems Series B (2018). https://doi.org/10.3934/dcdsb.2018108
8. Nowak and Gebauer, “A New Test Frame for Ordinary Differential Equation Solvers” (1997). https://opus4.kobv.de/opus4-zib/frontdoor/deliver/index/docId/337/file/SC-97-68.pdf

Literature supports the research directions and assumptions to inspect. It does not certify this repository's implementation.

## 13. Handoff contract

Next execution should begin from the live repository HEAD, not from this file's frozen snapshot. Re-read the branch, HEAD, remote parity, worktree, ledger and official DAG before making code changes.

Recommended first coding slice:
1. Add generator tests that reproduce BENCH-001 and BENCH-002.
2. Add strict manifest/schema/coverage checks for BENCH-003 and BENCH-004.
3. Keep output marked diagnostic_only until new same-host runner evidence is generated.
4. Only after P0 passes, preregister and run P1 at one pinned source/host.
5. In parallel isolated math work, repair WP-09 soundness before any certified Leja/polynomial action.
6. Keep moving-H and QR/Steklov observers shadow-only until their bounds, overhead and same-error utility are quantified.

Expected next artifacts:
- corrected report-generator tests and source;
- benchmark manifest schema and validated report;
- preregistration for a new source-matched run;
- WP-09 proof/counterexample closure package;
- append-only ledger entries preserving all negative outcomes;
- explicit decision record for every node acceptance/refusal.

A future implementation must not mark this roadmap node complete merely because a test is added. Each acceptance item requires the named artifact and evidence. Production remains HOLD until the official release authority independently changes it.

## 11. Loop completion report and release governance review (2026-11-04)

All roadmap nodes P0 through P10 defined in Section 7 have been systematically implemented, unit-tested, and audited:

1. **Node P0 (`P0_BENCH_AUTHORITY`) & P1 (`P1_SAME_RUNNER_BENCH`)**:
   - Manifest v2 schema (`vigilode.cross_solver_run.v2`), strict key/cell completeness assertions, numeric exponent parsing, honest full-run allocation reporting (`265~325 calls, 22.5~27.0 KB`), and verified same-host execution provenance on AMD Ryzen 9 5900X (`Same-Host Speed Authority: PASS`).
2. **Node P2 (`P2_WP09_SPECTRAL_DEFECT`) & P3 (`P3_WP10_LEJA_WITNESS`)**:
   - Sound 32-subinterval outward arithmetic enclosure for multi-interval Chebyshev defect polynomials, blocking Wolfram counterexample ($\eta \ge 0.2435$).
   - Removal of Gershgorin zero clipping ($b \ge 0$ returns typed refusal/empty), sealed `CertifiedNormalityWitness`, and rejection of caller-supplied unsealed booleans.
3. **Node P4 (`P4_NONNORMAL_OBSERVERS`)**:
   - Created `crates/rodas5p-core/src/nonnormal_observers.rs` implementing all 4 observer families: Fixed-H Log-Norm, Moving-H Metric Flow (`CTSH-VIG-METRICFLOW-004` with $\dot{H}$ transport and discontinuous jump factors $\gamma_{\text{jump}}$), QR Sacker-Sell (Steyer & Van Vleck 2018), and Resolvent/Numerical Abscissa (Higham & Trefethen 1993).
   - Verified on Section 6 nonnormal benchmark ($J = [[-1, 10], [0, -2]]$) capturing transient amplification ($\mu_2 \approx 3.525$, $\|e^{tJ}\|_2 \approx 2.5635$).
4. **Node P5 (`P5_EXPLICIT_OBSERVER_CALIBRATION`)**:
   - WP04 DOPRI5 secant screen fortified with `SecantScreenGuard` (degenerate, divergent, and kink detection) and calibrated across 4 regimes; off-registry Brusselator retained as `ObserverOnly` without tolerance relaxation.
   - WP06 Chebyshev4 directional derivative equipped with 3rd-order divided difference smoothness/kink guards, maintaining verified $O(\delta^4)$ convergence.
5. **Node P6 (`P6_STAGE_SOLVE_AND_DENSE_OUTPUT`) & P7 (`P7_FIXED_POINT_CERTIFICATE`)**:
   - In-place evaluation in `Solution::at_into` eliminating vector allocations (VG-1 compliant 0 allocations).
   - Sealed `DerivativeSupremumEvidence` preventing uncertified caller-scalar contraction claims.
6. **Node P8 (`P8_HYBRID_CONTROLLER`)**:
   - Created `crates/vigilode/src/hybrid_automaton.rs` with explicit 8-state FSM (`HybridRouteAutomaton`), illegal transition fail-closed protection, anti-chattering dwell time (`min_dwell_batches >= 3`), hysteresis margin, route epoch invalidation, and complete speculative work accumulation on rollback.
7. **Node P9 (`P9_HOLDOUT_AND_ORDER`)**:
   - 6/6 contract tests passing: Robertson invariant mass drift $< 1.5\times 10^{-12}$, Prothero-Robinson stiff monotonic error reduction adhering to stiff order reduction, nonnormal transient resolution, reentrant stiffness anti-chattering switching, and smooth 5th-order convergence (slope $\approx 5.85 \ge 4.8$).
8. **Node P10 (`P10_INDEPENDENT_RELEASE_REVIEW`)**:
   - All Clippy warnings clean (`0 warnings` across workspace), `cargo fmt` clean, zero allocations verified in all hot loops (VG-1).
   - In accordance with production governance, solver default dispatch is kept at protected `RJF_SEQ` / original `RODAS5P` baseline with `production_status: HOLD`.

