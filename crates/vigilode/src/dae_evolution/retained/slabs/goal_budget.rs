//! Bounded full-epoch goal-gated tolerance replay, not a new local controller.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GoalBudgetPlan {
    Fixed,
    StateReplay,
    SecantReplay,
    Predictive,
    CostAware,
    TieredCost,
    Structured12,
    Structured20,
    StructuredBudgeted,
}
impl GoalBudgetPlan {
    fn structured(self) -> Option<AdjointTaylorRoute> {
        match self {
            Self::Structured12 => Some(AdjointTaylorRoute::Fixed12),
            Self::Structured20 => Some(AdjointTaylorRoute::Fixed20),
            Self::StructuredBudgeted => Some(AdjointTaylorRoute::Budgeted),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GoalBudgetPolicy {
    pub absolute: f64,
    pub plan: GoalBudgetPlan,
}
#[derive(Clone, Debug, Serialize)]
pub struct GoalCertificateAttempt {
    pub route: SecantGoalRoute,
    pub work: Option<SecantGoalWork>,
    pub refusal: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GoalBudgetAttempt {
    pub multiplier: f64,
    pub rtol: f64,
    pub atol: f64,
    pub audit: NativeResourceAudit,
    pub physical_qualified: bool,
    pub state_bound: Option<f64>,
    pub terminal_state: Option<Vec<f64>>,
    pub certificates: Vec<GoalCertificateAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_work: Option<AdjointTaylorWork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_refusal: Option<String>,
    pub enclosure: Option<[f64; 2]>,
    pub accepted: bool,
    pub reason: Option<String>,
}
pub struct GoalBudgetRun {
    pub run: Option<SlabReplayRun>,
    pub attempts: Vec<GoalBudgetAttempt>,
    pub budget: GoalBudgetPolicy,
}
impl GoalBudgetRun {
    /// None denotes an entered driver without a returned stats receipt.
    pub fn total_rhs(&self) -> Option<u64> {
        self.attempts.iter().try_fold(0_u64, |n, a| {
            if a.audit.driver_invoked {
                n.checked_add(a.audit.stats.as_ref()?.counters.rhs_calls)
            } else {
                Some(n)
            }
        })
    }
    pub fn total_steps(&self) -> Option<usize> {
        self.attempts
            .iter()
            .try_fold(0_usize, |n, a| n.checked_add(a.audit.measured_attempts()?))
    }
}
/// Replay from the original initial data. Each trial keeps original primal
/// certificate authority. No convergence or monotonicity assumption about
/// tolerance versus goal error is used for admission. On exhaustion return
/// the entire audit and no accepted run.
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_goal_budget(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    retention: RetainedOutputRoute,
    goal: &[f64],
    budget: GoalBudgetPolicy,
) -> CoreResult<GoalBudgetRun> {
    let Atol::Scalar(atol) = opts.atol else {
        return Err(bad("goal budget requires scalar tolerance"));
    };
    if !budget.absolute.is_finite()
        || budget.absolute <= 0.
        || !opts.rtol.is_finite()
        || opts.rtol <= 0.
        || !atol.is_finite()
        || atol < 0.
        || epoch.incoming_x_error != Some(0.)
        || goal.len() != src.nx() + src.nz()
        || goal.iter().any(|x| !x.is_finite())
        || !opts.dense_output
        || route == EvolutionRoute::Mass
    {
        return Err(bad("goal budget contract"));
    }
    let mut out = GoalBudgetRun {
        run: None,
        attempts: Vec::with_capacity(7),
        budget,
    };
    let norm = goal.iter().try_fold(0., |s, v| add_up(s, v.abs()))?;
    // Dimensionful comparison in goal units; not a certified error model.
    let scale = x.iter().chain(z).map(|v| v.abs()).fold(0., f64::max);
    let nominal = 64. * (atol + opts.rtol * scale) * norm;
    let mut mult = if budget.plan == GoalBudgetPlan::Fixed
        || budget.plan.structured().is_some()
        || (budget.plan == GoalBudgetPlan::CostAware && budget.absolute < nominal)
    {
        1.
    } else {
        64.
    };
    if budget.plan == GoalBudgetPlan::TieredCost {
        mult = if budget.absolute >= nominal {
            64.
        } else if budget.absolute >= nominal / 8. {
            8.
        } else {
            1.
        };
    }
    let mut scratch: Option<Box<SecantGoalWorkspace>> = None;
    loop {
        let mut o = opts.clone();
        o.rtol *= mult;
        o.atol = Atol::Scalar(atol * mult);
        if !o.rtol.is_finite() || !(atol * mult).is_finite() {
            return Err(bad("goal tolerance overflow"));
        }
        let (r, a) = evolve_epoch_slab_replay_planned(
            src,
            x,
            z,
            branch,
            epoch,
            &o,
            policy,
            route,
            grid,
            retention,
            SlabReplayRoute::Flat,
            EnginePlan::SharedFused,
        );
        let mut rec = GoalBudgetAttempt {
            multiplier: mult,
            rtol: o.rtol,
            atol: atol * mult,
            audit: a,
            physical_qualified: false,
            state_bound: None,
            terminal_state: None,
            certificates: vec![],
            structured_work: None,
            structured_refusal: None,
            enclosure: None,
            accepted: false,
            reason: None,
        };
        let mut bound = None;
        let r = match r {
            Ok(r) => Some(r),
            Err(e) => {
                rec.reason = Some(e.to_string());
                None
            }
        };
        if let Some(r) = r.as_ref() {
            let endpoint = r.accepted(r.accepted_len().saturating_sub(1));
            if r.global_qualified()
                && r.driver().is_success()
                && endpoint
                    .as_ref()
                    .is_some_and(|q| q.t() == epoch.end && q.state().len() == goal.len())
            {
                if let Some(q) = endpoint {
                    if let Some(err) = q.physical_error_upper() {
                        rec.terminal_state = Some(q.state().to_vec());
                        let rad = mul_up(err, norm)?;
                        rec.physical_qualified = true;
                        rec.state_bound = Some(rad);
                        bound = Some(Iv::new(-rad, rad)?);
                        if rad > budget.absolute {
                            if let Some(route) = budget.plan.structured() {
                                // Proposal budget only. Admission uses the complete interval
                                // including Taylor truncation, rounding and physical endpoint.
                                let scale = x.iter().try_fold(1., |s, &v| add_up(s, v.abs()))?;
                                let mut iv = Iv::point(0.)?;
                                match r.homogeneous_goal_into(
                                    goal,
                                    route,
                                    budget.absolute / 4. / scale,
                                    &mut iv,
                                ) {
                                    Ok(work) => {
                                        rec.structured_work = Some(work);
                                        let b = bound.unwrap();
                                        match Iv::new(b.lo.max(iv.lo), b.hi.min(iv.hi)) {
                                            Ok(q) => bound = Some(q),
                                            Err(_) => {
                                                bound = None;
                                                rec.reason = Some(
                                                    "disjoint independent goal certificates".into(),
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => rec.structured_refusal = Some(e.to_string()),
                                }
                            }
                        }
                        if bound.is_some_and(|b| b.mag() > budget.absolute)
                            && budget.plan != GoalBudgetPlan::StateReplay
                        {
                            let workspace =
                                scratch.get_or_insert_with(|| Box::new(SecantGoalWorkspace::new()));
                            for route in [
                                SecantGoalRoute::WholeHorner,
                                SecantGoalRoute::Sub4Horner,
                                SecantGoalRoute::Sub4Bernstein,
                                SecantGoalRoute::Scaled16,
                            ] {
                                if matches!(
                                    budget.plan,
                                    GoalBudgetPlan::CostAware | GoalBudgetPlan::TieredCost
                                ) && mult > 1.
                                    && matches!(
                                        route,
                                        SecantGoalRoute::Sub4Horner | SecantGoalRoute::Scaled16
                                    )
                                {
                                    continue;
                                }
                                let mut iv = Iv::point(0.)?;
                                match r.secant_goal_into(goal, route, workspace, &mut iv) {
                                    Ok(work) => {
                                        rec.certificates.push(GoalCertificateAttempt {
                                            route,
                                            work: Some(work),
                                            refusal: None,
                                        });
                                        let b = bound.unwrap();
                                        match Iv::new(b.lo.max(iv.lo), b.hi.min(iv.hi)) {
                                            Ok(q) => bound = Some(q),
                                            Err(_) => {
                                                bound = None;
                                                rec.reason = Some(
                                                    "disjoint independent goal certificates".into(),
                                                );
                                                break;
                                            }
                                        }
                                        let b = bound.unwrap();
                                        if b.mag() <= budget.absolute
                                            || (matches!(
                                                budget.plan,
                                                GoalBudgetPlan::CostAware
                                                    | GoalBudgetPlan::TieredCost
                                            ) && (b.lo > budget.absolute
                                                || b.hi < -budget.absolute))
                                        {
                                            break;
                                        }
                                    }
                                    Err(e) => rec.certificates.push(GoalCertificateAttempt {
                                        route,
                                        work: None,
                                        refusal: Some(e.to_string()),
                                    }),
                                }
                            }
                        }
                    }
                }
            }
            if !rec.physical_qualified {
                rec.reason = Some(
                    r.reason()
                        .unwrap_or("physical or endpoint certificate unresolved")
                        .to_string(),
                );
            }
        }
        rec.enclosure = bound.map(|b| b.as_pair());
        rec.accepted = rec.physical_qualified
            && rec.reason.is_none()
            && bound.is_some_and(|b| b.mag() <= budget.absolute);
        if rec.accepted {
            out.attempts.push(rec);
            out.run = r;
            return Ok(out);
        }
        if rec.reason.is_none() {
            rec.reason = Some("goal budget not certified".into());
        }
        let disjoint = rec.reason.as_deref() == Some("disjoint independent goal certificates");
        out.attempts.push(rec);
        if disjoint || mult <= 1. {
            return Ok(out);
        }
        mult = if budget.plan == GoalBudgetPlan::Predictive {
            let ratio = bound.map_or(f64::INFINITY, |b| b.mag() / budget.absolute);
            // Cost proposal only; clamp to finite descending dyadic multipliers.
            let factor = if ratio > 64. {
                64.
            } else if ratio > 8. {
                8.
            } else {
                2.
            };
            (mult / factor).max(1.)
        } else if matches!(
            budget.plan,
            GoalBudgetPlan::CostAware | GoalBudgetPlan::TieredCost
        ) {
            1.
        } else {
            (mult / 8.).max(1.)
        };
    }
}
