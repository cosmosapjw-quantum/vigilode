# DESIGN_B: Auto route decided by measured cost rates

Node V27 (loop L1, wave 3, design phase), designer B. Angle: an economic comparison of measured costs.

- Code read at commit `80ba83881503c4b3253268df61c53b269740d6d3`, worktree
  `/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w3-design` (read only; nothing edited, built or run there).
- Owner decision OD-L1-17 (docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json:113-118):
  "최대한 hyperparameter 없는 방향으로 설계". PREREGISTRATION.md:277: V27 is pre-registered only after the owner
  approves the design.
- This is a design. Nothing in it has been implemented or measured. Numbers marked *estimate* are hand estimates
  from the V03b records; section 8 says how to replace them by measurements.

Marks used: **[code f:l]** read in the code at that file and line (paths relative to the worktree; `rodas5p-integrators`
means `crates/rodas5p-integrators/src`, `vigilode` means `crates/vigilode/src`); **[lit n]** literature, listed in
section 10; **[calc]** computed in this session by a scratch script outside the repository; text without a mark is my
own reasoning.

---

## 0. Summary

The route keeps the declared rules (strategy, declared regime, admission of DOPRI5). For an undeclared request it
removes the stiffness screen, the dimension limit 10, the trial budget `min(1000, max_steps/2)` and the rule that a
supplied derivative forces RODAS5P. In their place:

1. **Two currencies, one price.** Every run is charged in callback units `CB` (right-hand sides including
   finite-difference columns, time derivatives, JVPs, a user Jacobian priced as the `n` calls it replaces) and in
   arithmetic operations `OP` (factorizations, triangular solves, Krylov work, vector arithmetic of the stages), all
   deterministic counts. The true cost is `kappa * CB + OP` for an unknown price `kappa >= 0` (operations per callback).
2. **One comparison rule.** With a declared price the costs are compared at that price. Without one, cost `A` is at
   least cost `B` iff `A_cb * A_op >= B_cb * B_op`: the choice with the smallest worst-case cost ratio over every price
   in `[0, inf)`. The worst-case loss of such a choice, `Gamma`, is computed and reported; for dense problems with
   `n <= 10` and for banded problems of any size it is at most 1.42 [calc, section 2.3].
3. **Measured rates on the trajectory.** The incumbent method runs; its spend is counted. When its spend reaches the
   cost of a probe of the other method (break-even, then doubling after each losing probe), the other method takes
   real, committed steps from the current accepted state until its step-size controller is saturated, plus one
   measuring step. The two steady cost rates (cost per unit of integrated time) are compared with the one rule; the
   cheaper method continues. Nothing is integrated twice and nothing is rolled back.
4. **A free exclusion.** While RODAS5P runs with a stored Jacobian, `|tr J| / n` is a lower bound of the spectral
   radius, so DOPRI5 cannot be stable with a step above `3.9785 n / |tr J|` (3.9785 is the largest modulus of the
   DOPRI5 stability region [calc]). When even that best case of DOPRI5 is more expensive than RODAS5P's measured rate
   in both currencies, the probe of DOPRI5 is skipped. It costs O(n) operations and no callback.

Under a stated rate model, the route costs at most `Gamma` times (the better single method plus probes) for a
problem that does not change regime, and at most `2 Gamma` times the best one-switch schedule plus probes for one
change of regime (Robertson). Today's route has no relative bound: the reviewers measured 15 times the right-hand
sides of direct RODAS5P on Robertson without a Jacobian (REVIEW.json:369-374). The route is deterministic: same host,
same inputs, same route and bits. The route makes no callback for routing alone. One tunable number remains, the
doubling base 2, with a sensitivity argument. Two rules need an owner decision: which method starts on a
finite-difference problem, and what happens when a fatal error comes from a finite-difference column of a probe.

---

## 1. Census of today's Auto route

What `SolverStrategy::Auto` uses to decide, directly or through what it calls. Kinds as defined in the task:
method_constant, numerical_convention, resource_limit, tunable, rule.

| # | Item | Where | Value | Kind | Role today | Proposed |
|---|---|---|---|---|---|---|
| R1 | Strategy: `RobustStiff` runs RODAS5P, `AdaptiveReentrant` the switching adapter, `HighPerformance` routes as `Auto` | [code vigilode/unified/analyzer.rs:335-354], [code vigilode/unified/options.rs:16-31] | - | rule (user choice) | first rule | kept |
| R2 | Declared regime: `Stiff` runs RODAS5P; `Nonstiff` runs DOPRI5 with the whole budget and no second method | [code analyzer.rs:356-362, 377-385], [code options.rs:98-103] | - | rule (user declaration) | second rule | kept |
| R3 | DOPRI5 not admitted with a mass matrix, dense output or events, then RODAS5P | [code analyzer.rs:364-376], [code vigilode/selection/frontend.rs:857-871] | - | rule (structural) | third rule | kept |
| R4 | Both entry points validate before the first callback, so a refusal never follows a callback | [code analyzer.rs:402-412] | - | rule | guard of the screen | kept: before the first run |
| R5 | A supplied Jacobian, JVP or declared band sends an undeclared request to RODAS5P | [code analyzer.rs:388-390] | - | rule (designer choice) | fourth rule | replaced: a supplied derivative only makes RODAS5P the first run (no off-trajectory call, `tr J` at `t0`); measured costs decide afterwards. A declared band with finite differences forces nothing |
| R6 | `EXPLICIT_TRIAL_MAX_DIMENSION` | [code analyzer.rs:26-28, 391-396] | 10 | tunable | above it RODAS5P with a finite-difference Jacobian | removed: the dimension acts only through counted linear-algebra operations |
| R7 | `EXPLICIT_TRIAL_ATTEMPT_LIMIT` | [code vigilode/unified/fallback.rs:10-12] | 1000 | tunable | cap of the explicit trial | removed |
| R8 | Trial budget `B = min(1000, max_steps / 2)` | [code fallback.rs:20-25] | 1/2 of `max_steps` | tunable on a resource_limit | how long DOPRI5 runs before RODAS5P | removed: break-even triggers with doubling; `max_steps` is the only budget |
| R9 | `NoBudgetForTrial`: `B = 0` (max_steps <= 1) runs RODAS5P | [code analyzer.rs:397-400] | `max_steps <= 1` | rule | edge case | removed |
| R10 | Screen eligibility: no regime, finite differences, no band, `n <= 10`, both admitted | [code analyzer.rs:387-412] | - | rule | when the screen runs | removed (no screen) |
| R11 | Probe increment `delta_j = sqrt(eps) max(abs(y0_j), 1)`, represented difference | [code analyzer.rs:492-495], same as [code vigilode/fd.rs:441-455] | `sqrt(2^-52)` | numerical_convention | screen | removed from routing; stays only inside RODAS5P's own finite-difference Jacobian |
| R12 | Screen statistic `L = max_j norm2(f(t0, y0 + delta_j e_j) - f0) / delta_j` | [code analyzer.rs:482-515] | - | rule (norm 2, maximum over j) | Lipschitz lower bound at `t0` | removed |
| R13 | `DOPRI5_REAL_STABILITY_BOUND` | [code analyzer.rs:30-32] | 3.3 (exact 3.3065678926 [calc], [code rodas5p-integrators/dopri5_stiffness.rs:20]) | method_constant (rounded) | screen criterion | removed. Another constant of the same tableau, 3.9785, is used only by the exclusion of section 2.4 |
| R14 | Screen criterion `L (t1 - t0) > 3.3 B` | [code analyzer.rs:456, 511-515] | `3.3 B` | rule built on R13 and R8 | sends to RODAS5P | removed |
| R15 | Screen callback outcomes: recoverable or non-finite, screen silent; fatal, solve ends with no leg | [code analyzer.rs:468-480, 483-487, 496-502, 413-421], [code vigilode/unified/router.rs:70-72] | - | rule | the "third outcome" of REVIEW F2 | removed: no callback serves routing alone |
| R16 | Screen cost: `n + 1` charged right-hand sides at `t0` | [code analyzer.rs:174-175], [code router.rs:51-53] | `n + 1` | consequence | charged | removed (0 routing-only calls) |
| R17 | Fallback: a DOPRI5 stop by `AttemptBudget`, `StepUnderflow`, `NonFinite` or `Controller` lets RODAS5P continue; `Callback`, `Output` or no reason does not | [code fallback.rs:32-49], [code router.rs:168-173] | - | rule | second leg | generalized: a numerical stop of either method starts an immediate probe of the other; a callback stop is terminal |
| R18 | Every leg starts at the last accepted state; legs share `max_steps`; remaining = `max_steps - used` | [code fallback.rs:1-6, 27-30], [code router.rs:178-187] | - | resource_limit and rule | budget | kept for every run, incumbent or probe |
| R19 | No second method after a fatal or malformed callback | [code fallback.rs:38-41] | - | rule | | kept |
| R20 | Controller constants of both drivers: safety 0.9, `min_factor` 0.2, `max_factor` 5, `reject_max_factor` 0.9, integral controller, `min_step` 1e-14, `max_attempts = max_steps` | [code rodas5p-integrators/adaptive.rs:161-177, 404-408], [code vigilode/solve.rs:265-271], [code vigilode/explicit.rs:454-457] | - | tunables of the drivers | decide whether the trial ends within `B` | unchanged (a change of controller constants is not authorized, research/production_loop_20261116/PROGRESS.md:75); `max_factor` is read by the saturation test |
| R21 | Option defaults rtol 1e-6, atol 1e-9, `h0` 1e-3, `max_steps` 100000 | [code vigilode/options.rs:125-136] | - | resource_limit / user options | | unchanged; `h0` seeds the first step of each method |
| R22 | RODAS5P driver family from declared structure (dense, banded, matrix-free, fast dense, sequential) | [code solve.rs:290-311], [code frontend.rs:873-879], [code vigilode/registry.rs:428-443] | - | rule (structural) | planned route | kept; it fixes the linear-algebra operation counts |
| R23 | Switching adapter configuration, used only by `AdaptiveReentrant`: window_count 8, explicit_attempt_budget 32, pressure_steps 24, rejection_fraction 0.3, implicit_residence 2, probe_interval 4, probe_attempt_budget 16, return_cost_ratio 0.8, max_dense_dimension 64, implicit pilot 0 and 1, entry_cost_ratio 0.8, backoff factors 2 and 2, rearm_rate_ratio 2.0, EntryProbeAdaptation 64 / 2.0 / 0.5 / 0.2, `RhsCost::Cheap` | [code vigilode/switching.rs:335-377, 91-101], [code vigilode/unified/legs.rs:418-425] | about 20 numbers | tunables | not Auto | not used by the new Auto; `AdaptiveReentrant` unchanged |
| R24 | DOPRI5 secant stiffness estimate, computed at every accepted step (batch minimum kept in a thread-local); gate fraction 0.8; guard thresholds 1e3, 10, 1e-15, 1e15 | [code rodas5p-integrators/dopri5.rs:746-753, 884-887, 1217-1226], [code dopri5_stiffness.rs:46-71, 440-471] | 0.8 | tunable; classed ObserverOnly (agreement 0.5833 < 0.90 on an off-registry Brusselator, ledger L-0213, [code dopri5_stiffness.rs:415-437]) | not used by Auto | not used for decisions (may be recorded as a diagnostic) |
| R25 | WorkModel V2: RHS tiers 50 / 500 / 5000 Ir; linear-algebra weights `(2/3) n^3` per factorization, `2 n^2` per triangular solve, `100 + n` per linear solve, `10 + 2n` per Krylov iteration | [code vigilode/work_model.rs:41-62, 100-110] | - | tiers: tunable; `(2/3) n^3`, `2 n^2`: method_constant; 100, 10: tunable | not used by Auto | the new route uses only the factorization and solve operation counts (and driver-counted band operations), not the tiers |

Observations made while taking the census (not part of the route, but they matter for honest cost units):

- **O1** [code dopri5_stiffness.rs:17-19] says `R(x) = -1` at `x = -3.306568`. [calc] On the negative real axis the
  DOPRI5 stability polynomial `R(z) = sum_{k<=5} z^k / k! + z^6 / 600` never goes below 0.173 inside the region; the
  boundary point is `R(x) = +1` at `x = -3.3065678926`. The value is right, the sign in the comment is not.
- **O2** A finite-difference Jacobian is charged twice by every work proxy that adds `n * jacobian_builds` to
  `rhs_calls`: the facade adds the `n + 1` difference calls to `counters.rhs_calls` [code solve.rs:661-665], and the
  driver also counts the build [code rodas5p-integrators/problem.rs:884-888 via rodas5p_fast.rs:1174]. Such proxies:
  `work_units` [code registry.rs:371-375, 636-640] and WorkModel V2 [code work_model.rs:173-177]. The currency of this
  design counts builds of a *user* Jacobian only.
- **O3** The RODAS5P window driver returns no next-step proposal (`proposed_next_step: None`,
  [code registry.rs:673]); the native RODAS5P session exists only for an explicit dense Jacobian
  [code rodas5p-integrators/rodas5p_fast.rs:2800-2809]. DOPRI5 has a facade session with an attempt quota and the
  controller's proposal [code vigilode/session.rs:132-161].
- **O4** A fresh finite-difference RODAS5P attempt evaluates `f(t, y)` twice (in the build [code fd.rs:489] and in the
  driver [code rodas5p_fast.rs:1180]), `n + 9` right-hand sides per attempt (V03b: 192 calls in 16 attempts for
  `n = 3`, REPORT_KO.md:110). Not a routing matter.

---

## 2. The design

### 2.1 Principle

A method's cost per unit of integrated time (its *rate*) can be measured only by running it. Running it on the
trajectory, from an accepted state, makes the measurement productive: its accepted steps are kept. The design therefore
never asks "is the problem stiff?"; it asks "which of the two methods, as measured just now on this trajectory, costs
less per unit time?", and it pays for a measurement of the method not running only when the running method has spent
at least as much as the measurement costs (rent or buy, [lit 9]).

### 2.2 Cost model: two currencies and a price

For every attempt the drivers already count, or can count, two integers:

- `CB`, callback units: `rhs_calls` (finite-difference columns included, [code solve.rs:661-665]) + `ft_calls` +
  `jvp_vectors` + `n * user_jacobian_builds`. A user dense or band Jacobian is priced as the `n` calls of the finite
  difference Jacobian it replaces, a JVP and a time derivative as one call: the repository's own convention for work
  ceilings [code solve.rs:252-255]. Finite-difference builds are *not* added again (O2).
- `OP`, arithmetic operations in the repository's counting convention (multiply-subtract 2, division 1, assembly 1
  per entry, [code rodas5p_fast.rs:205-207]):
  - factorizations and triangular solves: counted by the banded kernel (`BandedWork`); for the dense kernel
    `(2/3) n^3` per factorization and `2 n^2` per solve (the frozen weights [code work_model.rs:105-106]) until the
    dense kernel counts its own operations (section 7); Krylov: `2n` per inner product and per vector update
    [code work_model.rs:103-104];
  - vector arithmetic of an attempt, a method constant of each tableau [calc]: DOPRI5 has 20 nonzero stage
    coefficients and 6 nonzero error weights [code dopri5.rs:36-90], about `56 n` operations per attempt; RODAS5P has 28
    nonzero entries in `A` and 28 in `C` (fixtures/rodas5p_coefficients_snapshot.json), about `116 n` operations plus
    the assembly of `W` (`n^2 + n` dense) per attempt.

The cost of a run at price `kappa` (operations per callback unit) is `C_kappa = kappa * CB + OP`. The price is a fact
about the user's right-hand side, not a property of the method. `VigilodeOptions::rhs_price` (new) has three values:

- `Unknown` (default): `kappa` anywhere in `[0, inf)`.
- `Declared(kappa)`: the user states it (the same idea as `RhsCost::DeclaredIr`, [code work_model.rs:49-50]).
- `Measured` (opt-in, not deterministic): section 3.4.

### 2.3 The comparison rule

Every decision of the route (trigger a probe, keep or switch after a probe, skip a probe) compares two cost vectors
`A = (A_cb, A_op)` and `B = (B_cb, B_op)`, spends or rates. There is one rule:

- `Declared(kappa)`: `A >= B` iff `kappa A_cb + A_op >= kappa B_cb + B_op`.
- `Unknown`: `A >= B` iff `A_cb * A_op >= B_cb * B_op`.

Why the product. `C_kappa(A) / C_kappa(B)` is monotone in `kappa`; its extreme values are `q_0 = A_op / B_op`
(`kappa = 0`) and `q_inf = A_cb / B_cb` (`kappa -> inf`). If both are on the same side of 1, one option is cheaper at
every price and the rule agrees. Otherwise choosing `B` loses at worst the factor `max(q_0, q_inf)` and choosing `A`
at worst `1 / min(q_0, q_inf)`; the choice with the smaller worst loss is `B` iff `q_0 q_inf >= 1`, which is the
product test. This is the minimax ratio-regret choice over all prices; it has no number. The worst loss of the choice is

`Gamma = sqrt(max(q_0, q_inf) / min(q_0, q_inf))` (1 when one option dominates).

For a rate comparison between the two methods the step sizes cancel in `q_0 / q_inf`, so `Gamma` is a structural
number of the request: `Gamma = sqrt((OP_I / OP_E) / (CB_I / CB_E))` per attempt, computable from the registry's facts
before any callback. [calc, with the per-attempt costs of 2.2; I = RODAS5P, E = DOPRI5]:

| Request | `CB_I / CB_E` | `OP_I / OP_E` | `Gamma` |
|---|---|---|---|
| dense, finite differences, `n = 1` | 1.67 | 2.39 | 1.20 |
| dense, finite differences, `n = 3` | 2.00 | 3.09 | 1.24 |
| dense, finite differences, `n = 10` | 3.17 | 6.30 | 1.41 |
| dense user Jacobian, `n = 64` | 12.0 | 70.3 | 2.42 |
| dense, finite differences, `n = 1000` | 168 | 12214 | 8.5 |
| tridiagonal band, finite differences, any `n` | 2.00 | about 3.1 | about 1.24 |

So without a declared price the route is within 1.42 of the true-price decision for small dense and for banded
problems, and the uncertainty grows like `sqrt(n / 14)` only for large dense problems, where the user should declare
the price or the structure. The solution reports `Gamma` and, for every decision, the break-even price
`kappa* = (B_op - A_op) / (A_cb - B_cb)` at which it would flip.

### 2.4 Measured quantities

- **Attempt record** (from the driver sessions, section 7): accepted or not, step `h`, cost `(CB, OP)`, and whether the
  controller factor of an accepted step is at its cap (`max_factor`, the clamp of [code adaptive.rs:404-408]).
- **Saturated accepted attempt**: its controller factor is below the cap, or the attempt just before it was rejected.
  Either way the controller has located its step-size limit (from below: accuracy no longer allows growth by the cap;
  or from above: it had to reject). With the default integral controller the cap test is an exact comparison
  [code adaptive.rs:372-378, 404-408].
- **Tenure** of the incumbent: its spend `U` (cost vector) since it became incumbent, and its *steady rate*: spend
  divided by accepted time, both counted from its first saturated accepted attempt in the tenure, or from the end of
  the last probe if that is later.
- **Probe** of the challenger: committed attempts from the current accepted state until the first saturated accepted
  attempt (the ramp), then one more accepted attempt (the measuring step). Its steady rate is the cost of the
  measuring step and of the rejected attempts directly before it, divided by its `h`. Its cost `P` is the cost of all
  its attempts.
- **Exclusion of DOPRI5** while RODAS5P is incumbent with a stored Jacobian (dense or band): `rho_lb = abs(tr J) / n`
  is a lower bound of the spectral radius (the trace is the sum of the eigenvalues, so `abs(tr J) <= n max abs(lambda)`;
  this holds for any matrix, normal or not). DOPRI5 is linearly stable only if `h abs(lambda) <= r_max` for every
  eigenvalue, where `r_max = 3.9785` is the largest modulus on the DOPRI5 stability region [calc: maximum 3.97841 at
  `1.020 +- 3.845 i`; on the left half-plane the maximum is 3.3990, on the imaginary axis the region reaches 0.9972, on
  the real axis 3.3066]; rounded up, so the exclusion is conservative. Hence DOPRI5's step is at most
  `h_max = r_max / rho_lb`, and its rate is at least `E_lb = (6, v_E) / h_max` (6 calls per FSAL attempt,
  `v_E` its vector operations). DOPRI5 is excluded when `E_lb >= r_I` in both components (it cannot be cheaper at any
  price) or, with a declared price, at that price. Cost: O(n) operations on a Jacobian the driver has already built; no
  callback.
- **A-priori probe costs** before any measurement, from structural facts only (dimension, Jacobian form, band,
  autonomy): one attempt of the method. DOPRI5: `(7, v_E)` (the first attempt of a run has no FSAL value). RODAS5P:
  `CB = n + 9` with finite differences (`groups + 9` with a band, `8 + n` with a user Jacobian, plus one time
  derivative call when the problem is not autonomous and no time derivative is supplied), `OP` = one factorization,
  8 solves and the vector work. After its first probe a method's probe cost is the measured one.

### 2.5 Algorithm

```
AUTO(problem, (t0, t1), y0, opts):
  F := MethodFacts::of(problem, opts)                  # declared facts, no callback
  # declared rules, unchanged (R1-R4)
  if strategy == RobustStiff           -> RODAS5P, whole budget
  if strategy == AdaptiveReentrant     -> switching adapter
  if regime == Stiff                   -> RODAS5P
  if DOPRI5 not admitted(F)            -> RODAS5P          # mass matrix, dense output, events
  if regime == Nonstiff                -> DOPRI5, whole budget
  validate both entry points           # a refusal never follows a callback

  # measured-cost route
  price := opts.rhs_price               # Unknown | Declared(kappa) | Measured
  m := RODAS5P if F.jacobian is supplied (Dense, Banded, Jvp, SparseJvp) else DOPRI5      # start rule
  c := other(m)
  (t, y) := (t0, y0); seed[DOPRI5] := seed[RODAS5P] := h0
  P[c] := a_priori_probe_cost(c, F)
  tenure := Tenure(m, threshold T := P[c])
  while t < t1:
    a := one attempt of m from (t, y) with step seed[m]      # session, quota 1; charged; budget -= 1
    tenure.add(a)                                            # spend U, saturation, steady window
    if a is accepted: (t, y) := a.end
    seed[m] := m's proposal
    if a stopped on a fatal or malformed callback: STOP(CallbackFailed)        # terminal, no other method
    if budget used up:                              STOP(MaxStepsReached(max_steps))
    if t == t1: break
    if a stopped numerically (step underflow, non-finite, controller, minimum step, linear solve):
        probe := PROBE(c)                                    # forced, no trigger, no skip
    else if tenure.saturated and U >= T (comparison rule) and not SKIP(c):
        probe := PROBE(c)
    else: continue
    if probe stopped on a fatal callback: STOP(CallbackFailed)
    (t, y) := probe.end; P[c] := probe.cost; seed[c] := c's proposal     # probe steps are committed
    if probe made no progress and m stopped numerically: STOP(the probe's typed stop)
    if m stopped numerically or tenure.rate > probe.rate (comparison rule, strict):
        swap(m, c); tenure := Tenure(m, T := P[c]); tenure.saturated := true     # the probe was saturated
    else:
        T := U + componentwise_max(U, P[c])       # spend since this probe >= max(spend before it, its cost)
        tenure.restart_rate_window()              # m resumes from probe.end with seed[m]
  return SUCCESS

PROBE(c): attempts of c from (t, y), seed = seed[c] if c ran before, else
          max(h0, seed[m]) for RODAS5P (an L-stable method can take at least the incumbent's step) and
          min(h0, seed[m]) for DOPRI5 (an explicit method should not start above it);
          until the first saturated accepted attempt, then one measuring accepted attempt;
          ends early at t1, at a stop, or when the budget is used up.

SKIP(c):  horizon:   P[c] >= tenure.rate * (t1 - t)          # cannot be recovered at any price / at kappa
          exclusion: c == DOPRI5, m == RODAS5P, J stored:  E_lb >= r_I (2.4)
          a skipped probe leaves T unchanged; SKIP is evaluated again after the next accepted attempt
```

Everything in this pseudocode is a counter, an accepted step size, a declared fact, or one of the method constants of
2.2 and 2.4. The only free number is the doubling in the update of `T` (section 4).

### 2.6 Evidence, callbacks and charges

- **Evidence**: only quantities that the integration produces anyway: the work counters of each attempt, accepted step
  sizes, the controller's own cap state, the stored Jacobian of RODAS5P. All of it is on the trajectory of the route.
  No norm of `f` or `J` is used to decide; no eigenvalue is computed.
- **Callbacks**: every call belongs to an attempt of DOPRI5 or RODAS5P that starts at an accepted state of the route
  and whose result (accepted or rejected) is part of the route's own integration. The only evaluation points that are
  not stage values of such attempts are the finite-difference columns and the finite-difference time derivative of
  RODAS5P attempts, the same calls that a direct RODAS5P run makes from that state. With a supplied derivative there is
  no such point at all. No callback is made to decide.
- **Charges**: every attempt of every run, incumbent or probe, is charged to the counters of the solution and to
  `max_steps`. Nothing is uncharged; there is no preflight work.
- **Record**: each run is a `UnifiedLeg` with a role (incumbent or probe); each comparison writes a decision record
  (time, both rates in both currencies, price mode, `Gamma`, `kappa*`, outcome); each exclusion writes `tr J`,
  `rho_lb`, `h_max`, `E_lb` and the rate it was compared with.

---

## 3. Properties

### 3.1 Model M (stated so that the bounds can be checked)

- M1 The cost of a run is additive over attempts: `kappa * CB + OP` with a fixed price `kappa`.
- M2 The accepted trajectory does not depend on which method produced it (both are error controlled), so each method
  has a steady rate `r_E(t)`, `r_I(t)`. `[t0, t1]` splits into regimes in which the same method is cheaper.
- M3 A saturated probe measures the steady rate of its regime; the ramp of a method entering a regime costs at most
  its probe cost `P`.
- M4 The exclusion uses the linearization: DOPRI5 needs `h abs(lambda) <= r_max` for every eigenvalue of the current
  Jacobian.

### 3.2 Cost bounds

Notation: `C_E`, `C_I` the costs of the direct runs; `P` the largest probe cost, `P_min` the smallest.

**Proposition 1 (one regime, declared price).** `C_auto <= min(C_E, C_I) + P (3 + log2(min(C_E, C_I) / P_min))`.
Sketch. If the starting method is the cheaper one, the route pays only losing probes. Probe k+1 needs an incumbent
spend since probe k of at least `max(spend before probe k, P_k)`, so the spend at least doubles between probes and
there are at most `1 + log2(C / P_min)` of them; each costs at most `P` more than the incumbent would have spent on its
interval. If the starting method is the dearer one, it spends at most the break-even amount `T_1 = P` before the first
probe, which (M3) wins; the ramp is part of `P`.

**Proposition 2 (one change of regime at `t*`, declared price).** Let `C_1 = C_E[t0, t*] + C_I[t*, t1]` be the best
schedule with one switch (DOPRI5 cheaper before `t*`, RODAS5P after, as in Robertson; the reverse case is symmetric).
`C_1 <= min(C_E, C_I)`. Then `C_auto <= 2 C_1 + P (4 + log2(C_1 / P_min))`.
Sketch. Let probe k be the last one before `t*`, at tenure spend `U_k`. The next probe comes when the spend since
probe k reaches `max(U_k, P_k)`; so DOPRI5 spends after `t*` at most `max(U_k, P_k) <= max(C_E[t0, t*], P)` before
RODAS5P is probed and (M3) wins. This delay is charged to the spend before `t*`: the factor 2. The probes before `t*`
are bounded as in Proposition 1. With several regime changes the same argument charges each delay to the tenure before
it, provided each regime lasts at least one probe; regimes shorter than a probe cannot be resolved by any method that
must measure before it switches.

**Proposition 3 (unknown price).** With the product rule each comparison loses at most its `Gamma` against the
comparison at the true price (2.3). Propositions 1 and 2 hold with `min(C_E, C_I)`, `C_1` and `P` multiplied by
`Gamma_max`, the largest `Gamma` of the comparisons made (reported). For dense `n <= 10` and banded requests
`Gamma_max <= 1.42`.

**Proposition 4 (the exclusion loses nothing under M4).** A skipped probe of DOPRI5 is one that DOPRI5 cannot win at
any price (or at the declared price): its best case `E_lb` already costs at least RODAS5P's measured rate. If M4 fails
(strong nonlinearity, a wrong user Jacobian), the error is one-sided: the route stays on RODAS5P, the protected method.

Assumptions that can fail are listed in 3.7. These are proof sketches, not machine-checked theorems; the evaluation
(D5 in section 8) checks every run against its own bound.

### 3.3 Today's route under the same model

Today there is no relative bound:

- stiffness the screen does not see (Robertson): an additive trial of up to 1000 DOPRI5 attempts; observed 6233 against
  415 right-hand sides of direct RODAS5P, about 15 times (REVIEW.json:369-374);
- a non-stiff problem the screen flags (fast oscillation, non-normal `J`, `L (t1 - t0) > 3.3 B`): RODAS5P with a
  finite-difference Jacobian for the whole span, cost `C_I`, unbounded relative to `C_E`
  (the documented limitation, [code analyzer.rs:163-167]);
- a supplied derivative on a non-stiff problem: `C_I` unless `regime(Nonstiff)` is declared (REPORT_KO.md:264);
- a finite-difference problem with `n > 10`, or a non-stiff one that needs more than 1000 attempts: RODAS5P with a
  dense finite-difference Jacobian for the rest (REPORT_KO.md:264).

Where today's route is better: a stiff-from-`t0` finite-difference problem that the screen detects costs `n + 1`
calls plus direct RODAS5P (P_heat_fd 723, P_scalar_fd 74 right-hand sides, REPORT_KO.md:75-76). The proposed default
starts such a problem on DOPRI5 and pays up to the break-even prefix (scenario S1, *estimate* up to 1.7 times today on
these two problems). Section 9 gives the owner the alternative start rule that removes this loss and its price.

### 3.4 Determinism and reproducibility

- `Unknown` and `Declared` prices: every input of every decision is an integer counter, an accepted step size or a
  stored Jacobian entry produced by deterministic driver arithmetic, or a constant. There is no clock, no thread
  scheduling, no hash-map iteration and no thread-local state in the decision path (the router must not read the
  thread-local secant minimum of [code dopri5.rs:884-887]). Spends are compared in exact `u128` arithmetic; rates in
  IEEE `f64` without fused operations. Hence the same host, binary and inputs give the same route and the same bits,
  and so do the golden comparisons. The wall time of a solve stays diagnostic only, as today
  ([code router.rs:47, 353-354]).
- Callbacks are called once per point of the committed integration; nothing is replayed. Unlike the switching adapter's
  same-window comparators ([code switching.rs:258, 264-265]: "Callbacks must be replay safe"), the route needs no
  replay safety.
- A solve in which no probe is made is bit for bit the direct call of its method, because a session does not change
  the native step sequence ([code session.rs:338-342] for DOPRI5; required of the new RODAS5P session). Examples: a
  stiff problem with a supplied matrix Jacobian whose DOPRI5 probes are all excluded; a short non-stiff run whose
  probes the horizon rule skips.
- `Measured` price (opt-in): the price is estimated from wall time (time per callback over time per counted operation of
  the factorizations in the same solve). The route can then change from run to run, and with it the bits (the
  difference is of the order of the tolerance, both methods being error controlled). Reproducibility: the solution
  records the measured `kappa`; re-running with `Declared(recorded kappa)` reproduces route and bits exactly. This is
  the scheme of FFTW: `FFTW_MEASURE` plans depend on timing and their results vary from run to run at the level of
  rounding, while `FFTW_ESTIMATE` or saved wisdom give identical results [lit 8]. The default stays deterministic.

### 3.5 Callbacks off the trajectory

- No callback is made for routing alone (today: `n + 1` at `t0` on every undeclared request with `n <= 10`).
- With a supplied derivative there is no finite-difference point; every call is a stage value or a Jacobian, JVP or
  time-derivative call at an accepted state.
- With finite differences, the points off the computed trajectory are the finite-difference columns (and time
  derivative) of RODAS5P attempts. Under the default start rule they occur only after DOPRI5 has spent the break-even
  amount, at an accepted state, and only as part of a committed RODAS5P step.
- The case of REVIEW F2 (a right-hand side fatal for `y > 1`, `y' = -y`, `y0 = 1`, REVIEW.json:327-332): today the
  solve ends at the preflight; under this design DOPRI5 starts, the first probe of RODAS5P comes after DOPRI5 has
  accepted steps with `y < 1`, its columns lie at `y + delta < 1`, and the solve completes like direct DOPRI5.
- A right-hand side that is fatal exactly at a column point of a state where a probe happens still ends the solve
  (rule R19). The stop record names the call as a finite-difference column of a RODAS5P probe at `t_k`, the same call
  direct RODAS5P would make there. The alternative is an owner decision (section 9).

### 3.6 Budget and termination

- All attempts of all runs count against `max_steps`; the route stops with `MaxStepsReached { max_steps }` and keeps the
  accepted prefix and the work of every run. No share of the budget is reserved, because probes are triggered by spend
  and not by a budget fraction.
- Every comparison either advances the threshold geometrically or swaps the methods after a committed probe; probes are
  committed attempts; so the loop ends at `t1`, at the budget, or at a stop of both methods.

### 3.7 Where the model fails

- Rates are noisy: DOPRI5 at its stability limit alternates accepted and rejected attempts; one measuring step can be
  off by a factor of about 2. In the ambiguous zone (methods within `Gamma` of each other) this can cause switches back
  and forth; each costs at most one probe and each tenure must spend break-even before the next probe, so the loss
  stays within a factor 2 of the zone in which both methods are nearly equal.
- Regimes shorter than a probe (fast alternation of stiff and non-stiff phases, Van der Pol with large `mu`) are not
  resolved; the route then behaves like the method that is incumbent most of the time, and its cost is bounded by
  Proposition 1 applied to the regime it sees.
- A probe measured on a step clipped by an output time or by `t1` gives a distorted rate; irrelevant at `t1`, and
  output landing is not used by the Auto route (only `t0` and `t1` are returned).
- M4 fails for strongly nonlinear problems; the error direction is safe (3.2, Proposition 4).
- The price model counts operations, not cycles; memory traffic and parallel kernels are not in it. This is why the
  price can be declared and why `Gamma` is reported.

---

## 4. Hyperparameters that remain

| Name | Value | Kind | Why it cannot be removed |
|---|---|---|---|
| Doubling base of the probe thresholds (`T := U + max(U, P)`) | 2 | tunable (convention) | Detecting a change of regime late costs a factor `b`, probing often costs `log_b` probes; no scale-free choice optimizes both. Any `b > 1` keeps the form of the bounds. [calc] In the simplest model (one change at an unknown spend, probes of equal cost) the worst ratio is about `b + 1 / (e ln b)`, minimal at `b = 1.61` with 2.38 against 2.53 for `b = 2` and 3.33 for `b = 3`: weakly sensitive. Base 2 is the standard doubling trick. |
| `r_max`, largest modulus of the DOPRI5 stability region | 3.9785 (3.97841 rounded up) | method_constant | Needed by the exclusion. Follows from `R(z)` of the tableau [calc]. Could be dropped together with the exclusion, at the price of `O(log)` futile probes of DOPRI5 in long stiff runs. |
| Operation counts: vector work per attempt (DOPRI5 about `56 n`, RODAS5P about `116 n + n^2 + n`), `(2/3) n^3` per dense factorization, `2 n^2` per dense solve | from tableaux and algorithms | method_constant | Needed to put the methods' own arithmetic into the `OP` currency. Exact once the drivers count them (section 7). |
| Price of a user Jacobian (`n` calls), of a JVP and a time derivative (1 call) | `n`, 1, 1 | numerical_convention | Puts user-Jacobian builds into the callback currency; it is the repository's convention for work ceilings [code solve.rs:252-255] and the price of the finite-difference Jacobian a user Jacobian replaces. A separate price for the Jacobian would make every comparison involving it degenerate (DOPRI5 builds none). |
| Finite-difference increment `sqrt(eps) max(abs(y_j), 1)` | `sqrt(2^-52)` | numerical_convention | Part of RODAS5P's own finite-difference Jacobian [code fd.rs:441-455]; no longer used by the router. |
| Controller constants of the drivers (safety 0.9, factors 0.2, 5, 0.9, `min_step` 1e-14) | as R20 | inherited tunables of the drivers | Not authorized to change; the saturation test reads `max_factor` but adds no number. |
| One measuring step after saturation | 1 | structural minimum | A rate needs at least one step at the located limit. |
| Rules without numbers | - | rule | the product rule (minimax ratio regret over the price), ties keep the incumbent, the start rule, a fatal callback is terminal, a numerical stop forces a probe, the horizon rule, the exclusion rule, the probe seeds `max(h0, .)` and `min(h0, .)`. |
| User inputs, not hyperparameters of the route | rtol, atol, `h0`, `max_steps`, `rhs_price`, regime, strategy | resource_limit / declarations | the user's tolerances, budget and knowledge. |

From today's route the tunables 10, 1000, 1/2 and the rule "a supplied derivative runs RODAS5P" disappear; 3.3 and the
screen disappear. The switching adapter's twenty numbers (R23) are not used.

---

## 5. Scenarios

Estimates use the V03b records; section 8 measures them.

| # | Scenario | Behaviour |
|---|---|---|
| S1 | Stiff from `t0` | With a supplied matrix Jacobian (P_heat_dense, P_scalar_dense): RODAS5P starts; after its ramp the DOPRI5 probe is excluded at every check (heat `n = 64`: `abs(tr J)/n = 8450`, `h_max = 4.7e-4`, `E_lb` about `(1.3e4, 7.6e6)` per unit time against RODAS5P's *estimate* `(288, 1.0e6)`; scalar: `tr J = -1e6`), so the solve is the direct RODAS5P call (bit for bit if the new RODAS5P session reproduces the direct driver's steps, a requirement of section 7): 328 and 48 right-hand sides, as today. The check happens only after RODAS5P is saturated; at `h0 = 1e-3` its rate would still be too high to exclude anything. With finite differences (P_scalar_fd, P_heat_fd): DOPRI5 starts; its first attempts at `h0` are rejected or accuracy-limited; after saturation and break-even (*estimate* 4 to 8 attempts) the probe of RODAS5P wins once its step exceeds DOPRI5's by `sqrt((CB_I/CB_E)(OP_I/OP_E))` (about 3.9 for `n = 8`); afterwards DOPRI5 is excluded. *Estimate* 100 to 125 calls against 74 today for the scalar, 800 to 1100 against 723 for the heat equation; the V03b bounds 181818 and 5890 hold, but the V03b assertion "no DOPRI5 leg" for these two problems does not (section 9). |
| S2 | Stiffness that develops later (Robertson) | Without a Jacobian: DOPRI5 starts (at `t0` `tr J = -0.04`, nothing to exclude); probes of RODAS5P at doubling spends; RODAS5P wins when its step is about 2.5 times DOPRI5's (product rule, `n = 3`), *estimate* after 20 to 40 DOPRI5 attempts (`t` about 0.01 to 0.05). Until RODAS5P's step has grown (`t` of order 0.1 to 1) one to three DOPRI5 probes lose; from then on `abs(tr J)/3` (about 700) excludes DOPRI5. *Estimate* 600 to 800 calls against 415 for direct RODAS5P and 6233 today (span 40): about 1.5 to 2 times direct instead of 15. With a dense Jacobian: RODAS5P starts, DOPRI5 may win a short early tenure in the non-stiff transient, RODAS5P returns at the next probe. |
| S3 | Non-stiff | DOPRI5 runs (it starts, or wins the first probe after one RODAS5P step when a derivative is supplied). Probes of RODAS5P at break-even, then doubling: at most `1 + log2(C_E / P_I)` of about one or two RODAS5P steps each (a run of `10^6` calls: about 16 probes, well under 1 percent); the horizon rule skips probes that cannot pay off. A very short run (the oscillator over span 1, about 10 attempts) can pay one losing probe, about one RODAS5P step (*estimate* +20 to 30 percent on a 60-call problem). |
| S4 | Oscillatory non-stiff (eigenvalues near the imaginary axis) | DOPRI5's stable interval on the imaginary axis is only `abs(h omega) <= 0.997` [calc], but accuracy at default tolerances needs smaller steps, so DOPRI5 is accuracy limited and RODAS5P, which needs similar steps at a higher price per step, loses its probes. `tr J` is near 0, so nothing is excluded wrongly. Today the screen sends `omega (t1 - t0) > 3.3 B` (for example `omega = 1e3`, span 4) to RODAS5P for the whole span. |
| S5 | Non-normal transient growth | `J = [[-1, 1e4], [0, -2]]` (non-stiff, large norm): today `L` is about 1e4, so any span above 0.33 goes to RODAS5P; here DOPRI5 runs and the probes lose. `J = [[-1e4, 1e6], [0, -1e3]]` (stiff, non-normal): DOPRI5 is stability limited and transiently amplified, RODAS5P wins the first probes, `tr J = -1.1e4` excludes DOPRI5. No norm of `J` decides anything; the trace bound holds for every matrix. |
| S6 | Fatal callback off the trajectory | No routing call exists. REVIEW F2 completes (3.5). A fatal error at a finite-difference column of a probe ends the solve (rule R19) with a stop that names the probe and the column; with a supplied derivative no such point exists. Owner option in section 9. |
| S7 | Budget exhaustion | One budget for all runs, probes included; stop `MaxStepsReached { max_steps }` with the accepted prefix and all work; no trial share. Robertson with `max_steps = 500` (V03b shared-budget case, 266 attempts today): *estimate* about 60 DOPRI5 and probe attempts, then RODAS5P; success. With `max_steps = 10` the solve stops where the budget ends, as any single method would. |
| S8 | Supplied Jacobian on a non-stiff problem | RODAS5P takes one ramp (Jacobian at the accepted state, no finite-difference point), then DOPRI5 is probed (not excluded: `tr J` small) and wins; DOPRI5 continues with `O(log)` probes of RODAS5P. Today: RODAS5P for the whole span unless `regime(Nonstiff)` is declared. |
| S9 | Large `n`, cheap right-hand side | Dense: `Gamma` grows like `sqrt(n / 14)` (8.5 at `n = 1000`); without a declared price the product rule asks RODAS5P for a step advantage of about `sqrt(168 * 12214)`, about 1430, at `n = 1000` (one dense factorization costs as much as many DOPRI5 steps when the right-hand side is nearly free). With a band (declared or a band Jacobian) `Gamma` is about 1.24 and the advantage needed is about 2.5. Advice in the record: declare the price or the structure. Today: `n > 10` runs RODAS5P with a dense finite-difference Jacobian whatever the stiffness. |
| S10 | Small `n`, expensive right-hand side | The true cost is dominated by callbacks; for `n <= 10` `Gamma <= 1.42`, so the unknown-price decision is within 1.42 of the right one, and a declared price makes it exact. The `n + 1` columns per RODAS5P attempt are counted as what they cost. Today the screen spends `n + 1` expensive calls at `t0` on every such request. |

---

## 6. Other automatic switching schemes and their hyperparameters

| Scheme | Evidence | Decision | Hyperparameters (defaults) |
|---|---|---|---|
| LSODA, Petzold 1983 [lit 1]; ODEPACK `stoda.f` as shipped in SciPy [lit 2] | step sizes that Adams and BDF "could have used", from the current error estimate (`rh = 1 / (1.2 dsm^(1/l) + 1.2e-6)`), a Lipschitz estimate from the corrector convergence rate (Adams) or the Jacobian norm `pdnorm` (BDF), stability sizes `sm1` | to BDF if `rh2 >= ratio * rh1`; back to Adams if `rh1 >= rh2` ("the step size advantage must be at least 5/ratio = 1"); no test for `icount` steps after a switch; no test when the Adams order exceeds 5 | `ratio = 5`, `icount = 20`, 12 `sm1` values (0.5 to 0.025), 1.2 and 1.2e-6 in `rh`, 1e-5 in the `pdh * rh1` test, roundoff tests `100 * pnorm * uround` and `1000 * uround * pnorm`, `rh2 = 2` when polluted, order limit 5, `alpha >= 0.001`, rate cap 1024 |
| DOPRI5 code of Hairer [lit 3] | `h lambda = h norm(k2 - k6) / norm(y1 - ysti)` (stages at the same node, Shampine's estimate [lit 6]) | after 15 consecutive values above 3.25 the problem "seems to become stiff" and the run stops (no switch); 6 non-stiff values reset the count; tested every `nstiff` accepted steps | 3.25, 15, 6, `nstiff = 1000`; controller safety 0.9, factors 0.2 and 10, `beta = 0.04` |
| OrdinaryDiffEq `AutoSwitch` [lit 4] | `stiffness = abs(eigen_est dt / alg_stability_size)` | to the stiff method after more than `maxstiffstep` consecutive positives, back after more than `maxnonstiffstep` negatives; `dt` multiplied or divided by `dtfac`; error-check bypass up to `switch_max` | `maxstiffstep = 10`, `maxnonstiffstep = 3`, `nonstifftol = 9/10`, `stifftol = 9/10`, `dtfac = 2`, `stiffalgfirst = false`, `switch_max = 5`, a stability size per method |
| Mathematica `NDSolve` `StiffnessSwitching` [lit 5] | dominant eigenvalue estimates (direct, power method, subspace or Krylov iteration, norm bounds) against the linear stability boundary with a safety factor | switch when the stiffness test fails; a non-stiffness test switches back | `StiffnessTest` `MaxRepetitions {3, 5}`, `NonstiffTest` `MaxRepetitions {2, inf}`, a `SafetyFactor` (value not legible in the fetched text), iteration limits and basis sizes of the estimators |
| Shampine 1977 [lit 6] | free estimate of the dominant eigenvalue from stage differences | diagnosis of stiffness, not a switch | thresholds of the codes that use it |
| VigilODE switching adapter | work of windows, same-checkpoint competitive pilots, work units | entry and return by cost ratios, pressure, backoff | about 20 (R23) |
| VigilODE Auto today | Lipschitz lower bound at `t0`, budgeted trial | `L (t1 - t0) > 3.3 B`, trial `min(1000, max_steps/2)`, `n <= 10`, supplied derivative | 10, 1000, 1/2, 3.3 |
| This design | measured cost rates of both methods on the trajectory, `tr J` | break-even triggers with doubling, product rule over the price | doubling base 2 |

What the comparison shows. LSODA's `ratio = 5` is a fixed estimate of the cost of a BDF step relative to an Adams step
(Jacobian, factorization, iterations); this design measures that ratio and keeps both currencies apart. LSODA's
`icount = 20` and AutoSwitch's 10 and 3 are fixed hystereses; here the hysteresis is the break-even spend, measured in
the problem's own cost. The thresholds 3.25, 9/10 and LSODA's `sm1` encode a stability boundary and a margin; here the
stability boundary appears only as an exact method constant in a bound that cannot misclassify under its model.
The competitive framing follows rent-or-buy [lit 9]; for two states with full information the best deterministic
ratio of a metrical task system is 3 [lit 10]. The bounds here rely instead on measuring the challenger by committed
probes under model M.

---

## 7. Code changes

What the drivers must expose (and what the lineage already has):

1. **DOPRI5 session**, `crates/vigilode/src/session.rs`. Exists: start at any `(t, y)` with `h0`, `advance_to(target,
   quota)` with quota 1, checkpoint with `proposed_h` and counters [code session.rs:132-161, 354-365]; native
   per-attempt telemetry (requested and trial step, error norm, accepted, calls) [code dopri5.rs:1005-1016,
   1100-1131]. Add: pass the attempt record through the facade session and add one field, `factor_capped`, set where
   the controller factor is applied [code dopri5.rs:1243-1259, adaptive.rs:404-408]. The secant estimate is not needed.
2. **RODAS5P session** (new facade API, for example `Rodas5p::with_session`, mirroring `Dopri5::with_session`):
   `advance_to(target, quota)`, checkpoint with `proposed_h`, per-attempt record (accepted, `h`, `factor_capped`,
   fresh Jacobian or reuse), `last_jacobian_trace()` (sum of the diagonal of the stored dense matrix or band, O(n)),
   counters. Base: the native `Rodas5pFastSession` for an explicit dense Jacobian (the facade's finite-difference fill
   is one) [code rodas5p_fast.rs:2757-2868]. New native sessions are needed for the banded fast driver and the
   matrix-free driver. Until they exist: one bounded leg per attempt with `h0` = the carried proposal
   (`Rodas5p::solve_observed_bounded`, [code solve.rs:256-271]); with the memoryless integral controller this gives the
   same steps, but a retry after a rejection rebuilds the Jacobian (extra charged calls, to be measured).
3. **Counters**: dense factorization and solve operation counts in the dense kernel, as `BandedWork` counts them for the
   band kernel [code rodas5p_fast.rs:205-211]; per-attempt vector operations (or the constants of 2.2); user versus
   finite-difference Jacobian builds (exists: `Stats::finite_difference`, [code vigilode/solution.rs:233-239]).
4. `crates/vigilode/src/registry.rs`: return `proposed_next_step` from `Rodas5pWindowDriver` [code registry.rs:673]
   (helps the switching adapter too; the router uses sessions).

Router (crates/vigilode/src/unified):

5. New `economy.rs`: `CostVector { cb: u128, op: u128 }`; `RhsPrice { Unknown, Declared(u64), Measured }`; the
   comparison rule (exact `u128` products for spends, `f64` for rates); `Gamma` and `kappa*`; `Tenure` (spend,
   threshold, saturation, steady window); `Probe` outcome; `DecisionRecord`; `ExclusionRecord`;
   `a_priori_probe_cost(method, &MethodFacts)`; `R_MAX_DOPRI5 = 3.9785` with a unit test that recomputes it from
   `DOPRI5_A` and `DOPRI5_B`.
6. `analyzer.rs`: remove `EXPLICIT_TRIAL_MAX_DIMENSION`, `DOPRI5_REAL_STABILITY_BOUND`, `StiffnessScreen`,
   `stiffness_screen` and the `RouteBasis` values `SuppliedDerivative`, `DimensionAboveTrialLimit`, `NoBudgetForTrial`,
   `StiffnessScreen`, `UndeclaredRegimeTrial`, `PreflightCallbackFailed`; add `RouteBasis::MeasuredCost { first }`.
   The analysis makes no callback in any case. Keep R1 to R4.
7. `fallback.rs`: remove `EXPLICIT_TRIAL_ATTEMPT_LIMIT` and `explicit_trial_attempts`; keep `remaining_attempts`;
   replace `follows_explicit_stop` by a classification of numerical stops for both methods.
8. `router.rs`: the loop of 2.5 over the two sessions; finish() as today (counters, typed stop, accepted prefix).
9. `legs.rs`: `UnifiedLeg.role` (incumbent or probe); `CallbackStop` gains the context of the failing call (stage,
   finite-difference column or time derivative, probe or incumbent).
10. `options.rs`: `VigilodeOptions::rhs_price` (default `Unknown`). `solution.rs`: `VigilodeSolution::economy` with the
    decision and exclusion records, `Gamma_max` and the price used. `mod.rs` and `prelude.rs`: documentation and
    exports; the removed public constants and enum values are source-breaking for callers outside the repository (as
    REVIEW F9 noted for V03b) and can be kept one release as deprecated items.
11. Tests: a new `tests/unified_economy_contracts.rs` (section 8 discriminators that run in CI: determinism, no
    routing-only call, budget, fatal, neutrality without probes, bound check from the records, scenario routes);
    re-registration of the V03b contracts that pin the screen and its constants
    (`crates/vigilode/tests/unified_router_contracts.rs:1186, 1910-1911, 1929` and the preflight charging tests) and of
    the two assertions "no DOPRI5 leg" for the finite-difference stiff problems if the owner keeps the default start
    rule. The default-path golden files do not contain the unified router (no match in
    `crates/vigilode/tests/default_path_golden.rs`), so they do not change.

---

## 8. Evaluation plan (ready to pre-register)

**Arms.** A0: today's `Auto` at 80ba8388. A1: this design, price `Unknown`. A1-I: A1 with "RODAS5P first" for every
request (the owner alternative). A1-noX: A1 without the exclusion. A1-b1.6 and A1-b3: doubling base 1.6 and 3
(sensitivity, exploratory). A1-K: A1 with declared prices (exploratory, for S9 and S10). References: R_E = direct
`Dopri5::solve`, R_I = direct `Rodas5p::solve`, both with the whole budget.

**Problems.** From PREDECLARED.json:33-43: P_heat_dense (`n = 64`, dense Jacobian, span 10), P_heat_fd (`n = 8`, spans
1 and 100), P_scalar_dense, P_scalar_fd (`y' = -1e6 (y - cos t)`, span 1), P_oscillator (spans 1 and 1000), P_robertson
(with and without a dense Jacobian, spans 10, 40 and 1e5), P_heat_jvp (`n` 12 and 25), P_heat_banded (band Jacobian; and
finite differences with `Problem::banded(1, 1)`), P_mass (routing unchanged, RODAS5P), P_fatal (fatal for `t > 0.3`),
P_blowup (`y' = y^2`). New: N1 the guarded right-hand side of REVIEW F2 (fatal for `y > 1`, `y0 = 1`); N2 a right-hand
side fatal only at a finite-difference column point (`y1' = 0` with `y1 = 1` conserved and `f` fatal for `y1 > 1`, plus a
decaying component), which pins the residual exposure; N3 non-stiff non-normal `[[-1, 1e4], [0, -2]]`, span 10; N4 stiff
non-normal `[[-1e4, 1e6], [0, -1e3]]`, span 1; N5 weakly damped fast oscillation, eigenvalues `-1 +- 1000 i`, span 1;
N6 Arenstorf orbit, one period; N7 Lorenz, span 10; N8 HIRES; N9 OREGO; N10 Pollution (the classical stiff test set);
N11 stiffness that disappears: `y' = -k(t) (y - sin t)`, `k(t) = 1e4 exp(-20 t) + 1`, span 10; N12 Van der Pol
`mu = 1000`, two periods; N13 heat `n = 1000` with a declared band and with a dense finite-difference Jacobian, span
0.1; first-order upwind advection `n = 1000`, span 1; N14 the oscillator and Arenstorf with a supplied dense Jacobian;
N15 budgets: Robertson without a Jacobian with `max_steps` 500, 50 and 10; heat `n = 64` with 10. Tolerances
`(rtol, atol)`: `(1e-6, 1e-9)` (default), `(1e-4, 1e-7)`, `(1e-8, 1e-11)`.

**Measures** (deterministic counters only, no wall time): `CB` and `OP` as defined in 2.2; right-hand sides (finite
differences included), user Jacobian builds, finite-difference builds, JVPs, factorizations, solves, attempts (accepted
and rejected), number of runs, probes and switches, the decision and exclusion records (`Gamma`, `kappa*`), achieved
error at `t1` against a reference (analytic where available, else RODAS5P at `rtol = 1e-12`), status and stop, callback
entries counted inside the user closures, and the final-state bits of three repeated runs. The cost of a case at price
`kappa` is `kappa * CB + OP` for `kappa` in `{0, n, 10 n, 100 n, inf}` (at `inf` the callback count alone). For each case
and price, `R(arm) = cost(arm) / min(cost(R_E), cost(R_I))`.

**Discriminators** (comparisons where possible; the numbers 3 and the V03b bounds are existing frozen values):

- D1 worst case: for every price, `max over cases R(A1) <= max over cases R(A0)`.
- D2 central tendency: for every price, geometric mean of `R(A1)` <= geometric mean of `R(A0)`.
- D3 Robertson: `R(A1) < R(A0)` on every Robertson case without a Jacobian.
- D4 non-stiff set (P_oscillator span 1000, N3, N5, N6, N7, N14): `cost(A1) <= cost(A0)` at every price.
- D5 bound consistency: every A1 run satisfies the bound of Propositions 1 to 3 computed from its own record (`P`,
  `P_min`, `Gamma_max`, its regime changes as recorded switches). One violation refutes the analysis.
- D6 no routing-only call: callback entries counted in the closures equal the sum of the calls of the recorded attempts
  (the V03b "uncharged = 0" check, PREDECLARED.json:50); N1 succeeds like R_E.
- D7 determinism: three runs give identical routes, counters and final-state bits.
- D8 budget: attempts over all runs `<= max_steps`; exhausted runs report `MaxStepsReached { max_steps }` with their
  prefix.
- D9 accuracy: achieved error of A1 at most 3 times `max(error(R_E), error(R_I))` per case (the factor of the A gate of
  addendum A6, PREREGISTRATION.md:175, 274).
- D10 V03b guards: the bounded-work thresholds of PREDECLARED.json:46 for the four stiff contractive problems; P_fatal
  status `CallbackFailed { Rhs, "sensor offline" }`; no second method after a fatal callback.
- D11 neutrality: an A1 run that made no probe has the final-state bits and counters of the direct call of its method.

**Expected losses, declared in advance** (not failures): P_heat_fd and P_scalar_fd under A1 (DOPRI5 prefix, within
the bound of D5); very short non-stiff runs that pay one losing probe. A1-I is expected to remove the first and to pay
`n + 1` finite-difference calls at `t0` plus one RODAS5P step on every non-stiff finite-difference request.

**Refutation.** D1, D5, D6, D7 or D8 failing refutes the design; D3 or D4 failing refutes its economic claim; D2
failing with D1 holding means the design trades average cost for worst case, a result to report, not a pass.

---

## 9. Risks and questions for the owner

1. **Start rule for finite-difference problems** (rule, no number). Default: DOPRI5 starts, so that no point off the
   trajectory is evaluated before the economics ask for RODAS5P; a stiff-from-`t0` finite-difference problem then pays a
   bounded DOPRI5 prefix and the two V03b contract assertions "no DOPRI5 leg" for P_heat_fd and P_scalar_fd have to be
   replaced by the bound check D5. Alternative A1-I: RODAS5P starts on every request; it matches today's stiff results
   without the duplicated screen calls but evaluates `n + 1` column points at `t0` on every finite-difference request
   (today's exposure, REVIEW F2) and pays one RODAS5P step on non-stiff ones. Which does the owner want?
2. **Fatal error at a finite-difference column of a probe.** Default: terminal (rule R19, as today). Alternative: the
   probe fails and DOPRI5 continues, since its trajectory never visits that point; this treats one fatal error as
   recoverable for the router, which changes the callback contract.
3. **Price.** Default `Unknown` (deterministic, `Gamma` reported). Should the route offer `Measured` (timing, recorded,
   replayable) at all, given the bit-reproducibility rule of the repository?
4. **Doubling base 2** stays as the one tunable (section 4), or the model-derived 1.61.
5. **Driver work.** Sessions for the banded and matrix-free RODAS5P drivers do not exist (O3); until they do, the
   per-attempt bounded-leg fallback adds a Jacobian rebuild after each rejection on those routes.
6. **Source compatibility.** Removing the public constants and enum values of the screen is source-breaking outside the
   repository.
7. **Model risk.** The bounds rest on model M; noisy rates near the stability limit, regimes shorter than a probe and
   strong nonlinearity weaken them (3.7). D5 checks each run against its own bound.
8. **Not authorized changes avoided.** No controller constant, tolerance or G0 changes; the switching adapter, RODAS5P,
   DOPRI5 and the default path are untouched by the router change itself (driver sessions and counters are additions).

---

## 10. Sources

Literature (fetched or searched in this session):

1. L. R. Petzold, "Automatic selection of methods for solving stiff and nonstiff systems of ordinary differential
   equations", SIAM J. Sci. Stat. Comput. 4 (1983) 136-148 (bibliographic record; the algorithm details below are from
   the code, not from the paper's full text).
2. ODEPACK `stoda.f` (LSODA core) as shipped in SciPy v1.10.0:
   https://raw.githubusercontent.com/scipy/scipy/v1.10.0/scipy/integrate/odepack/stoda.f (`ratio = 5.0d0`,
   `icount = 20`, `sm1` data, switch tests and comments quoted in section 6).
3. E. Hairer, `dopri5.f` as shipped in SciPy v1.10.0:
   https://raw.githubusercontent.com/scipy/scipy/v1.10.0/scipy/integrate/dop/dopri5.f (IWORK(4) `nstiff` default 1000;
   `HLAMB.GT.3.25D0`, `IASTI.EQ.15`, `NONSTI.EQ.6`; WORK(2) 0.9, WORK(3) 0.2, WORK(4) 10, WORK(5) 0.04).
4. OrdinaryDiffEq.jl, `lib/OrdinaryDiffEqCore/src/composite_algs.jl` (master):
   https://raw.githubusercontent.com/SciML/OrdinaryDiffEq.jl/master/lib/OrdinaryDiffEqCore/src/composite_algs.jl
   (`AutoSwitch` defaults and `is_stiff`).
5. Wolfram Language documentation, "Stiffness Detection":
   https://reference.wolfram.com/language/tutorial/NDSolveStiffnessTest.html.
6. L. F. Shampine, "Stiffness and nonstiff differential equation solvers, II: Detecting stiffness with Runge-Kutta
   methods", ACM TOMS 3 (1977) 44-53, doi:10.1145/355719.355722 (bibliographic record).
7. G. Hall, "Diagnosing stiffness for Runge-Kutta methods", SIAM J. Sci. Stat. Comput. 12 (1991),
   doi:10.1137/0912015 (bibliographic record, not used for a claim).
8. FFTW manual, "Planner Flags", https://www.fftw.org/fftw3_doc/Planner-Flags.html, and FFTW FAQ section 3,
   https://fftw.org/faq/section3.html (plans chosen by timing vary between runs; `FFTW_ESTIMATE` or wisdom give identical
   results).
9. A. R. Karlin, M. S. Manasse, L. Rudolph, D. D. Sleator, "Competitive snoopy caching", Algorithmica 3 (1988) 79-119,
   doi:10.1007/BF01762111 (rent-or-buy competitive analysis).
10. A. Borodin, N. Linial, M. Saks, "An optimal on-line algorithm for metrical task system", J. ACM 39 (1992)
    745-763 (`2n - 1` for `n` states).

Repository records: research/production_loop_20261116/unified_router/PREDECLARED.json, RECEIPT.json, REVIEW.json,
REPORT_KO.md, NOTES.txt; research/production_loop_20261116/PREREGISTRATION.md, PROGRESS.md;
docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json.

Computations [calc] (scratch scripts outside the repository): the DOPRI5 stability polynomial from the tableau
(`b^T A^5 e = 1/600`, `b^T A^6 e = 0`); its real boundary 3.3065678926 (`R = +1`), imaginary reach 0.99719, largest
modulus 3.97841 (left half-plane 3.39903); the minimax doubling base `b = 1.6123` with worst ratio 2.382 (2.531 at
`b = 2`); the `Gamma` table of 2.3 from the per-attempt counts of 2.2.
