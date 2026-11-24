# DESIGN_RECOMMENDED: Auto route by forward break-even pilots (node V27, judge)

Status: design for the owner's approval under OD-L1-17 ("최대한 hyperparameter 없는 방향으로 설계"). Nothing is
implemented. No file of any worktree was edited. Code read in the detached worktree
`/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-design` at `80ba83881503c4b3253268df61c53b269740d6d3`.

Evidence labels: **[C path:line]** code at that commit (paths below `crates/` unless absolute); **[M]** the V03 part B
records (`research/production_loop_20261116/unified_router/`); **[L]** literature, fetched in this session (section 10);
**[J]** my model `judge_model.py` in this directory (Python re-implementation of both drivers, not the Rust code;
appendix A); **[O]** my own reasoning.

---

## 요약 (소유자용)

**무엇이 바뀌나.** 아무것도 선언하지 않은 유한차분 요청은 차원과 관계없이 DOPRI5로 시작한다. t0에서 궤적 밖 점을
부르는 screen(n+1회), 차원 한계 10, trial 예산 min(1000, max_steps/2), 기준 3.3 B를 모두 없앤다. 대신 DOPRI5가 자기
보폭을 찾은 뒤(처음의 기각 탐색과 5배씩 키우는 구간은 세지 않음) DOPRI5 시도 수가 b, 2b, 4b, …에 이를 때마다 한 번,
"DOPRI5가 최근 b회 시도로 전진한 길이 H를 RODAS5P가 지금 상태에서 한 번의 시도로 덮는가"를 묻는다. 덮으면 그
걸음을 그대로 채택하고 RODAS5P가 끝까지 간다(되돌아가지 않음). 못 덮으면 그 한 시도만 버리고 DOPRI5는 아무 영향 없이
계속한다. b는 RODAS5P 한 시도의 비용이 DOPRI5 b회 시도의 비용을 호출 수와 연산 수 모두에서 넘지 않는 가장 작은
정수로, n과 tableau에서 계산된다(n=1: 3, n=2~3: 4, n=8: 7, n=64: 78).

**두 설계안의 판정.** 공통 핵심(손익분기 pilot, 2배 간격, 하나의 공유 예산, t0 탐침 없음)은 맞다. A안은 pilot 0을
t0에 묶어 DOPRI5의 초기 보폭 증가 구간과 비교하므로 비강성 문제를 RODAS5P로 잘못 넘긴다(모형: 진동자 구간 1에서
DOPRI5의 2.05배, Lotka-Volterra 느슨한 허용오차에서 3.3배). A의 "비강성에서 pilot은 모두 진다"는 주장은 반증되었다.
B안의 곱 규칙은 b를 낮춰 한 걸음 측정의 위상 잡음에 약하고(모형: 비강성 27회 중 6회 잘못 넘김), 구현에 보호 파일
(rodas5p_fast.rs 등) 수정이 필요한데 이를 밝히지 않았다. 권고안은 A의 한-시도 시험과 지배 비교, B의 "보폭을 찾은
뒤에만 잰다"와 앞으로만 가는 커밋을 합친 것이다.

**모형 결과**(레포 tableau와 제어기 상수로 만든 Python 모형, V03b 수치 8개를 정확히 재현): 기본 허용오차 22문제에서
오늘 경로 대비 비용의 기하평균 0.56(호출)과 0.47(연산), 더 나은 단일 방법 대비 최악 비율 25.6배/54배 → 2.0배/1.9배.
Robertson(FD, 구간 40) 6233 → 385회, Van der Pol μ=1000 5698 → 292회, Lorenz-96(n=40) 33467 → 3233회. 손해: screen이
잡던 t0부터 강성인 FD 문제(스칼라 74 → 145, 열방정식 n=8 723 → 945, y'=-3400y 436 → 687), 짧은 비강성 실행 +4~26%.

**사라지는 것**: 10, 1000, 1/2, 3.3, screen과 그 증분·통계·기준, NoBudgetForTrial, "탐침의 치명 오류가 풀이를 끝냄".
**남는 것**: 2배 간격(유일한 수, 관례), b를 정하는 연산 수 관례(2n³/3 등, dense kernel의 상한), 바꾸지 않는 드라이버
제어기 상수. 수 없는 규칙: 보폭 탐색 제외, 지배 비교, 남은 구간이 창보다 짧으면 pilot 생략, 한 방향 전환.

**결정이 필요한 것**: (1) pilot의 FD 열에서 치명 오류: 풀이 종료(권고) 또는 pilot만 버리고 계속, (2) Jacobian·JVP·
band 제공 시 RODAS5P로 시작하는 규칙 유지(권고), (3) RODAS5P→DOPRI5 복귀는 별도 노드, (4) V03b 계약 "DOPRI5 구간
없음"을 작업량 경계로 교체, (5) 오늘 경로의 공개 이름 제거 또는 폐기 예정, (6) 선언 가격 옵션과 dense 연산 계수
(보호 파일)는 나중. 승인 전에는 구현하지 않는다.

---

## 1. Verdict on designs A and B

### 1.1 What both get right (kept)

* No callback before DOPRI5's first stage; the n + 1 off-trajectory screen calls at t0 go [C unified/analyzer.rs:446-517].
* Ski-rental break-even [L Karlin et al. 1988]: ask about RODAS5P only after DOPRI5 has spent one RODAS5P attempt's cost,
  and space later questions geometrically (doubling), so that the questions never cost more than DOPRI5's progress.
* One shared attempt budget `max_steps`; the declared rules (strategy, regime, admission) unchanged.
* The DOPRI5 persistent session with quota-1 advances exists and does not change the step sequence
  [C rodas5p-integrators/src/dopri5.rs:1040-1082; vigilode/src/session.rs:132-147, 338-342;
  vigilode/tests/switching_persistent_explicit.rs:28-50].

### 1.2 Defects found

| # | Design | Claim | Finding | Evidence |
|---|---|---|---|---|
| F1 | A | "Non-stiff: every pilot loses" (B3), oscillator span 10 "+44 (+12%)", span 1000 "+0.3%" | Refuted. Pilot 0 is anchored at (t0, y0) and compares one RODAS5P step with DOPRI5's growth ramp from h0 (factor capped at 5): 1e-3, 5e-3, 0.025, 0.125. RODAS5P covers 0.156 in one step and takes over a non-stiff problem. | [J] A as specified: oscillator span 1: 113 vs Dopri5::solve 55 (2.05x); span 10: 499 vs 379; span 1000: 42612 vs 35461 callbacks and 2.25x operations; non-normal: 509 vs 379; at rtol 1e-3 Lotka-Volterra 3.30x and Lorenz 2.12x. |
| F2 | B | Product rule (minimax regret over the price) for every decision | Correct as minimax under exact rates, but it lowers the threshold (b = 3 for n = 2) and a single measured RODAS5P step is noisy (the error estimate depends on the phase of an oscillation). Applied to the pilot test it hands non-stiff problems to RODAS5P. | [J] product-rule threshold: 6 false handovers in 27 non-stiff runs (oscillator spans 10 and 1000, damped oscillator at two tolerances, non-normal at two); dominance threshold: 0. |
| F3 | B | "Driver sessions and counters are additions"; no protected change listed | Its plan needs RODAS5P sessions for the banded and matrix-free drivers, `last_jacobian_trace()`, and operation counters in the dense kernel. These live in protected blobs. | [C research/production_loop_20261116/CHECKPOINT.json protected_blob_sha1: rodas5p_fast.rs, rodas5p_matrix_free_fast.rs, rodas5p-core/src/work.rs, adaptive.rs]; native session dense only [C rodas5p-integrators/src/rodas5p_fast.rs:2757-2809]. |
| F4 | A | Pilot replays the interval [a, t_D] from a stored past state | Valid comparison, but calls the right-hand side at times before the last accepted time and discards the accepted pilot step. Not needed: a forward pilot from the current state gives the same decisions in the model and keeps its step. | [J] matched-backward variant vs forward variant: same decisions on all non-stiff runs; forward cheaper on Robertson (385 vs 517). |
| F5 | A | Stability witness from DOPRI5's stages schedules useful pilots | No measurable benefit; false witnesses cost pilots. Removed. | [J] A with and without witness: identical handovers on all stiff problems; damped oscillator 19 vs 11 lost pilots. |
| F6 | A | Estimates: heat n = 8 FD "800-850", Robertson span 40 "600-900" | Off in both directions. | [J] A as specified: heat 1038; Robertson 445. |
| F7 | A (B avoids it) | Initial step search counts as evidence | Rejections before DOPRI5 has located its step (h0 too large for the tolerance) are a one-time cost, not a rate. Counting them makes one RODAS5P step at h0 look cheap. | [J] window that counts the search, product threshold: damped oscillator at rtol 1e-8 handed over after 3 DOPRI5 attempts. B's "saturation" rule is the fix (adopted). |
| F8 | A | Karlin et al., Algorithmica 3 (1988) 77-119 | Pages are 79-119. | [L] Springer record. |
| F9 | A, B | Comment at dopri5_stiffness.rs:17-19 says R(x) = -1 | Both right that R = +1 at x = -3.3065678926. Missed: the constant 3.306568 is above the true boundary, so "rounded down ... never above the true boundary" is also wrong (by 1.1e-7). Documentation only. | [O] R(-3.306568) = 1.0000002; root of R - 1 at -3.3065678926349484 (sympy-free numpy check). |

### 1.3 Claims verified (selection; the full list is in the structured result)

| Claim | Verdict | Evidence |
|---|---|---|
| Today's constants: 10, 3.3, 1000, min(1000, N/2), screen with δ_j = sqrt(eps) max(abs(y0_j), 1), criterion L(t1-t0) > 3.3 B, supplied derivative -> RODAS5P, fatal probe ends the solve | confirmed | [C unified/analyzer.rs:26-32, 387-400, 413-421, 446-517; unified/fallback.rs:10-25; unified/router.rs:48-53, 70-72] |
| A fresh finite-difference RODAS5P attempt costs n + 9 right-hand sides (+2 non-autonomous); J, f and f_t reused only after a rejection | confirmed | [C vigilode/src/fd.rs:524-532; rodas5p-integrators/src/rodas5p_fast.rs:26-27, 1166-1194; rodas5p-integrators/src/problem.rs:818-825]; [M] 192 calls in 16 attempts for n = 3 |
| V03b counts | confirmed and reproduced exactly by [J] | scalar FD 74 and 72, y'=-2000y 4023 and 434, y'=-3200y 6043, y'=-3400y 436, heat n = 8 FD 723, Robertson span 40 6233 and 415, Robertson N = 500: DOPRI5 250 attempts to t = 0.3457 |
| Product rule = minimax ratio regret; Γ = sqrt(max/min) | confirmed | [O] algebra in section 3.5 |
| r_max = 3.97841 at 1.020 ± 3.845i (B) | confirmed, with a caveat | these points lie on two islands of the stability region in the right half-plane, disconnected from the main region (main region: max modulus 3.3986 at 120°, real boundary 3.30657, imaginary 0.99719) [O] |
| Hairer DOPRI5: 3.25, 15, 6, NSTIFF 1000; LSODA ratio 5, icount 20, nq > 5; AutoSwitch 10, 3, 9/10, 9/10, 2, false, 5 | confirmed | [L 3, 2, 4] |
| B: projected work units double count finite-difference builds | confirmed | [C vigilode/src/solve.rs:663-665; vigilode/src/registry.rs:371-375, 636-640] |
| A: no protected file needs an edit | confirmed (for A and for this design) | [C CHECKPOINT.json protected_blob_sha1]; `pub use unified::*` [C vigilode/src/lib.rs:215, 254] |

---

## 2. Census: today -> proposed

Kinds: method_constant, numerical_convention, resource_limit, tunable, rule.

| # | Item (today) | Where | Value | Kind | Proposed |
|---|---|---|---|---|---|
| 1 | `EXPLICIT_TRIAL_MAX_DIMENSION` | analyzer.rs:26-28, used :391-396 | 10 | tunable | removed: any n enters the break-even route; n acts only through the cost counts |
| 2 | `DOPRI5_REAL_STABILITY_BOUND` | analyzer.rs:30-32, used :456 | 3.3 (exact 3.3065678926) | method_constant, rounded | removed (no screen) |
| 3 | Stiffness screen | analyzer.rs:446-517; charged router.rs:51-53 | n + 1 calls at t0 | rule | removed |
| 4 | Screen increment | analyzer.rs:492-495 | sqrt(eps) max(abs(y0_j), 1) | numerical_convention | removed from routing (stays in RODAS5P's FD Jacobian, fd.rs:441-455) |
| 5 | Screen statistic L | analyzer.rs:482-515 | max_j 2-norm of Δf / δ_j | rule | removed |
| 6 | Screen criterion | analyzer.rs:456, 511-515 | L (t1 - t0) > 3.3 B | tunable | removed |
| 7 | Recoverable or non-finite probe: screen silent | analyzer.rs:468-487, 496-502 | - | rule | removed |
| 8 | Fatal probe ends the solve, no leg | analyzer.rs:414-421; router.rs:70-72 | - | rule | removed: no preflight call; a pilot starts only at a state DOPRI5 accepted after t0 |
| 9 | `EXPLICIT_TRIAL_ATTEMPT_LIMIT` | fallback.rs:10-12 | 1000 | tunable | removed |
| 10 | Trial budget | fallback.rs:20-25 | min(1000, N/2) | tunable | removed: one budget N for DOPRI5, pilots and RODAS5P |
| 11 | `NoBudgetForTrial` | analyzer.rs:397-400 | N <= 1 | rule | removed |
| 12 | Attempts left for a later leg | fallback.rs:27-30 | N - used | resource_limit | kept |
| 13 | RODAS5P after a numerical DOPRI5 stop | fallback.rs:32-49; router.rs:168-187 | AttemptBudget, StepUnderflow, NonFinite, Controller | rule | kept (AttemptBudget now only at N, reported as MaxStepsReached) |
| 14 | No second method after a fatal or malformed callback | fallback.rs:38-41 | - | rule | kept; applies to pilots too (owner decision D1) |
| 15 | DOPRI5 refuses before integrating -> RODAS5P with N | router.rs:154-159 | - | rule | kept |
| 16 | Strategy RobustStiff, AdaptiveReentrant; HighPerformance = Auto | analyzer.rs:335-354; unified/options.rs:16-31 | - | rule | kept |
| 17 | Declared regime Stiff / Nonstiff | analyzer.rs:356-362, 377-385 | - | rule | kept |
| 18 | DOPRI5 not admitted (mass, dense output, events) -> RODAS5P | analyzer.rs:364-376; selection/frontend.rs:857-871 | - | rule | kept |
| 19 | Supplied Jacobian, JVP or band -> RODAS5P | analyzer.rs:387-390 | - | rule | kept as a start rule (owner decision D2) |
| 20 | Both entry points validated before any callback | analyzer.rs:402-412 | - | rule | kept |
| 21 | Driver controller constants | rodas5p-integrators/src/adaptive.rs:161-177 (protected) | safety 0.9, min 0.2, max 5.0, after rejection 0.9, min_step 1e-14 | numerical_convention (driver) | unchanged; `max_factor` is read by the located test (no new number) |
| 22 | User options | vigilode/src/options.rs:125-136 | rtol 1e-6, atol 1e-9, h0 1e-3, max_steps 100000 | resource_limit | unchanged |
| 23 | RODAS5P FD Jacobian | fd.rs:441-455, 524-532 | n + 1 calls, same increment | numerical_convention | unchanged |
| 24 | RODAS5P LU choice | rodas5p_fast.rs:672-678 (protected) | in-place up to 64 rows, density 0.1 | tunable (driver) | unchanged; the route's operation count is an upper bound of both kernels |
| 25 | Min step for inexact stop reports | unified/legs.rs:116-118 | 1e-14 | numerical_convention | unchanged (reporting) |
| 26 | Switching adapter configuration | vigilode/src/switching.rs:335-377; unified/legs.rs:418-425 | about 20 numbers | tunable | not used by Auto; unchanged |
| 27 | DOPRI5 secant estimate, thread-local batch minimum | dopri5.rs:746-753, 884-887, 1217-1226 | gate 0.8 and guards | tunable (observer only) | not used; the router must not read the thread-local |
| 28 | WorkModel V2 tiers | work_model.rs:41-62 | 50/500/5000 Ir | tunable | not used |
| N1 | Break-even length b | new | max over units of ceil(c_R / c_D) | derived (method constants and conventions) | b = ceil((2n^2 + 54n + 384) / 156) for every n >= 1 |
| N2 | Doubling base of pilot times | new | 2 | tunable (convention) | pilots at DOPRI5 attempts >= b, 2b, 4b, ... |
| N3 | Located window | new | - | rule | DOPRI5 attempts count only after its first accepted attempt that was not growth-capped or followed a rejection |
| N4 | Pilot | new | one attempt | rule (structural minimum) | one fresh RODAS5P attempt from DOPRI5's accepted state with step H |
| N5 | Comparison across units | new | - | rule | dominance: the threshold b makes one RODAS5P attempt no dearer than b DOPRI5 attempts in callbacks and in operations |
| N6 | Horizon | new | - | rule | no pilot when t1 - t_D <= H |
| N7 | Direction | new | - | rule | one-way DOPRI5 -> RODAS5P (owner decision D3) |
| N8 | Fatal callback inside a pilot | new | - | rule | ends the solve, typed CallbackFailed with the accepted prefix (owner decision D1) |

---

## 3. Recommended design: forward break-even pilots (FBP)

### 3.1 Idea

DOPRI5 is the rent, RODAS5P the purchase [L 9]. The route never classifies the problem. It asks one question at
geometrically spaced times, on the trajectory it is integrating: *"DOPRI5 just spent b attempts, at least one RODAS5P
attempt's cost in every unit, to advance a length H. Can one RODAS5P attempt advance H from here?"* If yes, RODAS5P is
cheaper per unit time on the evidence of that window in every unit, at every price of a callback; the step is kept and
RODAS5P continues. If no, one attempt is lost and DOPRI5 continues bit-identically.

### 3.2 Constants (from n and autonomy, no callback; integer arithmetic)

```
c_D = (6, 52 n)                         # callbacks, operations per DOPRI5 attempt (FSAL; 7 calls on the first)
c_R = (n + 9 + e, (2n^3 + 54n^2 + 384n)/3)
      # one fresh FD RODAS5P attempt: n+1 FD calls, f(t,y), 7 stage calls, e = 2 for the FD time derivative
      # of a non-autonomous problem without f_t; operations: LU 2n^3/3, 8 solve pairs 16n^2, FD assembly 2n^2,
      # 64 multiply-adds per component 128n (upper bound: the in-place LU skips zeros)
b   = max(ceil(c_R.cb / c_D.cb), ceil(c_R.op / c_D.op)) = ceil((2n^2 + 54n + 384) / 156)   # the operation unit binds for every n
```

| n | 1 | 2 | 3 | 4 | 8 | 10 | 28 | 40 | 64 | 100 | 1000 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| b | 3 | 4 | 4 | 5 | 7 | 8 | 23 | 37 | 78 | 166 | 13170 |

The counts 26 (DOPRI5: 20 stage and 6 error multiply-adds, [C dopri5.rs:36-90]) and 64 (RODAS5P: 28 entries of A, 28
of C, 8 of b_code, [C fixtures/rodas5p_coefficients_snapshot.json]) are method constants; 2n^3/3 and 2n^2 are textbook
conventions. Because c_R <= b c_D holds componentwise, a pilot never costs more than the DOPRI5 window it is compared with.

### 3.3 Pseudocode

```
AUTO(problem, (t0, t1), y0, opts):                       # N = opts.max_steps
  # declared facts, unchanged, no callback (census 15-20)
  strategy RobustStiff or regime Stiff              -> RODAS5P leg with N
  strategy AdaptiveReentrant                        -> switching adapter (unchanged)
  DOPRI5 not admitted (mass, dense output, events)  -> RODAS5P leg with N
  regime Nonstiff                                   -> DOPRI5 alone with N
  Jacobian, JVP or band supplied                    -> RODAS5P leg with N        # start rule, owner decision D2
  validate both entry points                        -> Err before any callback on a refusal
  return FBP(...)

FBP:
  b := ceil((2n^2 + 54n + 384) / 156)
  Dopri5::with_session(problem, (t0, t1), y0, opts, times = [t0, t1]); attempt telemetry on
  k := 0; pilots_on := true; located := false; run := 0; prev_rejected := false; window := last b attempt records
  loop
    if attempts(DOPRI5) + attempts(pilots) == N: return stop MaxStepsReached{N}  (accepted prefix, all work, pilot records)
    outcome := session.advance_to(t1, quota 1); a := session.last_attempt(); window.push(a)
    if outcome == Complete: return the DOPRI5 result (bits of Dopri5::solve) and the pilot records
    if outcome == Stopped:
        fatal or malformed callback              -> stop CallbackFailed (as today)
        StepUnderflow, NonFinite, Controller     -> RODAS5P leg from the last accepted state with the attempts left (as today)
    capped := a.accepted and not a.clipped and checkpoint.proposed_h == a.trial_h * max_factor     # exact equality
    if located: run += 1
    elif a.accepted and (not capped or prev_rejected): located := true; run := 1
    prev_rejected := not a.accepted
    if pilots_on and run >= b and attempts(DOPRI5) >= b * 2^k:
        (t_D, y_D) := accepted state of the session; first := oldest of the last b attempts
        H := t_D - first.t_before;  if H == 0: H := first.requested_h          # DOPRI5 stuck: the step it tried
        if t1 - t_D <= H: pilots_on := false; continue                       # horizon: DOPRI5 finishes for less
        if no attempt left: continue
        pilot := Rodas5p::solve_observed_bounded(problem, (t_D, t1), y_D, opts with h0 = H, max_steps = 1, [], None)
        k += 1; charge pilot; record (t_D, H, window cost, pilot cost, verdict)
        if pilot ended on a fatal callback: return stop CallbackFailed (owner decision D1)
        if pilot.final_time > t_D:                                           # its one attempt was accepted
            return HANDOVER: legs [DOPRI5 t0..t_D, RODAS5P pilot step t_D..t_D+H],
                   then the RODAS5P leg from (t_D + H, y_pilot) with h0 = H and N - used attempts
        # else the attempt is lost; the session was not touched and continues
```

Notes:
* The located test is B's saturation rule, restricted to the start of the run: the initial search (rejections before
  the first located acceptance) and the growth ramp from a small h0 do not count; after that every attempt counts. A
  variant that also resets the count at every later capped acceptance gave the same decisions on every modelled problem
  [J], so the simpler rule is kept.
* `capped` needs no driver change: the controller returns `trial_h * factor` with `factor == max_factor` exactly when it
  clamps [C rodas5p-integrators/src/adaptive.rs:405, 692]; the session exposes `trial_h` and the next proposal
  [C dopri5.rs:870-882, 1029-1039].
* A pilot is the first step of the RODAS5P leg when it is accepted; nothing is replayed and no accepted work is thrown
  away. The RODAS5P leg rebuilds its Jacobian at the new state, as a continuing RODAS5P run would.
* Recoverable callback errors inside a pilot reject its attempt (lost pilot), as in any RODAS5P attempt.

### 3.4 What the route reads, charges and calls

Reads: DOPRI5's attempt records (accepted, trial and requested step, times), its proposed step, the pilot's final time
and status, counters. All exist. No wall time, no norm of f or J, no eigenvalue, no thread-local.
Charges: every DOPRI5 attempt, every pilot (accepted or lost) and the RODAS5P leg, to the counters and to `max_steps`.
Calls: DOPRI5's stages; per pilot, from an accepted DOPRI5 state with t > t0: n + 1 FD calls (n of them at
y_D + δ_j e_j), f(t_D, y_D), 7 stages, and for a non-autonomous problem two time-derivative calls at t_D ± ε. These are the
calls a direct RODAS5P run makes from that state.

### 3.5 Why dominance and not the product rule

For a threshold k on the step ratio h_R / h_D, RODAS5P is cheaper at price κ iff h_R / h_D >= b(κ), with b(κ) between
b_cb (κ -> ∞) and b_op (κ = 0). Choosing k = sqrt(b_cb b_op) minimizes the worst ratio regret over κ (B's result,
confirmed); choosing k = max(b_cb, b_op) never hands over against any price. Under exact rates the product rule is
better. But the pilot's acceptance is a single measurement whose noise (the error estimate of one step depends on the phase
of the solution) is of the size of the margin: the product threshold handed 6 of 27 non-stiff runs to RODAS5P, the
dominance threshold none [J]. The price of dominance is the grey zone of B4 below (regret <= b_op / b_cb per unit time,
1.7 to 2.5 for n <= 10). A declared price (owner option D6) would replace b by ceil(c_R(κ) / c_D(κ)).

---

## 4. Properties

### 4.1 Without any model

* **U1 Neutrality and the 2 C_D bound.** Pilots never touch the session, so a run without handover has the trajectory,
  final-state bits and DOPRI5 counters of `Dopri5::solve` [C session.rs:338-342]. Pilot k fires at DOPRI5 attempt count
  >= b 2^k and costs <= c_R <= b c_D, so after K + 1 pilots P <= (K + 1) b c_D <= 2^K b c_D <= S (DOPRI5's spend), in
  every unit. Hence cost <= C_D + P <= 2 C_D, and P / C_D <= (1 + log2(A_D / b)) b / A_D -> 0 for long runs.
* **U2 Budget.** DOPRI5 attempts, pilot attempts and RODAS5P attempts sum to at most N; exhaustion is MaxStepsReached{N}
  with the accepted prefix and all work.
* **U3 Callbacks.** None before DOPRI5's first stage. A pilot needs `located`, which needs an accepted attempt, so no pilot
  starts at t0: the review's case F2 (fatal for y > 1, y0 = 1) [M REVIEW.json findings[1]] completes like `Dopri5::solve`
  under either choice of D1. Every evaluation that is not a stage is an FD column or FD time derivative of a RODAS5P
  attempt from an accepted state.
* **U4 Evidence.** A handover happens only after one RODAS5P attempt advanced, from DOPRI5's current state, at least the
  length DOPRI5 advanced in its last b located attempts, which cost at least c_R in every unit.
* **U5 Determinism.** Decisions are functions of integer counters, of f64 values computed by fixed sequential arithmetic
  (DOPRI5 times and steps, exact equality for `capped`), and of the accept or reject of a deterministic RODAS5P attempt.
  No clock, thread, random number or hash order. faer is built without the rayon feature [C Cargo.toml:24], so the dense LU
  is sequential. Same host, binary and inputs give the same route and the same bits. Across hosts a different floating-point
  path (FMA, libm) can move an accept/reject and hence the route, as it can move any adaptive step sequence today: Auto
  goldens, if any are added, are host-pinned. The default path (`Rodas5p::solve`) and its golden files are untouched.

### 4.2 Under model M (stated so that it can be checked)

M1: [t0, t1] is a union of phases; in a phase DOPRI5's located progress per attempt is a constant ĥ_D and RODAS5P accepts
a step h iff h <= h_R. M2: costs per attempt are c_D and c_R.

* **B1 Stiff from t0** (h_R >= b ĥ_D): handover at the first pilot; cost <= C_R + (A_L + b) c_D, where A_L is DOPRI5's
  initial search and ramp. Additive, about one RODAS5P attempt plus the search.
* **B2 One onset at t_s** (before: h_R < b ĥ_D; after: h_R >= b ĥ_D): the first pilot whose window lies after t_s hands
  over; it fires at the first b 2^k >= A(t_s) + b, so DOPRI5 makes at most A(t_s) + 2b attempts after t_s and
  cost <= 2 C_D(t0, t_s) + C_R(t_s, t1) + 2b c_D + P <= 2 min(C_D, C_R) + 2b c_D + P.
* **B3 Non-stiff** (h_R < b ĥ_D throughout): no handover; U1.
* **B4 Grey zone.** Where RODAS5P is cheaper at the true price but h_R < b ĥ_D, the route stays on DOPRI5; its rate there
  is at most b / b(κ) <= b_op / b_cb times RODAS5P's.
* **B5 One-way.** After a handover a later DOPRI5-favourable phase is paid at RODAS5P's rate (bounded by C_R(t_h, t1)).
* **Outside M.** A single accepted RODAS5P step can be phase-lucky (section 3.5). Today's route has no relative bound at
  all: 15x direct RODAS5P on Robertson [M REVIEW.json findings[7]], 25.6x on Van der Pol μ = 1000 and 11x callbacks /
  54x operations on Lorenz-96 n = 40 [J].

---

## 5. Hyperparameters that remain

| Name | Value | Kind | Why it stays |
|---|---|---|---|
| Doubling base of pilot times | 2 | tunable (convention) | Some geometric spacing is needed to keep the pilot count logarithmic and the detection delay proportional to the work done (B2). Base β gives delay <= (β - 1) A(t_s) and log_β pilots; no scale-free optimum. B's model: optimum 1.61 with worst ratio 2.38 against 2.53 at 2 [O arithmetic checked]: weakly sensitive. 2 is "spend after a question = spend before it". |
| Operation counts | 52n; (2n^3 + 54n^2 + 384n)/3 | method_constant (26 and 64 multiply-adds per component, 8 stages) + numerical_convention (2n^3/3 per LU, 2n^2 per solve pair and per FD assembly) | They fix b. Upper bound of the dense kernel: the in-place LU up to 64 rows skips zeros and stops at row extents [C rodas5p_fast.rs:20-24, 672-678], so b is conservative for sparse Jacobians stored dense (heat n = 64: b = 78, although its tridiagonal factorization costs far less than 2n^3/3). Counting the kernel's own operations needs a protected file (owner decision D6). |
| Callback counts | 6 (7 first) and n + 9 (+2) | method_constant (exact) | Recorded; dominated by the operation unit for every n with the counts above. |
| Driver controller constants | 0.9, 0.2, 5.0, 0.9, 1e-14 | numerical_convention (protected driver) | Not router parameters; unchanged; `max_factor` defines "capped". |
| FD increment | sqrt(eps) max(abs(y_j), 1) | numerical_convention | RODAS5P's own rule, unchanged. |
| rtol, atol, h0, max_steps | user | resource_limit | User's; one budget for everything. |
| Rules without a number | located window; dominance; one attempt per pilot; horizon; one-way; start rule; fatal in a pilot ends the solve; numerical DOPRI5 stop -> RODAS5P | rule | Structural; D1 to D3 are offered to the owner. |

What disappears: 10, 1000, 1/2, 3.3, the screen with its increment, statistic, criterion and three outcomes, and
NoBudgetForTrial. B's product rule, exclusion radius 3.9785 and measured price are not adopted; A's witness and
stability polynomial are not adopted.

---

## 6. Scenarios (model [J], default tolerances; charged right-hand sides, operations in parentheses where they decide)

| Scenario | Problem | D | R | Today | FBP | A as specified |
|---|---|---|---|---|---|---|
| Stiff from t0, screen fires today | y' = -1e6 (y - cos t), FD | > 1.8e6 | 72 | 74 | 145 | 103 |
|  | heat n = 8 FD, span 100 | 58429 | 714 | 723 | 945 | 1038 |
|  | y' = -3400 y | 6631 | 434 | 436 | 687 | 453 |
|  | heat n = 64 FD, span 10 (today n > 10) | 314131 | 2993 | 2993 | 3328 (ops 0.98x today) | - |
| Stiff, screen blind today | y' = -3200 y | 6259 | 434 | 6043 | 687 | 453 |
|  | y' = -2000 y | 4021 | 434 | 4023 | 687 | 453 |
|  | Van der Pol μ = 1000, span 1 | 5695 | 223 | 5698 | 292 | 327 |
| Late onset | Robertson FD, span 40 | 216973 | 415 | 6233 | 385 | 445 |
|  | flame δ = 1e-3 | 2455 (21268) | 1008 (15371) | 2457 | 1263 (11597) | 1037 |
| Stiff test set | HIRES / OREGO | 64543 / capped | 2642 / 10821 | 2651 / 10825 | 2444 / 10870 | 2554 / 10923 |
| Non-stiff | oscillator spans 1 / 10 / 1000 | 55 / 379 / 35461 | 77 / 467 / 42580 | 58 / 382 / 41406 | 55 / 423 / 35582 | 113 / 499 / 42612 |
|  | Lotka-Volterra, Lorenz, Brusselator, VdP μ = 1 | 6055, 2125, 979, 1435 | 2.4-3.0x D | 1.00-1.02x D | 1.01-1.07x D | same as FBP |
|  | Arenstorf orbit | 1291 | 4338 | 4343 (screen) | 1369 | 1369 |
|  | Lorenz-96 n = 40 | 3037 (1.05e6) | 33467 (5.7e7) | 33467 (n > 10) | 3233 (1.36e6) | - |
| Oscillatory / non-normal | damped ω = 100, ζ = 1e-3 | 35461 | 42572 | 42575 (screen) | 35582 | 35582 |
|  | [[-1, 1e3], [0, -2]], span 10 | 379 | 484 | 487 (screen) | 423 | 509 |
| Fatal callback | F2: fatal for y > 1, y0 = 1 | 49 | fails at t0 | fails at t0 | 49, success | - |
|  | N2: fatal only at FD columns (y1 = 1 conserved, stiff y2) | 2161 | fails | fails at t0 | fails at the first pilot (D1 terminal); 2161 with D1 continue | - |
| Budget | Robertson FD span 40, max_steps 500 | - | - | 1769 (272 attempts) | 385 (36 attempts) | - |

Summary over the 22 default-tolerance problems: geometric mean FBP/today 0.56 (callbacks), 0.47 (operations); worst
ratio to the better single method today 25.6 / 54.3, FBP 2.01 / 1.86. At rtol 1e-3 and 1e-8 (12 and 13 problems): geometric
means 0.47 / 0.44 and 0.56 / 0.48; FBP's worst 1.93 and 2.01. Non-stiff losses come from lost pilots and are largest on
short runs (+0.3 to +26% in callbacks, up to +46% in operations at rtol 1e-3); no non-stiff run was handed over (27 runs).

Robertson in detail: the first pilot is lost, the second (after 8 DOPRI5 attempts, t = 0.0018) is accepted, and the run
takes 36 attempts in all (8 DOPRI5, 2 pilots, 26 RODAS5P). It is below direct RODAS5P (415 calls, 35 attempts) because
RODAS5P starts at t = 0.0018 with the pilot's step instead of its own ramp from h0. The oscillatory problems: DOPRI5's resolved steps stay inside its region; with b = 4 (n = 2) one
RODAS5P step never covers four DOPRI5 steps in these runs. The fatal cases: no pilot is made at t0, so F2 passes; N2 shows
the residual exposure that owner decision D1 settles.

---

## 7. Code changes and implementation plan

No protected blob is edited [C CHECKPOINT.json protected_blob_sha1]; `lib.rs` already re-exports `unified::*`.

| File | Change |
|---|---|
| `crates/vigilode/src/session.rs` | crate-private `set_attempt_telemetry` and `last_attempt` pass-through on `ExplicitSession` (the native session has both [C dopri5.rs:1005-1016]); `checkpoint().proposed_h` exists. |
| `crates/vigilode/src/unified/breakeven.rs` (new) | `break_even_length(n)` in integers; the loop of 3.3; `PilotRecord { t, h, window_attempts, window_cost, pilot_cost, verdict, fatal }`. |
| `unified/analyzer.rs` | drop rules 5-6 and the screen (`StiffnessScreen`, `stiffness_screen`, `NoBudgetForTrial`, `DimensionAboveTrialLimit`, `StiffnessScreen`/`UndeclaredRegimeTrial`/`PreflightCallbackFailed` bases); add `RouteBasis::BreakEven { b }`; the analysis makes no callback. |
| `unified/fallback.rs` | drop `EXPLICIT_TRIAL_ATTEMPT_LIMIT`, `explicit_trial_attempts`; keep `remaining_attempts`, `follows_explicit_stop`. |
| `unified/router.rs` | `explicit_then_fallback` becomes `break_even`: DOPRI5 leg from session snapshots (`snapshot`, `stats_between` exist [C session.rs:174-237]), pilots charged into `ws.work_counters`, RODAS5P leg via `registry_leg` from (t_D + H, y_pilot) with h0 = H; preflight charging and the preflight stop path go. |
| `unified/legs.rs`, `unified/solution.rs` | `VigilodeSolution::pilots`; a route-history entry for the handover with t_D, H and both costs; `UnifiedStop::Preflight` kept only for source compatibility (D5). |
| `unified/mod.rs`, `unified/options.rs` | documentation. |
| `rodas5p-integrators/src/dopri5_stiffness.rs` | optional comment fix (R = +1; 3.306568 is not rounded down). Not needed by the route. |
| Tests | new `tests/unified_breakeven_contracts.rs`; in `tests/unified_router_contracts.rs`: `p1_stiff_contractive_*_without_jacobian_is_not_explicit` (:573, :586) keep the work bound W and replace "no DOPRI5 leg" (D4); `p1_preflight_callbacks_are_charged_*` (:767, :811, :841) and `p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input` (:873) become pilot-charging and "no preflight call" tests; `p2_stiffness_screen_is_a_charged_lower_bound` (:1885) and `p2_fallback_rule_and_trial_budget` (:1953) are removed or rewritten; :1186, :1378, :1568, :1711 read screen names; `p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg` (:965) keeps its order and budget assertions, with the handover counted in `fallback_steps`. |

Steps (after the owner approves this design):
1. Pre-register section 8 (PREDECLARED.json of the implementation node), including the model predictions of section 6
   as expectations without a rule.
2. Failing-first tests against the base API: F2 succeeds under Auto; Lorenz-96 n = 40 under Auto has no RODAS5P leg and
   costs at most 2x `Dopri5::solve`; Robertson FD span 40 under Auto charges at most 2x direct RODAS5P plus 2b c_D + pilots
   (B2 form); oscillator span 1000 has no RODAS5P leg; Arenstorf has no RODAS5P leg. Each fails today by assertion.
3. `session.rs` pass-through, with a test that telemetry and quota-1 advances keep the bits of `Dopri5::solve`.
4. `breakeven.rs`, then analyzer, fallback, router, legs, solution; tests for U1-U5.
5. Contract changes per D4 and D5; full facade suite, clippy, fmt, doc; default-path golden files unchanged.
6. Evaluation runs (counters only), receipt, independent review.

---

## 8. Evaluation plan (ready for pre-registration)

**Arms.** T = today's Auto at 80ba8388. N = FBP. References: D = `Dopri5::solve`, R = `Rodas5p::solve`, same options.
Exploratory, reported without a rule: N-prod (product threshold ceil(sqrt(b_cb b_op))), N-cont (D1 = continue).

**Problems** (finite differences unless stated; spans fixed here):
* Stiff or late stiff: P_scalar_fd; y' = -λ y, λ in {2000, 3200, 3400}, span 1; P_heat_fd (n = 8, span 100); Robertson
  spans 40 and 1e5; HIRES; OREGO; Van der Pol μ = 1000 (y1' = y2, y2' = μ(1 - y1^2) y2 - y1, y0 = (2, 0), span 1);
  flame δ in {1e-2, 1e-3, 1e-4}, span 2/δ; heat n = 64 FD span 10; k(t) = 1e4 e^{-20t} + 1, y' = -k(t)(y - sin t),
  span 10 (stiffness that disappears; report only, B5).
* Non-stiff: oscillator spans 1, 10, 1000; Lotka-Volterra span 100; Lorenz span 10; Arenstorf one period; Brusselator
  (n = 2) span 20; Van der Pol μ = 1 span 20; damped oscillator ω = 100, ζ = 1e-3, span 10; [[-1, 1e3], [0, -2]] span 10;
  Lorenz-96 n = 40 span 10; pleiades (n = 28) span 3.
* Callbacks, budget, declared: F2 (fatal for y > 1, y0 = 1); N2 (y1' = 0, y1 = 1, fatal for y1 > 1; y2' = -1000 y2);
  P_fatal; P_blowup; P_mass; P_heat_dense, P_scalar_dense, P_heat_jvp, P_heat_banded (start rule); declared Stiff and
  Nonstiff on two problems; Robertson span 40 with max_steps 500 and 50.
* Tolerances: (1e-6, 1e-9) for all; (1e-3, 1e-6) and (1e-8, 1e-11) for the non-stiff set, P_scalar_fd, P_heat_fd,
  y' = -3200 y, Robertson span 40 and Van der Pol μ = 1000.

**Measures** (counters only, no wall time): charged callbacks and callback entries counted in the closures (by kind);
operations by one formula from the counters for all arms (52n per DOPRI5 attempt; 128n per RODAS5P attempt, 2n^3/3 per
factorization, 2n^2 per solve pair and per FD build); attempts; pilots (number, lost or accepted, H, window cost, pilot
cost); handover time and DOPRI5 attempts before it; status and stop; error at t1 against a reference (exact, or RODAS5P at
rtol 1e-12, atol 1e-14) in the user's weighted norm; final-state bits of three runs (two in one process, one fresh); route
digest.

**Discriminators** (comparisons; the factor 3 and W are existing frozen values):
* E1 economy: over all cases where T and N succeed, geometric mean of cost(N)/cost(T) < 1 in callbacks and in operations;
  and max over cases of cost/min(cost(D), cost(R)) is smaller for N than for T, in each unit.
* E2 no new failure: no case where T succeeds and N does not, at the same max_steps.
* E3 neutrality: every N run without handover has the final-state bits and the DOPRI5 counters of D, plus exactly the
  pilots' counters, and cost <= 2 cost(D) in each unit; every run decided by a declared fact equals T bit for bit.
* E4 budget: attempts over all legs and pilots <= max_steps; exhaustion reports MaxStepsReached{max_steps}.
* E5 callbacks: no entry before DOPRI5's first stage; no entry with t = t0 except DOPRI5's stages; every entry that is
  not a stage is an FD column or time-derivative call of a RODAS5P attempt that starts at an accepted state; uncharged = 0.
* E6 determinism: the three runs have equal digests, counters and bits.
* E7 accuracy: err(N) <= 3 max(err(D), err(R)) where both succeed.
* E8 callback cases: F2 succeeds under N; P_fatal reports CallbackFailed{Rhs, "sensor offline"}; N2 as D1 says.
* E9 V03b bounded work: P_heat_fd <= 5890 and P_scalar_fd <= 181818 charged right-hand sides, with a recorded handover.
* E10 pilot accounting: pilots <= 1 + floor(log2(A_D / b)) and pilot spend <= DOPRI5 spend in each unit, on every run.

**Declared expected losses** (not failures): P_scalar_fd (model 145 vs 74), P_heat_fd (945 vs 723), y' = -3400 y (687
vs 436), heat n = 64 FD (3328 vs 2993 callbacks), short non-stiff runs +4 to +26%. **Frozen decision:** the Auto route
becomes N iff E1-E10 pass; otherwise today's route stays. **Refutation:** E3, E4, E5, E6 or E10 failing refutes the
unconditional properties (design or implementation defect); E1 failing refutes the economic claim; E2 failing blocks
adoption. Run settings: `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p vigilode`, node-owned
`CARGO_TARGET_DIR`, failing-first logs.

---

## 9. Owner decisions

* **D1 Fatal callback inside a pilot.** Recommended: the solve ends with CallbackFailed and its accepted prefix
  (the callback contract: Fatal stops the run [C vigilode/src/explicit.rs:200-203]; Recoverable already rejects only the
  attempt). Alternative: lose the pilot and continue DOPRI5 (N2 then succeeds, slowly). F2 passes either way.
* **D2 Start rule.** Recommended: keep "a supplied Jacobian, JVP or band starts RODAS5P" (no number; avoids a DOPRI5
  prefix on stiff problems with a Jacobian). Alternative: always start DOPRI5 (needs the n-unit price of a user Jacobian).
* **D3 Return path.** Recommended: one-way now; a RODAS5P -> DOPRI5 return as its own node. One DOPRI5 step cannot show
  a stability limit (a decayed stiff mode grows only over several steps), so a sound return test needs B's committed
  probes and exclusion, and RODAS5P sessions that exist only for dense Jacobians.
* **D4 Contracts.** Replace the V03b assertion "no DOPRI5 leg" for P_heat_fd and P_scalar_fd by the work bound W plus a
  recorded handover.
* **D5 Public names.** `EXPLICIT_TRIAL_MAX_DIMENSION`, `DOPRI5_REAL_STABILITY_BOUND`, `EXPLICIT_TRIAL_ATTEMPT_LIMIT`,
  `StiffnessScreen`, the removed `RouteBasis` values and `UnifiedStop::Preflight`: remove (source-breaking outside the
  repository) or keep one release as deprecated. Only the router tests use them.
* **D6 Later options.** A declared right-hand-side price (exact b at that price), and operation counting in the dense
  kernel (protected file) to replace the upper bound.
* **Doubling base 2** accepted as the one remaining number.

Relation to the other items of the owner's message: item 2 (V26) concerns batching n extra right-hand sides; after this
design the only groups of n calls in the Auto route are RODAS5P's FD columns (pilots and leg), so a V26 result applies
there unchanged, and b does not depend on it (the operation unit binds). Items 1 and 4 belong to V25 and the CI split.

---

## 10. Sources

1. L. R. Petzold, Automatic selection of methods for solving stiff and nonstiff systems of ODEs, SIAM J. Sci. Stat.
   Comput. 4 (1983) 136-148.
2. ODEPACK `stoda.f` in SciPy v1.10.0 (`ratio = 5.0d0`, `icount = 20`, `if (nq .gt. 5)`, `sm1` data):
   https://raw.githubusercontent.com/scipy/scipy/v1.10.0/scipy/integrate/odepack/stoda.f
3. E. Hairer, `dopri5.f` in SciPy v1.10.0 (HLAMB = H sqrt(STNUM/STDEN) with K2-K6 and Y1-YSTI, `HLAMB.GT.3.25D0`,
   `IASTI.EQ.15`, `NONSTI.EQ.6`, IWORK(4) default 1000):
   https://raw.githubusercontent.com/scipy/scipy/v1.10.0/scipy/integrate/dop/dopri5.f
4. OrdinaryDiffEq.jl `lib/OrdinaryDiffEqCore/src/composite_algs.jl` (AutoSwitch: maxstiffstep 10, maxnonstiffstep 3,
   nonstifftol 9//10, stifftol 9//10, dtfac 2, stiffalgfirst false, switch_max 5).
5. Wolfram Language tutorial "Stiffness Detection" (MaxRepetitions {3, 5} and {2, ∞}; the SafetyFactor value is an image
   and was not read).
6. L. F. Shampine, Stiffness and nonstiff differential equation solvers, II: Detecting stiffness with Runge-Kutta methods,
   ACM TOMS 3 (1977) 44-53, doi:10.1145/355719.355722.
7. E. Hairer, G. Wanner, Solving Ordinary Differential Equations II, 2nd ed., Springer 1996, section IV.2.
8. A. R. Karlin, M. S. Manasse, L. Rudolph, D. D. Sleator, Competitive snoopy caching, Algorithmica 3 (1988) 79-119,
   doi:10.1007/BF01762111.
9. A. Borodin, N. Linial, M. Saks, An optimal on-line algorithm for metrical task system, J. ACM 39 (1992) 745-763.

Repository records: `research/production_loop_20261116/unified_router/{PREDECLARED,RECEIPT,REVIEW}.json`, `REPORT_KO.md`;
`research/production_loop_20261116/{PROGRESS.md, PREREGISTRATION.md, CHECKPOINT.json}`;
`docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json`; `DESIGN_A.md`, `DESIGN_B.md` (this
directory).

---

## Appendix A. The judge model

`judge_model.py` (this directory, sha256 3f43e325...): DOPRI5 and RODAS5P re-implemented in Python with the repository's
tableaus (DOPRI5 [C dopri5.rs:34-90]; RODAS5P transformed stages from `fixtures/rodas5p_coefficients_snapshot.json`,
stage loop as [C rodas5p_fast.rs:1194-1278], error u_8 in the WRMS norm), the controller of [C adaptive.rs:161-177, 340-420]
(integral, order 5, safety 0.9, factors 0.2/5.0, 0.9 after a rejection), FSAL, the FD Jacobian with the driver's extra
f(t, y) call, the central FD time derivative, reuse after a rejection. Operations by the counts of section 3.2. Routes:
T (today), A (as specified, with and without witness), F (forward pilots, product or dominance), M (matched backward
pilots), all with the shared budget. It reproduces the V03b counts listed in 1.3 exactly. It is not the Rust code:
represented-clock rules, output clipping and the facade guard are simplified, and RODAS5P's LU is dense. Results:
`judge_model_results.jsonl` (sha256 7e6b61a3...; runs tagged v3_* use the window rule of 3.3; v2_* rows keep only arms
that do not depend on it). Reproduce: `JM_ARMS=D,R,T,F,Fpareto python3 -I judge_model.py "Robertson"` (optional
`JM_RTOL`, `JM_ATOL`, `JM_DCAP`). In the model, FBP is the arm `Fpareto`.
