# Node c1-transaction: Transaction, Budget, AdmissionRule and Telemetry types

Production wave-2, Task 1. Plan: `docs/plan/PRODUCTION_PLAN_20261020.md` §4 idea 2, §5 T8, §6 C1.
Branch `production/c1-transaction` from `148b515`. Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0);
`rust-toolchain.toml` (1.94.1) was not used and not edited. No existing driver is migrated, and no
default behaviour changes: the module is new and nothing calls it yet.

## What was done

* `crates/rodas5p-integrators/src/transaction.rs` (new), exported as `rodas5p_integrators::transaction`, with
  the main types also re-exported from `lib.rs`.
  * `Transaction<S>` / `Trial<'_, S>`. `begin(&committed)` clones the caller's committed state (state,
    controller, history, Krylov space: whatever `S` holds). `charge(WorkCounters) -> units` puts the work
    into the ledger's attempted counters at once. `commit(self) -> S` returns the trial state;
    `rollback(self)` discards it. A drop without either is a rollback and also counts in `trials_dropped`.
    `TransactionLedger::is_balanced` checks `begun = committed + rolled_back`,
    `attempted_work = committed_work + rolled_back_work` (via `WorkCounters::accumulate`) and the same
    identity for units. The trial borrows the ledger mutably, so the ledger can only be read between
    trials, where the identities hold. Units come from a `WorkMeasure = fn(&WorkCounters) -> u64`. The
    default is `callback_work_units = rhs_evaluations + jvp_vectors`, the two counters the coupled crate's
    budget caps; `with_measure` takes a custom one.
  * `Budget`: a hard cap in work units. `reserve(units)` is a CAS loop on an `AtomicU64` (the
    `Pexprb54s4JvpBudget` pattern) and runs before the work is scheduled (the `vigilode-coupled`
    `Work::reserve` pattern). A refusal reserves nothing and returns the typed
    `BudgetExhausted { cap, used, requested }`; refusals are counted. Reservations are never refunded.
    `Trial::reserve(&Budget, units)` also records reservations and refusals in the ledger.
  * `AdmissionRule` (T8, Proposition 8). Each `RegimeBin` has its own Beta(α, β) posterior, prior
    (1, 1), changed only by `record_attempt`. That call requires positive charged work and otherwise
    returns `UnchargedAttempt` or `InvalidCost`. With τ = (C_a + C_v)/C_f and m = ⌈10 τ⌉ (at least 1),
    `decide` returns:
    1. `AdmitPosterior` when U(α, β) > τ;
    2. otherwise `AdmitThrottled` once ≥ m protected steps have been accepted in the bin since its last
       attempt, and `DeclineThrottle` before that;
    3. `DeclineOverhead` in place of either admission when net overhead + (C_a + C_v) > protected base / 10
       (the overhead guard; see Deviations).

    Costs are measured work units supplied by the caller. The rule uses no wall time and no randomness,
    and it serializes.
  * Upper bound U: the Wilson score upper bound at the posterior pseudo-counts n = α + β, p̂ = α/n,
    z = z₀.₉₅ = 1.6448536269514722. The doc comment explains the choice: it is closed form (so
    bit-deterministic), lies strictly in (0, 1), is at least the posterior mean, and moves monotonically
    with α and β. The comment also states that U approximates the Beta quantile but does not bound it.
    Against the scipy 0.95 quantile it reads low when successes dominate at small n (Beta(1, 1): 0.879 vs
    0.950) and high when failures dominate (Beta(1, 20): 0.188 vs 0.139). The overhead guarantee does not
    rely on this bound.
  * `RegimeTelemetry { ritz: Option<RitzSummary>, refresh_q_estimate, jacobian_drift_estimate, rejection_streak }`
    and `RitzSummary { spectral_radius_estimate, max_imag_over_real, ritz_values }` are plain serde data.
    `None` means the quantity was not estimated. The doc comment notes that an infinite |Im|/|Re| cannot
    be carried in JSON. Telemetry is not mapped to bins here; that mapping is left to the caller.
* `crates/rodas5p-integrators/tests/c1_transaction_contracts.rs` (new, 20 tests).

## Deviation from the brief: overhead guard

Read literally, the brief's rule (UCB admission, plus the m-throttle below threshold) does not meet its own
bounded-overhead test. The prior Beta(1, 1) gives U = 0.879, so a candidate that always fails is admitted
without throttling until U drops below τ. Those attempts come on top of the throttle's τ/m ≤ 10%. A
mutation check (guard disabled) fails the test at τ = 0.4, N = 100: the candidate is charged 120 against a
protected cost of 1000 (12%, 30 attempts). The guard charges both kinds of attempt against a single
allowance, 1/10 of the bin's counterfactual protected cost, where a success adds the protected step it
replaced and also subtracts that step from the net overhead. So:

* the bin's actual cost stays at or below 1.1 × the protected-only cost (exactly G-C), up to the gap
  between one estimated attempt cost and one measured attempt cost;
* the guard never blocks a candidate that pays for itself;
* a fresh bin makes its first attempt after about m protected steps.

With no successes the guard reduces to Proposition 8's one attempt per m steps. The factor 10 is
`AdmissionConfig::overhead_factor` and also sets m.

## Commands (all with `--locked -j 1`, `CARGO_TARGET_DIR=/home/claude/targets/c1-transaction`, debug off)

| Command | Exit | Result |
|---|---|---|
| `cargo test -p rodas5p-integrators --test c1_transaction_contracts` (RED, before the module) | 101 | E0432 unresolved import `rodas5p_integrators::transaction` |
| same (GREEN) | 0 | 20 passed |
| same with the guard disabled (mutation, reverted) | 101 | 19 passed, 1 failed: τ = 0.4, N = 100, charged 120 > 0.1 × 1000 |
| `cargo test -p rodas5p-integrators --lib` | 0 | 75 passed, 1 ignored |
| `cargo clippy -p rodas5p-integrators --lib --test c1_transaction_contracts` | 0 | no warnings in touched files (one `neg_cmp_op_on_partial_ord` fixed); existing warnings elsewhere unchanged |
| `rustfmt --edition 2024 --check` on the 3 touched files | 0 | clean |

## Test contracts

* Accounting: commit, rollback and drop, plus a 60-trial mixed sequence: attempted = committed +
  rolled back for counts, `WorkCounters` and units; only committed trials move the state. A custom measure
  is honoured. Trial reservations and refusals are recorded, and rollback does not refund them.
* Budget: an exact fill to the cap, then `BudgetExhausted { used: cap }`; overflowing requests are refused
  without changing `used`. Concurrency: 4 std threads × 20 000 deterministic requests of 1–17 units against
  caps of 1000, 4093 and 50 000. Used ≤ cap, used = Σ granted, refusals counted, and the budget fills to
  within one request.
* Bounded overhead: an always-failing candidate at τ ∈ {0.05, 0.4, 0.5, 0.9, 1.5, 6.0}, N ∈ {100, 1000}.
  Charged candidate cost ≤ 0.10 × protected cost, and at least one attempt is made in every case.
* An always-succeeding, cheaper candidate at τ ∈ {0.2, 0.5, 0.8}: at most m declines before the first
  attempt, and every decision admits after it, through 50 attempts plus 500 more.
* Distinct decline statuses (throttle vs overhead); the posterior changes only through charged attempts;
  bins are independent; U is monotone, inside (0, 1) and at least the mean; decisions and serialized state
  are deterministic; invalid costs and configs return typed errors; telemetry survives a serde round trip.

## Status of numbers

These are logical and accounting results; the module measures no wall time. The 10% bound is enforced by
construction (the guard) and checked on the tested cost grid. It is not a corpus measurement, and the
G-C gate still requires the corpus A/B of §6.
