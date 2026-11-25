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
        let xb = iv_pairs(&cert.x_range)?;
        let zb = iv_pairs(&cert.z_range)?;
        let rb = iv_pairs(&cert.branch_root_box)?;
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
        let reduced_lip = di::rows_norm(&[reduced[1..].to_vec()])?;
        // The straight segment between represented V and true Z(t,U)
        // lies in this hull; use it for the algebraic correction only.
        let mut hull = xb.clone();
        for (&v, &r) in zb.iter().zip(&rb) {
            hull.push(Iv::new(v.lo.min(r.lo), v.hi.max(r.hi))?);
        }
        let hh = di::evaluate(std::slice::from_ref(guard), time, &hull)?;
        let algebraic_guard_lip = di::rows_norm(&[hh.j[0][1 + nx..].to_vec()])?;
        let clock = [p(cert.times[0])?, p(cert.times[1])?.sub(p(cert.times[0])?)?];
        let h_poly = compose(guard, &clock, y)?;
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
