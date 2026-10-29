//! Event location: roots of user functions `g(t, y)` along the solution,
//! found by Illinois (modified regula falsi) on the continuous extension of
//! the accepted steps.
//!
//! This is post-processing. The integration runs unchanged, and the events
//! are located over the finished record of accepted steps, so it needs
//! [`crate::Options::dense_output`]. A terminal event therefore truncates the
//! outputs after the run: the work spent after the event stays charged in
//! [`crate::Stats::counters`]. Stopping the integration inside the step loop
//! is not available in this release.

use std::sync::Arc;

use serde::Serialize;

use crate::solution::{FailureReason, Solution, Status};

/// The sign change of an event function that counts as an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Direction {
    /// `g` goes from negative to nonnegative.
    Up,
    /// `g` goes from positive to nonpositive.
    Down,
    /// Either of the two. As the `direction` of an [`EventRecord`] it never
    /// occurs: the record holds the observed `Up` or `Down`.
    Either,
}

/// One located event.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct EventRecord {
    /// The event's declaration order among the events of the problem.
    pub index: usize,
    /// The event time: the end of the final bracket on the side after the
    /// crossing.
    pub t: f64,
    /// The state at `t` (the continuous extension).
    pub y: Vec<f64>,
    /// The observed direction, `Up` or `Down`.
    pub direction: Direction,
}

/// The event function `g(t, y)`.
pub(crate) type EventFn = Arc<dyn Fn(f64, &[f64]) -> f64 + Send + Sync>;

/// A declared event.
#[derive(Clone)]
pub(crate) struct EventSpec {
    pub(crate) g: EventFn,
    pub(crate) direction: Direction,
    pub(crate) terminal: bool,
}

/// Illinois iterations at most; after them bisection finishes the bracket.
const MAX_ITERATIONS: usize = 60;

/// A located crossing before it becomes an [`EventRecord`].
struct Crossing {
    index: usize,
    t: f64,
    up: bool,
}

/// Distinguish a user event failure from failure of its state provider.
enum EventFailure {
    NonFinite,
    Interpolation { t: f64 },
}

impl EventFailure {
    fn reason(self) -> FailureReason {
        match self {
            Self::NonFinite => FailureReason::EventFunctionNonFinite,
            Self::Interpolation { t } => FailureReason::EventInterpolationFailed { t },
        }
    }
}

struct Evaluator<'a> {
    solution: &'a Solution,
    spec: &'a EventSpec,
    evaluations: u64,
}

impl Evaluator<'_> {
    fn at_state(&mut self, t: f64, y: &[f64]) -> Result<f64, EventFailure> {
        self.evaluations += 1;
        let g = (self.spec.g)(t, y);
        if g.is_finite() {
            Ok(g)
        } else {
            Err(EventFailure::NonFinite)
        }
    }

    fn at_time(&mut self, t: f64) -> Result<f64, EventFailure> {
        let y = self
            .solution
            .at(t)
            .ok_or(EventFailure::Interpolation { t })?;
        self.at_state(t, &y)
    }
}

/// Locate the root of `sign * g` in `[a, b]` where `sign * g(a) < 0 <= sign * g(b)`.
/// Returns the end of the final bracket on the `b` side.
fn illinois(
    ev: &mut Evaluator<'_>,
    sign: f64,
    (mut a, mut ha): (f64, f64),
    (mut b, mut hb): (f64, f64),
) -> Result<f64, EventFailure> {
    if hb == 0.0 {
        return Ok(b);
    }
    // -1: the last update moved `a`; +1: it moved `b`.
    let mut last_side = 0i8;
    for _ in 0..MAX_ITERATIONS {
        if b - a <= 4.0 * f64::EPSILON * b.abs().max(a.abs()).max(1.0) {
            return Ok(b);
        }
        let mut c = (a * hb - b * ha) / (hb - ha);
        if !(c > a && c < b) {
            c = a + 0.5 * (b - a);
            if !(c > a && c < b) {
                return Ok(b);
            }
        }
        let hc = sign * ev.at_time(c)?;
        if hc >= 0.0 {
            b = c;
            hb = hc;
            if last_side == 1 {
                ha *= 0.5;
            }
            last_side = 1;
            if hc == 0.0 {
                return Ok(b);
            }
        } else {
            a = c;
            ha = hc;
            if last_side == -1 {
                hb *= 0.5;
            }
            last_side = -1;
        }
    }
    // The Illinois budget is spent: finish by bisection, so that the reported
    // end always meets the tolerance (a bracket is at most 1075 halvings from
    // adjacent floats, and the loop also stops when the midpoint is no longer
    // strictly inside).
    while b - a > 4.0 * f64::EPSILON * b.abs().max(a.abs()).max(1.0) {
        let c = a + 0.5 * (b - a);
        if !(c > a && c < b) {
            break;
        }
        if sign * ev.at_time(c)? >= 0.0 {
            b = c;
        } else {
            a = c;
        }
    }
    Ok(b)
}

/// Crossings of one event on one accepted step `[ta, tb]`, given `g(ta)`.
/// Pushes them in time order and returns `g(tb)`.
fn scan_step(
    ev: &mut Evaluator<'_>,
    (ta, ga): (f64, f64),
    tb: f64,
    yb: &[f64],
    out: &mut Vec<Crossing>,
    index: usize,
) -> Result<f64, EventFailure> {
    let gb = ev.at_state(tb, yb)?;
    let tm = ta + 0.5 * (tb - ta);
    // At most two halves: stack storage removes one heap allocation per
    // event per accepted step without changing sample or root order.
    let mut halves = [(ta, ga, tb, gb); 2];
    let count = if tm > ta && tm < tb {
        let gm = ev.at_time(tm)?;
        halves[0] = (ta, ga, tm, gm);
        halves[1] = (tm, gm, tb, gb);
        2
    } else {
        1
    };
    let want = ev.spec.direction;
    for &(a, ga, b, gb) in &halves[..count] {
        let up = ga < 0.0 && gb >= 0.0;
        let down = ga > 0.0 && gb <= 0.0;
        let (sign, is_up) = match (up, down, want) {
            (true, _, Direction::Up | Direction::Either) => (1.0, true),
            (_, true, Direction::Down | Direction::Either) => (-1.0, false),
            _ => continue,
        };
        let t = illinois(ev, sign, (a, sign * ga), (b, sign * gb))?;
        out.push(Crossing {
            index,
            t,
            up: is_up,
        });
    }
    Ok(gb)
}

/// Locate every event over the accepted steps of `solution`, record them in
/// `solution.events`, and apply a terminal event or a non-finite failure.
/// Returns the number of event-function evaluations and the index (into the
/// recorded events) of the terminal event.
pub(crate) fn locate(solution: &mut Solution, specs: &[EventSpec]) -> (u64, Option<usize>) {
    if specs.is_empty() {
        return (0, None);
    }
    if solution.extension.is_none() || solution.t.is_empty() {
        let t = solution
            .t
            .first()
            .copied()
            .unwrap_or(solution.stats.t_span.0);
        fail_event(solution, 0, EventFailure::Interpolation { t });
        return (0, None);
    }
    let mut evaluations = 0u64;
    let steps = solution.t.len() - 1;
    let mut found: Vec<(Crossing, Vec<f64>)> = Vec::new();
    let mut previous = Vec::with_capacity(specs.len());
    for spec in specs {
        let mut ev = Evaluator {
            solution,
            spec,
            evaluations: 0,
        };
        let g0 = ev.at_state(solution.t[0], &solution.y[0]);
        evaluations += ev.evaluations;
        match g0 {
            Ok(g) => previous.push(g),
            Err(error) => {
                fail_event(solution, 0, error);
                return (evaluations, None);
            }
        }
    }

    let mut terminal: Option<usize> = None;
    let mut failed = false;
    let mut crossings: Vec<Crossing> = Vec::new();
    let mut next = Vec::with_capacity(specs.len());
    'steps: for k in 0..steps {
        let (ta, tb) = (solution.t[k], solution.t[k + 1]);
        crossings.clear();
        next.clear();
        for (index, spec) in specs.iter().enumerate() {
            let mut ev = Evaluator {
                solution,
                spec,
                evaluations: 0,
            };
            let result = scan_step(
                &mut ev,
                (ta, previous[index]),
                tb,
                &solution.y[k + 1],
                &mut crossings,
                index,
            );
            evaluations += ev.evaluations;
            match result {
                Ok(g) => next.push(g),
                Err(error) => {
                    // Events of earlier steps stay; this step's are dropped.
                    fail_event(solution, k, error);
                    failed = true;
                    break 'steps;
                }
            }
        }
        std::mem::swap(&mut previous, &mut next);
        crossings.sort_by(|p, q| p.t.total_cmp(&q.t).then(p.index.cmp(&q.index)));
        let cut = crossings
            .iter()
            .find(|c| specs[c.index].terminal)
            .map(|c| c.t);
        let step_begin = found.len();
        for c in crossings.drain(..) {
            if cut.is_some_and(|tc| c.t > tc) {
                continue;
            }
            let Some(y) = solution.at(c.t) else {
                // Never attach an endpoint state to a different event time.
                // Roll back the whole step, just like a failed event call.
                found.truncate(step_begin);
                terminal = None;
                fail_event(solution, k, EventFailure::Interpolation { t: c.t });
                failed = true;
                break 'steps;
            };
            let is_terminal = terminal.is_none() && specs[c.index].terminal && Some(c.t) == cut;
            found.push((c, y));
            if is_terminal {
                terminal = Some(found.len() - 1);
            }
        }
        if cut.is_some() {
            break;
        }
    }

    if !failed && let Some(position) = terminal {
        let (te, ye) = (found[position].0.t, found[position].1.clone());
        solution.truncate_after(te);
        if solution.t.last().is_none_or(|&last| last < te) {
            solution.t.push(te);
            solution.y.push(ye);
        }
        solution.status = Status::Success;
    }
    solution.events = found
        .into_iter()
        .map(|(c, y)| EventRecord {
            index: c.index,
            t: c.t,
            y,
            direction: if c.up { Direction::Up } else { Direction::Down },
        })
        .collect();
    (evaluations, terminal)
}

/// A failed event evaluation on step `k`: truncate at its start, keeping
/// any existing driver failure reason. Missing interpolation is a solver
/// state failure, not a user event returning NaN/Inf.
fn fail_event(solution: &mut Solution, k: usize, error: EventFailure) {
    if let Some(&start) = solution.t.get(k) {
        solution.truncate_after(start);
    }
    if !matches!(solution.status, Status::Failed(_)) {
        solution.status = Status::Failed(error.reason());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallbackError, Options, Problem, Rodas5p};

    fn solved() -> Solution {
        let p = Problem::new(1, |_t, y, dy| {
            dy[0] = -y[0];
            Ok::<(), CallbackError>(())
        })
        .autonomous();
        Rodas5p::solve(
            &p,
            (0.0, 1.0),
            &[1.0],
            &Options {
                dense_output: true,
                ..Options::default()
            },
        )
        .unwrap()
    }

    fn event() -> EventSpec {
        EventSpec {
            g: Arc::new(|t, _y| t - 0.5),
            direction: Direction::Up,
            terminal: false,
        }
    }

    #[test]
    fn missing_extension_is_an_explicit_failure_without_calling_event() {
        let mut solution = solved();
        solution.extension = None;
        let (calls, terminal) = locate(&mut solution, &[event()]);
        assert_eq!((calls, terminal), (0, None));
        assert_eq!(
            solution.status,
            Status::Failed(FailureReason::EventInterpolationFailed { t: 0.0 })
        );
        assert_eq!(solution.t, vec![0.0]);
        assert!(solution.events.is_empty());
    }

    #[test]
    fn missing_interior_does_not_implicate_the_event_callback() {
        let mut solution = solved();
        let expected_time = 0.5 * solution.t[1];
        solution.extension = Some(crate::solution::Extension::Sequential(Vec::new()));
        let (calls, terminal) = locate(&mut solution, &[event()]);
        // The finite endpoint callbacks ran, but no midpoint callback did.
        assert_eq!((calls, terminal), (2, None));
        assert_eq!(
            solution.status,
            Status::Failed(FailureReason::EventInterpolationFailed { t: expected_time })
        );
        assert_eq!(solution.t, vec![0.0]);
        assert!(solution.events.is_empty());
    }
}
