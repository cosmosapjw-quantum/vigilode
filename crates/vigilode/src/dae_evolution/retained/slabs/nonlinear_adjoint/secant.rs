//! Goal residual, not true-flow sensitivity or adaptive discrete gradient.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SecantGoalRoute {
    WholeHorner,
    Sub4Horner,
    Sub4Bernstein,
    Diagonal4,
    Scaled4,
    Scaled16,
    Shifted4,
    Scaled8,
    Shifted8,
    Budgeted,
    BudgetedEconomy,
}
impl SecantGoalRoute {
    pub(in crate::dae_evolution) fn adaptive(self) -> bool {
        matches!(self, Self::Budgeted | Self::BudgetedEconomy)
    }
    fn cells(self) -> usize {
        match self {
            Self::WholeHorner => 1,
            Self::Scaled16 => 16,
            Self::Scaled8 | Self::Shifted8 => 8,
            _ => 4,
        }
    }
    fn bernstein(self) -> bool {
        !matches!(self, Self::WholeHorner | Self::Sub4Horner)
    }
    fn diagonal(self) -> bool {
        matches!(
            self,
            Self::Diagonal4
                | Self::Scaled4
                | Self::Scaled8
                | Self::Scaled16
                | Self::Budgeted
                | Self::BudgetedEconomy
        )
    }
    fn scale(self) -> f64 {
        if matches!(
            self,
            Self::Scaled4 | Self::Scaled8 | Self::Scaled16 | Self::Budgeted | Self::BudgetedEconomy
        ) {
            65536.
        } else {
            1.
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SecantGoalWork {
    pub kernel: de::SecantCellWork,
    pub cells: usize,
    pub slabs: usize,
    pub matrix_actions: usize,
    pub max_order: usize,
    pub scale: f64,
    pub max_cell_tail: f64,
    pub terminal_offset: [f64; 2],
    pub seam_offset: [f64; 2],
    pub initial_offset: [f64; 2],
    pub integrated: [f64; 2],
    pub signed_error: [f64; 2],
    pub width_upper: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_cells: Option<[usize; 3]>,
}
pub struct SecantGoalWorkspace {
    kernel: de::SecantCellWorkspace,
}
impl SecantGoalWorkspace {
    pub fn new() -> Self {
        Self {
            kernel: de::SecantCellWorkspace::new(),
        }
    }
}
impl Default for SecantGoalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
fn power(s: &CompactSlabView<'_>, n: usize, out: &mut [[Iv; 5]]) -> CoreResult<()> {
    let m = match &s.power {
        RetainedPowerRows::Owned(p) => p.len(),
        RetainedPowerRows::Flat(p) => p.len(),
    };
    if m != n || out.len() != n {
        return Err(bad("secant power rows"));
    }
    for i in 0..n {
        let row: &[[f64; 2]] = match &s.power {
            RetainedPowerRows::Owned(p) => &p[i],
            RetainedPowerRows::Flat(p) => &p[i],
        };
        if row.is_empty() || row.len() > 5 {
            return Err(bad("secant power degree"));
        }
        out[i] = [Z; 5];
        for (j, &v) in row.iter().enumerate() {
            out[i][j] = pair(v)?;
        }
    }
    Ok(())
}
impl SlabReplayRun {
    /// Exact initial-state continuous covector of a source-proved monomial
    /// clock system, after the original physical trajectory admission.
    pub fn structured_clock_adjoint_into(
        &self,
        weights: &[f64],
        out: &mut [Iv],
    ) -> CoreResult<de::StructuredClockWork> {
        let e = self.adjoint_admission(weights)?;
        let initial = self.initial();
        let start = self
            .accepted(0)
            .ok_or_else(|| bad("structured initial endpoint"))?;
        let end = self
            .accepted(self.accepted_len().saturating_sub(1))
            .ok_or_else(|| bad("structured terminal endpoint"))?;
        let nx = e.src.nx();
        if !initial.qualified
            || initial.x.len() != nx
            || start.state().len() != weights.len()
            || end.state().len() != weights.len()
            || start.state()[..nx] != initial.x[..]
            || start.t() != initial.t
            || self.driver().t.last() != Some(&end.t())
            || self.accepted_len() != self.slabs_len() + 1
        {
            return Err(bad("structured retained endpoint authority"));
        }
        de::structured_clock_adjoint_into(&e.src, start.t(), end.t(), &initial.x, weights, out)
    }
    pub fn structured_clock_goal_into(
        &self,
        weights: &[f64],
        route: de::TriangularGoalRoute,
        out: &mut Iv,
    ) -> CoreResult<de::TriangularGoalWork> {
        let e = self.adjoint_admission(weights)?;
        let initial = self.initial();
        let start = self
            .accepted(0)
            .ok_or_else(|| bad("structured initial endpoint"))?;
        let end = self
            .accepted(self.accepted_len().saturating_sub(1))
            .ok_or_else(|| bad("structured terminal endpoint"))?;
        let nx = e.src.nx();
        if !initial.qualified
            || initial.x.len() != nx
            || start.state().len() != weights.len()
            || end.state().len() != weights.len()
            || start.state()[..nx] != initial.x[..]
            || start.t() != initial.t
            || self.driver().t.last() != Some(&end.t())
            || self.accepted_len() != self.slabs_len() + 1
        {
            return Err(bad("structured retained endpoint authority"));
        }
        de::structured_clock_goal_into(
            &e.src,
            start.t(),
            end.t(),
            &initial.x,
            end.state(),
            weights,
            route,
            out,
        )
    }

    /// Source-proved closed-form triangular terminal goal on the retained
    /// physical trajectory; the underlying numerical state is unchanged.
    pub fn triangular_goal_into(
        &self,
        weights: &[f64],
        route: de::TriangularGoalRoute,
        out: &mut Iv,
    ) -> CoreResult<de::TriangularGoalWork> {
        let e = self.adjoint_admission(weights)?;
        let initial = self.initial();
        let start = self
            .accepted(0)
            .ok_or_else(|| bad("triangular initial endpoint"))?;
        let end = self
            .accepted(self.accepted_len().saturating_sub(1))
            .ok_or_else(|| bad("triangular terminal endpoint"))?;
        let nx = e.src.nx();
        if !initial.qualified
            || initial.x.len() != nx
            || start.state().len() != weights.len()
            || end.state().len() != weights.len()
            || start.state()[..nx] != initial.x[..]
            || start.t() != initial.t
            || self.driver().t.last() != Some(&end.t())
            || self.accepted_len() != self.slabs_len() + 1
        {
            return Err(bad("triangular retained endpoint authority"));
        }
        de::triangular_clock_goal(
            &e.src,
            start.t(),
            end.t(),
            &initial.x,
            end.state(),
            weights,
            route,
            out,
        )
    }

    /// Exact terminal goal difference for a homogeneous constant reduced
    /// source, using the retained physical endpoint (not its projected gradient).
    /// Initial differential state is exact under adjoint_admission.
    /// No certificate is returned for forcing, nonlinear or unqualified epochs.
    /// Output is unchanged on refusal.
    pub fn homogeneous_goal_into(
        &self,
        weights: &[f64],
        route: AdjointTaylorRoute,
        gradient_budget: f64,
        out: &mut Iv,
    ) -> CoreResult<AdjointTaylorWork> {
        let e = self.adjoint_admission(weights)?;
        let nx = e.src.nx();
        let initial = self.initial();
        let start = self
            .accepted(0)
            .ok_or_else(|| bad("homogeneous initial endpoint"))?;
        let end = self
            .accepted(self.accepted_len().saturating_sub(1))
            .ok_or_else(|| bad("homogeneous terminal endpoint"))?;
        if !initial.qualified
            || initial.x.len() != nx
            || start.state().len() != weights.len()
            || end.state().len() != weights.len()
            || start.state()[..nx] != initial.x[..]
            || start.t() != initial.t
            || self.driver().t.last() != Some(&end.t())
            || self.accepted_len() != self.slabs_len() + 1
        {
            return Err(bad("homogeneous retained endpoint authority"));
        }
        let model = self.inner.run.homogeneous_terminal_adjoint(weights)?;
        let mut gradient = [Z; 32];
        let work = model.enclose_into(route, gradient_budget, &mut gradient[..nx])?;
        let mut goal = Z;
        for (&g, &x) in gradient[..nx].iter().zip(&initial.x) {
            goal = goal.add(g.scale(x)?)?;
        }
        // Preserve the actual physical observation even if its algebraic
        // reconstruction is not exactly on the homogeneous graph.
        for (&c, &y) in weights.iter().zip(end.state()) {
            goal = goal.sub(Iv::point(c)?.scale(y)?)?;
        }
        *out = goal;
        Ok(work)
    }

    /// Actual signed error of a terminal linear observation of [x,z] on the
    /// retained numerical endpoint. Uses source-averaged derivatives on the
    /// full straight segment and includes residual, initial, seam and endpoint
    /// terms. Conditional on the original full qualified physical trajectory.
    /// Output is committed only on success.
    pub fn secant_goal_into(
        &self,
        weights: &[f64],
        route: SecantGoalRoute,
        w: &mut SecantGoalWorkspace,
        out: &mut Iv,
    ) -> CoreResult<SecantGoalWork> {
        self.secant_goal_impl(weights, route, None, w, out)
    }
    /// Per-slab refinement proposal from actual retained normalized defect
    /// and current covector scale. The outward signed transport, not the
    /// heuristic, determines the resulting certificate.
    pub fn secant_goal_budgeted_into(
        &self,
        weights: &[f64],
        route: SecantGoalRoute,
        absolute: f64,
        w: &mut SecantGoalWorkspace,
        out: &mut Iv,
    ) -> CoreResult<SecantGoalWork> {
        self.secant_goal_impl(weights, route, Some(absolute), w, out)
    }
    fn secant_goal_impl(
        &self,
        weights: &[f64],
        route: SecantGoalRoute,
        budget: Option<f64>,
        w: &mut SecantGoalWorkspace,
        out: &mut Iv,
    ) -> CoreResult<SecantGoalWork> {
        if route.adaptive() != budget.is_some() || budget.is_some_and(|b| !b.is_finite() || b <= 0.)
        {
            return Err(bad("secant adaptive budget contract"));
        }
        let e = self.adjoint_admission(weights)?;
        let src = &e.src;
        let nx = src.nx();
        let n = nx + src.nz();
        let dim = nx + 1;
        let m = self.slabs_len();
        if self.accepted_len() != m + 1 || !self.initial().qualified || self.initial().x.len() != nx
        {
            return Err(bad("secant retained endpoint authority"));
        }
        let terminal = self
            .accepted(m)
            .ok_or_else(|| bad("secant endpoint absent"))?;
        if terminal.t() != *self.driver().t.last().unwrap() || terminal.state().len() != n {
            return Err(bad("secant terminal clock/state"));
        }
        let mut work = SecantGoalWork {
            scale: route.scale(),
            adaptive_cells: budget.map(|_| [0; 3]),
            ..Default::default()
        };
        let mut v = [Z; 32];
        let mut a = [Z; 1024];
        let mut r = [Z; 32];
        let mut c = [Z; 32];
        let mut b = [Z; 1024];
        let mut start = [Z; 32];
        let mut end = [Z; 32];
        let mut next_start = [Z; 32];
        let mut terminal_offset = Z;
        let mut seams = Z;
        let mut trace = NonlinearAdjointWork::default();
        let mut rows = [[Z; 5]; 32];
        for k in (0..m).rev() {
            let s = self.slab(k)?;
            let cert = s
                .certificate
                .as_ref()
                .ok_or_else(|| bad("secant certificate"))?;
            power(&s, n, &mut rows[..n])?;
            let mut perturb = [0.; 32];
            if matches!(route, SecantGoalRoute::Shifted4 | SecantGoalRoute::Shifted8) {
                perturb[0] = if k % 2 == 0 {
                    1. / 1048576.
                } else {
                    -1. / 1048576.
                };
                perturb[nx] = 1. / 524288.;
            }
            let mut slab =
                w.kernel
                    .prepare(src, s.times, &rows[..n], &perturb[..n], &mut work.kernel)?;
            slab.point_into(0., &mut start[..n])?;
            slab.point_into(1., &mut end[..n])?;
            if k == m - 1 {
                let theta = Iv::point(1.)?;
                let (_, _, box_y) = physical_cell_box(src, &s, theta, true)?;
                let mut alg = Z;
                slab.enclose(
                    theta,
                    &box_y[..n],
                    cert.preconditioner,
                    weights,
                    route.bernstein(),
                    route.diagonal(),
                    &mut a[..nx * nx],
                    &mut r[..nx],
                    &mut v[..nx],
                    &mut alg,
                    &mut work.kernel,
                )?;
                terminal_offset = (0..n).try_fold(alg, |z, i| {
                    z.add(
                        end[i]
                            .sub(Iv::point(terminal.state()[i])?)?
                            .scale(weights[i])?,
                    )
                })?;
            } else {
                for i in 0..nx {
                    seams = seams.add(v[i].mul(end[i].sub(next_start[i])?)?)?;
                }
            }
            let cells = if let Some(budget) = budget {
                // Proposals only. Use dyadic subdivisions and retain the
                // same verified cell kernel; every chosen partition covers [0,1].
                let covector = v[..nx].iter().map(|x| x.mag()).sum::<f64>();
                let ratio = (*cert.normalized_reduced_defect * covector) / (budget / m as f64);
                let (a, b) = if route == SecantGoalRoute::BudgetedEconomy {
                    (8., 64.)
                } else {
                    (1., 8.)
                };
                let index = if ratio <= a {
                    0
                } else if ratio <= b {
                    1
                } else {
                    2
                };
                work.adaptive_cells.as_mut().unwrap()[index] += 1;
                4usize << index
            } else {
                route.cells()
            };
            for j in (0..cells).rev() {
                let theta = Iv::new(j as f64 / cells as f64, (j + 1) as f64 / cells as f64)?;
                let (_, _, box_y) =
                    physical_cell_box(src, &s, theta, route != SecantGoalRoute::WholeHorner)?;
                let mut ignored = Z;
                slab.enclose(
                    theta,
                    &box_y[..n],
                    cert.preconditioner,
                    weights,
                    route.bernstein(),
                    route.diagonal(),
                    &mut a[..nx * nx],
                    &mut r[..nx],
                    &mut c[..nx],
                    &mut ignored,
                    &mut work.kernel,
                )?;
                // B^T = [[h A^T,0],[r^T*scale,0]]; reverse normalized clock.
                b[..dim * dim].fill(Z);
                for i in 0..nx {
                    b[i * dim..i * dim + nx].copy_from_slice(&a[i * nx..(i + 1) * nx]);
                    b[i * dim + nx] = r[i].scale(work.scale)?;
                }
                advance(
                    &b[..dim * dim],
                    dim,
                    Iv::point(1. / cells as f64)?,
                    &mut v[..dim],
                    false,
                    &mut trace,
                )?;
                work.cells += 1;
            }
            next_start[..n].copy_from_slice(&start[..n]);
            work.slabs += 1;
        }
        let initial = (0..nx).try_fold(Z, |z, i| {
            z.add(v[i].mul(Iv::point(self.initial().x[i])?.sub(next_start[i])?)?)
        })?;
        let integral = v[nx].div(Iv::point(work.scale)?)?;
        let error = terminal_offset.add(seams)?.add(initial)?.add(integral)?;
        work.matrix_actions = trace.matrix_actions;
        work.max_order = trace.max_order;
        work.max_cell_tail = trace.max_cell_tail;
        work.terminal_offset = terminal_offset.as_pair();
        work.seam_offset = seams.as_pair();
        work.initial_offset = initial.as_pair();
        work.integrated = integral.as_pair();
        work.signed_error = error.as_pair();
        work.width_upper = sub_up(error.hi, error.lo)?;
        *out = error;
        Ok(work)
    }
}
