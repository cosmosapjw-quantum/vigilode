//! Economy contracts of the opt-in switching adapter: bounded trial work,
//! geometric back-off with trend re-arm, coalesced implicit tenure and a
//! bounded allocation budget for the explicit batch loop. None of these
//! relax a local error test; every discarded trial stays fully charged.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, Rodas5p, SwitchingConfig,
    SwitchingEvidence, SwitchingMethod, SwitchingSegment, SwitchingSolution, SwitchingSolver,
};

struct CountingAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    (result, COUNT.with(Cell::get))
}

fn rotation() -> Problem {
    Problem::new(2, |_, y, out| {
        out[0] = -100.0 * y[1];
        out[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, out| {
        out[0] = -100.0 * v[1];
        out[1] = 100.0 * v[0];
        Ok(())
    }))
    .autonomous()
}

/// `y' = -lambda(t) (y - g(t)) + g'(t)` with `g = cos(omega t)`: the exact
/// solution from `y(0) = 1` is `g`. `lambda` switches at `ts`.
fn forced(omega: f64, ts: f64, lambda_before: f64, lambda_after: f64) -> Problem {
    let lambda = move |t: f64| if t < ts { lambda_before } else { lambda_after };
    Problem::new(1, move |t, y, out| {
        let g = (omega * t).cos();
        let dg = -omega * (omega * t).sin();
        out[0] = -lambda(t) * (y[0] - g) + dg;
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _, j| {
        j[0] = -lambda(t);
        Ok(())
    }))
    .time_derivative(move |t, _, out| {
        let dg = -omega * (omega * t).sin();
        let ddg = -omega * omega * (omega * t).cos();
        out[0] = lambda(t) * dg + ddg;
        Ok(())
    })
}

fn stiff_forced() -> Problem {
    forced(1.0, f64::INFINITY, 10_000.0, 10_000.0)
}

fn options(h0: f64) -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0,
        max_steps: 200_000,
        ..Default::default()
    }
}

fn window_target(t0: f64, t1: f64, window: usize, count: usize) -> f64 {
    if window == count {
        t1
    } else {
        t0 + (t1 - t0) * (window as f64 / count as f64)
    }
}

fn callback_units(s: &SwitchingSegment, dimension: usize) -> u64 {
    s.rhs_calls + s.jvp_vectors + s.ft_calls + s.jacobian_builds * dimension as u64
}

fn conserved(r: &SwitchingSolution) {
    assert!(r.stats.accounting_complete);
    let segments = &r.stats.segments;
    assert_eq!(
        r.stats.attempts,
        segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert_eq!(
        r.stats.rhs_calls,
        segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
    assert_eq!(
        r.stats.jvp_vectors,
        segments.iter().map(|s| s.jvp_vectors).sum::<u64>()
    );
    assert_eq!(
        r.stats.jacobian_builds,
        segments.iter().map(|s| s.jacobian_builds).sum::<u64>()
    );
    assert_eq!(
        r.stats.linear_solves,
        segments.iter().map(|s| s.linear_solves).sum::<u64>()
    );
    assert_eq!(
        r.stats.accepted_steps + r.stats.rejected_steps,
        r.stats.attempts
    );
    let mut t = r.t[0];
    for s in segments {
        assert_eq!(
            s.start_time, t,
            "segment must start at the committed checkpoint"
        );
        if s.committed {
            t = s.reached_time;
        }
    }
    assert_eq!(t, r.final_time);
    assert!(r.t.windows(2).all(|w| w[1] > w[0]));
}

fn pilots(r: &SwitchingSolution) -> Vec<(usize, &SwitchingSegment)> {
    r.stats
        .segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_probe && s.method == SwitchingMethod::Rodas5p)
        .collect()
}

fn probes(r: &SwitchingSolution) -> Vec<(usize, &SwitchingSegment)> {
    r.stats
        .segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_probe && s.method == SwitchingMethod::Dopri5)
        .collect()
}

#[test]
fn discarded_pilot_work_is_bounded_by_the_entry_ceiling() {
    let c = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    let r =
        SwitchingSolver::solve(&rotation(), (0.0, 1.0), &[1.0, 0.0], &options(1e-7), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert_eq!(r.stats.switches, 0);
    conserved(&r);
    let pilots = pilots(&r);
    assert!(!pilots.is_empty());
    let mut ceiling_stops = 0;
    for (index, pilot) in &pilots {
        // The replay precedes the explicit candidate it was compared with.
        let comparator = &r.stats.segments[index + 1];
        assert!(comparator.committed && comparator.method == SwitchingMethod::Dopri5);
        assert_eq!(pilot.start_time, comparator.start_time);
        assert_eq!(pilot.target_time, comparator.reached_time);
        assert!(!pilot.committed);
        let cap = (c.entry_cost_ratio * comparator.work_units).floor() as u64;
        // At most one callback (a Jacobian of n columns at worst) past the cap.
        assert!(
            callback_units(pilot, 2) <= cap + 2,
            "pilot charged {} callback units against cap {cap}",
            callback_units(pilot, 2)
        );
        assert!(pilot.attempts <= c.implicit_pilot_attempt_budget);
        assert!(matches!(
            pilot.evidence,
            SwitchingEvidence::ImplicitPilotIncomplete
                | SwitchingEvidence::ImplicitNotCheaperOnPrefix
        ));
        if pilot.decision.contains("ceiling") {
            ceiling_stops += 1;
            assert!(!pilot.success);
        }
    }
    assert!(ceiling_stops > 0, "{:?}", r.stats.segments);
    assert!((r.final_state[0] - 100.0_f64.cos()).hypot(r.final_state[1] - 100.0_f64.sin()) < 1e-5);
}

#[test]
fn economical_pilot_is_never_stopped_by_the_ceiling() {
    let c = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    let r =
        SwitchingSolver::solve(&stiff_forced(), (0.0, 1.0), &[1.0], &options(1e-7), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(r.stats.switches >= 1);
    conserved(&r);
    let pilots = pilots(&r);
    let passed: Vec<_> = pilots
        .iter()
        .filter(|(_, s)| s.evidence == SwitchingEvidence::ImplicitCheaperOnPrefix)
        .collect();
    assert_eq!(passed.len(), 1, "{:?}", r.stats.segments);
    let (index, pilot) = passed[0];
    let comparator = &r.stats.segments[index + 1];
    assert!(pilot.success);
    assert!(!pilot.decision.contains("ceiling"));
    assert!(pilot.work_units <= c.entry_cost_ratio * comparator.work_units);
    assert!((r.final_state[0] - 1.0_f64.cos()).abs() < 1e-6);
}

#[test]
fn discarded_pilots_back_off_geometrically_and_rearm_on_a_work_rate_jump() {
    let ts = 0.6;
    let p = forced(400.0, ts, 1.0, 1.0e6);
    let c = SwitchingConfig {
        window_count: 8,
        ..SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly)
    };
    assert_eq!(c.pilot_backoff_factor, 2);
    assert!(c.rearm_rate_ratio > 1.0);
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(1e-4), &c).unwrap();
    assert!(r.success, "{}", r.message);
    conserved(&r);
    // Committed explicit batches between consecutive discarded pilots before
    // the transition grow geometrically.
    let mut committed_explicit = 0usize;
    let mut gaps = Vec::new();
    let mut previous: Option<usize> = None;
    let mut first_entry: Option<f64> = None;
    for s in &r.stats.segments {
        // A replay whose prefix ends before the transition is a smooth-phase pilot.
        if s.is_probe && s.method == SwitchingMethod::Rodas5p && s.target_time < ts {
            if let Some(last) = previous {
                gaps.push(committed_explicit - last);
            }
            previous = Some(committed_explicit);
            assert!(!s.committed);
        }
        if s.committed && s.method == SwitchingMethod::Dopri5 {
            committed_explicit += 1;
        }
        if s.committed && s.method == SwitchingMethod::Rodas5p && first_entry.is_none() {
            first_entry = Some(s.start_time);
        }
    }
    assert!(
        gaps.len() >= 2,
        "gaps {gaps:?} segments {:?}",
        r.stats.segments.len()
    );
    // One discard keeps the registered cooldown; each further discard doubles it.
    for (i, gap) in gaps.iter().enumerate() {
        let cooldown = c.entry_pilot_cooldown_batches * c.pilot_backoff_factor.pow(i as u32);
        assert!(
            *gap >= cooldown,
            "gap {i} of {gap} batches is below the grown cooldown {cooldown}: {gaps:?}"
        );
    }
    assert!(gaps.windows(2).all(|pair| pair[1] > pair[0]), "{gaps:?}");
    // The late stiff transition re-arms the pilot at once: entry within a
    // few explicit batches of the transition, not after the grown cooldown.
    let entry = first_entry.expect("the stiff phase must enter RODAS");
    assert!(
        entry >= ts - 1e-3 && entry - ts < 6e-4,
        "entry at {entry}, transition at {ts}"
    );
    assert!((r.final_state[0] - (400.0_f64).cos()).abs() < 1e-4);
    // Negative control: with the trend re-arm disabled (a ratio no finite
    // jump reaches), the grown cooldown delays the entry by many batches.
    let no_rearm = SwitchingConfig {
        rearm_rate_ratio: 1e300,
        ..c.clone()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(1e-4), &no_rearm).unwrap();
    assert!(r.success, "{}", r.message);
    let late_entry = r
        .stats
        .segments
        .iter()
        .find(|s| s.committed && s.method == SwitchingMethod::Rodas5p)
        .map(|s| s.start_time)
        .expect("the stiff phase must still enter RODAS eventually");
    assert!(
        late_entry - ts > 1e-3,
        "without re-arm the entry came at {late_entry}"
    );
}

#[test]
fn discarded_probes_back_off_and_are_work_bounded_on_a_persistently_stiff_problem() {
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 16,
        ..Default::default()
    };
    assert_eq!(c.probe_backoff_factor, 2);
    let r =
        SwitchingSolver::solve(&stiff_forced(), (0.0, 4.0), &[1.0], &options(1e-3), &c).unwrap();
    assert!(r.success, "{}", r.message);
    conserved(&r);
    let probes = probes(&r);
    // Residence 2 then interval 4: legacy probes at windows 5, 9 and 13.
    // Back-off (4, 4, 8): windows 5 and 9 only.
    assert!(!probes.is_empty());
    assert!(probes.len() <= 2, "{} probes", probes.len());
    for (index, probe) in &probes {
        assert!(!probe.committed && !probe.success);
        assert!(probe.attempts <= c.probe_attempt_budget);
        let previous = r.stats.segments[..*index]
            .iter()
            .rev()
            .find(|s| s.committed && s.method == SwitchingMethod::Rodas5p)
            .unwrap();
        let implicit_rate = previous.work_units / (previous.reached_time - previous.start_time);
        let cap = c.return_cost_ratio * implicit_rate * (probe.target_time - probe.start_time);
        // One unit of rounding slack plus at most one explicit attempt
        // (7 RHS) past the return ceiling.
        assert!(
            (probe.rhs_calls as f64) <= cap.floor() + 8.0,
            "probe charged {} RHS against return cap {cap}",
            probe.rhs_calls
        );
    }
    assert!(
        probes.iter().any(|(_, p)| p.decision.contains("ceiling")),
        "the return ceiling must have ended at least one recheck: {:?}",
        probes.iter().map(|(_, p)| p.decision).collect::<Vec<_>>()
    );
    assert!((r.final_state[0] - 4.0_f64.cos()).abs() < 1e-6);
}

#[test]
fn return_to_explicit_still_happens_after_a_stiff_to_smooth_transition() {
    let ts = 1.5;
    let p = forced(2.0, ts, 10_000.0, 1.0);
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 16,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 4.0), &[1.0], &options(1e-3), &c).unwrap();
    assert!(r.success, "{}", r.message);
    conserved(&r);
    let returned = r
        .stats
        .segments
        .iter()
        .find(|s| s.is_probe && s.committed && s.method == SwitchingMethod::Dopri5)
        .unwrap_or_else(|| panic!("no committed return probe: {:?}", r.stats.segments));
    // The first recheck after the transition is discarded at most once
    // (window 5 is still stiff), so the return comes within one grown
    // interval of the transition, well inside the span.
    let window = 4.0 / 16.0;
    let limit = ts + (c.probe_interval * c.probe_backoff_factor) as f64 * window;
    assert!(limit < 4.0);
    assert!(
        returned.start_time >= ts && returned.start_time <= limit,
        "returned at {} (transition {ts}, limit {limit})",
        returned.start_time
    );
    assert!(r.stats.switches >= 2);
}

#[test]
fn implicit_tenure_coalesces_windows_between_probes_and_seeds_its_step() {
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 16,
        ..Default::default()
    };
    let o = options(1e-3);
    let r = SwitchingSolver::solve(&stiff_forced(), (0.0, 4.0), &[1.0], &o, &c).unwrap();
    assert!(r.success, "{}", r.message);
    conserved(&r);
    let implicit: Vec<_> = r
        .stats
        .segments
        .iter()
        .filter(|s| s.committed && s.method == SwitchingMethod::Rodas5p)
        .collect();
    // One tenure for the residence, one up to the first recheck, then one
    // per backed-off recheck: legacy ran sixteen separate windows.
    let rechecks = probes(&r).len();
    assert!(
        implicit.len() <= rechecks + 2,
        "{} implicit batches for {rechecks} rechecks",
        implicit.len()
    );
    assert_eq!(implicit.iter().map(|s| s.windows).sum::<usize>(), 16);
    for w in 1..=16 {
        let target = window_target(0.0, 4.0, w, 16);
        assert!(
            r.t.contains(&target),
            "window endpoint {target} missing from {:?}",
            r.t
        );
    }
    for pair in implicit.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        let mean = (previous.reached_time - previous.start_time) / previous.accepted_steps as f64;
        let seed = mean.min(next.target_time - next.start_time);
        assert_eq!(next.initial_step.to_bits(), seed.to_bits());
        assert!(next.initial_step != o.h0);
    }
    let direct = Rodas5p::solve(&stiff_forced(), (0.0, 4.0), &[1.0], &o).unwrap();
    assert!(direct.is_success());
    let implicit_attempts: usize = implicit.iter().map(|s| s.attempts).sum();
    // One landing per window plus a short ramp per tenure restart.
    assert!(
        implicit_attempts <= direct.stats.attempts + 16 + 6 * implicit.len(),
        "implicit {implicit_attempts} vs direct {}",
        direct.stats.attempts
    );
}

#[test]
fn explicit_batch_loop_allocation_is_bounded() {
    // Every window completes inside its batch budget, so no batch ever shows
    // work pressure: one pure explicit tenure of 64 committed batches.
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 64,
        explicit_attempt_budget: 10_000,
        pressure_steps: usize::MAX,
        carry_explicit_step: true,
        ..Default::default()
    };
    let p = rotation();
    let o = options(1e-7);
    let (warm, _) =
        allocation_scope(|| SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &o, &c));
    let warm = warm.unwrap();
    let (r, allocations) =
        allocation_scope(|| SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0, 0.0], &o, &c));
    let r = r.unwrap();
    assert!(r.success && r.stats.switches == 0);
    assert_eq!(r.stats.attempts, warm.stats.attempts);
    let batches = r.stats.segments.len();
    assert!(batches > 40, "{batches} batches");
    // Each committed batch keeps its public driver record (boxed stats with
    // the fixed tolerance vector), the committed state and its output row:
    // four allocations plus amortized growth (measured 376 for 64 batches).
    // The pre-change loop cloned the statistics twice, the record once and
    // a String decision per batch, at least seven per batch, which this
    // bound refuses.
    let per_batch = 6;
    let setup = 32;
    assert!(
        allocations <= per_batch * batches + setup,
        "{allocations} allocations for {batches} batches"
    );
}

#[test]
fn bounded_pilot_on_a_declared_band_is_discarded_not_refused() {
    // A dense Jacobian with a declared band is verified before any step with
    // two Jacobian evaluations (2n = 16 charged units). The explicit batch of
    // one attempt gives an entry ceiling of floor(0.8 * 7) = 5, so a ceiling
    // that counted the verification would refuse the route and abort the
    // whole solve; the bounded pilot must instead be discarded normally.
    let n = 8;
    let p = Problem::new(n, |_, y, out| {
        for i in 0..y.len() {
            out[i] = -10_000.0 * y[i];
        }
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_, _, j| {
        for i in 0..n {
            j[i * n + i] = -10_000.0;
        }
        Ok(())
    }))
    .banded(0, 0)
    .autonomous();
    // One persistent explicit session: each one-attempt batch shows
    // pressure, so both permitted pilots run on the first two batches.
    let c = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 2,
        explicit_attempt_budget: 1,
        pressure_steps: 1,
        implicit_pilot_attempt_budget: 8,
        implicit_pilot_limit: 2,
        carry_explicit_step: true,
        ..Default::default()
    };
    let r = SwitchingSolver::solve(&p, (0.0, 0.01), &vec![1.0; n], &options(1e-6), &c).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(r.stats.accounting_complete, "{}", r.message);
    assert!(
        !r.message.contains("refused") && !r.message.contains("without a work record"),
        "{}",
        r.message
    );
    let pilots = pilots(&r);
    assert!(!pilots.is_empty(), "{:?}", r.stats.segments);
    for (_, pilot) in &pilots {
        assert!(!pilot.committed);
        assert!(
            pilot.jacobian_builds >= 1,
            "verification and driver builds are charged"
        );
    }
    assert!(
        pilots.iter().any(|(_, s)| s.decision.contains("ceiling")),
        "{:?}",
        pilots.iter().map(|(_, s)| s.decision).collect::<Vec<_>>()
    );
    conserved(&r);
}

#[test]
fn new_policy_controls_are_validated_before_callbacks() {
    let p = Problem::new(1, |_, _, _| panic!("invalid request entered RHS"));
    for c in [
        SwitchingConfig {
            pilot_backoff_factor: 0,
            ..Default::default()
        },
        SwitchingConfig {
            probe_backoff_factor: 0,
            ..Default::default()
        },
        SwitchingConfig {
            rearm_rate_ratio: 1.0,
            ..Default::default()
        },
        SwitchingConfig {
            rearm_rate_ratio: f64::NAN,
            ..Default::default()
        },
    ] {
        assert!(SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &options(1e-3), &c).is_err());
    }
}
