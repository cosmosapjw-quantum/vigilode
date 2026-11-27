//! Bounded scalar source jet. The caller supplies the certified physical box.
//! A proposed quadratic center is never a proof of a root.
use super::*;
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ScalarJetWork {
    pub source_rows: usize,
    pub root_passes: usize,
    pub divisions: usize,
    pub quadratic_seed_uses: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct ScalarAdjointJet {
    pub coefficient: Iv,
    pub goal: Iv,
    pub flow: Iv,
}
pub fn scalar_autonomous_source(src: &DaeSource) -> bool {
    src.nx() == 1
        && src.nz() == 1
        && src.f.iter().chain(&src.g).all(|eq| {
            eq.terms()
                .iter()
                .all(|t| t.coefficient == 0. || t.factors.iter().all(|f| f.time == 0.))
        })
}
/// Newton inclusion using the original source derivative interval, with
/// optional pre-existing quadratic branch chart only for center selection.
pub fn scalar_adjoint_jet(
    src: &DaeSource,
    time: Iv,
    mut y: [Iv; 2],
    weights: &[f64],
    passes: usize,
    quadratic_seed: bool,
    work: &mut ScalarJetWork,
) -> CoreResult<ScalarAdjointJet> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    if src.nx() != 1
        || src.nz() != 1
        || weights.len() != 2
        || passes > 2
        || weights.iter().any(|v| !v.is_finite())
    {
        return Err(bad("scalar adjoint source/goal shape"));
    }
    for v in y.iter().chain(std::iter::once(&time)) {
        Iv::new(v.lo, v.hi)?;
    }
    let mut g = [zero(); 3];
    di::evaluate_row_bounded(&src.g[0], time, &y, &mut g)?;
    work.source_rows += 1;
    if g[2].contains_zero() {
        return Err(bad("scalar adjoint g_z contains zero"));
    }
    let mut seed = None;
    if quadratic_seed {
        // Degree in z at most two, proved from declared affine factors.
        let degree = src.g[0]
            .terms()
            .iter()
            .filter(|t| t.coefficient != 0.)
            .map(|t| {
                t.factors
                    .iter()
                    .filter(|f| f.state.iter().any(|&(i, a)| i == 1 && a != 0.))
                    .count()
            })
            .max()
            .unwrap_or(0);
        if degree != 2 {
            return Err(bad("quadratic seed requires declared degree two"));
        }
        let t = p(di::mid(time))?;
        let x = p(di::mid(y[0]))?;
        let mut q0 = [zero(); 3];
        let mut q1 = [zero(); 3];
        let b = -di::mid(di::evaluate_row_bounded(
            &src.g[0],
            t,
            &[x, zero()],
            &mut q0,
        )?);
        di::evaluate_row_bounded(&src.g[0], t, &[x, p(1.)?], &mut q1)?;
        work.source_rows += 2;
        let a = di::mid(q0[2]);
        let c = (di::mid(q1[2]) - a) / 2.;
        let q = crate::quadratic_branch::quadratic_homotopy_root(a, b, c, 1.)
            .map_err(|_| bad("quadratic seed chart refused"))?;
        seed = Some(q.value.clamp(y[1].lo, y[1].hi));
        work.quadratic_seed_uses += 1;
    }
    for k in 0..passes {
        let center = p(if k == 0 {
            seed.unwrap_or_else(|| di::mid(y[1]))
        } else {
            di::mid(y[1])
        })?;
        let mut unused = [zero(); 3];
        let value = di::evaluate_row_bounded(&src.g[0], time, &[y[0], center], &mut unused)?;
        work.source_rows += 1;
        y[1] = intersect(y[1], center.sub(value.div(g[2])?)?)?;
        work.divisions += 1;
        work.root_passes += 1;
        di::evaluate_row_bounded(&src.g[0], time, &y, &mut g)?;
        work.source_rows += 1;
        if g[2].contains_zero() {
            return Err(bad("scalar adjoint contracted g_z contains zero"));
        }
    }
    let zx = (-g[1]).div(g[2])?;
    work.divisions += 1;
    let mut f = [zero(); 3];
    let flow = di::evaluate_row_bounded(&src.f[0], time, &y, &mut f)?;
    work.source_rows += 1;
    Ok(ScalarAdjointJet {
        coefficient: f[1].add(f[2].mul(zx)?)?,
        goal: p(weights[0])?.add(zx.scale(weights[1])?)?,
        flow,
    })
}
