# Published benchmark inventory, 25 October 2026

This inventory was recomputed from published artifacts; no solver or remote-host benchmark was launched. The current branch references below were read through the GitHub connector. The compact packet was additionally checked with its repository validator (exit 0), then task classification and Robertson summaries were independently recomputed by `inventory_extract.py`.

## Current versions

| Branch | Head | Meaning for the report |
|---|---|---|
| research/local-frontier-20261024 | aaf1f834d4c589e734ca7851a56c11bb44f7047a | Frozen original solver and local campaign implementation |
| research/local-frontier-native-diffsol-20261024 | 2a4bf90d1445b67542bdac5030b4c5d10da20445 | Latest published local audit packet and native comparator/reference development |
| research/d2-stage-domain-20261024 | 1c829f206ab67585191231be7f5135fd0d4c804e | Separate stage-domain protection study |
| research/d3-dense-certificate-20261024 | 02060cc9dc4551dc5cf6ec86ad9b53b697a2c3ce | Separate continuous-output error certification study |
| research/d4-effect-journal-20261025 | 3c8877ea0409495752dcfab42426e679e3ba92d3 | Separate accepted-output delivery/transaction study |
| research/d5-callback-contract-20261025 | a01f1c5c78e6b06e23565cd4e482716f99efbe97 | Separate strict callback contract study |
| main | 8d0c79184e09efb5bdadc24a6315c60a71a44264 | Older GMRES integration; not the latest benchmark source |

## Original local suite

The source package used `rtol=1e-6`, `atol=1e-9`, maximum 200,000 attempts, 60 s per process, three timing repeats, threads 1/2/4/8/12, and 256 independent trajectories per small-system ensemble task. Tasks include serial latency, component-parallel RHS evaluation, and ensemble throughput; those quantities are separate comparisons.

The published index contains **1,539 tasks**, comprising **1,458 uninstrumented timing tasks and 81 diagnostic tasks**. The compact index omitted the instrumentation flag. We recovered it by hashing the exact original task key with each Boolean alternative and requiring a unique match to every task ID. All 1,539 identities match. This avoids silently mixing profiling samples into performance counts.

| Outcome | Uninstrumented | Diagnostic | All |
|---|---:|---:|---:|
| Endpoint criterion met |306|17|323|
| Endpoint criterion exceeded |138|8|146|
| Solver reported failure |54|3|57|
| Timed out without result |199|9|208|
| Reference unavailable |761|44|805|
| Total |1458|81|1539|

These are task counts, not 1,539 independent mathematical problems. The suite has 24 model definitions. Repetition, threading and ensemble choices contribute multiple tasks for one model. Dense integration was excluded above dimension 256, so per-method totals also differ. A global success-rate ranking would mix reference coverage and task selection.

All 146 recorded endpoint failures occur in the gyromotion and exponential-chirp cases: 76 gyromotion tasks, 70 chirp tasks, plus 6 chirp timeouts. Five model families satisfy the recorded endpoint criterion throughout their scheduled tasks: cases 6, 7, 16, 19 and 22. Case 14 has 57 timeouts; case 12 has 38 explicit/automatic-switching failures and 19 matrix-free timeouts. These are useful discriminating cases for the next report.

The 199 uninstrumented timeout streams contain headers only. No accepted-step progress or nonempty process-resource report was retained. Their elapsed times are valid censored process durations, but do not tell us whether an integration was progressing or stalled. Of these 199 timeouts, 126 occur in cases 14, 23 and 24.

The 54 uninstrumented solver failures in cases 5 and 12 exhausted the attempt budget with roughly 96–98% accepted attempts. For case 5 DOPRI, the recorded first member has 195,112 accepted steps, 4,888 rejected steps and reaches t=0.645495998875149 of t=2. For KS, DOPRI reaches t=3.757461472896645 with 195,117 accepted steps; automatic switching reaches t=3.6958786369398413 with 191,964 accepted steps. These measurements point to very small sustained steps rather than rejection storms. A new numerical diagnosis can test the stability-limited interpretation; a larger attempt allowance alone is not an algorithmic improvement.

## Robertson comparison with full endpoint values

`audit_packet_20261025/evidence/references/case01/REASSESSMENT_V2.json` contains 72 original solve records: six methods, one warmup and five timed samples, at t=0.01 and t=40. The reference uses precision-refined Wolfram BDF and an independently checked published endpoint. Its stated 1e-12 uncertainty is a numerical estimate. All six methods meet the common 1e-4 maximum-component endpoint criterion in both intervals.

| Method | Median at t=40 (s) | Maximum endpoint error across 6 samples |
|---|---:|---:|
| Native DVODE-BDF |0.000139534|8.957898744998971e-7|
| diffsol BDF |0.000211389|2.946990079744083e-8|
| VigilODE matrix-free RODAS5P |0.000622867|4.5498489575912515e-7|
| VigilODE DOPRI5 |0.01681717|5.424384275087846e-9|
| Native DVODE-Adams |0.021946212|1.2304320904932275e-6|
| VigilODE automatic switching |0.040000129|4.992795288316643e-9|

The matrix-free implicit route is about 27.0 times faster than DOPRI5 and 64.2 times faster than automatic switching on this published t=40 cohort. Native DVODE-BDF and diffsol BDF are about 4.46 and 2.95 times faster than VigilODE matrix-free integration. The accuracy levels differ, and implementations time somewhat different setup scopes; present these as observations under the shared endpoint threshold, not equal-achieved-error performance curves.

The measured followup source is 9b31da59b6d30a092af52357d3bc257b13ce043f, built against frozen VigilODE aaf1f834. It is distinct from the later packet-publishing commit. Raw endpoint/timing values are available in this packet; most other local campaign raw is represented by hashes only. The whole-suite index cannot supply a timing ranking.

## Separate development results

The stage-domain study checks admissibility **before** RHS evaluation. On twelve manufactured trajectories, endpoint-only checking evaluates 507 forbidden stage inputs; both guarded alternatives evaluate zero. A rate-limited retry policy uses 602 attempts and 4,816 vector RHS evaluations, versus 1,032 and 5,323 for unconditional step halving: 41.67% fewer attempts and 9.52% fewer RHS evaluations. The published scalar-linear result covers h lambda in [0,2]; the nonlinear cap is a heuristic protected by the actual stage check. No comparative wall-time result was published.

The continuous-output study integrates diagonal Riccati equations z'_j=-lambda_j z_j-alpha z_j^2, with lambda in {1,20,300}, alpha 0 or 4. It certifies the actual stored cubic/quintic curves via Bernstein bounds on domain and defect, then propagates error with the contraction estimate. At equal full-state error budgets, quintic output reduces stage-plus-curve RHS evaluations by factors from 2.34 to 7.61 relative to cubic output. Analytic Jf evaluations and interval arithmetic add work, so these are operation-count reductions. The eight published trajectories have 1,188 accepted segments and 1,484 evaluated queries. All 48,972 query components were checked against Decimal80 reference solutions; the maximum error/bound ratio is 0.4774119143317735.

The output-delivery study contributes 22 contract tests and 16,384 finite protocol cases; it contains no full trajectory benchmark. The strict callback study contributes 35 tests, 729 manufactured points and 6,561 exact value matches, with zero ODE trajectories. Both improve the evidence for interface robustness; neither supplies a runtime speedup. These separate branches are research consumers and have not been identified as integrated production solver measurements.

Exact published D3 result and independent saved-data review were retrieved into `notes/remote_D3_S2_RESULT.json` and `notes/remote_D3_REVIEW_SUMMARY.json`. D2–D5 reports and D4/D5 result summaries are also copied there. Their complete source URLs follow the immutable branch-head SHA table above and `research/d{2,3,4,5}_.../` paths.

## Implications for the running local benchmark

Preserve the active source, executable, result directories and running processes. The published packet already distinguishes next reference and accuracy campaigns from historical results. Current Julia comparison and new accuracy-cohort execution are recorded as not run in that snapshot. New reference candidates for HIRES, Pleiades and the boundary-layer model do not reclassify earlier runs that did not save the final vector.

Useful followup observations, after the existing run finishes: export complete raw vectors and identity; compare accuracy versus elapsed cost at several tolerances; record phase and amplitude error separately for gyromotion/chirp; capture accepted physical time in separate timeout pilots; and inspect stiff-mode persistence before allowing automatic switching back to explicit integration. For external ports of cases 21 and 23, the authoritative state layout is **interleaved per-cell u,v**, not contiguous halves.

No in-place solver modification is required solely to produce this report. The report should request post-run extraction and separately identified followup measurements rather than silently changing an active benchmark.
