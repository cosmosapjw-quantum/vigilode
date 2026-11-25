//! Original-source a posteriori certificate for represented dense DAE slabs.
//! The fixed branch is declared by the caller. A global physical certificate
//! requires a strict bootstrap tube at EVERY slab, not only point projections.
use crate::{
    ParallelExecution,
    conditioned_polynomial::eval_poly_interval,
    retro_causal,
    retro_dae_init::{
        self as di, DaeSource, InitMode, InitPolicy, InitReport, InitRoute, InitWork,
    },
    retro_event_poly::DeclaredPolynomial,
    retro_foundation::{PencilTerm, QuotientMode, TensorPencil},
    retro_green::RetroTable,
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix,
    directed::{Interval, add_up, div_up, exp_interval, mul_up, sub_down},
    retro_dictionary::{Dictionary, RealMode},
};
use serde::Serialize;
type Iv = Interval;
type Poly = Vec<Iv>;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("DAE evolution: {s}"))
}
fn p(v: f64) -> CoreResult<Iv> {
    Iv::point(v)
}
fn zero() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn add(a: &[Iv], b: &[Iv]) -> CoreResult<Poly> {
    let mut v = vec![zero(); a.len().max(b.len())];
    for (i, &x) in a.iter().enumerate() {
        v[i] = v[i].add(x)?;
    }
    for (i, &x) in b.iter().enumerate() {
        v[i] = v[i].add(x)?;
    }
    Ok(v)
}
fn scale(a: &[Iv], b: Iv) -> CoreResult<Poly> {
    a.iter().map(|&v| v.mul(b)).collect()
}
fn mul(a: &[Iv], b: &[Iv]) -> CoreResult<Poly> {
    let mut v = vec![zero(); a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            v[i + j] = v[i + j].add(x.mul(y)?)?;
        }
    }
    Ok(v)
}
fn eval(p: &[Iv], x: Iv) -> CoreResult<Iv> {
    p.iter().rev().try_fold(zero(), |s, &c| s.mul(x)?.add(c))
}
fn choose(n: usize, k: usize) -> usize {
    (0..k).fold(1, |v, i| v * (n - i) / (i + 1))
}
fn bernstein(a: &[Iv]) -> CoreResult<Poly> {
    let n = a.len() - 1;
    (0..=n)
        .map(|i| {
            (0..=i).try_fold(zero(), |s, j| {
                s.add(a[j].mul(p(choose(i, j) as f64)?.div(p(choose(n, j) as f64)?)?)?)
            })
        })
        .collect()
}
fn range(a: &[Iv]) -> CoreResult<Iv> {
    let b = bernstein(a)?;
    Iv::new(
        b.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min),
        b.iter().map(|v| v.hi).fold(f64::NEG_INFINITY, f64::max),
    )
}
fn compose(e: &DeclaredPolynomial, t: &[Iv], y: &[Poly]) -> CoreResult<Poly> {
    let mut out = vec![zero()];
    for term in e.terms() {
        let mut v = vec![p(term.coefficient)?];
        for f in &term.factors {
            let mut a = add(&[p(f.constant)?], &scale(t, p(f.time)?)?)?;
            for &(i, w) in &f.state {
                a = add(&a, &scale(&y[i], p(w)?)?)?;
            }
            v = mul(&v, &a)?;
        }
        out = add(&out, &v)?;
    }
    Ok(out)
}
fn pol_norm(p: &[Poly]) -> CoreResult<f64> {
    p.iter()
        .map(|r| range(r).map(|v| v.mag()))
        .try_fold(0.0_f64, |m, v| Ok(m.max(v?)))
}
fn c_polys(c: &DenseMatrix, v: &[Poly]) -> CoreResult<Vec<Poly>> {
    (0..c.nrows())
        .map(|i| {
            c.row(i)
                .iter()
                .zip(v)
                .try_fold(vec![zero()], |s, (&a, v)| add(&s, &scale(v, p(a)?)?))
        })
        .collect()
}
fn jz(j: &di::Eval, nx: usize) -> Vec<Vec<Iv>> {
    j.j.iter().map(|r| r[1 + nx..].to_vec()).collect()
}
fn product_bound(q: f64, gap: f64, w: &[Iv]) -> CoreResult<Vec<Iv>> {
    let e = div_up(mul_up(q, di::norm(w))?, gap)?;
    w.iter().map(|&v| v.add(Iv::new(-e, e)?)).collect()
}
/// Exact original represented quartic, with outward coefficient intervals.
/// Inputs are actual native endpoint/stage rows, not rounded dot products.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum DenseBasis {
    SequentialD,
    FastH,
}
impl DenseBasis {
    pub fn coefficients(self) -> CoreResult<&'static DenseMatrix> {
        let c = rodas5p_core::rodas5p_coefficients()?;
        Ok(match self {
            Self::SequentialD => &c.dense_d,
            Self::FastH => &c.dense_h,
        })
    }
}
/// Construct ideal represented polynomial using the owning driver basis.
pub fn dense_power(
    y0: &[f64],
    y1: &[f64],
    stages: &[Vec<f64>],
    basis: DenseBasis,
) -> CoreResult<Vec<Poly>> {
    let n = y0.len();
    if n == 0 || y1.len() != n || stages.len() != 8 || stages.iter().any(|v| v.len() != n) {
        return Err(bad("dense packet shape"));
    }
    let h = basis.coefficients()?;
    (0..n)
        .map(|i| {
            let mut d = [zero(); 3];
            for (k, v) in d.iter_mut().enumerate() {
                for (j, u) in stages.iter().enumerate() {
                    *v = v.add(p(h[(k, j)])?.scale(u[i])?)?;
                }
            }
            let a = p(y0[i])?;
            let b = p(y1[i])?;
            Ok(vec![
                a,
                b.sub(a)?.add(d[0])?,
                d[1].sub(d[0])?,
                d[2].sub(d[1])?,
                -d[2],
            ])
        })
        .collect()
}
fn is_zero(v: &f64) -> bool {
    *v == 0.
}
/// Immutable quotient predictor. The finite Taylor quotient is ONLY a
/// candidate seed for the original source projection; no infinite tail
/// or physical residual is certified by the quotient report.
#[derive(Clone, Debug, Serialize)]
pub struct WarpSeed {
    #[serde(skip_serializing_if = "is_zero")]
    pub start: f64,
    pub end: f64,
    pub coeff: Vec<Vec<f64>>,
    pub quotient_error: f64,
    pub iterations: usize,
    pub n: usize,
}
impl WarpSeed {
    pub fn new(src: &DaeSource, end: f64, mode: QuotientMode) -> CoreResult<Self> {
        Self::new_window(src, 0., end, mode)
    }
    /// Reanchor a source-owned affine pencil at the actual physical start.
    pub fn new_window(
        src: &DaeSource,
        start: f64,
        end: f64,
        mode: QuotientMode,
    ) -> CoreResult<Self> {
        if !start.is_finite()
            || !end.is_finite()
            || end <= start
            || !(end - start).is_finite()
            || src.nz() > 8
        {
            return Err(bad("WARP domain"));
        }
        let nx = src.nx();
        let n = src.nz();
        // A z-containing factor is linear; any other factor in that term
        // must be time-only, and there can be at most one such factor.
        for eq in &src.g {
            for term in eq.terms() {
                let zf = term
                    .factors
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.state.iter().any(|&(i, w)| i >= nx && w != 0.))
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if !zf.is_empty()
                    && (zf.len() != 1
                        || term.factors.len() > 2
                        || term
                            .factors
                            .iter()
                            .enumerate()
                            .any(|(i, f)| i != zf[0] && f.state.iter().any(|&(_, w)| w != 0.)))
                {
                    return Err(bad("not a time-affine algebraic pencil"));
                }
            }
        }
        let y = vec![zero(); nx + n];
        let a = jz(&di::evaluate(&src.g, p(start)?, &y)?, nx);
        let b = jz(&di::evaluate(&src.g, p(end)?, &y)?, nx);
        let mut delta = Vec::new();
        for (r, s) in a.iter().zip(&b) {
            for (&v, &w) in r.iter().zip(s) {
                let d = w.sub(v)?;
                if v.lo != v.hi || d.lo != d.hi {
                    return Err(bad("pencil coefficients not exact represented"));
                }
                delta.push(d.lo);
            }
        }
        if delta.iter().all(|&v| v == 0.) {
            return Err(bad("constant pencil; WARP not needed"));
        }
        let a = di::center_matrix(&a)?;
        let d = DenseMatrix::new(n, n, delta)?;
        let mut pencil = TensorPencil::new(
            a,
            &[vec![0.; 6]],
            &[PencilTerm {
                powers: vec![1],
                operator: d,
            }],
        )?;
        let mut coeff = vec![vec![0.; 6]; n * n];
        let mut err = 0.0_f64;
        let mut iterations = 0;
        for j in 0..n {
            let mut rhs = vec![vec![0.; n]; 6];
            rhs[0][j] = 1.;
            let r = pencil.solve(&rhs, mode, 1e-12, 8)?;
            if r.error_upper > 1e-12 {
                return Err(bad("WARP quotient error"));
            }
            err = err.max(r.error_upper);
            iterations += r.iterations;
            for (k, row) in r.values.iter().enumerate() {
                for (i, &v) in row.iter().enumerate() {
                    coeff[i * n + j][k] = v;
                }
            }
        }
        Ok(Self {
            start,
            end,
            coeff,
            quotient_error: err,
            iterations,
            n,
        })
    }
    fn seed(&self, src: &DaeSource, t: f64, x: &[f64]) -> CoreResult<Vec<f64>> {
        let u = p(t)?
            .sub(p(self.start)?)?
            .div(p(self.end)?.sub(p(self.start)?)?)?;
        let mut y = di::points(x)?;
        y.extend(vec![zero(); self.n]);
        let g = di::evaluate(&src.g, p(t)?, &y)?.v;
        (0..self.n)
            .map(|i| {
                let mut s = zero();
                for (j, &g) in g.iter().enumerate() {
                    let v = eval_poly_interval(&self.coeff[i * self.n + j], u)
                        .map_err(|_| bad("WARP Horner"))?;
                    s = s.sub(v.mul(g)?)?;
                }
                Ok(di::mid(s))
            })
            .collect()
    }
}
/// One original-source projection. The caller owns the fixed branch.
// Keep source, branch, clock and policy authority explicit at this boundary.
#[allow(clippy::too_many_arguments)]
pub fn project(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
) -> CoreResult<InitReport> {
    if branch
        .iter()
        .any(|b| !b[0].is_finite() || !b[1].is_finite() || b[0] >= b[1])
    {
        return Err(bad("branch shape"));
    }
    let mut seed = if let Some(w) = warp {
        w.seed(src, t, x)?
    } else {
        seed.to_vec()
    };
    if seed.len() != branch.len() {
        return Err(bad("seed shape"));
    }
    for (v, b) in seed.iter_mut().zip(branch) {
        *v = v.clamp(b[0], b[1]);
    }
    src.initialize(
        t,
        x,
        &seed,
        branch,
        InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            ..policy
        },
        route,
    )
}
/// Reconstruct algebraic polynomial using actual dictionary nodes and
/// monomial basis. Rounded coefficient centers define a candidate only.
// Keep source, branch, clock and policy authority explicit at this boundary.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
    nodes: usize,
) -> CoreResult<(Vec<Poly>, Vec<InitWork>)> {
    let b = Dictionary::get()?.basis(nodes)?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    let mut out = vec![vec![zero(); nodes]; src.nz()];
    let mut work = vec![];
    for (j, node) in b.nodes.iter().enumerate() {
        let t = di::mid(p(times[0])?.add(h.scale(node.value)?)?);
        let v = x
            .iter()
            .map(|a| eval(a, p(node.value)?).map(di::mid))
            .collect::<CoreResult<Vec<_>>>()?;
        let r = project(src, t, &v, seed, branch, policy, route, warp)?;
        work.push(r.work);
        for (i, &z) in r.z.iter().enumerate() {
            for (k, a) in out[i].iter_mut().enumerate() {
                *a = a.add(b.monomial[j][k].enclosure.scale(z)?)?;
            }
        }
    }
    for a in &mut out {
        for v in a {
            *v = p(di::mid(*v))?;
        }
    }
    Ok((out, work))
}
#[derive(Clone, Debug, Serialize)]
pub struct SlabCertificate {
    pub times: [f64; 2],
    pub tube_radius: f64,
    pub x_range: Vec<[f64; 2]>,
    pub z_range: Vec<[f64; 2]>,
    pub branch_root_box: Vec<[f64; 2]>,
    pub self_map_box: Vec<[f64; 2]>,
    pub self_map_center: Vec<f64>,
    pub preconditioner: Vec<f64>,
    pub branch_defect: f64,
    pub condition_upper: f64,
    pub original_g_upper: f64,
    pub algebraic_defect: f64,
    pub differential_defect: f64,
    pub normalized_reduced_defect: f64,
    pub reduced_mu_upper: f64,
    pub zx_norm_upper: f64,
    pub incoming: f64,
    pub outgoing: Option<f64>,
    pub whole_x_error: Option<f64>,
    pub whole_z_error: Option<f64>,
    pub green_used: bool,
    pub global_qualified: bool,
    pub global_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conservation: Option<ConservationReceipt>,
}
/// Uniform existence/uniqueness on the represented slab and inflated
/// differential tube. Strict physical bootstrap is a separate condition.
// Keep source, branch, clock and policy authority explicit at this boundary.
#[allow(clippy::too_many_arguments)]
pub fn certify_slab(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
) -> CoreResult<SlabCertificate> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let nx = src.nx();
    let nz = src.nz();
    if y.len() != nx + nz
        || branch.len() != nz
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !tube.is_finite()
        || tube <= 0.
        || incoming.is_some_and(|v| !v.is_finite() || v < 0.)
    {
        return Err(bad("slab shape"));
    }
    let t = Iv::new(times[0], times[1])?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo <= 0. {
        return Err(bad("clock"));
    }
    let clock = [p(times[0])?, h];
    let ranges = y.iter().map(|a| range(a)).collect::<CoreResult<Vec<_>>>()?;
    let b = branch
        .iter()
        .map(|&[a, b]| Iv::new(a, b))
        .collect::<CoreResult<Vec<_>>>()?;
    if !di::subset(&ranges[nx..], &b) {
        return Err(bad("algebraic polynomial leaves branch"));
    }
    let xb = ranges[..nx]
        .iter()
        .map(|&v| v.add(Iv::new(-tube, tube)?))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut all = xb.clone();
    all.extend_from_slice(&b);
    let zc = ranges[nx..]
        .iter()
        .copied()
        .map(di::mid)
        .collect::<Vec<_>>();
    let mut center = xb.iter().copied().map(di::mid).collect::<Vec<_>>();
    center.extend_from_slice(&zc);
    let gc = di::evaluate(&src.g, p(di::mid(t))?, &di::points(&center)?)?;
    let a = di::center_matrix(&jz(&gc, nx))?;
    let c = di::preconditioner(
        &a,
        InitRoute::Reuse,
        &ParallelExecution::sequential(),
        &mut InitWork::default(),
    )?;
    let ga = di::evaluate(&src.g, t, &all)?;
    let j = jz(&ga, nx);
    let e = di::defect(&c, &j)?;
    let q = di::rows_norm(&e)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("uniform branch contraction not proved"));
    }
    let mut fix = xb.clone();
    fix.extend(di::points(&zc)?);
    let g0 = di::evaluate(&src.g, t, &fix)?;
    let cg = di::matvec(&c, &g0.v)?;
    // Uniqueness is established on the entire declared B. For existence
    // use either B itself or a smaller local self-map, with its own
    // interval Jacobian. The latter avoids dependency inflation on B.
    let image = |local: &[Iv], error: &[Vec<Iv>]| -> CoreResult<Vec<Iv>> {
        let db = local
            .iter()
            .zip(&zc)
            .map(|(&v, &z)| v.sub(p(z)?))
            .collect::<CoreResult<Vec<_>>>()?;
        let ed = di::iv_matvec(error, &db)?;
        zc.iter()
            .zip(&cg)
            .zip(ed)
            .map(|((&z, &v), w)| p(z)?.sub(v)?.add(w))
            .collect()
    };
    let mut local = b.clone();
    let mut root = image(&local, &e)?;
    if !di::subset(&root, &local) {
        let eta = di::norm(&cg);
        let mut admitted = false;
        for factor in [1.25, 2., 4., 8., 16., 32.] {
            let rad = mul_up(factor, eta)?;
            let trial = zc
                .iter()
                .map(|&z| p(z)?.add(Iv::new(-rad, rad)?))
                .collect::<CoreResult<Vec<_>>>()?;
            if !di::subset(&trial, &b) {
                continue;
            }
            let mut box_y = xb.clone();
            box_y.extend_from_slice(&trial);
            let jlocal = jz(&di::evaluate(&src.g, t, &box_y)?, nx);
            let elocal = di::defect(&c, &jlocal)?;
            let candidate = image(&trial, &elocal)?;
            if di::subset(&candidate, &trial) {
                local = trial;
                root = candidate;
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(bad("uniform self-map leaves branch"));
        }
    }
    let ci = (0..nz)
        .map(|i| di::points(c.row(i)))
        .collect::<CoreResult<Vec<_>>>()?;
    let condition_upper = mul_up(div_up(di::rows_norm(&ci)?, gap)?, di::rows_norm(&j)?)?;
    if condition_upper > condition_cap {
        return Err(bad("uniform condition cap"));
    }
    // Original polynomial residuals. Preconditioned g is bounded on the
    // entire normalized slab, not from point residual samples.
    let gp = src
        .g
        .iter()
        .map(|g| compose(g, &clock, y))
        .collect::<CoreResult<Vec<_>>>()?;
    let original_g_upper = pol_norm(&gp)?;
    let az = div_up(pol_norm(&c_polys(&c, &gp)?)?, gap)?;
    let fp = src
        .f
        .iter()
        .map(|f| compose(f, &clock, y))
        .collect::<CoreResult<Vec<_>>>()?;
    let residual = (0..nx)
        .map(|i| {
            let d = y[i]
                .iter()
                .enumerate()
                .skip(1)
                .map(|(k, v)| v.scale(k as f64))
                .collect::<CoreResult<Vec<_>>>()?;
            add(&d, &scale(&fp[i], -h)?)
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let dx = pol_norm(&residual)?;
    // Include the approximate algebraic polynomial AND true root in f_z
    // bounds: all already lie in B. J_red computed over this safe box.
    let f = di::evaluate(&src.f, t, &all)?;
    let fz = f.j.iter().map(|r| r[1 + nx..].to_vec()).collect::<Vec<_>>();
    let rnorm = add_up(dx, mul_up(h.hi, mul_up(di::rows_norm(&fz)?, az)?)?)?;
    let mut zx = vec![vec![zero(); nx]; nz];
    for k in 0..nx {
        let rhs = ga.j.iter().map(|r| -r[k + 1]).collect::<Vec<_>>();
        let w = di::matvec(&c, &rhs)?;
        let u = product_bound(q, gap, &w)?;
        for (i, row) in zx.iter_mut().enumerate() {
            row[k] = u[i];
        }
    }
    let lip = di::rows_norm(&zx)?;
    let mut mu = f64::NEG_INFINITY;
    for (i, row) in f.j.iter().enumerate() {
        let mut sum = 0.;
        for k in 0..nx {
            let v = row[1 + nx..]
                .iter()
                .zip(&zx)
                .try_fold(row[k + 1], |s, (&a, b)| s.add(a.mul(b[k])?))?;
            sum = add_up(sum, if i == k { v.hi } else { v.mag() })?;
        }
        mu = mu.max(sum);
    }
    let mut out = SlabCertificate {
        times,
        tube_radius: tube,
        x_range: xb.iter().map(|v| v.as_pair()).collect(),
        z_range: ranges[nx..].iter().map(|v| v.as_pair()).collect(),
        branch_root_box: root.iter().map(|v| v.as_pair()).collect(),
        self_map_box: local.iter().map(|v| v.as_pair()).collect(),
        self_map_center: zc,
        preconditioner: c.as_slice().to_vec(),
        branch_defect: q,
        condition_upper,
        original_g_upper,
        algebraic_defect: az,
        differential_defect: dx,
        normalized_reduced_defect: rnorm,
        reduced_mu_upper: mu,
        zx_norm_upper: lip,
        incoming: incoming.unwrap_or(0.),
        outgoing: None,
        whole_x_error: None,
        whole_z_error: None,
        green_used: false,
        global_qualified: false,
        global_reason: None,
        conservation: None,
    };
    let Some(ein) = incoming else {
        out.global_reason = Some("prior physical tube unresolved".into());
        return Ok(out);
    };
    let propagate = (|| -> CoreResult<(f64, f64, bool)> {
        if green && mu < 0. {
            let table = RetroTable::with_dictionary(RealMode::Hybrid)?;
            let gain = retro_causal::gain(&table, -mu, h)?;
            let u = h.scale(-mu)?;
            let ph = table.phi(Iv::new(u.lo.max(0.), u.hi)?, 1)?.hi;
            let e = add_up(mul_up(gain, ein)?, mul_up(ph, rnorm)?)?;
            Ok((e, e.max(ein), true))
        } else {
            let g = if mu > 0. {
                exp_interval(mul_up(mu, h.hi)?)?.hi
            } else {
                1.
            };
            let e = mul_up(g, add_up(ein, rnorm)?)?;
            Ok((e, e.max(ein), false))
        }
    })();
    match propagate {
        Ok((e, w, used)) => {
            out.outgoing = Some(e);
            out.whole_x_error = Some(w);
            out.green_used = used;
            out.whole_z_error = Some(add_up(az, mul_up(lip, w)?)?);
            out.global_qualified = w < tube;
            if !out.global_qualified {
                out.global_reason = Some("physical bootstrap tube exceeded".into());
            }
        }
        Err(e) => out.global_reason = Some(e.to_string()),
    }
    Ok(out)
}
/// Rounded query correction to the ideal polynomial is charged explicitly.
pub fn query_x_error(
    power: &[Poly],
    times: [f64; 2],
    t: f64,
    x: &[f64],
    ideal: f64,
) -> CoreResult<f64> {
    let h = p(times[1])?.sub(p(times[0])?)?;
    let u = p(t)?.sub(p(times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let e = power
        .iter()
        .zip(x)
        .map(|(v, &x)| eval(v, u)?.sub(p(x)?).map(|v| v.mag()))
        .try_fold(0.0_f64, |m, v| Ok::<f64, CoreError>(m.max(v?)))?;
    add_up(ideal, e)
}

/// Physical differential derivative enclosure on an already certified slab.
/// Only valid when the certificate comes from this source and this slab.
pub fn threshold_velocity(src: &DaeSource, c: &SlabCertificate) -> CoreResult<Iv> {
    if src.nx() != 1 || !c.global_qualified {
        return Err(bad("threshold requires scalar certified tube"));
    }
    let y = c
        .x_range
        .iter()
        .chain(&c.branch_root_box)
        .map(|v| Iv::new(v[0], v[1]))
        .collect::<CoreResult<Vec<_>>>()?;
    let f = di::evaluate(&src.f, Iv::new(c.times[0], c.times[1])?, &y)?;
    Ok(f.v[0])
}

/// Alternative physical tangent enclosures. A prepared inverse is only a
/// candidate; its defect against the original interval Jacobian is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GuardDerivativeRoute {
    Tangent,
    Schur,
    PreparedSchur,
    /// Opt-in tighter physical derivative box from the actual L1 receipt.
    ConservedSchur,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GuardValueRoute {
    Box,
    Transport,
}

#[derive(Clone, Debug, Serialize)]
pub struct GuardPointEvidence {
    pub time: f64,
    pub differential_radius: f64,
    pub value: [f64; 2],
}
#[derive(Clone, Debug, Serialize)]
pub struct GuardSlabEvidence {
    pub evaluations: Vec<GuardPointEvidence>,
    pub times: [f64; 2],
    pub derivative_route: GuardDerivativeRoute,
    pub value_route: GuardValueRoute,
    pub velocity: [f64; 2],
    pub reduced_gradient: Vec<[f64; 2]>,
    pub reduced_clock: [f64; 2],
    pub inverse_defect: f64,
    pub reduced_lip: f64,
    pub algebraic_guard_lip: f64,
    pub prepared_work: InitWork,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conserved_derivative_box: Option<Vec<[f64; 2]>>,
}

/// Conditional on an authentic certificate for the same original source,
/// polynomial and slab. Only the facade constructs event authority.
pub struct GuardProof<'a> {
    guard: &'a DeclaredPolynomial,
    y: &'a [Poly],
    nx: usize,
    times: [f64; 2],
    x_range: Vec<Iv>,
    z_range: Vec<Iv>,
    root_box: Vec<Iv>,
    h_poly: Poly,
    cg_poly: Vec<Poly>,
    gap: f64,
    zx_lip: f64,
    whole_x: f64,
    pub evidence: GuardSlabEvidence,
}
fn iv_pairs(v: &[[f64; 2]]) -> CoreResult<Vec<Iv>> {
    v.iter().map(|a| Iv::new(a[0], a[1])).collect()
}
fn intersect(a: Iv, b: Iv) -> CoreResult<Iv> {
    Iv::new(a.lo.max(b.lo), a.hi.min(b.hi))
}
fn guard_dot(a: &[Iv], b: &[Iv]) -> CoreResult<Iv> {
    a.iter()
        .zip(b)
        .try_fold(zero(), |s, (&a, &b)| s.add(a.mul(b)?))
}
impl<'a> GuardProof<'a> {
    pub fn new(
        src: &DaeSource,
        guard: &'a DeclaredPolynomial,
        y: &'a [Poly],
        cert: &SlabCertificate,
        derivative: GuardDerivativeRoute,
        value: GuardValueRoute,
    ) -> CoreResult<Self> {
        let nx = src.nx();
        let nz = src.nz();
        guard.validate(nx + nz)?;
        if !cert.global_qualified
            || y.len() != nx + nz
            || cert.x_range.len() != nx
            || cert.z_range.len() != nz
            || cert.branch_root_box.len() != nz
            || cert.preconditioner.len() != nz * nz
        {
            return Err(bad("guard source/slab shape"));
        }
        let time = Iv::new(cert.times[0], cert.times[1])?;
        let mut xb = iv_pairs(&cert.x_range)?;
        let zb = iv_pairs(&cert.z_range)?;
        let mut rb = iv_pairs(&cert.branch_root_box)?;
        let structural = cert.conservation.is_some() && nx == 2 && nz == 1;
        let tight = derivative == GuardDerivativeRoute::ConservedSchur;
        if tight {
            if !structural {
                return Err(bad(
                    "conserved derivative requires source-owned L1 certificate",
                ));
            }
            let r = cert
                .whole_x_error
                .ok_or_else(|| bad("conserved derivative radius"))?;
            let err = Iv::new(-r, r)?;
            for i in 0..2 {
                xb[i] = intersect(xb[i], range(&y[i])?.add(err)?)?;
            }
            let lift = add(&[p(1.)?], &scale(&add(&y[0], &y[1])?, p(-1.)?)?)?;
            // Exact affine lift: |delta z| <= ||delta x||_1, not 2r.
            rb[0] = intersect(rb[0], range(&lift)?.add(err)?)?;
        }
        let mut physical = xb.clone();
        physical.extend_from_slice(&rb);
        let f = di::evaluate(&src.f, time, &physical)?;
        let g = di::evaluate(&src.g, time, &physical)?;
        let hg = di::evaluate(std::slice::from_ref(guard), time, &physical)?;
        let h = &hg.j[0];
        let j = jz(&g, nx);
        let original = DenseMatrix::new(nz, nz, cert.preconditioner.clone())?;
        let mut work = InitWork::default();
        let c = if derivative == GuardDerivativeRoute::PreparedSchur {
            let center = di::center_matrix(&j)?;
            di::preconditioner(
                &center,
                InitRoute::PreparedBatch,
                &ParallelExecution::sequential(),
                &mut work,
            )?
        } else {
            original.clone()
        };
        let q = di::rows_norm(&di::defect(&c, &j)?)?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("guard inverse defect"));
        }
        let solve =
            |rhs: &[Iv]| -> CoreResult<Vec<Iv>> { product_bound(q, gap, &di::matvec(&c, rhs)?) };
        let hz = &h[1 + nx..];
        let mut reduced = Vec::with_capacity(nx + 1);
        for (k, &hk) in h[..=nx].iter().enumerate() {
            let rhs = g.j.iter().map(|r| -r[k]).collect::<Vec<_>>();
            reduced.push(hk.add(guard_dot(hz, &solve(&rhs)?)?)?);
        }
        let velocity = if derivative == GuardDerivativeRoute::Tangent {
            let rhs =
                g.j.iter()
                    .map(|r| r[0].add(guard_dot(&r[1..1 + nx], &f.v)?).map(|v| -v))
                    .collect::<CoreResult<Vec<_>>>()?;
            h[0].add(guard_dot(&h[1..1 + nx], &f.v)?)?
                .add(guard_dot(hz, &solve(&rhs)?)?)?
        } else {
            reduced[0].add(guard_dot(&reduced[1..], &f.v)?)?
        };
        // The structural receipt owns a reduced L1 radius. Its dual
        // norm is the maximum absolute reduced gradient component.
        let reduced_lip = if structural {
            reduced[1..].iter().map(|v| v.mag()).fold(0., f64::max)
        } else {
            di::rows_norm(&[reduced[1..].to_vec()])?
        };
        // The straight segment between represented V and true Z(t,U)
        // lies in this hull; use it for the algebraic correction only.
        let mut hull = xb.clone();
        for (&v, &r) in zb.iter().zip(&rb) {
            hull.push(Iv::new(v.lo.min(r.lo), v.hi.max(r.hi))?);
        }
        let hh = di::evaluate(std::slice::from_ref(guard), time, &hull)?;
        let algebraic_guard_lip = di::rows_norm(&[hh.j[0][1 + nx..].to_vec()])?;
        let clock = [p(cert.times[0])?, p(cert.times[1])?.sub(p(cert.times[0])?)?];
        let (h_poly, algebraic_guard_lip) = if structural && value == GuardValueRoute::Transport {
            // The source-owned Robertson constraint is exact affine
            // elimination. The comparison polynomial lives on the same
            // constraint manifold, so no separate algebraic defect term.
            let lift = add(&[p(1.)?], &scale(&add(&y[0], &y[1])?, p(-1.)?)?)?;
            (
                compose(guard, &clock, &[y[0].clone(), y[1].clone(), lift])?,
                0.,
            )
        } else {
            (compose(guard, &clock, y)?, algebraic_guard_lip)
        };
        let gp = src
            .g
            .iter()
            .map(|g| compose(g, &clock, y))
            .collect::<CoreResult<Vec<_>>>()?;
        let cg_poly = c_polys(&original, &gp)?;
        let original_gap = sub_down(1., cert.branch_defect)?;
        if original_gap <= 0. {
            return Err(bad("guard original branch defect"));
        }
        let whole_x = cert
            .whole_x_error
            .ok_or_else(|| bad("guard physical radius"))?;
        Ok(Self {
            guard,
            y,
            nx,
            times: cert.times,
            x_range: xb,
            z_range: zb,
            root_box: rb,
            h_poly,
            cg_poly,
            gap: original_gap,
            zx_lip: cert.zx_norm_upper,
            whole_x,
            evidence: GuardSlabEvidence {
                evaluations: vec![],
                times: cert.times,
                derivative_route: derivative,
                value_route: value,
                velocity: velocity.as_pair(),
                reduced_gradient: reduced[1..].iter().map(|v| v.as_pair()).collect(),
                reduced_clock: reduced[0].as_pair(),
                inverse_defect: q,
                reduced_lip,
                algebraic_guard_lip,
                prepared_work: work,
                conserved_derivative_box: tight
                    .then(|| physical.iter().map(|v| v.as_pair()).collect()),
            },
        })
    }
    /// Only an advisory polynomial, never an authority on physical roots.
    pub fn hint_coefficients(&self) -> Vec<f64> {
        self.h_poly.iter().map(|&v| di::mid(v)).collect()
    }
    /// At a physical time in the certified slab; error must be an externally
    /// established differential radius (endpoint or whole-slab).
    pub fn value_at(&self, t: f64, error: f64) -> CoreResult<Iv> {
        if !(self.times[0] <= t && t <= self.times[1])
            || !error.is_finite()
            || error < 0.
            || error > self.whole_x
        {
            return Err(bad("guard query outside physical radius"));
        }
        let u = p(t)?
            .sub(p(self.times[0])?)?
            .div(p(self.times[1])?.sub(p(self.times[0])?)?)?;
        let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
        let cg = self
            .cg_poly
            .iter()
            .map(|v| eval(v, u))
            .collect::<CoreResult<Vec<_>>>()?;
        let az = div_up(di::norm(&cg), self.gap)?;
        match self.evidence.value_route {
            GuardValueRoute::Transport => {
                let r = add_up(
                    mul_up(self.evidence.reduced_lip, error)?,
                    mul_up(self.evidence.algebraic_guard_lip, az)?,
                )?;
                eval(&self.h_poly, u)?.add(Iv::new(-r, r)?)
            }
            GuardValueRoute::Box => {
                let mut values = Vec::with_capacity(self.y.len());
                let ez = add_up(az, mul_up(self.zx_lip, error)?)?;
                for (i, v) in self.y.iter().enumerate() {
                    let a = eval(v, u)?;
                    if i < self.nx {
                        values.push(intersect(a.add(Iv::new(-error, error)?)?, self.x_range[i])?);
                    } else {
                        let k = i - self.nx;
                        let a = intersect(a, self.z_range[k])?;
                        values.push(intersect(a.add(Iv::new(-ez, ez)?)?, self.root_box[k])?);
                    }
                }
                Ok(di::evaluate(std::slice::from_ref(self.guard), p(t)?, &values)?.v[0])
            }
        }
    }
}

/// Reset envelope routes. The polynomial is composed from the original
/// source on a certified native slab, not from rounded trajectory samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResetEnvelopeRoute {
    Box,
    Transport,
    GreenMoment,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResetSlabEvidence {
    pub times: [f64; 2],
    pub reset_x: [f64; 2],
    pub synchronized_x: [f64; 2],
    pub reset_error_upper: f64,
    pub reduced_lip: f64,
    pub algebraic_error_upper: f64,
    pub moment_derivative: Option<[f64; 2]>,
    pub green_calls: usize,
    pub prepared_work: InitWork,
}
/// Only for a genuine source-bound certified slab; the facade owns event
/// authority. Synchronize to s under scalar x'=-a(x-e).
#[allow(clippy::too_many_arguments)]
pub fn reset_slab(
    src: &DaeSource,
    reset: &DeclaredPolynomial,
    y: &[Poly],
    cert: &SlabCertificate,
    times: [f64; 2],
    s: f64,
    a: f64,
    e: f64,
    route: ResetEnvelopeRoute,
    derivative: GuardDerivativeRoute,
    table: &RetroTable,
) -> CoreResult<ResetSlabEvidence> {
    if !(cert.times[0] <= times[0]
        && times[0] <= times[1]
        && times[1] <= cert.times[1]
        && times[1] <= s)
        || !a.is_finite()
        || a <= 0.
        || !e.is_finite()
    {
        return Err(bad("reset slab clock/domain"));
    }
    let value = if route == ResetEnvelopeRoute::Box {
        GuardValueRoute::Box
    } else {
        GuardValueRoute::Transport
    };
    let proof = GuardProof::new(src, reset, y, cert, derivative, value)?;
    let h = p(cert.times[1])?.sub(p(cert.times[0])?)?;
    let clock = Iv::new(times[0], times[1])?;
    let normalize = |t: Iv| -> CoreResult<Iv> {
        let u = t.sub(p(cert.times[0])?)?.div(h)?;
        Iv::new(u.lo.max(0.), u.hi.min(1.))
    };
    let u = normalize(clock)?;
    let ex = proof.whole_x;
    let cg = proof
        .cg_poly
        .iter()
        .map(|v| eval(v, u))
        .collect::<CoreResult<Vec<_>>>()?;
    let az = div_up(di::norm(&cg), proof.gap)?;
    let er = add_up(
        mul_up(proof.evidence.reduced_lip, ex)?,
        mul_up(proof.evidence.algebraic_guard_lip, az)?,
    )?;
    let r = if value == GuardValueRoute::Box {
        let ez = add_up(az, mul_up(proof.zx_lip, ex)?)?;
        let mut values = Vec::with_capacity(y.len());
        for (i, v) in y.iter().enumerate() {
            let b = eval(v, u)?;
            if i < proof.nx {
                values.push(intersect(b.add(Iv::new(-ex, ex)?)?, proof.x_range[i])?);
            } else {
                let j = i - proof.nx;
                values.push(intersect(
                    intersect(b, proof.z_range[j])?.add(Iv::new(-ez, ez)?)?,
                    proof.root_box[j],
                )?);
            }
        }
        di::evaluate(std::slice::from_ref(reset), clock, &values)?.v[0]
    } else {
        eval(&proof.h_poly, u)?.add(Iv::new(-er, er)?)?
    };
    let eq = p(e)?;
    let rate = p(a)?;
    let gain = |time: Iv| -> CoreResult<Iv> {
        let x = p(s)?.sub(time)?.mul(rate)?;
        table.phi(Iv::new(x.lo.max(0.), x.hi)?, 0)
    };
    let mut calls = 1;
    let rectangle = eq.add(gain(clock)?.mul(r.sub(eq)?)?)?;
    let (sync, moment) = if route == ResetEnvelopeRoute::GreenMoment {
        // P is the represented reset polynomial in normalized native time.
        // d/dt[e+exp(-a(s-t))(P(t)-e)] = exp(-a(s-t))*(P'(t)+a(P-e)).
        // Compose before evaluation to preserve cancellation.
        let mut d = proof
            .h_poly
            .iter()
            .enumerate()
            .skip(1)
            .map(|(k, v)| v.scale(k as f64)?.div(h))
            .collect::<CoreResult<Vec<_>>>()?;
        if d.is_empty() {
            d.push(zero());
        }
        let mut shifted = proof.h_poly.clone();
        shifted[0] = shifted[0].sub(eq)?;
        let q = add(&d, &scale(&shifted, rate)?)?;
        let derivative = eval(&q, u)?;
        let right = p(times[1])?;
        let gb = gain(right)?;
        let base = eq.add(gb.mul(eval(&proof.h_poly, normalize(right)?)?.sub(eq)?)?)?;
        let dt = right.sub(clock)?;
        let dt = Iv::new(dt.lo.max(0.), dt.hi)?;
        let adt = dt.mul(rate)?;
        let weight = gb
            .mul(dt)?
            .mul(table.phi(Iv::new(adt.lo.max(0.), adt.hi)?, 1)?)?;
        calls += 2;
        let propagated = mul_up(gain(clock)?.hi, er)?;
        calls += 1;
        let corrected = base
            .sub(weight.mul(derivative)?)?
            .add(Iv::new(-propagated, propagated)?)?;
        (intersect(rectangle, corrected)?, Some(derivative.as_pair()))
    } else {
        (rectangle, None)
    };
    Ok(ResetSlabEvidence {
        times,
        reset_x: r.as_pair(),
        synchronized_x: sync.as_pair(),
        reset_error_upper: er,
        reduced_lip: proof.evidence.reduced_lip,
        algebraic_error_upper: az,
        moment_derivative: moment,
        green_calls: calls,
        prepared_work: proof.evidence.prepared_work,
    })
}

/// Uniform branch authority for all (t,x) in the stated transition box,
/// independent of an integrator step or a pointwise initial root.
#[derive(Clone, Debug, Serialize)]
pub struct TransitionBranchProof {
    pub times: [f64; 2],
    pub differential_box: [f64; 2],
    pub branch: Vec<[f64; 2]>,
    pub root_box: Vec<[f64; 2]>,
    pub self_map_box: Vec<[f64; 2]>,
    pub defect: f64,
    pub condition_upper: f64,
    pub initial: InitReport,
    pub work: InitWork,
}
/// Reuse an initialized point only as a predictor. Strict contraction
/// is checked on the entire declared B, and a self-map proves existence
/// uniformly in the time/state box. No assumption of nominal uniqueness.
#[allow(clippy::too_many_arguments)]
pub fn certify_transition_branch(
    src: &DaeSource,
    times: [f64; 2],
    xb: [f64; 2],
    center: f64,
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
) -> CoreResult<TransitionBranchProof> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let t = Iv::new(times[0], times[1])?;
    let x = Iv::new(xb[0], xb[1])?;
    if src.nx() != 1
        || !x.contains(center)
        || policy.mode != InitMode::ProjectAlgebraic
        || branch.len() != src.nz()
    {
        return Err(bad("transition branch contract"));
    }
    let initial = src.initialize(times[1], &[center], z_seed, branch, policy, route)?;
    let nz = src.nz();
    let b = iv_pairs(branch)?;
    let zc = di::points(&initial.z)?;
    let mut point = vec![p(center)?];
    point.extend_from_slice(&zc);
    let jp = jz(&di::evaluate(&src.g, p(times[1])?, &point)?, 1);
    let mut work = InitWork::default();
    let c = di::preconditioner(
        &di::center_matrix(&jp)?,
        route,
        &ParallelExecution::sequential(),
        &mut work,
    )?;
    let mut full = vec![x];
    full.extend_from_slice(&b);
    let j = jz(&di::evaluate(&src.g, t, &full)?, 1);
    let err = di::defect(&c, &j)?;
    let q = di::rows_norm(&err)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("transition uniform branch contraction not proved"));
    }
    let mut base = vec![x];
    base.extend_from_slice(&zc);
    let cg = di::matvec(&c, &di::evaluate(&src.g, t, &base)?.v)?;
    let image = |local: &[Iv], error: &[Vec<Iv>]| -> CoreResult<Vec<Iv>> {
        let delta = local
            .iter()
            .zip(&zc)
            .map(|(&v, &z)| v.sub(z))
            .collect::<CoreResult<Vec<_>>>()?;
        let ed = di::iv_matvec(error, &delta)?;
        zc.iter()
            .zip(&cg)
            .zip(&ed)
            .map(|((&z, &v), &w)| z.sub(v)?.add(w))
            .collect()
    };
    let mut local = b.clone();
    let mut root = image(&local, &err)?;
    if !di::subset(&root, &local) {
        let eta = di::norm(&cg);
        let rad = div_up(eta, gap)?;
        let mut admitted = false;
        for factor in [1.25, 2., 4., 8., 16., 32.] {
            let radius = mul_up(factor, rad)?;
            let trial = zc
                .iter()
                .map(|&v| v.add(Iv::new(-radius, radius)?))
                .collect::<CoreResult<Vec<_>>>()?;
            if !di::subset(&trial, &b) {
                continue;
            }
            let mut v = vec![x];
            v.extend_from_slice(&trial);
            let el = di::defect(&c, &jz(&di::evaluate(&src.g, t, &v)?, 1))?;
            let candidate = image(&trial, &el)?;
            if di::subset(&candidate, &trial) {
                local = trial;
                root = candidate;
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(bad("transition uniform branch self-map not proved"));
        }
    }
    let ci = (0..nz)
        .map(|i| di::points(c.row(i)))
        .collect::<CoreResult<Vec<_>>>()?;
    let condition = mul_up(div_up(di::rows_norm(&ci)?, gap)?, di::rows_norm(&j)?)?;
    if condition > policy.condition_cap {
        return Err(bad("transition condition cap"));
    }
    Ok(TransitionBranchProof {
        times,
        differential_box: xb,
        branch: branch.to_vec(),
        root_box: root.iter().map(|v| v.as_pair()).collect(),
        self_map_box: local.iter().map(|v| v.as_pair()).collect(),
        defect: q,
        condition_upper: condition,
        initial,
        work,
    })
}

/// Synchronization of a vector reset under an original polynomial post DAE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum VectorSyncRoute {
    BoxPicard,
    SchurPicard,
    Centered,
    GreenMoment,
}
#[derive(Clone, Debug, Serialize)]
pub struct VectorResetComponent {
    pub enclosure: [f64; 2],
    pub transport_error: f64,
    pub reduced_lip: f64,
    pub prepared_work: InitWork,
    #[serde(skip)]
    power: Poly,
    #[serde(skip)]
    native_times: [f64; 2],
}
/// Old-mode physical reset image on a subinterval of its authentic slab.
pub fn vector_reset_component(
    src: &DaeSource,
    reset: &DeclaredPolynomial,
    y: &[Poly],
    c: &SlabCertificate,
    times: [f64; 2],
    route: VectorSyncRoute,
    derivative: GuardDerivativeRoute,
) -> CoreResult<VectorResetComponent> {
    if !(c.times[0] <= times[0] && times[0] <= times[1] && times[1] <= c.times[1]) {
        return Err(bad("vector reset slab domain"));
    }
    let value = if route == VectorSyncRoute::BoxPicard {
        GuardValueRoute::Box
    } else {
        GuardValueRoute::Transport
    };
    let proof = GuardProof::new(src, reset, y, c, derivative, value)?;
    let clock = Iv::new(times[0], times[1])?;
    let h = p(c.times[1])?.sub(p(c.times[0])?)?;
    let u = clock.sub(p(c.times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let cg = proof
        .cg_poly
        .iter()
        .map(|v| eval(v, u))
        .collect::<CoreResult<Vec<_>>>()?;
    let az = div_up(di::norm(&cg), proof.gap)?;
    let ex = proof.whole_x;
    let er = add_up(
        mul_up(proof.evidence.reduced_lip, ex)?,
        mul_up(proof.evidence.algebraic_guard_lip, az)?,
    )?;
    let r = if value == GuardValueRoute::Box {
        let ez = add_up(az, mul_up(proof.zx_lip, ex)?)?;
        let mut values = Vec::with_capacity(y.len());
        for (i, v) in y.iter().enumerate() {
            let b = eval(v, u)?;
            if i < proof.nx {
                values.push(intersect(b.add(Iv::new(-ex, ex)?)?, proof.x_range[i])?);
            } else {
                let j = i - proof.nx;
                values.push(intersect(
                    intersect(b, proof.z_range[j])?.add(Iv::new(-ez, ez)?)?,
                    proof.root_box[j],
                )?);
            }
        }
        di::evaluate(std::slice::from_ref(reset), clock, &values)?.v[0]
    } else {
        eval(&proof.h_poly, u)?.add(Iv::new(-er, er)?)?
    };
    Ok(VectorResetComponent {
        enclosure: r.as_pair(),
        transport_error: er,
        reduced_lip: proof.evidence.reduced_lip,
        prepared_work: proof.evidence.prepared_work,
        power: proof.h_poly,
        native_times: c.times,
    })
}
#[derive(Clone, Debug, Serialize)]
pub struct VectorBranchProof {
    pub times: [f64; 2],
    pub differential_box: Vec<[f64; 2]>,
    pub root_box: Vec<[f64; 2]>,
    pub self_map_box: Vec<[f64; 2]>,
    pub defect: f64,
    pub condition_upper: f64,
    pub initial: InitReport,
    pub work: InitWork,
    pub field: Vec<[f64; 2]>,
    pub reduced_jacobian: Vec<Vec<[f64; 2]>>,
    pub reduced_clock: Vec<[f64; 2]>,
    pub nominal_field: Vec<[f64; 2]>,
}
/// Existence/uniqueness of the declared branch for every time and x in a
/// rectangular domain, followed by signed Schur bounds of original f.
#[allow(clippy::too_many_arguments)]
pub fn certify_vector_branch(
    src: &DaeSource,
    times: [f64; 2],
    xb: &[[f64; 2]],
    center: &[f64],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
) -> CoreResult<VectorBranchProof> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let nx = src.nx();
    let nz = src.nz();
    let t = Iv::new(times[0], times[1])?;
    let x = iv_pairs(xb)?;
    if x.len() != nx
        || center.len() != nx
        || !x.iter().zip(center).all(|(a, &b)| a.contains(b))
        || policy.mode != InitMode::ProjectAlgebraic
        || branch.len() != nz
        || !matches!(
            route,
            InitRoute::Reuse | InitRoute::PreparedBatch | InitRoute::ScalarRoot
        )
    {
        return Err(bad("vector uniform branch contract"));
    }
    let initial = src.initialize(times[1], center, z_seed, branch, policy, route)?;
    let b = iv_pairs(branch)?;
    let zc = di::points(&initial.z)?;
    let mut point = di::points(center)?;
    point.extend_from_slice(&zc);
    let jp = jz(&di::evaluate(&src.g, p(times[1])?, &point)?, nx);
    let mut work = InitWork::default();
    let c = di::preconditioner(
        &di::center_matrix(&jp)?,
        route,
        &ParallelExecution::sequential(),
        &mut work,
    )?;
    let mut full = x.clone();
    full.extend_from_slice(&b);
    let j = jz(&di::evaluate(&src.g, t, &full)?, nx);
    let err = di::defect(&c, &j)?;
    let q = di::rows_norm(&err)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("vector uniform branch contraction not proved"));
    }
    let mut base = x.clone();
    base.extend_from_slice(&zc);
    let cg = di::matvec(&c, &di::evaluate(&src.g, t, &base)?.v)?;
    let image = |local: &[Iv], error: &[Vec<Iv>]| -> CoreResult<Vec<Iv>> {
        let delta = local
            .iter()
            .zip(&zc)
            .map(|(&v, &z)| v.sub(z))
            .collect::<CoreResult<Vec<_>>>()?;
        let ed = di::iv_matvec(error, &delta)?;
        zc.iter()
            .zip(&cg)
            .zip(&ed)
            .map(|((&z, &v), &w)| z.sub(v)?.add(w))
            .collect()
    };
    let mut local = b.clone();
    let mut root = image(&local, &err)?;
    if !di::subset(&root, &local) {
        let rad = div_up(di::norm(&cg), gap)?;
        let mut admitted = false;
        for factor in [1.25, 2., 4., 8., 16., 32.] {
            let radius = mul_up(factor, rad)?;
            let trial = zc
                .iter()
                .map(|&v| v.add(Iv::new(-radius, radius)?))
                .collect::<CoreResult<Vec<_>>>()?;
            if !di::subset(&trial, &b) {
                continue;
            }
            let mut v = x.clone();
            v.extend_from_slice(&trial);
            let el = di::defect(&c, &jz(&di::evaluate(&src.g, t, &v)?, nx))?;
            let candidate = image(&trial, &el)?;
            if di::subset(&candidate, &trial) {
                local = trial;
                root = candidate;
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(bad("vector uniform branch self-map not proved"));
        }
    }
    let ci = (0..nz)
        .map(|i| di::points(c.row(i)))
        .collect::<CoreResult<Vec<_>>>()?;
    let condition = mul_up(div_up(di::rows_norm(&ci)?, gap)?, di::rows_norm(&j)?)?;
    if condition > policy.condition_cap {
        return Err(bad("vector condition cap"));
    }
    let mut y = x.clone();
    y.extend_from_slice(&root);
    let g = di::evaluate(&src.g, t, &y)?;
    let f = di::evaluate(&src.f, t, &y)?;
    let qr = di::rows_norm(&di::defect(&c, &jz(&g, nx))?)?;
    let gr = sub_down(1., qr)?;
    if gr <= 0. {
        return Err(bad("vector reduced inverse defect"));
    }
    let mut columns = Vec::with_capacity(nx + 1);
    for k in 0..=nx {
        let rhs = g.j.iter().map(|r| -r[k]).collect::<Vec<_>>();
        columns.push(product_bound(qr, gr, &di::matvec(&c, &rhs)?)?);
    }
    let mut reduced = Vec::with_capacity(nx);
    for r in &f.j {
        let v = (0..=nx)
            .map(|k| r[k].add(guard_dot(&r[1 + nx..], &columns[k])?))
            .collect::<CoreResult<Vec<_>>>()?;
        reduced.push(v);
    }
    let mut nominal = di::points(center)?;
    nominal.extend(iv_pairs(&initial.root_box)?);
    let f0 = di::evaluate(&src.f, p(times[1])?, &nominal)?.v;
    // Centered reduced mean-value bound intersects the direct source box.
    let mut delta = vec![t.sub(p(times[1])?)?];
    for (&v, &a) in x.iter().zip(center) {
        delta.push(v.sub(p(a)?)?);
    }
    let field =
        f.v.iter()
            .zip(&f0)
            .zip(&reduced)
            .map(|((&f, &fc), r)| intersect(f, fc.add(guard_dot(r, &delta)?)?))
            .collect::<CoreResult<Vec<_>>>()?;
    Ok(VectorBranchProof {
        times,
        differential_box: xb.to_vec(),
        root_box: root.iter().map(|v| v.as_pair()).collect(),
        self_map_box: local.iter().map(|v| v.as_pair()).collect(),
        defect: q,
        condition_upper: condition,
        initial,
        work,
        field: field.iter().map(|v| v.as_pair()).collect(),
        reduced_jacobian: reduced
            .iter()
            .map(|r| r[1..].iter().map(|v| v.as_pair()).collect())
            .collect(),
        reduced_clock: reduced.iter().map(|r| r[0].as_pair()).collect(),
        nominal_field: f0.iter().map(|v| v.as_pair()).collect(),
    })
}
#[derive(Clone, Debug, Serialize)]
pub struct VectorTubeProof {
    pub attempts: Vec<VectorBranchProof>,
    pub field: Vec<[f64; 2]>,
    pub tube: Vec<[f64; 2]>,
    pub picard_image: Vec<[f64; 2]>,
}
/// Strict self-map over original mode extension for every allowed reset
/// clock and state. Uniform branch proof makes the reduced field C1.
pub fn certify_vector_tube(
    src: &DaeSource,
    times: [f64; 2],
    r: &[[f64; 2]],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
) -> CoreResult<VectorTubeProof> {
    let reset = iv_pairs(r)?;
    if reset.len() != src.nx() {
        return Err(bad("vector tube shape"));
    }
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo < 0. {
        return Err(bad("vector tube time"));
    }
    let elapsed = Iv::new(0., h.hi)?;
    let pad = |v: Iv| -> CoreResult<Iv> {
        let w = v.sub(p(v.lo)?)?.hi;
        let margin = mul_up(0.125, w)?.max(1e-10_f64.max(mul_up(1e-14, v.mag())?));
        v.add(Iv::new(-margin, margin)?)
    };
    let mut b = reset
        .iter()
        .map(|&v| pad(v))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut attempts = Vec::with_capacity(8);
    for _ in 0..8 {
        let center = b.iter().map(|&v| di::mid(v)).collect::<Vec<_>>();
        let xb = b.iter().map(|v| v.as_pair()).collect::<Vec<_>>();
        let proof = certify_vector_branch(src, times, &xb, &center, z_seed, branch, policy, route)?;
        let field = iv_pairs(&proof.field)?;
        let image = reset
            .iter()
            .zip(&field)
            .map(|(&r, &f)| r.add(elapsed.mul(f)?))
            .collect::<CoreResult<Vec<_>>>()?;
        let admitted = image
            .iter()
            .zip(&b)
            .all(|(a, b)| b.lo < a.lo && a.hi < b.hi);
        attempts.push(proof);
        if admitted {
            return Ok(VectorTubeProof {
                field: field.iter().map(|v| v.as_pair()).collect(),
                tube: xb,
                picard_image: image.iter().map(|v| v.as_pair()).collect(),
                attempts,
            });
        }
        b = b
            .iter()
            .zip(&image)
            .map(|(&a, &v)| pad(Iv::new(a.lo.min(v.lo), a.hi.max(v.hi))?))
            .collect::<CoreResult<Vec<_>>>()?;
    }
    Err(bad("vector Picard tube budget exhausted"))
}
#[derive(Clone, Debug, Serialize)]
pub struct VectorSyncEvidence {
    pub synchronized: Vec<[f64; 2]>,
    pub green_calls: usize,
    pub forcing_remainder: Vec<[f64; 2]>,
}
// Local mean-value chart for an interval-coefficient represented polynomial.
fn reset_centered_eval(poly: &[Iv], u: Iv) -> CoreResult<Iv> {
    let center = p(di::mid(u))?;
    let d = poly
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, v)| v.scale(k as f64))
        .collect::<CoreResult<Vec<_>>>()?;
    let tight = eval(poly, center)?.add(u.sub(center)?.mul(eval(&d, u)?)?)?;
    intersect(eval(poly, u)?, tight)
}
/// Synchronize every reset clock in times to s. The tube must be an
/// authentic proof on [times[0],s] for this exact post source and reset.
pub fn synchronize_vector(
    reset: &[VectorResetComponent],
    times: [f64; 2],
    s: f64,
    tube: &VectorTubeProof,
    route: VectorSyncRoute,
    table: &RetroTable,
) -> CoreResult<VectorSyncEvidence> {
    let proof = tube
        .attempts
        .last()
        .ok_or_else(|| bad("missing vector tube proof"))?;
    let n = reset.len();
    if n != tube.field.len() || proof.times != [times[0], s] || times[1] > s {
        return Err(bad("vector sync domain"));
    }
    let clock = Iv::new(times[0], times[1])?;
    let elapsed = p(s)?.sub(clock)?;
    let elapsed = Iv::new(elapsed.lo.max(0.), elapsed.hi)?;
    let field = iv_pairs(&tube.field)?;
    let xb = iv_pairs(&tube.tube)?;
    let mut out = VectorSyncEvidence {
        synchronized: vec![],
        green_calls: 0,
        forcing_remainder: vec![],
    };
    for (i, r) in reset.iter().enumerate() {
        let rect = Iv::new(r.enclosure[0], r.enclosure[1])?.add(elapsed.mul(field[i])?)?;
        let h = p(r.native_times[1])?.sub(p(r.native_times[0])?)?;
        let u = clock.sub(p(r.native_times[0])?)?.div(h)?;
        let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
        let v = match route {
            VectorSyncRoute::BoxPicard | VectorSyncRoute::SchurPicard => rect,
            VectorSyncRoute::Centered => {
                let k = di::mid(field[i]);
                let shifted = add(
                    &r.power,
                    &[p(s)?.sub(p(r.native_times[0])?)?.scale(k)?, h.scale(-k)?],
                )?;
                let remainder = field[i].sub(p(k)?)?;
                out.forcing_remainder.push(remainder.as_pair());
                let corrected = reset_centered_eval(&shifted, u)?
                    .add(Iv::new(-r.transport_error, r.transport_error)?)?
                    .add(elapsed.mul(remainder)?)?;
                intersect(rect, corrected)?
            }
            VectorSyncRoute::GreenMoment => {
                // Diagonal unit damping is an exact split, not an
                // approximation of the post Jacobian: F=-(X-e)+B.
                let e = p(proof.initial.x[i]
                    + di::mid(Iv::new(
                        proof.nominal_field[i][0],
                        proof.nominal_field[i][1],
                    )?))?;
                let mut residual = Iv::new(proof.nominal_field[i][0], proof.nominal_field[i][1])?
                    .add(p(proof.initial.x[i])?)?
                    .sub(e)?;
                residual = residual.add(
                    Iv::new(proof.reduced_clock[i][0], proof.reduced_clock[i][1])?
                        .mul(Iv::new(-p(s)?.sub(p(times[0])?)?.hi, 0.)?)?,
                )?;
                for j in 0..n {
                    let d = Iv::new(
                        proof.reduced_jacobian[i][j][0],
                        proof.reduced_jacobian[i][j][1],
                    )?
                    .add(p(if i == j { 1. } else { 0. })?)?;
                    residual = residual.add(d.mul(xb[j].sub(p(proof.initial.x[j])?)?)?)?;
                }
                let remainder = intersect(residual, field[i].add(xb[i])?.sub(e)?)?;
                out.forcing_remainder.push(remainder.as_pair());
                let mut d = r
                    .power
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(k, v)| v.scale(k as f64)?.div(h))
                    .collect::<CoreResult<Vec<_>>>()?;
                if d.is_empty() {
                    d.push(zero());
                }
                let mut shifted = r.power.clone();
                shifted[0] = shifted[0].sub(e)?;
                let q = eval(&add(&d, &shifted)?, u)?;
                let b = p(times[1])?;
                let ub = b.sub(p(r.native_times[0])?)?.div(h)?;
                let ub = Iv::new(ub.lo.max(0.), ub.hi.min(1.))?;
                let db = p(s)?.sub(b)?;
                let db = Iv::new(db.lo.max(0.), db.hi)?;
                let gb = table.phi(db, 0)?;
                let yb = e.add(gb.mul(eval(&shifted, ub)?)?)?;
                let d = b.sub(clock)?;
                let d = Iv::new(d.lo.max(0.), d.hi)?;
                let weight = gb.mul(d)?.mul(table.phi(d, 1)?)?;
                let ge = table.phi(elapsed, 0)?;
                let err = mul_up(ge.hi, r.transport_error)?;
                let forcing = elapsed.mul(table.phi(elapsed, 1)?)?.mul(remainder)?;
                out.green_calls += 4;
                let corrected = yb
                    .sub(weight.mul(q)?)?
                    .add(Iv::new(-err, err)?)?
                    .add(forcing)?;
                intersect(rect, corrected)?
            }
        };
        out.synchronized.push(intersect(v, xb[i])?.as_pair());
    }
    Ok(out)
}

/// Negative guard proof on an authenticated vector transition branch tube.
/// The source/branch proof binding is established by the owning facade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GapGuardRoute {
    Box,
    Schur,
}
#[derive(Clone, Debug, Serialize)]
pub struct GapGuardEvidence {
    pub enclosure: [f64; 2],
    pub direct: [f64; 2],
    pub reduced_gradient: Option<Vec<[f64; 2]>>,
    pub reduced_clock: Option<[f64; 2]>,
    pub work: InitWork,
}
pub fn gap_guard_enclosure(
    src: &DaeSource,
    guard: &DeclaredPolynomial,
    bp: &VectorBranchProof,
    route: GapGuardRoute,
    init_route: InitRoute,
) -> CoreResult<GapGuardEvidence> {
    let nx = src.nx();
    let nz = src.nz();
    guard.validate(nx + nz)?;
    if bp.differential_box.len() != nx
        || bp.root_box.len() != nz
        || bp.initial.x.len() != nx
        || bp.initial.z.len() != nz
        || bp.defect >= 1.
    {
        return Err(bad("gap guard branch shape"));
    }
    let t = Iv::new(bp.times[0], bp.times[1])?;
    let xb = iv_pairs(&bp.differential_box)?;
    let mut full = xb.clone();
    full.extend(iv_pairs(&bp.root_box)?);
    let h = di::evaluate(std::slice::from_ref(guard), t, &full)?;
    let direct = h.v[0];
    let mut work = InitWork::default();
    let mut out = GapGuardEvidence {
        enclosure: direct.as_pair(),
        direct: direct.as_pair(),
        reduced_gradient: None,
        reduced_clock: None,
        work: work.clone(),
    };
    if route == GapGuardRoute::Box {
        return Ok(out);
    }
    let g = di::evaluate(&src.g, t, &full)?;
    let mut point = di::points(&bp.initial.x)?;
    point.extend(di::points(&bp.initial.z)?);
    let jc = jz(&di::evaluate(&src.g, p(bp.times[1])?, &point)?, nx);
    let c = di::preconditioner(
        &di::center_matrix(&jc)?,
        init_route,
        &ParallelExecution::sequential(),
        &mut work,
    )?;
    let q = di::rows_norm(&di::defect(&c, &jz(&g, nx))?)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("gap guard inverse defect"));
    }
    let hr = &h.j[0];
    let mut reduced = Vec::with_capacity(nx + 1);
    for k in 0..=nx {
        let rhs = g.j.iter().map(|r| -r[k]).collect::<Vec<_>>();
        let v = product_bound(q, gap, &di::matvec(&c, &rhs)?)?;
        reduced.push(hr[k].add(guard_dot(&hr[1 + nx..], &v)?)?);
    }
    let mut nominal = di::points(&bp.initial.x)?;
    nominal.extend(iv_pairs(&bp.initial.root_box)?);
    let hc = di::evaluate(std::slice::from_ref(guard), p(bp.times[1])?, &nominal)?.v[0];
    let mut dx = vec![t.sub(p(bp.times[1])?)?];
    for (&a, &b) in xb.iter().zip(&bp.initial.x) {
        dx.push(a.sub(p(b)?)?);
    }
    out.enclosure = intersect(direct, hc.add(guard_dot(&reduced, &dx)?)?)?.as_pair();
    out.reduced_gradient = Some(reduced[1..].iter().map(|v| v.as_pair()).collect());
    out.reduced_clock = Some(reduced[0].as_pair());
    out.work = work;
    Ok(out)
}

/// Structural residual routes on the exact source-owned Robertson simplex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ConservationRoute {
    Max,
    Integral4,
    Integral16,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConservationCell {
    pub theta: [f64; 2],
    pub v_minus: f64,
    pub z_minus: f64,
    pub mu_l1_upper: f64,
    pub residual_integral: f64,
    pub growth_upper: f64,
    pub incoming_l1: f64,
    pub outgoing_l1: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConservationReceipt {
    pub route: ConservationRoute,
    pub rate: f64,
    pub norm: &'static str,
    pub cells: Vec<ConservationCell>,
    pub incoming_l1: f64,
    pub outgoing_l1: Option<f64>,
}
/// Constructs the original DAE itself. No user-provided arbitrary polynomial
/// can be substituted while retaining the invariant-source authority.
#[derive(Clone)]
pub struct RobertsonModel {
    src: DaeSource,
    rate: f64,
}
impl RobertsonModel {
    pub fn new(rate: f64) -> CoreResult<Self> {
        use crate::retro_event_poly::{AffineFactor as A, EventTerm as T};
        if !matches!(rate, 3e7 | 6e7) {
            return Err(bad("Robertson rate outside scoped invariant model"));
        }
        let term = |c, factors| T {
            coefficient: c,
            factors,
        };
        let l = |i, c| term(c, vec![A::component(i, 0.)]);
        let pair = |i, j, c| term(c, vec![A::component(i, 0.), A::component(j, 0.)]);
        let src = DaeSource::new(
            vec![
                DeclaredPolynomial::new(vec![l(0, -0.04), pair(1, 2, 1e4)])?,
                DeclaredPolynomial::new(vec![l(0, 0.04), pair(1, 2, -1e4), pair(1, 1, -rate)])?,
            ],
            vec![DeclaredPolynomial::new(vec![
                l(0, 1.),
                l(1, 1.),
                l(2, 1.),
                term(-1., vec![]),
            ])?],
        )?;
        Ok(Self { src, rate })
    }
    pub fn source(&self) -> &DaeSource {
        &self.src
    }
    /// Low-level certificate: incoming L1 radius must come from an actual
    /// physical simplex family, not an arbitrary infinity-norm ball.
    /// The facade owns this precondition. Reuses original branch proof.
    #[allow(clippy::too_many_arguments)]
    pub fn certify(
        &self,
        times: [f64; 2],
        y: &[Poly],
        branch: &[[f64; 2]],
        tube: f64,
        incoming_l1: Option<f64>,
        condition_cap: f64,
        route: ConservationRoute,
    ) -> CoreResult<SlabCertificate> {
        let mut out = certify_slab(
            &self.src,
            times,
            y,
            branch,
            tube,
            None,
            false,
            condition_cap,
        )?;
        out.incoming = incoming_l1.unwrap_or(0.);
        let mut evidence = ConservationReceipt {
            route,
            rate: self.rate,
            norm: "reduced-l1",
            cells: vec![],
            incoming_l1: out.incoming,
            outgoing_l1: None,
        };
        if let Some(mut e) = incoming_l1 {
            if !e.is_finite() || e < 0. {
                return Err(bad("invalid incoming L1 radius"));
            }
            let h = p(times[1])?.sub(p(times[0])?)?;
            let lift = add(&[p(1.)?], &scale(&add(&y[0], &y[1])?, p(-1.)?)?)?;
            // Compose the original source with exact affine elimination.
            let clock = [p(times[0])?, h];
            let reduced = [y[0].clone(), y[1].clone(), lift.clone()];
            let defect = self
                .src
                .f
                .iter()
                .take(2)
                .enumerate()
                .map(|(i, f)| {
                    let mut d = y[i]
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(k, v)| v.scale(k as f64))
                        .collect::<CoreResult<Vec<_>>>()?;
                    if d.is_empty() {
                        d.push(zero());
                    }
                    add(&d, &scale(&compose(f, &clock, &reduced)?, -h)?)
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let n = match route {
                ConservationRoute::Max => 1,
                ConservationRoute::Integral4 => 4,
                ConservationRoute::Integral16 => 16,
            };
            evidence.cells.reserve(n);
            let w = 1. / n as f64;
            for k in 0..n {
                let l = k as f64 * w;
                let vminus = (-range(&conservation_restrict(&y[1], l, w)?)?.lo).max(0.);
                let zminus = (-range(&conservation_restrict(&lift, l, w)?)?.lo).max(0.);
                let a = mul_up(
                    2.,
                    rodas5p_core::directed::sub_up(mul_up(1e4, vminus)?, 0.04)?.max(0.),
                )?;
                let b = add_up(mul_up(2. * self.rate, vminus)?, mul_up(2e4, zminus)?)?;
                let mu = a.max(b);
                let growth_arg = mul_up(mul_up(h.hi, w)?, mu)?;
                let den = sub_down(1., growth_arg)?;
                if den <= 0. {
                    out.global_reason = Some("conservation growth denominator not positive".into());
                    out.conservation = Some(evidence);
                    return Ok(out);
                }
                let gain = div_up(1., den)?;
                let mut r = 0.;
                for d in &defect {
                    let coeff = bernstein(&conservation_restrict(d, l, w)?)?;
                    let m = if route == ConservationRoute::Max {
                        coeff.iter().map(|v| v.mag()).fold(0., f64::max)
                    } else {
                        div_up(
                            coeff.iter().try_fold(0., |s, v| add_up(s, v.mag()))?,
                            coeff.len() as f64,
                        )?
                    };
                    r = add_up(r, m)?;
                }
                r = mul_up(r, w)?;
                let next = mul_up(gain, add_up(e, r)?)?;
                evidence.cells.push(ConservationCell {
                    theta: [l, l + w],
                    v_minus: vminus,
                    z_minus: zminus,
                    mu_l1_upper: mu,
                    residual_integral: r,
                    growth_upper: gain,
                    incoming_l1: e,
                    outgoing_l1: next,
                });
                e = next;
            }
            // All gains >=1; endpoint envelope also bounds the entire slab.
            evidence.outgoing_l1 = Some(e);
            out.outgoing = Some(e);
            out.whole_x_error = Some(e);
            out.whole_z_error = Some(add_up(out.algebraic_defect, e)?);
            out.global_qualified = e < tube;
            out.global_reason = (!out.global_qualified)
                .then_some("conservation physical bootstrap tube exceeded".into());
        } else {
            out.global_reason = Some("prior conservation authority unresolved".into());
        }
        out.conservation = Some(evidence);
        Ok(out)
    }
}
/// Restrict power polynomial to exact dyadic subcell using directed Horner
/// composition. Degrees remain bounded by the original polynomial degree.
fn conservation_restrict(a: &[Iv], l: f64, w: f64) -> CoreResult<Poly> {
    let mut out = vec![
        *a.last()
            .ok_or_else(|| bad("empty conservation polynomial"))?,
    ];
    let v = [p(l)?, p(w)?];
    for &c in a[..a.len() - 1].iter().rev() {
        out = add(&mul(&out, &v)?, &[c])?;
    }
    Ok(out)
}
/// Rounded differential state correction in the actual L1 norm.
pub fn conservation_query_l1(
    power: &[Poly],
    times: [f64; 2],
    t: f64,
    x: &[f64],
    ideal: f64,
) -> CoreResult<f64> {
    if power.len() < 2 || x.len() != 2 || t < times[0] || t > times[1] {
        return Err(bad("conservation query domain"));
    }
    let u = p(t)?
        .sub(p(times[0])?)?
        .div(p(times[1])?.sub(p(times[0])?)?)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    power[..2]
        .iter()
        .zip(x)
        .try_fold(ideal, |s, (v, &x)| add_up(s, eval(v, u)?.sub(p(x)?)?.mag()))
}
/// The exact affine lift gives full physical infinity error <= reduced L1
/// error; only the returned projected z's own discrepancy is extra.
pub fn conservation_query_full(
    power: &[Poly],
    times: [f64; 2],
    t: f64,
    state: &[f64],
    ideal: f64,
) -> CoreResult<f64> {
    if state.len() != 3 || power.len() < 2 || t < times[0] || t > times[1] {
        return Err(bad("conservation full query shape"));
    }
    let u = p(t)?
        .sub(p(times[0])?)?
        .div(p(times[1])?.sub(p(times[0])?)?)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let a = eval(&power[0], u)?;
    let b = eval(&power[1], u)?;
    let lift = p(1.)?.sub(a)?.sub(b)?;
    let correction = [a, b, lift]
        .iter()
        .zip(state)
        .map(|(&a, &b)| a.sub(p(b)?).map(|x| x.mag()))
        .collect::<CoreResult<Vec<_>>>()?
        .into_iter()
        .fold(0., f64::max);
    add_up(ideal, correction)
}

/// Exact nonnegative column-stochastic map on the original three-species
/// simplex. A floating-point near-equality is not a conservation proof.
#[derive(Clone, Debug, Serialize)]
pub struct SimplexReset {
    matrix: [[f64; 3]; 3],
    operator_l1: f64,
}
impl SimplexReset {
    pub fn new(matrix: [[f64; 3]; 3]) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        if matrix
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || *v < 0. || *v > 1.)
        {
            return Err(bad("simplex reset nonnegative coefficients"));
        }
        for j in 0..3 {
            let sum = (0..3).try_fold(zero(), |s, i| s.add(p(matrix[i][j])?))?;
            if sum.lo != 1. || sum.hi != 1. {
                return Err(bad("simplex reset exact column sum"));
            }
        }
        let mut norm = 0_f64;
        for j in 0..2 {
            let mut sum = 0.;
            for row in &matrix[..2] {
                sum = add_up(sum, p(row[j])?.sub(p(row[2])?)?.mag())?;
            }
            norm = norm.max(sum);
        }
        Ok(Self {
            matrix,
            operator_l1: norm,
        })
    }
    pub fn matrix(&self) -> &[[f64; 3]; 3] {
        &self.matrix
    }
    pub fn operator_l1(&self) -> f64 {
        self.operator_l1
    }
    pub fn polynomials(&self) -> CoreResult<Vec<DeclaredPolynomial>> {
        use crate::retro_event_poly::{AffineFactor, EventTerm};
        self.matrix[..2]
            .iter()
            .map(|row| {
                DeclaredPolynomial::new(
                    row.iter()
                        .enumerate()
                        .map(|(i, &c)| EventTerm {
                            coefficient: c,
                            factors: vec![AffineFactor::component(i, 0.)],
                        })
                        .collect(),
                )
            })
            .collect()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SimplexSyncRoute {
    BoxLift,
    JointTransport,
}
#[derive(Clone, Debug, Serialize)]
pub struct SimplexTransportCell {
    pub times: [f64; 2],
    pub native_times: [f64; 2],
    pub old_l1: f64,
    pub mapped_l1: f64,
    pub reference_field: [f64; 2],
    pub field_box: [[f64; 2]; 2],
    pub signed_predictor: [[f64; 2]; 4],
    pub predictor_l1: f64,
    pub remainder_l1: f64,
    pub joint_l1: f64,
}
/// The facade supplies actual source-bound pre slab and post Picard tube.
/// Centered signed polynomial keeps the common clock and reduced L1
/// correlation. It is independent of componentwise rectangle transport.
pub fn simplex_transport_cell(
    reset: &SimplexReset,
    y: &[Poly],
    cert: &SlabCertificate,
    times: [f64; 2],
    s: f64,
    tube: &VectorTubeProof,
    center: [f64; 2],
) -> CoreResult<SimplexTransportCell> {
    if y.len() != 3
        || cert.conservation.is_none()
        || !cert.global_qualified
        || !(cert.times[0] <= times[0]
            && times[0] <= times[1]
            && times[1] <= cert.times[1]
            && times[1] <= s)
        || tube.field.len() != 2
        || tube
            .attempts
            .last()
            .is_none_or(|v| v.times != [times[0], s])
    {
        return Err(bad("simplex transport authority/domain"));
    }
    let old = cert.whole_x_error.ok_or_else(|| bad("missing L1 radius"))?;
    let mapped = mul_up(reset.operator_l1, old)?;
    let h = p(cert.times[1])?.sub(p(cert.times[0])?)?;
    let clock = Iv::new(times[0], times[1])?;
    let u = clock.sub(p(cert.times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let dt = p(s)?.sub(clock)?;
    let fields = iv_pairs(&tube.field)?;
    let k = [di::mid(fields[0]), di::mid(fields[1])];
    let lift = add(&[p(1.)?], &scale(&add(&y[0], &y[1])?, p(-1.)?)?)?;
    let represented = [&y[0], &y[1], &lift];
    let mut shifted = Vec::with_capacity(2);
    for i in 0..2 {
        let mut r = vec![zero()];
        for (j, v) in represented.iter().enumerate() {
            r = add(&r, &scale(v, p(reset.matrix[i][j])?)?)?;
        }
        r = add(
            &r,
            &[p(s)?.sub(p(cert.times[0])?)?.scale(k[i])?, h.scale(-k[i])?],
        )?;
        r[0] = r[0].sub(p(center[i])?)?;
        shifted.push(r);
    }
    let mut signed = [[0.; 2]; 4];
    let mut pred = 0_f64;
    for (i, (a, b)) in [(1., 1.), (1., -1.), (-1., 1.), (-1., -1.)]
        .into_iter()
        .enumerate()
    {
        let q = add(&scale(&shifted[0], p(a)?)?, &scale(&shifted[1], p(b)?)?)?;
        let v = reset_centered_eval(&q, u)?;
        signed[i] = v.as_pair();
        pred = pred.max(v.hi);
    }
    let remainder = mul_up(
        dt.hi.max(0.),
        add_up(
            fields[0].sub(p(k[0])?)?.mag(),
            fields[1].sub(p(k[1])?)?.mag(),
        )?,
    )?;
    let joint = add_up(add_up(pred, mapped)?, remainder)?;
    Ok(SimplexTransportCell {
        times,
        native_times: cert.times,
        old_l1: old,
        mapped_l1: mapped,
        reference_field: k,
        field_box: [tube.field[0], tube.field[1]],
        signed_predictor: signed,
        predictor_l1: pred,
        remainder_l1: remainder,
        joint_l1: joint,
    })
}

/// On a source-owned affine conservation manifold, use the original
/// lifted guard, or its centered reduced gradient with an L1 dual bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SimplexGapRoute {
    Box,
    AffineDual,
}
#[derive(Clone, Debug, Serialize)]
pub struct SimplexGapEvidence {
    pub route: SimplexGapRoute,
    pub times: [f64; 2],
    pub differential_box: [[f64; 2]; 2],
    pub center: [f64; 2],
    pub incoming_l1: Option<f64>,
    pub direct: [f64; 2],
    pub reduced_gradient: Option<[[f64; 2]; 2]>,
    pub reduced_clock: Option<[f64; 2]>,
    pub center_value: Option<[f64; 2]>,
    pub centered: Option<[f64; 2]>,
    pub enclosure: [f64; 2],
}
impl RobertsonModel {
    /// Tube input is a genuine certified branch rectangle supplied by the
    /// owning facade. Restart input is the full L1 ball intersected with
    /// the physical simplex; build its rectangular superset here, rather
    /// than silently reusing the smaller old synchronization box.
    pub fn simplex_gap_enclosure(
        &self,
        guard: &DeclaredPolynomial,
        times: [f64; 2],
        xb: [[f64; 2]; 2],
        center: [f64; 2],
        radius: Option<f64>,
        route: SimplexGapRoute,
    ) -> CoreResult<SimplexGapEvidence> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        guard.validate(3)?;
        let t = Iv::new(times[0], times[1])?;
        let c = [p(center[0])?, p(center[1])?];
        let x = if let Some(r) = radius {
            if times[0] != times[1] || !r.is_finite() || r < 0. {
                return Err(bad("simplex gap L1 restart domain"));
            }
            [c[0].add(Iv::new(-r, r)?)?, c[1].add(Iv::new(-r, r)?)?]
        } else {
            [Iv::new(xb[0][0], xb[0][1])?, Iv::new(xb[1][0], xb[1][1])?]
        };
        if !x.iter().zip(center).all(|(v, c)| v.contains(c)) {
            return Err(bad("simplex gap center outside box"));
        }
        let lift = p(1.)?.sub(x[0])?.sub(x[1])?;
        let e = di::evaluate(std::slice::from_ref(guard), t, &[x[0], x[1], lift])?;
        let direct = e.v[0];
        let mut out = SimplexGapEvidence {
            route,
            times,
            differential_box: [x[0].as_pair(), x[1].as_pair()],
            center,
            incoming_l1: radius,
            direct: direct.as_pair(),
            reduced_gradient: None,
            reduced_clock: None,
            center_value: None,
            centered: None,
            enclosure: direct.as_pair(),
        };
        if route == SimplexGapRoute::Box {
            return Ok(out);
        }
        let h = &e.j[0];
        let reduced = [h[1].sub(h[3])?, h[2].sub(h[3])?];
        let hc = di::evaluate(
            std::slice::from_ref(guard),
            p(times[1])?,
            &[c[0], c[1], p(1.)?.sub(c[0])?.sub(c[1])?],
        )?
        .v[0];
        let mut bound = hc.add(h[0].mul(t.sub(p(times[1])?)?)?)?;
        if let Some(r) = radius {
            let er = mul_up(r, reduced[0].mag().max(reduced[1].mag()))?;
            bound = bound.add(Iv::new(-er, er)?)?;
        } else {
            for i in 0..2 {
                bound = bound.add(reduced[i].mul(x[i].sub(c[i])?)?)?;
            }
        }
        out.reduced_gradient = Some([reduced[0].as_pair(), reduced[1].as_pair()]);
        out.reduced_clock = Some(h[0].as_pair());
        out.center_value = Some(hc.as_pair());
        out.centered = Some(bound.as_pair());
        out.enclosure = intersect(direct, bound)?.as_pair();
        Ok(out)
    }
}

/// Absolute-time query of a source-owned relative phase certificate.
/// Both methods use the exact affine lift, not the rounded algebraic
/// reconstruction. The owning facade supplies genuine native slabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhaseQueryRoute {
    Horner,
    Centered,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseCellEvidence {
    pub native_times: [f64; 2],
    pub phase: [f64; 2],
    pub represented: [[f64; 2]; 3],
    pub enclosure: [[f64; 2]; 3],
    pub error_l1: f64,
}
pub fn phase_slab_enclosure(
    y: &[Poly],
    c: &SlabCertificate,
    phase: [f64; 2],
    route: PhaseQueryRoute,
) -> CoreResult<PhaseCellEvidence> {
    if y.len() != 3
        || !c.global_qualified
        || c.conservation.is_none()
        || !(c.times[0] <= phase[0] && phase[0] <= phase[1] && phase[1] <= c.times[1])
    {
        return Err(bad("relative phase slab authority"));
    }
    let error = c
        .whole_x_error
        .ok_or_else(|| bad("missing phase L1 error"))?;
    let h = p(c.times[1])?.sub(p(c.times[0])?)?;
    let u = Iv::new(phase[0], phase[1])?.sub(p(c.times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let z = add(&[p(1.)?], &scale(&add(&y[0], &y[1])?, p(-1.)?)?)?;
    let mut represented = [[0.; 2]; 3];
    let mut enclosure = [[0.; 2]; 3];
    for (i, row) in [&y[0], &y[1], &z].into_iter().enumerate() {
        let v = match route {
            PhaseQueryRoute::Horner => eval(row, u)?,
            PhaseQueryRoute::Centered => reset_centered_eval(row, u)?,
        };
        represented[i] = v.as_pair();
        enclosure[i] = intersect(v.add(Iv::new(-error, error)?)?, Iv::new(0., 1.)?)?.as_pair();
    }
    Ok(PhaseCellEvidence {
        native_times: c.times,
        phase,
        represented,
        enclosure,
        error_l1: error,
    })
}
