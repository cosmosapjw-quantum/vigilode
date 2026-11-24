# DESIGN A: Auto route by on-trajectory break-even pilots (node V27, designer A)

Status: design only, under owner decision OD-L1-17 ("최대한 hyperparameter 없는 방향으로 설계").
Nothing in any worktree was edited; no test was run. Code read in the detached worktree
`/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-design` at
`80ba83881503c4b3253268df61c53b269740d6d3`.

Evidence labels: **[C path:line]** code at that commit; **[M]** measured in the repository's
own records (V03 part B packet `research/production_loop_20261116/unified_router/`);
**[L]** literature; **[O]** my own reasoning or computation (Python/sympy in my scratchpad,
not the Rust drivers). Every number marked "est." is a prediction of the cost model of
section 3, to be measured by the plan of section 7.

Scope note. The owner's message of 2026-11-24 has four items; PROGRESS.md:153-155 maps them
to V25 (item 1, matrix-free P1), V26 (item 2, the n extra right-hand sides of the assembly
check), V27 (item 3, this node) and the CI split (item 4), and rules that V27 is presented
before any implementation. This document is that presentation for item 3. It touches item 2
only in one place: the design removes the screen's n + 1 calls; the only remaining groups of
n calls are RODAS5P's finite-difference Jacobian columns (in pilots and in the RODAS5P leg),
so whatever V26 finds about batching n extra calls applies to them.

요약 (KO): 오늘의 Auto 경로는 t0에서 궤적 밖 n+1회 screen과 1000회 trial 예산으로 결정한다.
이 설계는 screen을 없애고, DOPRI5가 이미 적분한 구간을 RODAS5P 한 번의 시도로 다시 덮을 수
있는지("한 번의 RODAS5P 시도가, DOPRI5가 이미 RODAS5P 한 시도 이상의 비용을 쓴 구간을 더 싸게
덮는가")만 묻는다(ski-rental 손익분기). 질문 시점은 2배 간격과 DOPRI5 자신의 stage에서 나오는
매개변수 없는 안정성 증인으로 정하고, 질문 비용은 DOPRI5의 진행 비용을 넘지 않는다. 남는 설계
숫자는 2배 간격(관례) 하나뿐이다.

---

## 1. Census of today's Auto route

Kinds: method_constant (tableau or theorem), numerical_convention (accepted convention),
resource_limit (set by the user), tunable (number or rule chosen by a designer), rule
(structural, no number). "Replaced by" refers to this design (section 2).

| # | Item | Value | Where | Kind | Replaced by |
|---|---|---|---|---|---|
| 1 | Dimension limit of the explicit trial | n <= 10 | [C crates/vigilode/src/unified/analyzer.rs:26-28, 391-396] | tunable | removed; the break-even test scales with n by itself (one RODAS5P attempt costs (n+9) callbacks and ~2n^3/3 operations, so pilots fire later for large n) |
| 2 | DOPRI5 real stability bound | 3.3 | [C analyzer.rs:30-32, 456] | method_constant, rounded (exact 3.3065678926, [C crates/rodas5p-integrators/src/dopri5_stiffness.rs:17-20], [O] App. A) | the full stability polynomial of DOPRI5 in the witness (method_constant) |
| 3 | Stiffness screen: n+1 right-hand sides at y0 and y0 + delta_j e_j | n+1 calls | [C analyzer.rs:446-517] | rule | removed; no callback before DOPRI5's first stage |
| 4 | Probe increment | delta_j = sqrt(eps) max(abs(y0_j), 1) | [C analyzer.rs:492] | numerical_convention | removed with the screen (RODAS5P's own FD rule [C crates/vigilode/src/solution.rs:226-227] is unchanged) |
| 5 | Screen criterion | L (t1-t0) > 3.3 B | [C analyzer.rs:456, 514] | tunable (method constant times the tunable budget B; L is a Lipschitz lower bound, not an eigenvalue) | matched pilot: R is chosen only if one RODAS5P attempt is cheaper in every unit on an interval DOPRI5 has just paid for |
| 6 | Recoverable error or non-finite value at a probe: "screen shows nothing" | - | [C analyzer.rs:468-487, 497-502] | rule | removed |
| 7 | Fatal error at a probe ends the solve before any leg | - | [C analyzer.rs:414-421; router.rs:70-72]; review finding F2 [M REVIEW.json findings[1]] | rule | no preflight calls; a fatal error inside a pilot loses the pilot and DOPRI5 continues (owner decision D2, section 8) |
| 8 | Trial attempt limit | 1000 | [C crates/vigilode/src/unified/fallback.rs:10-12] | tunable | removed; DOPRI5 has no budget of its own |
| 9 | Trial budget | min(1000, max_steps / 2) | [C fallback.rs:20-25] | tunable (both the 1000 and the 1/2) | removed; pilots at doubling times and at witnesses |
| 10 | No trial when the budget gives 0 (max_steps <= 1) | - | [C analyzer.rs:397-400] | rule (consequence of 9) | removed; the shared budget applies to every attempt |
| 11 | Attempts left for the next leg | max_steps - used | [C fallback.rs:27-30] | resource_limit | kept |
| 12 | RODAS5P follows DOPRI5 only after a numerical stop | AttemptBudget, StepUnderflow, NonFinite, Controller | [C fallback.rs:32-49; router.rs:168-187] | rule | kept (for DOPRI5 stops other than handover) |
| 13 | DOPRI5 refuses before integrating: RODAS5P with the whole budget | - | [C router.rs:154-159] | rule | kept |
| 14 | Strategy RobustStiff -> RODAS5P; AdaptiveReentrant -> switching adapter | - | [C analyzer.rs:335-354] | rule | kept |
| 15 | Declared regime Stiff -> RODAS5P; Nonstiff -> DOPRI5 with the whole budget, no second method | - | [C analyzer.rs:356-362, 377-385] | rule | kept |
| 16 | Explicit method not admitted (mass matrix, dense output, events) -> RODAS5P | - | [C analyzer.rs:364-376; crates/vigilode/src/selection/frontend.rs:858-866] | rule | kept |
| 17 | A supplied Jacobian, JVP or band -> RODAS5P | - | [C analyzer.rs:387-390] | rule | kept as the start rule; "start DOPRI5 anyway" is an evaluated arm and owner decision D3 |
| 18 | HighPerformance routes as Auto | - | [C unified/options.rs:23-25] | rule | kept |
| 19 | Screen calls charged to the solve | n+1 | [C router.rs:48-53] | rule | moot (no screen); pilots are charged the same way |
| 20 | Controller constants of both drivers (read by the trial's outcome) | safety 0.9, min_factor 0.2, max_factor 5.0, reject_max_factor 0.9, min_step 1e-14 | [C crates/rodas5p-integrators/src/adaptive.rs:161-177] (protected) | numerical_convention | kept, untouched (not the router's) |
| 21 | User options | rtol 1e-6, atol 1e-9, h0 1e-3, max_steps 100000 by default | [C crates/vigilode/src/options.rs:125-137] | resource_limit | kept |
| 22 | FD Jacobian of RODAS5P legs | sqrt(eps) max(abs(y_j),1), n+1 calls per build | [C solution.rs:226-227; fd.rs:532] | numerical_convention | kept |
| 23 | Minimum step used to report non-exact stops | AdaptiveStepConfig::default().min_step | [C unified/legs.rs:116-118] | numerical_convention | kept (reporting only) |
| 24 | Switching adapter defaults (AdaptiveReentrant only, not Auto) | window_count 8, explicit_attempt_budget 32, pressure_steps 24, rejection_fraction 0.3, implicit_residence 2, probe_interval 4, probe_attempt_budget 16, return_cost_ratio 0.8, max_dense_dimension 64, implicit_pilot_limit 1, entry_cost_ratio 0.8, pilot/probe backoff 2, rearm_rate_ratio 2.0 | [C crates/vigilode/src/switching.rs:335-377; unified/legs.rs:418-425] (switching.rs is protected) | tunable | out of scope (Auto never calls it); listed so that the router's full census is visible |

Measured consequences of items 1-9 [M]: Robertson without a Jacobian, span (0,40): 6233 charged
right-hand sides against 415 for direct RODAS5P (1000 + 19 attempts); y' = -3200 y: 6043 vs 434;
y' = -2000 y: 4023 vs 434; y' = -3400 y: 436 vs 434 (screen fires) [M REVIEW.json findings[7]].
A guarded right-hand side that is fatal for y > 1 on y' = -y, y0 = 1 fails at t0 under Auto
although Dopri5::solve succeeds [M REVIEW.json findings[1]].

Two further consequences of the same items [O, from the code above, not measured]:
* A long non-stiff problem is moved to RODAS5P after 1000 DOPRI5 attempts (items 8, 9, 12):
  an AttemptBudget stop "qualifies" for the fallback. In my DOPRI5 model the oscillator
  y'' = -y on (0, 1000) needs 5910 attempts, so today it runs about 1/6 of the span on DOPRI5
  and the rest on RODAS5P with a finite-difference Jacobian.
* The screen's L is a column norm of J. For y'' + 2 zeta omega y' + omega^2 y = 0 written in
  (y, y'), L = omega^2; for omega = 100 and span 10, L (t1 - t0) = 1e5 > 3300, so today a
  resolved oscillation goes to RODAS5P. For the non-normal A = [[-1, 1e3], [0, -2]] (no stiff
  eigenvalue), L ~ 1000 and span 10 also go to RODAS5P.

---

## 2. Decision algorithm

### 2.1 Idea

DOPRI5 is the rent and RODAS5P the purchase (ski rental, [L] Karlin, Manasse, Rudolph,
Sleator, Algorithmica 3 (1988) 77-119). The router never classifies the problem. It repeatedly
asks the one question that decides economy, on the trajectory DOPRI5 has just integrated:

> Can one RODAS5P attempt cover an interval on which DOPRI5 has already spent at least as much
> as one RODAS5P attempt costs, at a lower cost in every unit?

The interval is the shortest recent stretch of DOPRI5's own accepted trajectory that makes
the question meaningful (the break-even interval). If the answer is yes, RODAS5P takes over
from DOPRI5's current state; if no, DOPRI5 continues and the question is asked again later,
at times fixed by doubling and by a parameter-free stability witness taken from DOPRI5's own
stages. The questions never cost more than DOPRI5's progress (ski-rental balance).

### 2.2 Cost units, and why nothing is converted

A cost is a vector c = (callbacks, operations, attempts):

* callbacks: the facade's guard units, one per right-hand side, JVP or time-derivative call
  and n per Jacobian callback [C crates/vigilode/src/solve.rs:251-255] (numerical_convention:
  a Jacobian costs what its finite-difference replacement costs). A finite-difference Jacobian
  is n + 1 right-hand sides [C fd.rs:532]; a finite-difference time derivative is 2
  [C crates/rodas5p-integrators/src/problem.rs:818-825].
* operations: multiply-adds counted as 2 flops, from tableau counts and textbook linear-algebra
  counts (numerical_convention): DOPRI5 attempt: 20 stage and 6 error-weight vector updates
  [C dopri5.rs:36-90] = 52n; RODAS5P attempt: 28 + 28 + 8 = 64 vector updates
  [C fixtures/rodas5p_coefficients_snapshot.json] = 128n, plus 2n^3/3 per
  `direct_factorizations`, 2n^2 per `direct_solve_calls`, 2n^2 per finite-difference Jacobian
  assembly. Computed in integers (3x scaled) from the counters, so the comparison is exact.
* attempts: accepted plus rejected attempts, the unit of `Options::max_steps`.

Comparison: R beats D (written c_R < c_D) iff c_R <= c_D in every unit and < in at least one.
No exchange rate between a callback and an operation is used. The cost at an unknown rate
kappa (operations per right-hand side) is kappa * callbacks + operations, linear in kappa, so
dominance in callbacks and in operations is exactly "R is cheaper for every kappa >= 0". The
rate kappa cannot be fixed without a tuned number: it is the user's right-hand side cost, which
the repository could only observe by timing (not deterministic) or by the WorkModel V2 presets
50/500/5000 Ir [C crates/vigilode/src/work_model.rs:41-63], which are tunables. If the caller
declares kappa (an optional declared fact, like the regime), the first two units collapse into
kappa * callbacks + operations and the test becomes two-dimensional.

Price of not converting: when the units disagree, DOPRI5 stays. The switch can then come one
doubling later than with the true kappa; for large n with an expensive right-hand side it can
come up to (n^2/78) / (n/6) times later in attempts (section 5, row 9). All bounds of section 3
hold componentwise, with this delay inside their additive term.

Constants of one attempt (autonomous problem, dense finite-difference Jacobian; a supplied f_t
replaces the 2 by 1; a non-autonomous problem without f_t adds 2 callbacks):

    c_R = (n + 9,  2n^3/3 + 18 n^2 + 128 n,  1)      # 8 stages + FD Jacobian; LU, 8 solves, FD assembly, stages
    c_D = (6,      52 n,                          1)   # 6 new stages (7 on the first attempt), FSAL
    b   = max over units of ceil(c_R / c_D)           # break-even length in DOPRI5 attempts

    n:  1  2  3  8  10  28  64  100   1000
    b:  3  4  4  7   8  23  78  166  13170           [O]

### 2.3 Pseudocode

```
AUTO(problem, (t0, t1), y0, opts):                       # N = opts.max_steps
  # declared facts: rules 14-17 of the census, unchanged, no callback
  if strategy == RobustStiff or regime == Stiff                  -> R_LEG(t0, y0, N)
  if strategy == AdaptiveReentrant                               -> switching adapter (unchanged)
  if DOPRI5 not admitted (mass, dense output, events)            -> R_LEG(t0, y0, N)
  if regime == Nonstiff                                          -> D alone with N, no pilots
  if a Jacobian, JVP or band is supplied                         -> R_LEG(t0, y0, N)   # start rule (D3)
  if RODAS5P refuses the request                                 -> Err before any callback (as today, analyzer.rs:404)
  return BREAK_EVEN(problem, (t0, t1), y0, opts)

BREAK_EVEN:
  c_R, c_D, b as in 2.2 (from n and the declared facts; no callback)
  S := 0; P := 0                         # committed DOPRI5 cost; pilot cost (vectors)
  session := DOPRI5 session on (t0, t1) from y0 with opts, attempt telemetry on   # session.rs, dopri5.rs
  anchor  := (t0, y0, S)                 # pilot 0 is armed at the start
  interval k := 0; witness_used_in_k := false
  loop
    if attempts(S) + attempts(P) == N:  return STOP(MaxStepsReached N), legs = [D prefix], pilots
    outcome := session.advance_to(t1, quota = 1)                  # one attempt, nothing else changes
    att := session.last_attempt();  S += cost(att)
    if att.accepted: remember S at the time att.t_after        # needed for the matched cost
    if outcome == Complete: return D result (bits of Dopri5::solve), pilots
    if outcome == Stopped:  numerical stop and attempts left -> R_LEG(t_D, y_D, N - used)   # rule 12
                            otherwise                        -> typed stop of DOPRI5
    w := WITNESS(att)                                           # 2.4; no callback
    k_now := the k with b*2^k <= attempts(S) < b*2^(k+1)  (k_now = 0 below 2b)
    if k_now > k: k := k_now; witness_used_in_k := false
    if no anchor:
       if attempts(S) >= 2b and attempts(S) >= b*2^k and the interval k has no armed pilot yet:
            anchor := (t_D, y_D, S at t_D)                       # tenure pilot of interval k
       elif w and not witness_used_in_k:
            anchor := (t_D, y_D, S at t_D); witness_used_in_k := true    # witness pilot
    if anchor = (a, y_a, S_a):
       if t_D > a:  M := S(at t_D) - S_a;      h := t_D - a       # matched: DOPRI5's cost on [a, t_D]
       else:        M := S - S_a + c_D;        h := first DOPRI5 trial step from a   # no progress yet:
                                                                  # lower bound of DOPRI5's cost to pass a
       if M >= c_R (every unit) and P + c_R <= S (every unit) and attempts left >= 1:
            pilot := Rodas5p::solve_observed_bounded(problem, (a, a + h), y_a,
                         opts with h0 = h and max_steps = 1, no interior, ceiling = callbacks(M))
            P += cost(pilot); record pilot; anchor := none
            if pilot reached a + h (its one attempt was accepted) and cost(pilot) < M:
                 return HANDOVER: legs = [D on (t0, t_D)], R_LEG(t_D, y_D, N - used)
            if pilot hit a fatal callback: record it; continue DOPRI5            # D2
  end

R_LEG(t, y, budget): MethodRegistry route + solve of RODAS5P on (t, t1) from y with the
  user's options and max_steps = budget (exactly today's fallback leg, router.rs:191-225).
```

Notes on the pseudocode:
* "Pilot 0" is armed at (t0, y0) and fires after about b attempts: it is the only question
  asked near t0, and it is asked after DOPRI5 has paid for it.
* At most one pilot is armed at a time, at most one tenure pilot and one witness pilot per
  doubling interval [b 2^k, b 2^(k+1)) of DOPRI5 attempts.
* A pilot is always one RODAS5P attempt (max_steps = 1). Its state is never committed: it is
  evidence only. The handover starts RODAS5P from DOPRI5's frontier (t_D, y_D), so DOPRI5's
  progress is kept and the RODAS5P leg is today's fallback leg from a state on the trajectory.
* The pilot's step h is the whole interval DOPRI5 covered since the anchor, so the comparison
  is matched exactly: both methods covered [a, t_D] from the same accepted state.

### 2.4 The stability witness (on the trajectory, no callback, no threshold)

For every DOPRI5 attempt whose seven stages were computed (accepted or rejected by the error
test), with d = Y7 - Y6 = h sum_j W_j k_j (weights [C dopri5_stiffness.rs:8-15]) and
g = k7 - k6 (the two stages at t + h):

    q   = h <d, g> / <d, d>          # signed Rayleigh secant, as dopri5_secant_h_lambda [C dopri5_stiffness.rs:445-465]
    rho = h |g| / |d|                # Hairer's modulus estimate
    z   = q + i sqrt(max(rho^2 - q^2, 0))
    WITNESS  iff  q < 0  and  |R(z)| >= 1,   R(z) = 1 + z + z^2/2 + z^3/6 + z^4/24 + z^5/120 + z^6/600

R is the stability polynomial of DOPRI5 (method_constant, [O] App. A). The witness says: this
attempt was outside DOPRI5's stability region for a mode that the exact flow damps. On the
real axis it is q <= -3.3065678926. It is used only to arm a pilot, never to choose a method:
a false witness costs at most one pilot per doubling interval, a missed witness delays the
pilot to the next doubling time. The repository already computes q on every accepted step
[C dopri5.rs:746-753, 1217-1226] and classifies it as an estimate only
[C dopri5_stiffness.rs:415-437, "ObserverOnly ... 0.5833 < 0.90"]; this design is consistent
with that status. Cost: about 20n flops per attempt, no right-hand side.

Literature for the secant: Hairer and Wanner, Solving ODEs II, 2nd ed., p. 21, implemented in
DOPRI5 [L, cited by Mazzia and Nagy, ICNAAM 2010]. The Rust port ode_solvers
[L docs.rs/ode_solvers/0.3.1/src/ode_solvers/dopri5.rs] declares stiffness after 15 tests with
h*rho > 3.25 and clears the count after 6 negative tests, with a test schedule tied to
n_stiff = 1000 (its documentation says "when the step count is a multiple of n_stiff"; its code
tests the opposite condition). Hairer's Fortran could not be fetched (HTTP 403), so these numbers
are quoted from the port. They are tunables; this design needs none of them because the witness
only schedules a matched test.

### 2.5 What the design uses, charges, and when it calls the user's code

Uses: DOPRI5's attempts (outcome, trial step, stages for the witness), its accepted times, the
charged counters of both drivers, and RODAS5P's acceptance of one attempt (its error estimate
<= 1). No wall time, no off-trajectory probe, no new controller.

Charges: every DOPRI5 attempt, every pilot (recorded as discarded work with its verdict, its
matched DOPRI5 cost and its own counters) and the RODAS5P leg, all in the solve's counters and
attempt total.

Callbacks: until the first pilot fires (about b attempts after t0, b - 1 at the earliest when
DOPRI5 cannot accept a step) only DOPRI5's stages. A pilot makes, from a stored accepted DOPRI5 state y_a: n + 1 right-hand sides for the
finite-difference Jacobian (n of them at y_a + delta_j e_j, off the trajectory, the same kind of
point every finite-difference Jacobian of Rodas5p::solve uses), 8 stage right-hand sides near
the step from y_a, and the time-derivative calls of a non-autonomous problem. The RODAS5P leg
calls what Rodas5p::solve calls.

---

## 3. Properties

Notation: C_D, C_R are the costs of Dopri5::solve and Rodas5p::solve on the whole span with the
user's options; S, P are as in 2.3; A_D the attempts of the DOPRI5 tenure; vector inequalities
are componentwise.

### 3.1 Unconditional properties (no model)

* **U1 (ski-rental balance).** A pilot fires only if P + c_R <= S and costs at most c_R
  (one attempt; c_R is an upper bound of one attempt's charge), so P <= S at all times.
  Without a handover the run is DOPRI5's own run plus pilots: cost = C_D + P <= 2 C_D in every
  unit, and P <= (number of pilots) c_R with number of pilots <= 2 log2(A_D / b) + 4.
  (S = C_D requires the session run to equal Dopri5::solve bit for bit; the switching adapter
  already relies on this [C crates/vigilode/tests/switching_persistent_explicit.rs:28-50];
  discriminator E4 verifies it for this use.)
* **U2 (no handover against its own evidence).** RODAS5P takes over only after one of its
  attempts covered exactly the interval [a, t_D] that DOPRI5 had covered from the same accepted
  state (or an interval DOPRI5 had failed to pass), at a cost lower in every unit.
* **U3 (budget).** DOPRI5 attempts, pilot attempts and RODAS5P attempts sum to at most
  `max_steps`; the router enforces it with quota-1 advances; no leg has a budget of its own.
* **U4 (callbacks).** No user callback before DOPRI5's first stage at (t0, y0); every callback is
  a DOPRI5 stage, a pilot evaluation from an accepted DOPRI5 state, or a RODAS5P-leg evaluation.
* **U5 (determinism).** See 3.4.

### 3.2 Cost bound against the better single method, under a stated model

Model M (piecewise regimes) [O]:
* M1: [t0, t1] is a union of phases. Inside a phase, a run of method X costs r_X per unit time
  (a vector), DOPRI5 pays c_D per attempt and RODAS5P c_R per step (it builds a Jacobian and an
  LU on every step; the Jacobian is reused only after a rejection [C rodas5p_fast.rs:26-27]).
* M2: inside a phase, a RODAS5P attempt of size h from any state of the trajectory is accepted
  iff h <= h_R (its accuracy-limited step in that phase); both methods see the same phases.

Lemma (the pilot decides the regime). For a pilot whose interval [a, t_D] lies in one phase:
DOPRI5's matched cost is about r_D (t_D - a) and the pilot fires when that first reaches c_R,
so t_D - a is about c_R / r_D. The pilot hands over iff its one attempt is accepted, iff
h_R >= c_R / r_D, iff r_R = c_R / h_R <= r_D in every unit. So a pilot hands over exactly when
RODAS5P is cheaper per unit time in the current phase (in every unit).

* **B1 (stiff from t0).** If RODAS5P is cheaper in the first phase and that phase lasts at least
  two break-even intervals: handover at pilot 0, so
  cost <= C_R + b c_D + c_R  (the RODAS5P leg from t_D costs at most C_R under M1).
  In the binding unit b c_D is about c_R; in another unit b c_D can exceed that unit's c_R
  (large n: callbacks), see 2.2.
* **B2 (stiffness that develops at t_s).** DOPRI5 cheaper before t_s, RODAS5P after.
  Without any witness: the first doubling time after t_s is at most twice DOPRI5's attempts at
  t_s, so DOPRI5's attempts after t_s are at most A(t_s) + 2b and
  cost <= 2 C_D(t0, t_s) + C_R(t_s, t1) + 2b c_D + P  <=  2 min(C_D, C_R) + 2b c_D + P,
  because OPT_switch = C_D(t0, t_s) + C_R(t_s, t1) is at most both single costs.
  If a witness appears within the first break-even interval after t_s and the current doubling
  interval has not spent its witness pilot on a false witness (assumption W):
  cost <= C_D(t0, t_s) + C_R(t_s, t1) + 3b c_D + P <= min(C_D, C_R) + 3b c_D + P
  (one more b for a tenure pilot that may be armed across t_s and must fire first).
* **B3 (non-stiff).** DOPRI5 cheaper everywhere: every pilot loses (Lemma), cost = C_D + P with
  P <= (2 log2(A_D/b) + 4) c_R and P <= C_D (U1).
* **B4 (limits).** One-way: after a handover RODAS5P runs to t1, so a later phase where DOPRI5
  is cheaper is paid at RODAS5P's rate. Between two doubling times a short phase where RODAS5P
  wins can be missed; U1 still caps the loss at 2 C_D.

Against today's route: the worst observed ratio of today's route to direct RODAS5P is about 15
[M]; under M the design's ratio to the better single method is at most 2 plus the additive
2b c_D + P (B2), and about 1 when a witness marks the onset.

Ski rental gives ratio 2 for a known purchase price [L Karlin et al. 1988]; the two-state
metrical task system has deterministic ratio 3 when switching back is allowed
[L Borodin, Linial, Saks, J. ACM 39 (1992) 745-763]; with a purchase that only reduces the
running cost (the Bahncard problem) the optimal deterministic ratio is 2 - beta
[L Fleischer, MPI report 1997]. The design is one-way (no return), measures the price by the
pilot instead of knowing it, and uses doubling for the times of the questions.

### 3.3 Off-trajectory calls

None before DOPRI5's first stage, none at t0 except DOPRI5's own, none on a run where no pilot
fires (problems shorter than b attempts). On longer non-stiff runs the pilots make
(n + 1) finite-difference probes and 8 stage evaluations each, from accepted DOPRI5 states, at
most 2 log2(A_D / b) + 4 times. Variant A1 (section 8, D1) removes the tenure pilots and then
makes no off-trajectory call on a run without a witness, at the price of relying on W.

### 3.4 Determinism

Every decision is a function of integer counters, of f64 values that the drivers and the
witness compute by fixed sequential arithmetic (trial steps, accepted times, q, rho), and of the
accept/reject outcome of a deterministic RODAS5P attempt. There is no clock, thread, random seed
or hash iteration. On the same host and binary the route and the bits are the same on every run
(discriminator E7). Across hosts a different floating-point path (FMA contraction, libm) can move
a witness or an accepted time and so the route, as it can move any adaptive step sequence today;
the cost is that Auto-route golden records are host-pinned, like the existing host-dependent
anchors. The default path (Rodas5p::solve) and its golden files are not touched.

---

## 4. Hyperparameters that remain

| Name | Value | Kind | Why it cannot be removed |
|---|---|---|---|
| Doubling base of the pilot times | 2 | numerical_convention | Some geometric spacing is needed to keep both the number of pilots logarithmic and the detection delay proportional to the elapsed work; a base beta gives delay <= (beta - 1) A(t_s) and log_beta pilots, so B2 becomes beta C_D(t0,t_s) + ... No theorem fixes beta for this cost structure; 2 is the textbook doubling choice, not fitted to data. |
| Ski-rental balance | pilot cost <= committed DOPRI5 cost (ratio 1) | method_constant (break-even of the ski-rental argument) | It is what makes U1 unconditional; a ratio theta gives (1 + theta) C_D. |
| One tenure pilot and one witness pilot per doubling interval | 1 and 1 | rule | Bounds the cost of false witnesses by the same logarithm. |
| Jacobian callback = n callback units | n | numerical_convention | Needed to put an analytic Jacobian on the callback axis; it is the facade's own charging [C solve.rs:251-255]. |
| Flop counts | 2n^3/3 per LU, 2n^2 per pair of triangular solves and per FD assembly, 2 flops per multiply-add | numerical_convention | Textbook counts; the operation axis needs some count. |
| Tableau counts | DOPRI5 26 vector updates per attempt, RODAS5P 64 and 8 stage calls | method_constant | From the tableaus. |
| DOPRI5 stability polynomial | 1, 1, 1/2, 1/6, 1/24, 1/120, 1/600 | method_constant | Defines the witness. |
| Controller constants of both drivers | 0.9, 0.2, 5.0, 0.9, 1e-14 | numerical_convention (protected) | Not router parameters; unchanged. |
| FD increment of RODAS5P | sqrt(eps) max(abs(y_j),1) | numerical_convention | Unchanged driver rule. |
| max_steps, rtol, atol, h0 | user | resource_limit | User's. |
| Start rule for a supplied derivative or band | - | rule | Kept (no number); owner decision D3. |
| One-way handover | - | rule | Scope of "when to leave DOPRI5"; a symmetric return would reuse the same test (D4). |
| Fatal callback inside a pilot | pilot lost, DOPRI5 continues | rule | Owner decision D2. |
| RODAS5P after a numerical DOPRI5 stop | - | rule | Kept from today. |

Optional declared fact (not a hyperparameter): the cost of one right-hand side in operations
(kappa), if the caller wants a scalar comparison.

---

## 5. Behaviour on the scenarios

Costs in charged right-hand sides unless stated; "est." = cost-model estimate [O], "model" =
my Python model of DOPRI5 with the repository's tableau and controller constants (App. B).

| Scenario | Today | This design |
|---|---|---|
| Stiff from t0 | Screen fires if L (t1-t0) > 3.3 B: y' = -1e6 (y - cos t) FD 74, heat n=8 FD span 100 723 [M]; otherwise the trial: y' = -3200 y 6043 vs 434 direct [M]. | DOPRI5's first attempts are rejected or stability-limited; the witness fires on attempt 1 for y' = -1e6(...) and on attempt 57 for y' = -3200 y (model). Pilot 0 fires after b attempts (b = 3 for n = 1, 7 for n = 8); its one attempt covers DOPRI5's interval, so handover (B1). est.: y' = -1e6(...) about 100-110, y' = -3200 y about 500-600, heat n=8 span 100 about 800-850. Slightly worse than the screen where the screen fires, much better where it does not. |
| Stiffness that develops later (Robertson) | Not seen at t0 (L = 0.63 [M]); DOPRI5 trial of 1000 attempts then RODAS5P: span 40 6233 vs 415 [M]; span 10, max_steps 500: 266 attempts, 1697 [M]. | First witness at t = 0.0074, attempt 16 (model; Hairer's DOPRI5 flags stiffness at t ~ 0.027-0.034 [L Mazzia-Nagy Table 1]). Pilots in the transient lose, the first pilot on the stability-limited stretch hands over (B2). est. span 40: 600-900 (about 7-10x below today, about 1.5-2x direct RODAS5P). |
| Non-stiff | Screen n+1 calls, DOPRI5 up to 1000 attempts, then RODAS5P for the rest of a long run (section 1). | DOPRI5 to t1, bit-identical to Dopri5::solve; log-many pilots, each one rejected RODAS5P attempt (B3). est. oscillator span 10 (63 attempts, 4 pilots): +44 (+12%); span 1000 (5910 attempts, 10 pilots): +110 (+0.3%), while today switches to RODAS5P after 1000 attempts; Lotka-Volterra span 100 (1002 attempts, 7 pilots): +77 (+1.3%). No witness occurs (model). |
| Oscillatory non-stiff (eigenvalues near the imaginary axis) | The column-norm screen sends omega = 100, span 10 to RODAS5P (section 1). | DOPRI5 steps stay inside the region (imaginary stability interval 0.997, App. A; resolved steps have abs(h omega) ~ 0.4). The complex witness can fire falsely on badly scaled (y, y') forms: 88 of 5910 attempts at rtol 1e-6 (model), costing at most one extra lost pilot per doubling interval. A truly stiff oscillatory mode (lambda = -100 +- 1e4 i) gives witnesses from attempt 1 (model) and a handover. |
| Non-normal transient growth | A = [[-1, 1e3], [0, -2]], span 10: screen L ~ 1000 -> RODAS5P (section 1). | DOPRI5 (no stiff eigenvalue); one false witness at t = 2.12 (model) -> at most one lost pilot. The stiff non-normal A = [[-1, 1e3], [0, -1e4]] gives a witness on attempt 1 (model) and a handover. The secant can mislead on non-normal J; the tenure pilots do not depend on it. |
| Fatal callback off the trajectory | Screen probe at y0 + delta e_j fatal -> solve ends at t0 although DOPRI5 succeeds [M F2]. | No probe at t0. A pilot's probe can still meet the fatal region (pilot 0 is anchored at y0, so the review's case y > 1 is reached by it): under D2 the pilot is lost, recorded with the message, and DOPRI5 continues and succeeds like Dopri5::solve. A fatal error on the committed trajectory (DOPRI5 or the RODAS5P leg) ends the solve as today. |
| Budget exhaustion | Trial gets min(1000, N/2); RODAS5P the rest. | One shared budget; DOPRI5, pilots (1 attempt each) and the RODAS5P leg draw on it; exhaustion is a typed MaxStepsReached{N} with the accepted prefix and all work (U3). est. Robertson span 10, N = 500: handover after about 40-70 DOPRI5 attempts, about 60-90 attempts in all (today 266 [M]). |
| Supplied Jacobian on a non-stiff problem | RODAS5P (rule 17). | Unchanged under the kept start rule: RODAS5P all the way (cost C_R, typically 1.5-3x C_D for small n, est.). Arm "start DOPRI5" (D3): C_D + pilots on non-stiff problems, but a stiff problem with a Jacobian pays about b c_D + c_R more than today (heat n=64 dense Jacobian: c_R = (72, about 248500, 1), b = 75, est. about 2.6x today's 328). |
| Large n, cheap right-hand side | n > 10 -> RODAS5P with a dense FD Jacobian: n + 1 calls and O(n^3) per step. | DOPRI5. b ~ n^2/78 (operations bind), so a pilot fires only after DOPRI5 has spent one LU worth of operations since its anchor. Non-stiff: C_D + P, P <= C_D; est. for n = 1000 and 1e5 attempts (b = 13170): 3 pilots, +0.5% callbacks, about +40% operations (bounded by 100%). Stiff: handover after about b attempts; additive about one LU in operations and 6b right-hand sides. |
| Small n, expensive right-hand side | Screen and trial as above. | Callbacks dominate the true cost; Pareto also asks for fewer operations, so for n = 3 the pilot waits for 4 instead of 2 DOPRI5 attempts. The bounds hold; a declared kappa removes the delay. |

---

## 6. Code changes (none touches a protected file)

Protected blobs [C research/production_loop_20261116/CHECKPOINT.json protected_blob_sha1]
include switching.rs, rodas5p_fast.rs, rodas5p_fast_dense.rs, adaptive.rs, core work.rs and
vigilode lib.rs. None needs an edit: lib.rs already re-exports `unified::*`
[C crates/vigilode/src/lib.rs:215, 254] and RODAS5P needs no new entry point.

1. `crates/rodas5p-integrators/src/dopri5_stiffness.rs`: pure functions
   `dopri5_secant_pair(h, stages, n) -> Option<(q, rho)>` and
   `dopri5_stability_witness(q, rho) -> bool` with the exact polynomial. Correct the comment at
   :17-19: R(x) = +1 (not -1) at x = -3.306568 [O App. A].
2. `crates/rodas5p-integrators/src/dopri5.rs` (`Dopri5Session::advance_to`): after every
   completed trial, accepted or rejected, keep (q, rho) of that attempt when attempt telemetry
   is on (a new accessor next to `last_attempt`; `Dopri5Attempt` is only re-exported, never
   built outside dopri5.rs, so a new field would also compile). Pure read of the stages:
   trajectories unchanged (check: existing DOPRI5 goldens and the session tests bit-identical).
3. `crates/vigilode/src/session.rs`: crate-private pass-through of `set_attempt_telemetry` and
   `last_attempt` on `ExplicitSession`; `snapshot`/`stats_between` already build a leg record.
4. `crates/vigilode/src/solve.rs`: no change; pilots call the existing crate-private
   `Rodas5p::solve_observed_bounded` [C solve.rs:248-263] with max_steps = 1 and h0 = h (the
   switching adapter's bounded pilots use the same function).
5. New `crates/vigilode/src/unified/breakeven.rs`: the loop of 2.3; cost vectors in u128 (3x
   scaled operations); doubling intervals; witness arming; invariant P + c_R <= S; pilot record
   `PilotRecord { trigger: Tenure | Witness, anchor_t, t_d, h, attempts_of_d, matched_cost,
   pilot_cost, accepted, verdict, fatal: Option<CallbackStop> }`.
6. `unified/analyzer.rs`: drop rules 5-6 (dimension limit, NoBudgetForTrial, screen,
   `StiffnessScreen`, `stiffness_screen`, `PreflightCallbackFailed`); new
   `RouteBasis::UndeclaredBreakEven`; `PreflightAnalysis` makes no callback in any case.
7. `unified/fallback.rs`: drop `EXPLICIT_TRIAL_ATTEMPT_LIMIT` and `explicit_trial_attempts`;
   keep `remaining_attempts` and `follows_explicit_stop`.
8. `unified/router.rs`: `explicit_then_fallback` becomes `break_even_then_rodas5p` (DOPRI5 leg
   from session snapshots, pilots charged into `ws.work_counters` and listed, RODAS5P leg =
   `registry_leg` from (t_D, y_D) with the attempts left); preflight charging and the preflight
   stop path are removed.
9. `unified/legs.rs`, `unified/solution.rs`: `VigilodeSolution::pilots`; a route-history entry
   for the handover that carries the matched costs; `UnifiedStop::Preflight` kept only if the
   owner wants source compatibility (never produced).
10. `unified/mod.rs`, `unified/options.rs`: documentation; optional declared kappa field.
11. Public API: `EXPLICIT_TRIAL_MAX_DIMENSION`, `DOPRI5_REAL_STABILITY_BOUND`,
    `StiffnessScreen`, `EXPLICIT_TRIAL_ATTEMPT_LIMIT` leave the route: remove (breaking) or keep
    as `#[deprecated]` for one release (D6).
12. Tests: new contract tests written first against the base (section 7, E4-E9); V03b contracts
    that encode the screen change and need the owner's approval (D5):
    `p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit` and
    `p1_stiff_contractive_scalar_without_jacobian_is_not_explicit`
    [C crates/vigilode/tests/unified_router_contracts.rs:505-596] (a short DOPRI5 leg now
    precedes RODAS5P; their work bound W stays and the assertion "no DOPRI5 leg" becomes the
    B1 bound), `p1_preflight_callbacks_are_charged_without_a_jacobian`,
    `p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input` and any test reading
    `analysis.screen`.

What the drivers must expose, and what the switcher lineage already provides: DOPRI5 per
attempt (outcome, trial step, t_after, rhs_delta) exists [C dopri5.rs:870-882, 1007-1016];
the secant exists for accepted steps [C dopri5.rs:1217-1226]; only (q, rho) on every attempt is
new. The persistent DOPRI5 session with attempt quotas exists [C dopri5.rs:1040-1082;
session.rs:112-189]. The bounded RODAS5P pilot with a callback ceiling exists
[C solve.rs:248-324]. Counters for factorizations and solves exist
[C crates/rodas5p-core/src/work.rs:4-48]. Nothing else is needed.

---

## 7. Evaluation plan (to pre-register before any run)

Arms: T = today's route at 80ba8388; A2 = this design (recommended); A1 = this design without
tenure pilots (witness pilots and pilot 0 only); A2-noW = without witness pilots; D = Dopri5::solve;
R = Rodas5p::solve; for supplied-derivative problems also A2-startD (D3).

Problems (FD Jacobian unless stated; default options unless stated; spans fixed in the
pre-registration):
* From PREDECLARED.json of V03b: P_heat_dense, P_heat_fd, P_scalar_dense, P_scalar_fd,
  P_oscillator, P_robertson (with and without J; spans 10 and 40; max_steps 500 case),
  P_heat_jvp, P_heat_banded, P_mass, P_fatal, P_blowup; the review's probes y' = -lambda y for
  lambda in {2000, 3200, 3400} and the guarded right-hand side (fatal for y > 1).
* New stiff from t0: diagonal linear n = 4 with eigenvalues -1, -10, -1e3, -1e5; E5; HIRES (n=8);
  OREGO (n=3); Van der Pol mu = 1e3 (several transitions, report only for E1).
* New late onset: flame y' = y^2 - y^3, y0 = delta, span 2/delta, delta in {1e-2, 1e-3, 1e-4};
  Robertson span 1e5.
* New non-stiff: Lotka-Volterra, Arenstorf orbit, pleiades (n = 28), Lorenz (short span),
  oscillator span 1000 (more than 1000 DOPRI5 attempts).
* New oscillatory: damped oscillator omega = 100, zeta = 1e-3 in (y, y') at rtol 1e-6 and 1e-3;
  stiff oscillatory block -100 +- 1e4 i with amplitude 1e-8.
* New non-normal: A = [[-1, 1e3], [0, -2]] (span 10) and [[-1, 1e3], [0, -1e4]] (span 1).
* New large n: heat n = 64 FD (stiff), 1D advection-diffusion n = 200 in a non-stiff regime,
  Brusselator 1D n = 64 FD (stiff).
* New callback probe F_probe: y = (u, v), v' = -v, v(0) = 0, right-hand side fatal for v > 0
  (every finite-difference probe in v is fatal; DOPRI5 keeps v = 0 exactly).
* Budgets: max_steps in {7, 40, 500} on a subset.

Measures (counters only, no wall time): charged callbacks by kind and entries counted in the
user closures; operations from the counters with the conventions of 2.2; attempts; pilots
(number, trigger, verdict, cost); witnesses (number, first attempt); handover time; status and
stop; achieved error at t1 against a reference (exact solution or RODAS5P at rtol 1e-12,
atol 1e-14) in the user's weighted norm; bits of the final state; FNV-1a digest of the route
record (as V03b's digest module).

Discriminators (comparisons, no new number):
* E1 bound (problems with at most one regime change): cost(A2) <= 2 min(cost(D), cost(R))
  + 2b c_D + P, componentwise, with the measured P. A failure falsifies model M or the code and
  is reported with its cause.
* E2 vs today (adoption): (i) no problem where T succeeds and A2 fails at the same max_steps;
  (ii) geometric mean over the frozen set of cost(A2)/cost(T) below 1 in callbacks, operations
  and attempts; (iii) the largest ratio cost/min(cost(D), cost(R)) of A2 below that of T.
* E3 accuracy: err(A2) <= err(D) + err(R) wherever both single methods succeed.
* E4 neutrality: no handover -> final state bit-identical to Dopri5::solve and charged cost equal
  to Dopri5::solve's plus the pilots'; start-RODAS5P problems -> bit-identical to Rodas5p::solve.
* E5 charging: closure entries equal charged units for every callback kind (uncharged = 0).
* E6 budget: all attempts (legs and pilots) <= max_steps; exhaustion is MaxStepsReached{N}.
* E7 determinism: two runs in one process and one in a fresh process give equal digests.
* E8 callbacks: zero callbacks before DOPRI5's first stage; every off-trajectory call is inside a
  pilot or the RODAS5P leg (instrumented closures record (t, y) and the caller).
* E9 fatal: F_probe and the guarded right-hand side succeed under A2 (D2 as proposed) where T
  fails at preflight; P_fatal still reports CallbackFailed(Rhs, "sensor offline").
* Reported without a rule: A1, A2-noW and A2-startD against A2; witness timing and false
  witnesses; pilots per problem; costs per scenario row of section 5 against the est. values.

Frozen decision: the Auto route becomes A2 iff E1 (on its subset), E2 (i)-(iii), E3, E4, E5, E6,
E7, E8 and E9 pass; otherwise today's route stays (the node's failure_action). The choice between
A2 and A1 (D1) and the other owner decisions are reported with the numbers, not frozen.
Run settings for the implementation node: `taskset -c 0-5,12-17 cargo +1.94.1 test --locked -j 4
--release -p vigilode`, a node-owned CARGO_TARGET_DIR, failing-first logs as in V03b.

---

## 8. Risks and owner decisions

Risks:
* Model M is an idealization; rates change continuously and pilots sample at doubling times. A
  short window where RODAS5P wins can be missed; U1 still caps the loss at 2 C_D.
* One-way: a stiff transient followed by a long non-stiff phase stays on RODAS5P (costly for
  large n with a finite-difference Jacobian).
* Pareto conservativeness delays the switch for large n with an expensive right-hand side unless
  kappa is declared (section 2.2).
* Short non-stiff runs pay a few lost pilots: est. +10 to +25% of a few hundred right-hand sides
  versus today's n + 1 screen calls; long runs pay well under 1%.
* The witness can fire falsely on badly scaled or non-normal problems (model: 88 of 5910 and 1 of
  63 attempts); each costs at most one lost pilot per doubling interval.
* Pilots re-evaluate the right-hand side near the trajectory: callbacks must be functions
  (replay-safe), as the switching adapter already requires.
* The session run must equal Dopri5::solve bit for bit (E4); if not, U1's "= C_D" weakens to
  "= the session's cost".
* Floating-point paths differ across hosts: Auto-route goldens are host-pinned.

Owner decisions:
* D1: tenure pilots (A2, bounded without trusting the secant; small overhead and a few
  off-trajectory calls on non-stiff runs) or witness-only (A1, zero overhead and no
  off-trajectory call on non-stiff runs; B2 then depends on W).
* D2: a fatal callback inside a pilot: lose the pilot and continue DOPRI5 (proposed; the
  committed trajectory never visited that state) or end the solve (honours "Fatal ends the run"
  for the whole solve, and keeps F2 for pilot 0).
* D3: keep "a supplied derivative starts RODAS5P" or start DOPRI5 always.
* D4: one-way only, or a later node with symmetric return pilots (same test with the roles
  swapped; no new number).
* D5: replace the V03b contract "no DOPRI5 leg on stiff contractive finite-difference problems"
  by the bounded-work contract B1.
* D6: remove the four public names of today's route or deprecate them for one release.
* D7: optional declared kappa (right-hand side cost) for a scalar comparison.

---

## Appendix A. DOPRI5 stability constants [O]

Computed exactly (sympy) from the tableau of [C dopri5.rs:36-70]:
* R(z) = 1 + z + z^2/2 + z^3/6 + z^4/24 + z^5/120 + z^6/600 (b^T A^6 1 = 0).
* Real axis: R(x) - 1 has roots 0 and -3.3065678926349465037; R(x) + 1 has no real root. So
  0 < R(x) < 1 on (-3.30657, 0) and R(x) > 1 for x < -3.30657: beyond the boundary the
  stiff mode grows without changing sign. The comment at [C dopri5_stiffness.rs:17-18] says
  R = -1 there; the value is right, the sign is not.
* Imaginary axis: abs(R(iy))^2 - 1 = y^6 (y^6 - 25 y^4 + 225 y^2 - 200) / 360000; the only
  positive root is y = 0.99718900863, the constant 0.997189 of [C dopri5.rs:411].
* Boundary radius in the open left half-plane: largest 3.39903 at about 120.2 degrees, smallest
  tends to 0.99719 at 90 degrees.

## Appendix B. Witness on my DOPRI5 model [O]

Python re-implementation of DOPRI5 (tableau, error weights, safety 0.9, factors 0.2/5.0, 0.9
after a rejection, rtol 1e-6, atol 1e-9, h0 1e-3, RMS norm), witness of 2.4 on every completed
attempt. Not the Rust driver.

| Problem | Attempts of DOPRI5 alone (rejected) | Witnesses accepted/rejected | First witness |
|---|---|---|---|
| Robertson, span 40 | 40503 (5968) | 16777 / 5966 | t = 0.0074, attempt 16 |
| y' = -3200 y, span 1 | 1175 (162) | 466 / 160 | t = 0.0075, attempt 57 |
| y' = -2000 y, span 1 | 749 (98) | 290 / 96 | t = 0.0121, attempt 57 |
| y' = -1e6 (y - cos t), span 1 | capped at 4000 (t = 0.011) | 1667 / 568 | t = 0, attempt 1 |
| oscillator omega = 1, span 10 / 1000 | 63 / 5910 | 0 / 0 | none |
| damped omega = 100, zeta = 1e-3, rtol 1e-6 / 1e-3 | 5910 / 1035 | 88 / 0 and 83 / 0 | attempt 48 / 14 (false: badly scaled form) |
| A = [[-1,1e3],[0,-2]], span 10 | 63 | 1 / 0 | t = 2.12 (false) |
| A = [[-1,1e3],[0,-1e4]], span 1 | 3565 (500) | 1463 / 498 | t = 0, attempt 1 |
| block -100 +- 1e4 i, span 0.1 | 529 (1) | 497 / 1 | t = 0, attempt 1 |
| Van der Pol mu = 1e3, span 1 | 1079 (133) | 440 / 131 | t = 0.0078, attempt 50 |
| flame delta = 1e-3, span 2000 | 443 (81) | 144 / 44 | t = 1020, attempt 105 |
| heat n = 8, span 100 | 11069 (1580) | 4551 / 1563 | t = 1.1, attempt 119 |
| Lotka-Volterra, span 100 | 1002 (125) | 0 / 0 | none |

Reading: the witness appears on every stiff case, early on stiff-from-t0 and at the onset of
the late cases (flame at t ~ 1/delta), never on the resolved non-stiff ones, and falsely on two
badly scaled or non-normal non-stiff ones. Heat n = 8 shows why the tenure pilots matter: DOPRI5
sits just inside the boundary for 119 attempts before the first crossing, while RODAS5P is
already cheaper there.

## Sources

* Code: the worktree files cited as [C ...] at 80ba8388.
* Hairer, E. and Wanner, G., Solving Ordinary Differential Equations II, 2nd ed., Springer 1996,
  p. 21 (dominant-eigenvalue stiffness test of DOPRI5), as cited by Mazzia, F. and Nagy, A. M.,
  "Stiffness detection strategy for explicit Runge Kutta methods", ICNAAM 2010
  (people.dimai.unifi.it/brugnano/NMSDE/papers/mazziaICNAAM2010.pdf).
* DOPRI5 stiffness constants 3.25, 15, 6, NSTIFF 1000 as in the ode_solvers port
  (docs.rs/ode_solvers/0.3.1/src/ode_solvers/dopri5.rs); Hairer's Fortran was not reachable
  (HTTP 403).
* Shampine, L. F., "Stiffness and nonstiff differential equation solvers, II: Detecting stiffness
  with Runge-Kutta methods", ACM TOMS 3(1) (1977) 44-53.
* LSODA method switch (ratio = 5, icount = 20, nq <= 5): scipy's stoda.f
  (raw.githubusercontent.com/scipy/scipy/v1.11.0/scipy/integrate/odepack/stoda.f).
* OrdinaryDiffEq AutoSwitch defaults (maxstiffstep 10, maxnonstiffstep 3, nonstifftol 9/10,
  stifftol 9/10, dtfac 2, switch_max 5): SciML/OrdinaryDiffEq.jl v6.66.0 src/composite_algs.jl.
* Karlin, A. R., Manasse, M. S., Rudolph, L., Sleator, D. D., "Competitive snoopy caching",
  Algorithmica 3 (1988) 77-119.
* Borodin, A., Linial, N., Saks, M., "An optimal on-line algorithm for metrical task system",
  J. ACM 39(4) (1992) 745-763.
* Fleischer, R., "On the Bahncard problem", Max-Planck-Institut fuer Informatik report (1997).
