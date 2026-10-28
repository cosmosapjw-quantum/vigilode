# Adaptive reconsideration of causal projected roots

## Result and implementation scope

We implemented `integrate_switching_causal_observed` in `vigilode-coupled`. A
projected candidate that repeatedly cannot satisfy the original full-space
residual screen is temporarily bypassed. A later probe restores access when its
work and validity again become favourable. This makes the original causal root
an available specialization without charging a doomed basis construction on
every physical step. `integrate_sequential_raw_observed` is the same-target
protected comparator; no deliberate budget failure is needed to reach it.

The controller supports the explicitly declared candidate families Forward,
Picard and ResidualFirst. Forward and Picard retain the original frozen-base JVP
requirement and small projected matrices. ResidualFirst retains its existing
homotopy/corrector logic and may use current-stage JVPs and Newton updates. The
controller does not claim those three families have equal derivative contracts.
The user chooses the candidate to reconsider; the implemented automatic decision
is between that candidate and the guarded sequential raw-U solve.

## Fresh construction

Let the common raw-U step target be `F_h(U)=0`, with accepted endpoint computed
from its eight stages. The original causal code solves the projected target and
checks the original-space residual. When a projection fails, executing the same
projection on the next nearby step is a useful experiment only if its expected
benefit exceeds its cost. This is a sampling problem rather than a new stability
restriction on the Rosenbrock formula.

The algorithm first accepts one protected step. Denote its measured work score
by `B`. For a candidate step, let `C` be its actual score, including attempted
callbacks and any weighted projection operations. The default score is simply
`rhs_attempted + jvp_attempted`; it is neither elapsed time nor a flop estimate.
Optional nonnegative weights expose the workload's measured callback/projection
cost assumptions. Each accepted protected step updates `B <- 0.75 B + 0.25 C_p`.

A valid accepted candidate remains active if `C <= 1.25 B`. A root decline, or an
accepted candidate with a larger score, starts a protected cooldown. The initial
implementation doubled cooldown lengths from four to 64 accepted protected
steps. Its seven-workload benchmark exposed missed successful candidate
opportunities: in the broad-spectrum case, fixed scheduling had 11 successful
projected roots among 61 attempts, while the initial adaptive schedule's four
probes all failed. This increased JVP work from 22,657 to 25,443. The changing-rate
case similarly increased JVP work from 11,131 to 13,993. We preserved those results
and used them to revise the failure cadence once more.

The final default sets `exploration_overhead_fraction = Some(epsilon)` with
`epsilon = 0.25`. A failed probe with actual candidate score `C` uses

`L = clamp(ceil(C/(epsilon P)) - 1, 1, max_cooldown)`.

Here `P` is the successfully accepted same-attempt protected score when available;
otherwise it is the last finite positive protected reference. The next probe is
allowed after `L` accepted protected steps. For fixed representative finite
positive costs and no upper clipping, the stationary attempted-cost model gives

`C / ((L + 1) P) <= epsilon`.

Indeed `L+1 >= C/(epsilon P)` by construction. To interpret the denominator as
an accepted-step cycle, the failed-probe attempt must itself return an accepted
protected endpoint; that endpoint and the following `L` accepted protected steps
then contribute `(L+1)P`. If its protected endpoint is rejected, the receipt uses
a previous successful cost and is only a prediction. In particular, it does not
bound realized cost per accepted progress in a rejection-heavy interval.
Thus a cheap failed probe is tested
again sooner: `C <= 2 epsilon P` gives `L = 1`. The formula and its budget statement
refer to exact real arithmetic; computed binary64 cost estimates do not constitute
an interval certificate. The receipt reports maximum clipping and the predicted
proxy ratio. With clipped `L`, the ratio can exceed the requested fraction.

Setting the new parameter to `None` preserves the original exponential failure
policy. Accepted but expensive candidates still use exponential cooldown. A
re-entry probe needs `C <= B`; successful re-entry resets backoff. Embedded
local-error rejection instead retains the root route and lets the existing
controller decrease `h`. An overflowed score is recorded as null with
`work_scores_finite=false`, and cannot justify candidate retention.

This deliberately separates three observations: validity of the selected stages,
accuracy of the physical step, and usefulness of the root algorithm. Candidate
failure work is retained even when the protected endpoint is subsequently
accepted. The cost reference can come from another `h` or state; it is explicitly
a heuristic for work selection and has no uniform wall-time or regret guarantee.

## Proposition CS1: acceptance invariance under routing

Assume the existing candidate and protected routines satisfy their stated raw-U
contracts, callbacks are pure for fixed state and model epoch, and the same
error-policy assumptions hold for all attempted stage domains. Then replacing
fixed candidate scheduling with this controller does not weaken the selected
step's residual/embedded/conditional acceptance conditions.

Proof. The controller chooses only whether to invoke the existing projected
routine before the protected routine. Both choices return through the same
`RawStepAttempt` interface. Candidate success retains the existing full-space
residual screen; the direct protected path executes the same original-space
validation and error assessment as fallback. The outer trajectory commits only
when `effective_error <= 1`, the endpoint is finite and has the correct dimension,
the model epoch is unchanged, and output acceptance succeeds. The new route
choice has no branch that commits state independently of those conditions.
Callback/input/model failures remain terminal; trusted numerical failures use
the existing smaller-step retry classification. Thus every accepted endpoint
satisfies exactly the relevant existing conditions. This is a program contract
argument; the embedded estimator alone is not a global error theorem.

## Proposition CS2: bounded repetition of an unsuccessful specialization

Consider an interval in which every projected probe declines and the controller
is not reinitialized. Let `c >= 1` be a lower bound on cooldown lengths, `P` the
number of projected probes and `A` the number of accepted protected steps during
this interval after the first probe. For the final priced policy, use `c=1`;
for the legacy policy, use its initial cooldown. Then

`P <= 1 + floor(A/c)`.

Proof. Between two successive probes, the controller requires at least `c`
accepted protected steps. Numerical retries do not decrement the cooldown.
The priced policy enforces its one-step minimum, and legacy backoff only
increases its initial requirement. Hence `(P-1)c <= A`. Each probe's RHS
and JVP attempts remain bounded by the candidate's existing finite budgets, so
its additional callback work is bounded by `P (max_rhs + max_jvp)` under the
default score. Successful probes can remain active and are outside this
persistent-decline bound. The theorem bounds repeated exploration work, not
elapsed time or total protected integration cost.

The acceptance-clock design matters: a difficult protected step with many local
rejections does not repeatedly reopen the failed projected route. Hysteresis
separates retention and re-entry, and periodic probes permit recovery after a
problem changes regime.

## Executable verification

Parent integration ran all 73 existing/new coupled tests after the first code
revision, including five new integration contracts. The new rank-one projection
case uses a two-rate diagonal system with a deliberately insufficient actual
basis, not a fake storage-budget failure. Switching reduces both RHS and JVP
counts while preserving bit-identical protected output samples. All three
candidate choices pass the analytic two-rate endpoint comparison. Fatal callback
errors with a misleading `CoreError::LinearSolve` tag remain terminal and are
counted once; invalid policies and protected inputs do no callback work. Direct
protected integration works independently of the candidate storage ceiling.

Three additional unit contracts exercise accepted-step cooldown timing,
hysteresis and successful re-entry; distinguish local-error rejection from
projection decline; and verify weighted protected work includes validation
without double-counting callback receipts. After environment cache recovery,
all 76 coupled contracts passed; see `evidence/runs/causal-recovered-tests`.
Five more unit contracts for the final priced policy cover cheap-probe cadence
versus legacy backoff, the proxy-overhead formula and upper clipping, use of the
last successful reference after a rejected protected trial, overflow telemetry,
and invalid exploration fractions. All 81 coupled contracts passed in the final execution; see
`evidence/runs/causal-round3-tests.receipt.json` (exit 0). New benchmark measurements and source identities live
in the package's benchmark and validation files rather than being inferred here.

## Revision record

1. Initial implementation: explicit protected dispatch, shared input validation,
   adaptive driver integration, policy receipts and five integration contracts.
   All 73 coupled tests passed in the parent execution.
2. Added focused controller unit contracts for hysteresis/recovery and work
   accounting. All 76 coupled contracts passed after cache recovery. The
   seven-workload benchmark diagnosed missed candidate opportunities under
   long cooldowns, with increased total work despite fewer failed probes.
3. Final permitted algorithm revision: cost-priced failure cadence, reproducible
   legacy option, explicit overflow telemetry and five targeted unit contracts.
   Primary workloads and a separate persistent-projection-failure case are
   rerun once under the final default. Final measurements remain authoritative;
   the formula alone does not establish a net speed advantage.

The finite prefix propagation theorem for Picard remains useful, but its work
cost and synchronization can still dominate. This selector therefore exposes
Picard as a concrete, re-testable candidate instead of asserting that finite
propagation alone establishes a wall-clock advantage. Similarly, the existing
homotopy corrector retains its role for a caller-chosen root hypothesis without
being labelled globally Newton-free.

Calibration requires an accepted protected step. This controller therefore does
not rescue an initial failure of the protected method. In serialized results,
`fallback_attempted` includes intentionally selected protected calibration and
cooldown work; use `candidate_skipped` to distinguish these from fallback after
an actual projected trial.
