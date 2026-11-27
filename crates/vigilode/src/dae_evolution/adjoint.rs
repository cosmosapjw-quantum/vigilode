//! Continuous terminal-goal sensitivity for source-proved constant affine
//! reduced DAEs. It never replaces state evolution or physical tube proof.
use super::*;
pub use de::ConstantTerminalAdjoint;
use rodas5p_core::directed::{div_up, exp_interval, sub_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GradientRoute {
    RodasAdjoint,
    DopriAdjoint,
    DopriForwardBlock,
    RodasForwardColumns,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AdjointTaylorRoute {
    Fixed12,
    Fixed20,
    Budgeted,
}
#[derive(Clone, Debug, Serialize)]
pub struct GradientRun {
    pub route: GradientRoute,
    pub values: Vec<f64>,
    pub dimension: usize,
    pub solves: usize,
    pub rhs_calls: u64,
    pub scalar_rhs_items: u64,
    pub jacobian_builds: u64,
    pub linear_solves: u64,
    pub steps: usize,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct AdjointTaylorWork {
    pub cells: usize,
    pub matrix_actions: usize,
    pub scalar_products: usize,
    pub maximum_order: usize,
    pub truncation_upper: f64,
    pub width_upper: f64,
}
pub struct TerminalAdjointProblem {
    model: ConstantTerminalAdjoint,
    duration: f64,
}
impl EvolutionRun {
    pub fn terminal_adjoint(&self, weights: &[f64]) -> CoreResult<TerminalAdjointProblem> {
        if !self.global_qualified
            || self.engine.conservation.is_some()
            || self.engine.warp.is_some()
            || self.route == EvolutionRoute::Mass
        {
            return Err(bad(
                "terminal adjoint needs qualified general reduced epoch",
            ));
        }
        let t0 = self.driver.t[0];
        let t1 = *self
            .driver
            .t
            .last()
            .ok_or_else(|| bad("empty adjoint trajectory"))?;
        let h = Iv::point(t1)?.sub(Iv::point(t0)?)?;
        if h.lo != h.hi || h.lo <= 0. {
            return Err(bad("adjoint duration not exactly represented"));
        }
        Ok(TerminalAdjointProblem {
            model: ConstantTerminalAdjoint::from_source(&self.engine.src, weights)?,
            duration: h.lo,
        })
    }
}
impl TerminalAdjointProblem {
    pub fn dimension(&self) -> usize {
        self.model.dimension()
    }
    pub fn duration(&self) -> f64 {
        self.duration
    }
    pub fn matrix(&self) -> &[f64] {
        self.model.matrix()
    }
    pub fn terminal_gradient(&self) -> &[f64] {
        self.model.terminal_gradient()
    }
    /// Run the existing integrators on the continuous variational ODE.
    /// Reverse clock s=T-t, so the adjoint evolves with +A^T, not -A^T.
    /// Solver results are numerical proposals, independently checked below.
    pub fn integrate(&self, route: GradientRoute, opts: &Options) -> CoreResult<GradientRun> {
        let n = self.dimension();
        let c = self.terminal_gradient();
        let reverse = matches!(
            route,
            GradientRoute::RodasAdjoint | GradientRoute::DopriAdjoint
        );
        let block = route == GradientRoute::DopriForwardBlock;
        let dim = if block { n * n } else { n };
        let mat = Arc::new(self.model.clone());
        let m = mat.clone();
        let p = Problem::new(dim, move |_t, y, out| {
            for (v, w) in y.chunks_exact(n).zip(out.chunks_exact_mut(n)) {
                for i in 0..n {
                    w[i] = (0..n).fold(0., |s, j| {
                        let k = if reverse { j * n + i } else { i * n + j };
                        m.matrix()[k].mul_add(v[j], s)
                    });
                }
            }
            Ok(())
        })
        .jacobian(Jacobian::dense(move |_t, _y, out| {
            out.fill(0.);
            for k in 0..(dim / n) {
                for i in 0..n {
                    for j in 0..n {
                        out[(k * n + i) * dim + k * n + j] =
                            mat.matrix()[if reverse { j * n + i } else { i * n + j }];
                    }
                }
            }
            Ok(())
        }))
        .autonomous();
        let mut o = opts.clone();
        o.dense_output = false;
        let mut result = GradientRun {
            route,
            values: vec![0.; n],
            dimension: dim,
            solves: 0,
            rhs_calls: 0,
            scalar_rhs_items: 0,
            jacobian_builds: 0,
            linear_solves: 0,
            steps: 0,
        };
        let count = if route == GradientRoute::RodasForwardColumns {
            n
        } else {
            1
        };
        for k in 0..count {
            let mut y = vec![0.; dim];
            if reverse {
                y.copy_from_slice(c);
            } else if block {
                for i in 0..n {
                    y[i * n + i] = 1.;
                }
            } else {
                y[k] = 1.;
            }
            let values = if matches!(
                route,
                GradientRoute::DopriAdjoint | GradientRoute::DopriForwardBlock
            ) {
                let s = crate::Dopri5::solve(&p, (0., self.duration), &y, &o)
                    .map_err(bad)?
                    .ensure_success()
                    .map_err(bad)?;
                result.rhs_calls += s.stats.counters.rhs_calls;
                result.jacobian_builds += s.stats.counters.jacobian_builds;
                result.linear_solves += s.stats.counters.linear_solves;
                result.steps += s.stats.accepted_steps;
                s.final_state
            } else {
                let s = Rodas5p::solve(&p, (0., self.duration), &y, &o)
                    .map_err(bad)?
                    .ensure_success()
                    .map_err(bad)?;
                result.rhs_calls += s.stats.counters.rhs_calls;
                result.jacobian_builds += s.stats.counters.jacobian_builds;
                result.linear_solves += s.stats.counters.linear_solves;
                result.steps += s.stats.accepted_steps;
                s.final_state()
                    .ok_or_else(|| bad("empty gradient trajectory"))?
                    .to_vec()
            };
            result.solves += 1;
            if reverse {
                result.values.copy_from_slice(&values);
            } else if block {
                for (o, v) in result.values.iter_mut().zip(values.chunks_exact(n)) {
                    *o = c.iter().zip(v).fold(0., |s, (&a, &b)| a.mul_add(b, s));
                }
            } else {
                result.values[k] = c.iter().zip(values).fold(0., |s, (&a, b)| a.mul_add(b, s));
            }
        }
        result.scalar_rhs_items = result.rhs_calls * (dim as u64);
        Ok(result)
    }
    /// Independent directed Taylor matrix-action enclosure of the exact
    /// continuous gradient. Budgeted order uses the existing causal
    /// residual adjoint weights on the accepted subcell chain. The budget
    /// controls Taylor truncation; interval operations also enclose rounding.
    /// Output is committed only after all directed operations succeed.
    pub fn enclose_into(
        &self,
        route: AdjointTaylorRoute,
        budget: f64,
        out: &mut [Iv],
    ) -> CoreResult<AdjointTaylorWork> {
        let n = self.dimension();
        let a = self.matrix();
        if out.len() != n || !budget.is_finite() || budget <= 0. {
            return Err(bad("gradient enclosure shape/budget"));
        }
        let mut norm = 0_f64;
        for i in 0..n {
            let mut s = 0.;
            for j in 0..n {
                s = add_up(s, a[j * n + i].abs())?;
            }
            norm = norm.max(s);
        }
        let mut cells = 1usize;
        let mut m = mul_up(norm, self.duration)?;
        while m > 0.5 {
            cells *= 2;
            if cells > 64 {
                return Err(bad("adjoint subcell cap"));
            }
            m = div_up(m, 2.)?;
        }
        let h = Iv::point(self.duration)?.div(Iv::point(cells as f64)?)?;
        let gain = exp_interval(m)?.hi;
        let mut allowance = [0.; 64];
        if route == AdjointTaylorRoute::Budgeted {
            // e_i <= d_i + gain e_{i-1}. Existing transpose-causal
            // recurrence allocates local tail tolerances to terminal error.
            let mut propagation = [0.; 4096];
            let mut weights = [0.; 64];
            let mut influences = [0.; 64];
            for i in 1..cells {
                propagation[i * cells + i - 1] = gain;
            }
            weights[cells - 1] = 1.;
            crate::causal_budget::causal_residual_allowances(
                &propagation[..cells * cells],
                &weights[..cells],
                budget,
                &mut influences[..cells],
                &mut allowance[..cells],
            )
            .map_err(bad)?;
        }
        let z = Iv { lo: 0., hi: 0. };
        let mut b = [z; 1024];
        for i in 0..n {
            for j in 0..n {
                b[i * n + j] = h.scale(a[j * n + i])?;
            }
        }
        let mut v = [z; 32];
        for (o, &c) in v[..n].iter_mut().zip(self.terminal_gradient()) {
            *o = Iv::point(c)?;
        }
        let mut work = AdjointTaylorWork {
            cells,
            matrix_actions: 0,
            scalar_products: 0,
            maximum_order: 0,
            truncation_upper: 0.,
            width_upper: 0.,
        };
        for i in 0..cells {
            let size = v[..n].iter().map(|x| x.mag()).fold(0., f64::max);
            let tail = |order: usize| -> CoreResult<f64> {
                let mut t = mul_up(size, gain)?;
                for k in 1..=order + 1 {
                    t = div_up(mul_up(t, m)?, k as f64)?;
                }
                Ok(t)
            };
            let order = match route {
                AdjointTaylorRoute::Fixed12 => 12,
                AdjointTaylorRoute::Fixed20 => 20,
                AdjointTaylorRoute::Budgeted => {
                    let mut found = None;
                    for k in 2..=24 {
                        if tail(k)? <= allowance[i] {
                            found = Some(k);
                            break;
                        }
                    }
                    found.ok_or_else(|| bad("adjoint Taylor budget cap"))?
                }
            };
            let radius = tail(order)?;
            work.maximum_order = work.maximum_order.max(order);
            let mut term = v;
            let mut sum = v;
            let mut next = [z; 32];
            for k in 1..=order {
                for j in 0..n {
                    let mut s = z;
                    for l in 0..n {
                        let a = b[j * n + l];
                        if a.lo != 0. || a.hi != 0. {
                            s = s.add(a.mul(term[l])?)?;
                            work.scalar_products += 1;
                        }
                    }
                    next[j] = s.div(Iv::point(k as f64)?)?;
                    sum[j] = sum[j].add(next[j])?;
                }
                term[..n].copy_from_slice(&next[..n]);
                work.matrix_actions += 1;
            }
            let err = Iv::new(-radius, radius)?;
            for j in 0..n {
                v[j] = sum[j].add(err)?;
            }
            work.truncation_upper = add_up(mul_up(gain, work.truncation_upper)?, radius)?;
        }
        for v in &v[..n] {
            work.width_upper = work.width_upper.max(sub_up(v.hi, v.lo)?);
        }
        out.copy_from_slice(&v[..n]);
        Ok(work)
    }
    pub fn numerical_error_upper(&self, values: &[f64], enclosure: &[Iv]) -> CoreResult<f64> {
        if values.len() != self.dimension() || enclosure.len() != self.dimension() {
            return Err(bad("gradient comparison shape"));
        }
        values
            .iter()
            .zip(enclosure)
            .try_fold(0_f64, |m, (&v, &a)| Ok(m.max(a.sub(Iv::point(v)?)?.mag())))
    }
}
