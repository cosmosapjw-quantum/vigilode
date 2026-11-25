//! Single source-declared polynomial event on the fast dense transaction.
//! Root brackets certify the exact represented polynomial only; the returned
//! state uses the driver's rounded continuous extension at the bracket end.
#![allow(missing_docs)] // Research receipt field names are explicit; module contract above.
use crate::{Direction, EventRecord, FailureReason, Solution, SolveError, Status};
pub use rodas5p_integrators::retro_event_poly::{
    AffineFactor, DeclaredPolynomial, EventTerm, RepresentedPolynomial, RootInterval, RootKind,
    RootReport, RootRoute, RootWork,
};
use rodas5p_integrators::{
    AcceptControl, FastDenseSegments,
    retro_portfolio::{Attempt, Portfolio},
};
use serde::Serialize;

/// A source declaration and event policy (Either includes touches here).
#[derive(Clone, Debug, Serialize)]
pub struct PolynomialEventSpec {
    pub polynomial: DeclaredPolynomial,
    pub direction: Direction,
    pub terminal: bool,
}
/// Explicit research policy. None uses existing RETRO adaptive selection.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PolynomialEventPolicy {
    pub route: Option<RootRoute>,
    pub theta_tolerance: f64,
    pub max_step: Option<f64>,
    pub capture_all: bool,
}
impl Default for PolynomialEventPolicy {
    fn default() -> Self {
        Self {
            route: None,
            theta_tolerance: 2f64.powi(-40),
            max_step: None,
            capture_all: false,
        }
    }
}
/// Complete exact-root report for a numerical proposal, including roots
/// beyond a terminal cut. These are diagnostics, not committed events.
#[derive(Debug, Serialize)]
pub struct PolynomialSlab {
    pub index: usize,
    pub times: [f64; 2],
    pub coefficients: Option<Vec<String>>,
    pub clock: [String; 2],
    pub report: RootReport,
    pub attempts: Vec<Attempt>,
}
/// Distinct trace; event_evaluations remains zero (no user event callbacks).
#[derive(Debug, Serialize)]
pub struct PolynomialEventTrace {
    pub policy: PolynomialEventPolicy,
    pub numerical_end: f64,
    pub committed_end: f64,
    pub inspected_steps: usize,
    pub slabs: Vec<PolynomialSlab>,
    pub failure: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct PolynomialEventRun {
    pub solution: Solution,
    pub trace: PolynomialEventTrace,
}

pub(crate) struct PolyTransaction<'a> {
    spec: &'a PolynomialEventSpec,
    pub(crate) policy: PolynomialEventPolicy,
    portfolio: Portfolio,
    pub(crate) trace: PolynomialEventTrace,
    events: Vec<EventRecord>,
    terminal: Option<usize>,
    failed: Option<(f64, String)>,
}
impl<'a> PolyTransaction<'a> {
    pub(crate) fn new(
        spec: &'a PolynomialEventSpec,
        policy: PolynomialEventPolicy,
        n: usize,
    ) -> Result<Self, SolveError> {
        spec.polynomial
            .validate(n)
            .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        if !policy.theta_tolerance.is_finite()
            || !(2f64.powi(-50)..=1e-3).contains(&policy.theta_tolerance)
            || policy.max_step.is_some_and(|h| !h.is_finite() || h < 1e-14)
        {
            return Err(SolveError::InvalidInput(
                "invalid polynomial event policy".into(),
            ));
        }
        let portfolio =
            Portfolio::new(4, 5).map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        Ok(Self {
            spec,
            policy,
            portfolio,
            trace: PolynomialEventTrace {
                policy,
                numerical_end: 0.,
                committed_end: 0.,
                inspected_steps: 0,
                slabs: vec![],
                failure: None,
            },
            events: vec![],
            terminal: None,
            failed: None,
        })
    }
    pub(crate) fn inspect(&mut self, s: &FastDenseSegments) -> AcceptControl {
        let k = s.len() - 1;
        let a = s.t_old(k);
        let b = s.t_new(k);
        self.trace.inspected_steps += 1;
        match self.scan(s, k) {
            Ok(stop) => {
                if stop {
                    AcceptControl::Stop
                } else {
                    AcceptControl::Continue
                }
            }
            Err(e) => {
                self.failed = Some((a, e.clone()));
                self.trace.failure = Some(e);
                self.trace.numerical_end = b;
                AcceptControl::Stop
            }
        }
    }
    fn scan(&mut self, s: &FastDenseSegments, k: usize) -> Result<bool, String> {
        let poly = RepresentedPolynomial::from_segment(s, k, &self.spec.polynomial)
            .map_err(|e| e.to_string())?;
        let (report, attempts) = if let Some(route) = self.policy.route {
            (
                poly.isolate(route, self.policy.theta_tolerance)
                    .map_err(|e| e.to_string())?,
                vec![],
            )
        } else {
            let r = poly
                .adaptive_isolate(&mut self.portfolio, self.policy.theta_tolerance)
                .map_err(|e| e.to_string())?;
            let Some(v) = r.value else {
                return Err(format!("all root routes refused: {:?}", r.attempts));
            };
            (v, r.attempts)
        };
        let times = [s.t_old(k), s.t_new(k)];
        let mut events = Vec::new();
        let mut terminal = false;
        // Conservative clock admission: distinct root enclosures must be
        // separated in represented physical time, even for nonselected roots.
        if report
            .roots
            .windows(2)
            .any(|w| w[0].time[1] >= w[1].time[0])
        {
            return Err("distinct roots not separated in represented physical clock".into());
        }
        for r in &report.roots {
            let direction = match r.kind {
                RootKind::Up => Direction::Up,
                RootKind::Down => Direction::Down,
                RootKind::Touch => Direction::Touch,
            };
            if self.spec.direction != Direction::Either && self.spec.direction != direction {
                continue;
            }
            let t = r.time[1];
            if !(t > times[0] && t <= times[1]) {
                return Err("root time not representable in proposal".into());
            }
            let mut y = vec![0.; s.dimension()];
            s.evaluate_into(t, &mut y).map_err(|e| e.to_string())?;
            events.push(EventRecord {
                index: 0,
                t,
                y,
                direction,
            });
            if self.spec.terminal {
                terminal = true;
                break;
            }
        }
        // Do not change output until the whole current root report passed.
        let capture = self.policy.capture_all || !report.roots.is_empty();
        self.trace.slabs.push(PolynomialSlab {
            index: k,
            times,
            coefficients: capture.then(|| poly.exact_coefficients()),
            clock: poly.exact_clock(),
            report,
            attempts,
        });
        let start = self.events.len();
        self.events.append(&mut events);
        if terminal {
            self.terminal = Some(start);
        }
        Ok(terminal)
    }
    pub(crate) fn finish(&mut self, sol: &mut Solution) {
        let end = *sol.t.last().expect("initial state");
        self.trace.numerical_end = end;
        if let Some((t, message)) = self.failed.take() {
            sol.truncate_after(t);
            sol.status = Status::Failed(FailureReason::PolynomialEventUnresolved { t, message });
        } else if let Some(i) = self.terminal {
            let e = &self.events[i];
            sol.truncate_after(e.t);
            if sol.t.last().is_none_or(|&t| t < e.t) {
                sol.t.push(e.t);
                sol.y.push(e.y.clone());
            }
            sol.status = Status::Success;
            sol.stats.terminal_event = Some(i);
        }
        sol.events = std::mem::take(&mut self.events);
        self.trace.committed_end = *sol.t.last().expect("initial state");
        sol.stats.schema = "vigilode.stats.polynomial-event.v1";
    }
}

/// Source-owned physical root proof, not a termination API.
pub use rodas5p_integrators::retro_physical_event;

/// Deterministic scalar hybrid reset and physical synchronization research.
pub use rodas5p_integrators::retro_hybrid_reset;

/// Source-owned consistent DAE initialization.
pub use rodas5p_integrators::retro_dae_init;

/// An initialized mass-matrix client using the same immutable declared source.
/// The initial report certifies only the fixed initial point and local branch,
/// not the later constrained evolution. No mass solver policy is changed.
#[allow(missing_docs)]
pub struct InitializedDae {
    pub problem: crate::Problem,
    pub initial_state: Vec<f64>,
    pub report: retro_dae_init::InitReport,
}

/// Prepare an existing mass-matrix driver client from a certified initial
/// point. No callbacks can be supplied separately from the declared source.
pub fn initialize_mass_problem(
    source: &retro_dae_init::DaeSource,
    t: f64,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    policy: retro_dae_init::InitPolicy,
    route: retro_dae_init::InitRoute,
) -> Result<InitializedDae, crate::SolveError> {
    let report = source
        .initialize(t, x, z, branch, policy, route)
        .map_err(|e| crate::SolveError::InvalidInput(e.to_string()))?;
    let nx = source.nx();
    let n = nx + source.nz();
    let mut m = vec![0.; n * n];
    for i in 0..nx {
        m[i * n + i] = 1.;
    }
    let a = source.clone();
    let b = source.clone();
    let c = source.clone();
    let problem = crate::Problem::new(n, move |t, y, out| {
        let r = a
            .rhs_values(t, y)
            .map_err(|e| crate::CallbackError::Fatal(e.to_string()))?;
        out.copy_from_slice(&r);
        Ok(())
    })
    .jacobian(crate::Jacobian::dense(move |t, y, out| {
        let r = b
            .jacobian_values(t, y)
            .map_err(|e| crate::CallbackError::Fatal(e.to_string()))?;
        out.copy_from_slice(&r);
        Ok(())
    }))
    .time_derivative(move |t, y, out| {
        let r = c
            .time_values(t, y)
            .map_err(|e| crate::CallbackError::Fatal(e.to_string()))?;
        out.copy_from_slice(&r);
        Ok(())
    })
    .mass(m);
    let mut initial_state = report.x.clone();
    initial_state.extend_from_slice(&report.z);
    Ok(InitializedDae {
        problem,
        initial_state,
        report,
    })
}
