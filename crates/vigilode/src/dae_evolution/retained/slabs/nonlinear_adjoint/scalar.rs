use super::*;

fn add_work(w: &mut ScalarAdjointWork, j: de::ScalarJetWork) {
    w.source_rows += j.source_rows;
    w.root_passes += j.root_passes;
    w.divisions += j.divisions;
    w.quadratic_seed_uses += j.quadratic_seed_uses;
}
fn jet(
    src: &DaeSource,
    s: &CompactSlabView<'_>,
    theta: Iv,
    goal: &[f64],
    passes: usize,
    seed: bool,
    w: &mut ScalarAdjointWork,
) -> CoreResult<(Iv, de::ScalarAdjointJet)> {
    let (h, time, y) = physical_cell_box(src, s, theta, true)?;
    let mut work = de::ScalarJetWork::default();
    let j = de::scalar_adjoint_jet(src, time, [y[0], y[1]], goal, passes, seed, &mut work)?;
    add_work(w, work);
    Ok((h, j))
}
fn adaptive(
    src: &DaeSource,
    s: &CompactSlabView<'_>,
    goal: &[f64],
    l: f64,
    r: f64,
    depth: usize,
    duration: Iv,
    w: &mut ScalarAdjointWork,
) -> CoreResult<Iv> {
    let (h, j) = jet(src, s, Iv::new(l, r)?, goal, 2, false, w)?;
    let h = h.scale(r - l)?;
    let value = h.mul(j.coefficient)?;
    w.split_evaluations += 1;
    let threshold = rodas5p_core::directed::div_down(
        rodas5p_core::directed::mul_down(1e-4, h.lo)?,
        duration.hi,
    )?;
    if sub_up(value.hi, value.lo)? <= threshold || depth == 6 {
        w.cells += 1;
        if depth == 6 && sub_up(value.hi, value.lo)? > threshold {
            w.cap_leaves += 1;
        }
        return Ok(value);
    }
    let m = (l + r) * 0.5;
    let a = adaptive(src, s, goal, l, m, depth + 1, duration, w)?;
    let b = adaptive(src, s, goal, m, r, depth + 1, duration, w)?;
    intersection(value, a.add(b)?)
}
fn copy_old(r: NonlinearAdjointWork) -> ScalarAdjointWork {
    ScalarAdjointWork {
        source_rows: r.schur.source_rows,
        cells: r.cells,
        exponentials: r.scalar_exponentials,
        root_passes: r.schur.root_passes,
        column_solves: r.schur.column_solves,
        refinement_sweeps: r.schur.refinement_sweeps,
        width_upper: r.width_upper,
        ..Default::default()
    }
}
impl SlabReplayRun {
    /// Scalar continuous sensitivity, actual original-source primal authority.
    /// Integral paths aggregate an outward bound before one interval exp.
    /// FlowRatio is the autonomous scalar tangent identity, not a fit.
    pub fn scalar_adjoint_into(
        &self,
        weights: &[f64],
        route: ScalarAdjointRoute,
        w: &mut NonlinearAdjointWorkspace,
        out: &mut [Iv],
    ) -> CoreResult<ScalarAdjointWork> {
        let e = self.adjoint_admission(weights)?;
        let src = &e.src;
        if src.nx() != 1 || src.nz() != 1 || out.len() != 1 {
            return Err(bad("scalar adjoint requires nx=nz=1"));
        }
        if route == ScalarAdjointRoute::Repeated16 {
            return self
                .nonlinear_adjoint_into(weights, NonlinearAdjointRoute::ScalarExp16, w, out)
                .map(copy_old);
        }
        let mut work = ScalarAdjointWork::default();
        let m = self.slabs_len();
        let last = self.slab(m - 1)?;
        let duration = Iv::point(self.driver().t[m])?.sub(Iv::point(self.driver().t[0])?)?;
        let mut value = Z;
        let result = match route {
            ScalarAdjointRoute::IntegralGuard16 => {
                let mut old = NonlinearAdjointWork::default();
                let mut a = [Z];
                let mut c = [Z];
                bound_cell(
                    src,
                    &last,
                    Iv::point(1.)?,
                    true,
                    2,
                    weights,
                    w,
                    &mut a,
                    &mut c,
                    &mut old,
                )?;
                let goal = c[0];
                for k in (0..m).rev() {
                    let s = self.slab(k)?;
                    for j in (0..16).rev() {
                        let h = bound_cell(
                            src,
                            &s,
                            Iv::new(j as f64 / 16., (j + 1) as f64 / 16.)?,
                            true,
                            2,
                            weights,
                            w,
                            &mut a,
                            &mut c,
                            &mut old,
                        )?;
                        value = value.add(h.scale(1. / 16.)?.mul(a[0])?)?;
                        old.cells += 1;
                    }
                }
                work = copy_old(old);
                work.exponentials = 1;
                goal.mul(interval_exp(value)?)?
            }
            ScalarAdjointRoute::FlowRatio => {
                if !de::scalar_autonomous_source(src) {
                    return Err(bad("flow ratio requires autonomous scalar source"));
                }
                let (_, t) = jet(src, &last, Iv::point(1.)?, weights, 2, false, &mut work)?;
                let first = self.slab(0)?;
                let (_, time, y) = physical_cell_box(src, &first, Iv::point(0.)?, true)?;
                let initial = &self.inner.run.initial;
                if !initial.qualified || initial.x.len() != 1 || initial.root_box.len() != 1 {
                    return Err(bad("flow ratio missing initial root"));
                }
                let x = intersection(y[0], Iv::point(initial.x[0])?)?;
                let z = intersection(y[1], pair(initial.root_box[0])?)?;
                let mut j = de::ScalarJetWork::default();
                let f0 = de::scalar_adjoint_jet(src, time, [x, z], weights, 2, false, &mut j)?;
                add_work(&mut work, j);
                if f0.flow.contains_zero() {
                    return Err(bad("flow ratio initial reduced flow contains zero"));
                }
                work.divisions += 1;
                t.goal.mul(t.flow.div(f0.flow)?)?
            }
            ScalarAdjointRoute::Direct16
            | ScalarAdjointRoute::DirectBare16
            | ScalarAdjointRoute::QuadraticSeed16
            | ScalarAdjointRoute::Adaptive => {
                let passes = if route == ScalarAdjointRoute::DirectBare16 {
                    0
                } else {
                    2
                };
                let seed = route == ScalarAdjointRoute::QuadraticSeed16;
                let (_, t) = jet(src, &last, Iv::point(1.)?, weights, passes, seed, &mut work)?;
                for k in (0..m).rev() {
                    let s = self.slab(k)?;
                    if route == ScalarAdjointRoute::Adaptive {
                        value = value
                            .add(adaptive(src, &s, weights, 0., 1., 0, duration, &mut work)?)?;
                    } else {
                        for j in (0..16).rev() {
                            let (h, a) = jet(
                                src,
                                &s,
                                Iv::new(j as f64 / 16., (j + 1) as f64 / 16.)?,
                                weights,
                                passes,
                                seed,
                                &mut work,
                            )?;
                            value = value.add(h.scale(1. / 16.)?.mul(a.coefficient)?)?;
                            work.cells += 1;
                            work.split_evaluations += 1;
                        }
                    }
                }
                work.exponentials = 1;
                t.goal.mul(interval_exp(value)?)?
            }
            ScalarAdjointRoute::Repeated16 => unreachable!(),
        };
        work.width_upper = sub_up(result.hi, result.lo)?;
        out[0] = result;
        Ok(work)
    }
}
