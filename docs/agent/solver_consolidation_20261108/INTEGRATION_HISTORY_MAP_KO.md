# VigilODE 통합 연구 이력 지도

| 연구축 | 대표 이력 | 재사용 가능한 결과 | 주장 경계 |
|---|---|---|---|
| 원래 mission/governance | PR176; 28-method inventory; 23-node G0 DAG | G0 target, method identity, maturity gate | scoped completion != production |
| JVP·coupled stage | PR74/75/77 | native JVP-only coupled stage, residual-first correction, reflecting-grid packet/AD | source scoped; JVP is derivative action |
| Jacobian-free/Krylov/cost | PR90–93, PR105–112 | workspace/allocation audit, true residual, Givens/GMRES/GCRO-DR, fused phi/polynomial action | no explicit J != no derivative; work reduction != wall gain |
| RHS-only/Newton-free | PR91/92/117, PR154/156/177 | monotone root/homotopy, inverse-Picard BE, source-owned residual | PR129 owner-substitution counterexample, PR131 frozen source binding; auxiliary BE root != original RODAS stage |
| Algebraic/singular geometry | R-STRUCT, PR93, WP6/WP8 | structural reduction, polynomial chart/resolvent, weighted observable, Bernstein defect | named source/domain; CAD offline |
| Causal depth/graph | PR177/179/182/184/185/186/187 | original-U residual, Cayley-Hamilton, path-forest/core, affine monoid | R6 extra W-solves; R10 538 vs 94 synthetic; C5 negative full-IVP |
| Adaptive switcher | `automatic.rs`, `switching.rs`, SWITCHING.md, `routing.rs` | opt-in DOPRI5↔authorized RODAS, bounded pilots, accepted-window transitions, RHS-only default, declared backend route | not full multi-method adaptive portfolio; pressure trigger is not stiffness proof |
| Symbolic router/dynamics | C8 R2/R4 branch 3336d682 | nonnormal/lognorm observer, finite-state regime alphabet, dwell/hysteresis, graph economics | C8 numeric pending; transition stability proof absent |
| Symbolic execution | PR98 branch bf308589 | exact trace support-only evaluation; guarded prepare/observe ownership-transfer induction | controller-state proof, not physical ODE Markov coding |
| Volterra/Fourier/RVJ | PR135/136; PP05/06/15 | damped causal rank-2 kernel, harmonic polynomial, involution, FFT predictor | PP05 G2/G3 FAIL; PP06 harmful alias FAIL; PP15 no timing; RVJ source-specific |
| Global error/observables | PR158/160/163/167/170/171/172/174 | continuous residual, output authority, Bernstein shape, weighted observable, parameter response | PR158 reference uncertainty; WP9 no out-of-sample win |
| Peer-W/time-parallel | Podhaisky 2002; Schmitt/Weiner 2004; multi-implicit 2005; Kulikov/Weiner 2011/2015; Lang/Schmitt 2026 | independent peer stages, variable-step coefficients, method-specific error control, stiff norm | established prior art; no native VigilODE runtime |
| Spatial/physical client | Gray-Scott 2D; P06 sphere FV; PR175/178/180/181/183 | source-validated MOL and geometry-aware contracts | not generic 3D/sphere integrator evidence |
| Cross-project transfer | Dropbox /ode_solver plan; PHYS70 WU088_HH | directed error/work accounting, exact block solve, SIMD, independent IVP batch | pattern only; no physics/result imported |

## Adaptive switching evidence and lessons

The current `AutomaticOde` is opt-in and primarily compares a persistent RHS-only DOPRI5 path with an explicitly authorized RODAS path. It defaults to derivative-forbidden mode and explicitly does not infer a chart, certify stiffness or choose a thread count. The `SwitchingSolver` provides bounded accepted-window switching, optional derivative challenger, pilots/rechecks/backoff and complete attempt accounting. This is the foundation, not the full target portfolio.

The 2026-10-23 864-configuration single-thread campaign found rapid oscillation JVP-switching about 32.5x slower than whole DOPRI5 under its common endpoint-error cap; FD switching was also slower in that case. A later bounded-pilot study rejected an expensive rapid-rotation JVP entry but exposed a stiff-transition case where an early negative pilot missed later stiffness and became about 3.05x more expensive. Thus:
- rejection/pressure alone is not a stiffness classifier;
- one initial losing probe cannot be trusted for the full horizon;
- regime-specific negative evidence needs bounded expiry and re-probe on measured deterioration/relaxation;
- probe, setup, switch, startup, history reset and discarded/fallback work must be fully charged.

## Symbolic dynamics: three different meanings

1. C8 solver-router symbolic dynamics: finite alphabet of observed numerical regimes and admissible solver/backend changes with hysteresis/dwell. It is not a Markov partition of physical phase space. Numerical stability needs error-transfer-map bounds under allowed words.
2. PR98 symbolic execution: finite guarded labelled transition system of prepare/observe state. Its scoped proof permits snapshot ownership transfer while preserving future guards/decisions/baseline bits. It is controller-state semantics, not physical ODE symbolic dynamics.
3. BKL/Mixmaster cosmology symbolic/topological work in ChatGPT Library is adjacent but distinct. Kasner words, continued fractions, modular geodesics and topological entropy concern physical cosmological dynamics, not generic solver routing.

## Source and archive limitations

- T01 enumerated all 187 PR metadata records and 361 branch refs, and captured PR176's exact 28-row method inventory. Major lineages were inspected; all 187 diffs were not independently reviewed.
- Current context, recent summaries, memory, indexed Library and relevant Dropbox paths were searched. A complete hidden/archived account-wide ChatGPT transcript export was not available.
- Remote Desktop device cosmosapjw was OFFLINE. This session could not edit `/home/cosmosapjw/vigilode` or execute Cargo/Rust/CI locally.
- Dropbox `/ode_solver/implementation_plan.md` is a historical design snapshot, not current code authority.
- PHYS70/WU088_HH is a different project. Its source-bound block/SIMD/ensemble patterns may inspire transfer questions, but its physics/results are not VigilODE evidence.


## Peer-W 우선 계보 복원 (2026-11-09)

| 계열 | 확보된 자료 | 현재 판정 |
|---|---|---|
| PTSW2C | exact variable-step 2-stage coefficients; C(2), Gamma(2), B(3); forward startup; embedded estimator; source A(81.1 degree) / rho-infinity 0.6796; numerical root-locus audit | 첫 order-3 implementation prototype; stability sector not interval-certified; Rust NOT_RUN |
| PTSW2B | exact algebraic 2-stage coefficients; C(2), Gamma(2), B(2); RHS rearrangement; startup; embedded pair; source L(90), rho-infinity 0 | stiff-damping order-2 control; step-count risk; Rust NOT_RUN |
| PTSW3B/3C | source-reported stability/order records | full coefficient/estimator transcription pending |
| p>=5 parallel W-Peer | 2002 p=s,q=s and 2004 p=s-1 families | original G0 candidate; concrete exact table not yet recovered |
| RHS-only implicit Peer | separate homotopy root proposal | conjectural; no stability/order inheritance |

The pinned C5 code contains no Peer integrator/history/session, but provides a bounded Rayon executor, batch operator application, block-GMRES and fair-ab authority. The code-source absence does not erase the independent research/documentation certificates. See PEERW_FIRST_IMPLEMENTATION_PLAN_KO.md.


## PW07 error-certification / global-control lineage

| Prior work | Reusable result | What it preempts | Remaining VigilODE question |
|---|---|---|---|
| Kulikov-Weiner 2011 Peer-W | local/global estimators and adaptive control | first Peer-W global control | heterogeneous method change/history transfer with original-W residual |
| Kulikov-Weiner 2015 ITSP3 | true scaled global-error bound for SDI Peer triple | first Peer true-global control | independent-stage W-Peer and transitions between unlike history spaces |
| Constantinescu 2017 coupled GL | coupled solution/defect global estimator | first solution-defect GL estimator | transactional transfer of historical multistep state plus inexact source residual |
| Arevalo et al. 2021 | adaptive LMM error models depend on step ratios | constant-step LTE as complete variable-step model | Peer-specific controller bounds under switching |
| Lang-Schmitt 2026 | common norm for a selected variable-step Peer family | first variable-step Peer norm estimate | cannot inherit constants for PTSW2B/C or RODAS |

PW07 adds conditional residual/transfer algebra and mock evidence; it does not establish a generic global-error theorem or production method.
