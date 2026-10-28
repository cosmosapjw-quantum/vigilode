# 다음 개발 단계 상세 목록

Machine authority: [NEXT_DEVELOPMENT_DAG.json](NEXT_DEVELOPMENT_DAG.json). 아래 순서는 유효한 위상 정렬의 하나이며, dependency가 없는 항목은 병행할 수 있다. 모든 항목은 아직 제안 상태다.

## R4-ARITH-DEV-01 · Zero-aware ExpBound ordering

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-core/src/transform_bound.rs`.

구현:

- Define one total order for nonnegative normalized ExpBound with ZERO below every positive value. Use it for row/column maxima, final norm choice, class minimization and future comparisons.

검증/완료 조건:

- A=[[0,.25],[0,0]],h=1000.1,b2=[0,1] with stored=weight_phi_vectors passes exact Fraction transform-error enclosure.
- Permuting rows and columns and transposing the nilpotent matrix preserves validity.
- Zero, tiny positive, one, huge exponent bounds obey antisymmetry and transitivity.
- Original R3 nilpotent lost-weight rejection remains unchanged.

중단/거절 조건: Exact transform counterexample closes and existing transform contracts pass; no speed claim.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-HOM-DEV-01 · InverseWitness를 검증된 값과 wire-format으로 분리

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/outward_certificate.rs`.

구현:

- upper/identity를 private으로 하고 validated constructor만 제공
- Deserialize는 UnverifiedWitness로 받고 shape·유한 비음수·operator identity·certificate proof를 검증한 뒤 승격
- 단순 digest 비교를 inverse bound proof로 취급하지 않기; 구조 witness 재생성 또는 approximate inverse V/residual witness 검증

검증/완료 조건:

- zero-upper 및 empty-row native 반례가 typed rejection 또는 재검증 경로로 닫힘
- 정상 diagonal/small/approximate witness와 24 fixture enclosure 유지

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: 재검증 비용과 cache invalidation 비용; witness 자체를 다시 조립할 때 source operator 변경 누락 금지

## R4-HOM-DEV-02 · StageTarget 단계수 계약과 doubling depth를 일치

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/outward_certificate.rs`.

구현:

- s=8 전용이면 생성자와 certificate 진입점에서 제한
- 일반화한다면 strict block structure의 nilpotency 상계 s로 ceil(log2 s) 계산
- zero stage·shape mismatch·부정 반지름을 typed rejection 처리

검증/완료 조건:

- s=1,2,4,8,9,16 exact chain 모두 sound 또는 unsupported rejection
- native 8-stage worker-count bit identity 유지

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: API generalization probe이며 현재 native8 경로의 잘못된 출력 증거는 아님

## R4-HOM-DEV-04 · integration lifetime의 persistent pool과 scratch 재사용

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/transactional_q1_q2.rs`, `crates/rodas5p-integrators/src/outward_certificate.rs`.

구현:

- ParallelExecution을 step context 외부 owner가 보관
- outward upper_matmul도 per-product pool 생성을 제거
- workers1은 순차 경로 유지; ordering과 accumulation 순서 고정

검증/완료 조건:

- outputs/stages/decisions bit identity 또는 사전 합의 error contract 유지
- pool creation 수가 integration당 1 이하; 모든 step의 비용 포함

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: 공유 scratch alias/race와 nested parallel oversubscription

## R4-STAT-DEV-01 · Propagate statistical authority separately from the numeric timing decision

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-fair-ab/src/paired_receipt.rs`, `crates/rodas5p-fair-ab/src/paired_timing.rs`, `crates/rodas5p-cli/src/main.rs`.

구현:

- Add a typed TimingAuthority with study identity, design/estimand version, validated domain and Hold/Admissible/NotEvaluated status.
- Keep diagnostic Promote/Block/Inconclusive and raw receipts immutable; expose a separate admissible_decision consumed by CLI wall_criterion.
- Make absent, mismatched or failed coverage authority yield NotEvaluated and a machine-readable reason; do not erase the historical numeric POLY03 PASS.

검증/완료 조건:

- Current single-case synthetic receipt retains its diagnostic Promote but cannot pass the consumer under current failed study.
- Published POLY03 numeric summary remains preserved and reports effective authority hold.
- Forged or unrelated authority identifiers fail; preregistered future passing domain may authorize only matching designs.

중단/거절 조건: A bool copied from user JSON without an externally selected study/domain binding is not an authority implementation.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-STAT-DEV-02 · Validate every raw session cell before merging

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-fair-ab/src/paired_receipt.rs`, `crates/rodas5p-fair-ab/src/paired_timing.rs`.

구현:

- Require candidate_seconds.len == reference_seconds.len == process_blocks.len == order.len == protocol.pairs for every session cell.
- Require at least protocol.warmups finite nonnegative warmup samples in every record; preserve the existing producer's 2*warmups convention explicitly.
- Bind fixed batch calibration to the first calibration session/case, require each later cell's batch match it, and keep each session's own warmups. Do not demand re-calibration from each later warmup because the campaign intentionally fixes the first batch.
- Check finite ordered start/finish timestamps and nonempty arm/workload identities; retain failures explicitly.

검증/완료 조건:

- Empty or singleton later warmups reject before assessment.
- 29/31 candidate or reference samples in adjacent sessions reject even when the merged flat vector is unchanged.
- Existing genuine fixed-batch campaigns and all five original R3 malformed-case rejections remain valid.

중단/거절 조건: Passing by dropping malformed cells, merging first and validating only totals, or deleting old raw receipts is forbidden.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-TIME-DEV-01 · Use actual BDF step ratios without an absolute time-unit floor

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/bdf.rs:same_step`, `crates/rodas5p-integrators/src/bdf.rs:bdf_step_impl`.

구현:

- Allow the constant-coefficient shortcut only when successive positive finite represented steps are exactly equal, or use variable_bdf2_coefficients unconditionally.
- Keep stability restart on the actual ratio; an equality heuristic must not bypass the ratio limit.
- Retain actual represented step sizes in history and preserve the indexed clock.

검증/완료 조건:

- For boundary powers -40, 0, 10, 40 and the preregistered grid, constant-flow error at every stored state is at most 1e-12 against exact binary64-input rational arithmetic.
- Power-of-two unit changes preserve the dimensionless solution within roundoff and do not change equal/unequal geometry classification.
- An exactly equal-step control keeps its existing coefficient path; unequal steps exercise the variable coefficients; ratio above 1+sqrt(2) exercises restart.
- Fixed, fixed observed, and fixed dense BDF wrappers inherit the same kernel rule.

중단/거절 조건: Do not call an unequal-step constant-coefficient solve successful merely because Newton solves that different residual.

주장 한계: Time-geometry consistency for the reference BDF implementation; not production-comparator promotion.

## R4-TIME-DEV-02 · Bind Richardson error estimation to represented substep geometry

우선순위: P2. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/radau.rs:adaptive_radau_trial`, `crates/rodas5p-integrators/src/adaptive.rs:step_doubling_wrms_error`, `crates/rodas5p-integrators/src/output.rs:split_clock`.

구현:

- Add a geometry-aware estimator receiving h1 and h2, or reject materially unequal splits until its assumptions are satisfied.
- For Radau1, use eta/(1-eta) with eta=(h1/H)^2+(h2/H)^2 under the stated smooth local-error model; record both actual intervals.
- Keep no-representable-midpoint failures explicit; ensure denominator and finite arithmetic checks fail closed.
- Document that the estimate is asymptotic except on declared exact families, not a rigorous ODE-error certificate.

검증/완료 조건:

- The fixed 3-ULP quadratic-primitive witness has analytic fine error 5/9 and estimator-difference magnitude 4/9; the new estimate is 5/9 within binary64 roundoff.
- At atol=0.5 and rtol=1e-12 the witness is rejected or explicitly fails on time resolution, rather than accepting the old 8/9 normalized estimate.
- Equal halves reproduce the previous 1/(2^p-1) formula, and near-equal halves have a continuous correction.
- Both clipped and dense adaptive Radau1 wrappers use the same corrected trial geometry.

중단/거절 조건: Typed invalid/unresolvable split; do not transfer equal-half estimator authority to an unequal partition.

주장 한계: Radau1 local estimator geometry; global tolerance or arbitrary nonlinear-LTE certification remains out of scope.

## R4-TIME-DEV-04 · Make max_step semantics explicit and test the selected cap policy

우선순위: P3. 선행 항목: 없음.

대상: `crates/rodas5p-integrators/src/output.rs:represent`, `crates/rodas5p-integrators/src/adaptive.rs:AdaptiveStepConfig`, `crates/rodas5p-integrators/tests/r3_represented_clock_contracts.rs`.

구현:

- Choose and name StrictRepresentedCap versus AllowClockResolutionSlack; do not document the latter as a hard maximum.
- Under the strict policy return a typed time-resolution failure when no represented advancing endpoint fits the cap.
- Under a slack policy report the actual excess and state that a sub-ULP max_step may become a full-ULP step.
- Replace the current acceptance helper with separate endpoint and cap assertions so a correct endpoint does not satisfy a cap-refusal test.

검증/완료 조건:

- The 0.49,0.75,1,1.5,2 ULP cap grid records success, actual accepted h, and h/max_step.
- Strict policy has no accepted h above cap; slack policy enforces its exact documented allowance.
- Decimal-grid controls remain accurate; a roundoff microstep is not used to disguise policy changes.

중단/거절 조건: Treat a policy ambiguity as such, separately from a wrong state value.

주장 한계: The current one-resolution slack is disclosed in closure notes; this is an API-contract recommendation.

## R4-VERIFY-DEV-01 · Resolve the named native tests omitted by the bounded audit

우선순위: P2. 선행 항목: 없음.

대상: `evidence/runtime/LOCAL_REMAINDER_PLAN.json`, `crates`.

구현:

- Use the exact source/toolchain and retain each first failed/timed-out record.
- Run only the 20 unresolved and 40 budget-skipped named tests on a suitable host with explicit per-target budgets; handle ignored tests/doctests as separate intended gates.

검증/완료 조건:

- Publish per-test status and stderr plus source/dependency identity.
- Never convert timeout or absence of output to PASS; classify any assertion failure and repair only under a separate authorized source task.

중단/거절 조건: Stop once each named remainder has a clear result or explicit environmental blocker; no fresh broad audit loop.

주장 한계: Complete previously bounded regression evidence only; no speed claim.

## R4-ARITH-DEV-02 · Bound reciprocal factorial without forming factorial

우선순위: P2. 선행 항목: R4-ARITH-DEV-01.

대상: `crates/rodas5p-core/src/transform_bound.rs`.

구현:

- Use normalized outward reciprocal recurrence r_k=r_(k-1)/k and nilpotent series term updates. Alternatively reject unsupported orders explicitly, with a documented bound.

검증/완료 조건:

- p=4,30,50,170,171,172 reciprocal bounds enclose exact 1/p!.
- Generated-weight A=0,h=1.1 transform errors are enclosed at all declared supported orders.
- No finite or infinite factorial denominator silently changes a certified upper bound.

중단/거절 조건: Declared order range is sound under exact-rational checks; no unbounded API promise.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-ARITH-DEV-03 · Validate signed stored weights and tolerance domain

우선순위: P3. 선행 항목: R4-ARITH-DEV-01.

대상: `crates/rodas5p-core/src/transform_bound.rs`.

구현:

- For order zero use outward absolute signed difference or reject any stored weight inconsistent with b0. Reject negative/NaN tolerance before magnitude conversion; define positive infinity policy explicitly.

검증/완료 조건:

- A=0,h=1,b0=1,stored0=-1 returns upper>=2 or typed invalid-input error.
- Negative and NaN tolerances never admit a bound.
- Legal zero tolerance admits exactly zero error.

중단/거절 조건: All malformed-input cases fail closed without changing valid source-produced weights.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-HOM-DEV-03 · 모델과 certificate target의 내용 identity 결합

우선순위: P2. 선행 항목: R4-HOM-DEV-01.

대상: `crates/rodas5p-integrators/src/stage_target.rs`, `crates/rodas5p-integrators/src/transactional_q1_q2.rs`, `crates/rodas5p-integrators/src/outward_certificate.rs`.

구현:

- target 전체 coefficient bits와 problem y,h,J,q 및 output scale의 canonical digest 기록
- quadratic model 한 타입에서 RHS/JVP/certificate source를 생성
- 일반 callback은 model-defect enclosure를 요구하고 sampling agreement를 proof로 승격하지 않기

검증/완료 조건:

- coefficient/scale/model parameter 변경마다 stale certificate 거절
- 생성된 동일 모델의 q2 replacement fixture 유지

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: identity는 수학적 enclosure의 대체가 아님

## R4-HOM-DEV-05 · diagonal component-factorized majorant를 Rust research path로 이식

우선순위: P2. 선행 항목: R4-HOM-DEV-01, R4-HOM-DEV-02, R4-HOM-DEV-04.

대상: `crates/rodas5p-integrators/src/outward_certificate.rs`.

구현:

- U와 J의 verified diagonal 구조를 enum으로 표현
- n개의 8x8 stage blocks를 구성하고 성분 축으로 병렬 처리
- actual allocated bytes/nonzero count/directed arithmetic를 분리 계측

검증/완료 조건:

- Fraction full/blocked/serial 동일성 fixtures 통과
- outward path가 serial exact oracle를 enclosure
- memory가 O(ns²), 형식적 dense work가 O(ns³ log s)이며 walltime은 별도 보고

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: diagonal 이외에 구조를 조용히 적용하지 않기; fill-in 있는 banded는 별도증명 필요

## R4-POLY-DEV-01 · Use scaled outward norms in certified polynomial path

우선순위: P2. 선행 항목: R4-ARITH-DEV-01.

대상: `crates/rodas5p-core/src/polynomial_action.rs`.

구현:

- Port and independently review standalone power-of-two scaled norm candidate. Keep mantissa/exponent bounds until a justified binary64 conversion; normalize actual recurrence inputs if needed.

검증/완료 조건:

- [1e300,1e300], [1e-300,1e-300], minimum subnormal, mixed-range and zero norm bounds enclose exact sum of squares.
- Amplitude-scaled polynomial actions either return a valid total bound or typed range rejection, never a spurious zero certificate.
- Separate norm-only improvement from full action dynamic-range support.

중단/거절 조건: Bounded dynamic-range contract passes; unsupported extremes remain explicit.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-STAT-DEV-03 · Decide and version the estimand before changing intervals

우선순위: P2. 선행 항목: R4-STAT-DEV-01.

대상: `crates/rodas5p-fair-ab/src/paired_receipt.rs`, `crates/rodas5p-fair-ab/src/paired_timing.rs`, `docs/TIMING_DESIGN_CONTRACT.md`.

구현:

- Resolve the discrepancy between the contract ID/module description naming case-session medians and prose/code pooling pair logs.
- Choose explicitly between the existing pooled-pair median target and the proposed median of population session-cell medians; state conditions for equality.
- Version inputs and outputs so intervals for distinct estimands cannot be compared or substituted silently.

검증/완료 조건:

- An asymmetric cell-law fixture demonstrates that the two estimands can differ.
- The symmetric additive timing-design family yields the same theta by proof.
- Published R3 evidence remains attached to its original estimand and algorithm.

중단/거절 조건: A new interval that changes the scientific target without a new contract is rejected.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-TIME-DEV-03 · Derive BDF startup estimates for actual order and history on unequal halves

우선순위: P2. 선행 항목: R4-TIME-DEV-01, R4-TIME-DEV-02.

대상: `crates/rodas5p-integrators/src/bdf.rs:adaptive_bdf_trial`.

구현:

- Separate BDF1+BDF1 startup from the BDF1+BDF2 sequence used by the default order-two configuration.
- Derive the coarse/fine defect relation on the actual nodes and expose the geometry/order identity in estimator metadata.
- Do not copy the Radau1 correction to mixed-order BDF startup without that derivation.

검증/완료 조건:

- Exact polynomial primitives of degrees one, two and three distinguish geometry, startup order and predictor effects.
- Equal-step existing contracts pass; odd-ULP and power-of-two unit transformations give the declared asymptotic behavior.
- Record rejected and unresolvable startup trials without advancing history.

중단/거절 조건: Keep this item unresolved until the mixed-order derivation and native tests exist.

주장 한계: Source-level shared-estimator risk only in this audit; no newly executed nonconstant BDF-startup counterexample.

## R4-HOM-DEV-06 · 구조상 불가능한 witness를 speculative work 전에 판별

우선순위: P2. 선행 항목: R4-HOM-DEV-03.

대상: `crates/rodas5p-integrators/src/transactional_q1_q2.rs`.

구현:

- default witness가 n>2 non-diagonal에서 unavailable인 조건을 빠른 capability check로 노출
- 지원하지 않는 구조는 즉시 baseline을 선택하거나 검증된 approximate inverse witness를 제공
- 단순 dimension cutoff와 수학적 reject를 telemetry에서 구별

검증/완료 조건:

- coupled-linear-6에서 certificate unavailable을 얻기 위해 먼저 7 batches를 낭비하지 않음
- 기존 baseline 정확도와 failure semantics 유지

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: p1 operational fast path까지 중단할지 전략은 명시적 별도 arm으로 사전등록

## R4-POLY-DEV-02 · Add explicit total-error admission

우선순위: P2. 선행 항목: R4-POLY-DEV-01.

대상: `crates/rodas5p-core/src/polynomial_action.rs`.

구현:

- Expose an admission method that requires Certified bound<=requested absolute budget; keep truncation budget as a distinct field. EstimateOnly must not satisfy certified admission.

검증/완료 조건:

- A tiny truncation budget may be met while total roundoff bound exceeds it; the admission method rejects this case.
- Cancellation-heavy sums report absolute error and conditioning separately.
- The timing unbounded path remains labeled separately from certified execution.

중단/거절 조건: Consumer semantics cannot confuse truncation success with total-accuracy acceptance.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-STAT-DEV-04 · Port the exact simultaneous session-median interval candidate and validate its domain

우선순위: P2. 선행 항목: R4-STAT-DEV-02, R4-STAT-DEV-03.

대상: `crates/rodas5p-fair-ab/src/paired_receipt.rs`, `crates/rodas5p-fair-ab/src/paired_timing.rs`, `crates/rodas5p-fair-ab/src/timing_design.rs`.

구현:

- Compute binomial tails using exact integers or a rigorously bounded recurrence; avoid overflow in 2^S.
- Return unbounded intervals when the chosen construction cannot meet alpha/C with a finite order statistic.
- Require complete fixed case sets, unique sessions, fixed stopping and explicit iid-session assumptions; reject missing/failing cells.
- Preregister a new coverage and power study at S={6,8,12,24}, C={1,5}, including case_sd=0, identical case responses, dependence, atoms and symmetric heavy tails. Use separate data/test streams and do not select seeds after seeing outcomes.

검증/완료 조건:

- Reproduce all 32 exact design checks, six exhaustive sign enumerations, 12 controls and four invalid-input checks.
- S6/C1 interval has per-case coverage31/32; S6/C5 is unbounded; S8/C5 simultaneous lower bound123/128.
- Coverage claims apply only to the new explicit target and valid session model; power and wall-time outcomes remain separate.

중단/거절 조건: Pair multiplication or extra case labels must never substitute for independent sessions.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-HOM-DEV-07 · 직렬·blocked·doubling 인증의 조건부 비용 선택 및 matched campaign

우선순위: P2. 선행 항목: R4-HOM-DEV-04, R4-HOM-DEV-05, R4-HOM-DEV-06, R4-STAT-DEV-02, R4-STAT-DEV-04.

대상: `crates/rodas5p-cli/src/r3_campaigns.rs`, `crates/rodas5p-integrators/src/transactional_q1_q2.rs`.

구현:

- 동일 candidate에 대해 serial/doubling 인증비용과 enclosure 폭을 분리 측정
- 7-batch saving과 witness/RHS/pool/fallback 전체비용을 함께 기록
- 새 seed·고정 corpus·host quota·thread affinity·warm/cold scope를 사전등록

검증/완료 조건:

- 모든 accepted candidate가 동일 target/accuracy contract를 만족
- p1,pf와 (7-p1)ceil(8/P)+8pf 예산식의 적용가정 공개
- full-path paired evidence가 기준을 넘기 전 SPEEDUP_UNPROVEN 유지

중단/거절 조건: Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.

주장 한계: microbenchmark 인증만 빠르다는 결과로 전체 적분 speedup을 주장하지 않기

## R4-POLY-DEV-03 · Bound Laguerre recurrence propagation before adding certification

우선순위: P3. 선행 항목: R4-POLY-DEV-02.

대상: `crates/rodas5p-core/src/polynomial_action.rs`.

구현:

- Implement the derived scalar recurrence majorant as a correctness baseline; evaluate tightness before optimizing transfer bounds. Preserve EstimateOnly until the full certificate is independently reviewed.

검증/완료 조건:

- Local residual and coefficient rounding are independently enclosed for the registered symmetric domain.
- Degree and scale sweeps report certificate tightness and rejection rate, not just observed error.
- No transfer of symmetric bounds to nonnormal operators.

중단/거절 조건: Either useful certified region is established or looseness is documented and this candidate is rejected.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-POLY-DEV-04 · Benchmark separate certified and unbounded frozen-operator regimes

우선순위: P3. 선행 항목: R4-POLY-DEV-02, R4-STAT-DEV-02, R4-STAT-DEV-04.

대상: `crates/rodas5p-cli/src/r3_campaigns.rs`.

구현:

- Predeclare separate cached/uncached and certified/unbounded arms, identical accuracy targets, operator construction ownership and session design. Include block and vector product counts plus coefficient costs.

검증/완료 조건:

- Timing is evaluated only after statistical-authority gate is resolved.
- All reported speed claims name exact backend and certificate policy.
- Reuse benefits are restricted to unchanged operator/enclosure/h/degree/scale cache identity.

중단/거절 조건: A preregistered study yields either bounded-population evidence or retained negative result; no general speed promotion.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-POLY-DEV-06 · Prototype a separately versioned Leja or scaled-Taylor block-phi backend

우선순위: P3. 선행 항목: R4-POLY-DEV-02.

대상: `crates/rodas5p-core/src/polynomial_action.rs`, `crates/rodas5p-krylov/src`.

구현:

- Define block action/operator epoch/domain-witness interfaces and keep Arnoldi as reference/fallback.
- Choose one initial non-Arnoldi backend; record scaling, degree, all matrix/vector products, coefficient and domain costs.
- Derive a valid truncation/backward-to-forward bound for the declared operator class; do not label a two-small-terms heuristic Certified.

검증/완료 조건:

- Exact-input diagonal and nilpotent cases; independently computed small nonnormal cases with explicit conditioning.
- Capability rejection outside validated domain; no silent symmetry inference from eigenvalues.
- Separate cold/warm/certified timings only after the statistics gates pass.

중단/거절 조건: Reject certification or stop candidate if domain bound cannot be established or total-cost improvement disappears; retain typed EstimateOnly research status.

주장 한계: Literature-supported design proposal; neither implemented nor timed in R4.

## R4-STAT-DEV-05 · Separate kernel timing, certificate cost, cache setup and amortized production work

우선순위: P3. 선행 항목: R4-STAT-DEV-01, R4-STAT-DEV-02, R4-STAT-DEV-04.

대상: `crates/rodas5p-cli/src/r3_campaigns.rs`.

구현:

- Keep existing POLY03 as an unbounded-action kernel campaign with post-hoc independent accuracy verification.
- For certified-path timing, include coefficient enclosure, recurrence propagation, cache validation and rejection/fallback costs in both timed work and counters.
- For frozen symmetric operators compare warm polynomial action with an explicitly amortized cached eigensystem/Schur baseline; preregister reuse count, changing-h policy and setup allocation.
- Move homotopy worker-pool construction outside the step loop before the next timing campaign, then count dispatch, RHS/JVP, certificate and fallback work separately.

검증/완료 조건:

- Every timing arm's work definition matches the output verification path and declared claim.
- Cold, warm, setup and full-certified costs are separately machine-readable.
- No speed claim crosses the still-open statistical authority gate.

중단/거절 조건: A faster unbounded kernel cannot be used as measured evidence that the certified production path is faster.

주장 한계: Named supported domain only; no production/speed promotion from finite fixtures.

## R4-POLY-DEV-05 · Joint Laguerre degree-scale selection under the existing scale cap

우선순위: P3. 선행 항목: R4-POLY-DEV-03.

대상: `crates/rodas5p-core/src/polynomial_action.rs`.

구현:

- Use the derived fixed-degree optimum beta=rho/[2(m+1)-h*rho] when positive, constrained to rho/beta<=16; compare continuous candidates with the fixed grid without relaxing error gates.

검증/완료 조건:

- Analytic tail has the claimed unique minimum for 2(m+1)>h*rho.
- Current exact zero branches remain separate.
- A new preregistered experiment measures recurrence bounds, coefficient cost, error and degree; no unbounded cap lifting from truncation alone.

중단/거절 조건: Retain or reject the candidate based on total-error and work evidence, not degree alone.

주장 한계: derived; not numerically checked or implementation-verified

