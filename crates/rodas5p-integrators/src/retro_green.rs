//! RETRO-WARP research client: diagonal Green path with an original continuous
//! defect bound. Autonomous source-owned contract, not an arbitrary RHS solver.
//! Forecasts initialize an untrusted global Picard iteration, never a boundary.
//! The exact mathematical path uses the stored forcing polynomial and exact
//! binary64 model coefficients; directed evaluation and residual bounds are
//! separate from the predictor. No production solver defaults are changed.
use rodas5p_core::{
    CoreError, CoreResult, compute_normalized_damped_moment,
    directed::{Interval, add_up, div_down, div_up, exp_interval, mul_down, mul_up, sub_up},
    sha256_hex,
};

const PHI_BITS: &str = include_str!("retro_phi_cell.hex");
pub const PHI_PACK_SHA256: &str =
    "2f108ef659f85b938e782d23cd15a20e1b0e1476dbe1aced0bcb5502a996f142";
pub const SOURCE_SHA256: &str = "fc777013ec8a740a9028e8893502552b7f20c6be9ba4af0249357e48a9ea9c29";
pub const CERT_SHA256: &str = "3b7cb46088886e2049c564a192d07b8c4ba55eb9231fff64e4f450fcf2048505";
const M: usize = 6;
const MAX_CELLS: usize = 1024;
const MAX_STEPS: usize = 4096;
const MAX_ATTEMPTS: usize = 8192;
type Iv = Interval;

fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro Green: {s}"))
}
fn limit(s: &str) -> CoreError {
    CoreError::NonlinearSolve(format!("retro Green: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn zero() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn mid(x: Iv) -> f64 {
    0.5 * x.lo + 0.5 * x.hi
}
fn rad(x: Iv, y: f64) -> CoreResult<f64> {
    Ok(sub_up(y, x.lo)?.max(sub_up(x.hi, y)?).max(0.))
}
fn finite(x: &[f64]) -> bool {
    x.iter().all(|x| x.is_finite())
}
fn mag_sum(x: &[Iv]) -> CoreResult<f64> {
    x.iter().try_fold(0., |a, b| add_up(a, b.mag()))
}

/// One immutable, bit-exact projection of the earlier certified table:
/// z in [-1,0], phi0..phi3, degree20. The original *polynomial* error
/// upper bound is used; online Horner and uncertain argument are directed.
/// Middle arguments reuse damped moments; large arguments use the exact\n/// scaled inverse-damping recurrence with directed exp.
#[derive(Clone)]
pub struct RetroTable {
    data: [f64; 92],
    #[cfg(feature = "retro-dictionary")]
    dictionary_mode: Option<rodas5p_core::retro_dictionary::RealMode>,
}
impl RetroTable {
    pub fn new() -> CoreResult<Self> {
        Self::from_projection(PHI_BITS)
    }
    /// Accept only the pinned coefficient and bound bytes. Text whitespace
    /// is irrelevant; the binary64 payload hash is the authority.
    pub fn from_projection(hex: &str) -> CoreResult<Self> {
        if hex.len() > 8192 {
            return Err(bad("table projection size"));
        }
        let mut data = [0.; 92];
        let mut bytes = Vec::with_capacity(736);
        let mut n = 0;
        for w in hex.split_whitespace() {
            if n == 92 || w.len() != 16 {
                return Err(bad("table projection shape"));
            }
            let b = u64::from_str_radix(w, 16).map_err(|_| bad("invalid projection word"))?;
            data[n] = f64::from_bits(b);
            bytes.extend_from_slice(&b.to_le_bytes());
            n += 1;
        }
        if n != 92 || sha256_hex(&bytes) != PHI_PACK_SHA256 {
            return Err(bad("table projection identity"));
        }
        Ok(Self {
            data,
            #[cfg(feature = "retro-dictionary")]
            dictionary_mode: None,
        })
    }
    /// Explicitly select the complete authenticated dictionary. The legacy
    /// constructor and its narrower numerical contract are unchanged.
    #[cfg(feature = "retro-dictionary")]
    pub fn with_dictionary(mode: rodas5p_core::retro_dictionary::RealMode) -> CoreResult<Self> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        rodas5p_core::retro_dictionary::Dictionary::get()?;
        let mut t = Self::new()?;
        t.dictionary_mode = Some(mode);
        Ok(t)
    }
    #[cfg(feature = "retro-dictionary")]
    pub fn dictionary_mode(&self) -> Option<rodas5p_core::retro_dictionary::RealMode> {
        self.dictionary_mode
    }
    fn point_phi(&self, x: f64, j: usize, w: &mut Work) -> CoreResult<Iv> {
        #[cfg(feature = "retro-dictionary")]
        if let Some(mode) = self.dictionary_mode {
            use rodas5p_core::retro_dictionary::{Dictionary, RealMode};
            if x >= 64. && matches!(mode, RealMode::Scaled | RealMode::Hybrid) {
                w.scaled_evals += 1;
            } else {
                w.table_evals += 1;
            }
            return Dictionary::get()?.phi(pt(x)?, j, mode);
        }
        if x == 0. {
            return pt(1.)?.div(pt([1., 1., 2., 6.][j])?);
        }
        if x <= 1. {
            w.table_evals += 1;
            let u = pt(-x)?.sub(pt(self.data[2])?)?.div(pt(self.data[3])?)?;
            let a = &self.data[4 + 21 * j..4 + 21 * (j + 1)];
            let mut v = pt(a[20])?;
            for &c in a[..20].iter().rev() {
                v = v.mul(u)?.add(pt(c)?)?;
            }
            let eps = self.data[88 + j];
            v.add(Iv::new(-eps, eps)?)
        } else if x >= 4. {
            // Exact inverse-damping response (p=k in Round02 scaled moment
            // family). For x>=4, the recurrence divides previous enclosure
            // errors by at least four. No close large-x subtraction.
            w.scaled_evals += 1;
            let mut v = exp_interval(-x)?;
            for k in 1..=j {
                v = pt([1., 1., 0.5][k - 1])?.sub(v)?.div(pt(x)?)?;
            }
            Ok(v)
        } else {
            w.fallback_evals += 1;
            if j == 0 {
                exp_interval(-x)
            } else {
                compute_normalized_damped_moment(x, 1., 0, j - 1)?.div(pt([1., 1., 1., 2.][j])?)
            }
        }
    }
    fn phi_counted(&self, x: Iv, j: usize, w: &mut Work) -> CoreResult<Iv> {
        Iv::new(x.lo, x.hi)?;
        if x.lo < 0. || x.hi > 16384. || j > 3 {
            return Err(bad("phi argument or index outside contract"));
        }
        // phi_j(-x) is nonnegative and decreasing for x>=0 by its
        // integral representation (phi0 by exp). Hull endpoint enclosures.
        let left = self.point_phi(x.lo, j, w)?;
        if x.lo == x.hi {
            return Iv::new(left.lo.max(0.), left.hi);
        }
        let right = self.point_phi(x.hi, j, w)?;
        Iv::new(right.lo.max(0.), left.hi)
    }
    pub fn phi(&self, x: Iv, j: usize) -> CoreResult<Iv> {
        self.phi_counted(x, j, &mut Work::default())
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Work {
    pub source_evals: usize,
    pub picard_sweeps: usize,
    pub certificate_cells: usize,
    pub coarse_certificates: usize,
    pub table_evals: usize,
    pub fallback_evals: usize,
    pub scaled_evals: usize,
    pub attempts: usize,
    pub rejections: usize,
}
impl Work {
    fn absorb(&mut self, w: &Self) {
        self.source_evals += w.source_evals;
        self.picard_sweeps += w.picard_sweeps;
        self.certificate_cells += w.certificate_cells;
        self.coarse_certificates += w.coarse_certificates;
        self.table_evals += w.table_evals;
        self.fallback_evals += w.fallback_evals;
        self.scaled_evals += w.scaled_evals;
    }
}

/// Exact represented polynomial vector field:
/// y_i'=-kappa_i y_i+b_i+sum_j C_ij y_j-beta_i y_i^3.
/// Admission is a global infinity log-norm sufficient condition, not a
/// stiffness classifier. Matrix C is row-major; no deserialized authority.
#[derive(Clone, Debug)]
pub struct Model {
    kappa: Vec<f64>,
    b: Vec<f64>,
    c: Vec<f64>,
    beta: Vec<f64>,
    mu: f64,
}
impl Model {
    pub fn new(kappa: Vec<f64>, b: Vec<f64>, c: Vec<f64>, beta: Vec<f64>) -> CoreResult<Self> {
        let n = kappa.len();
        if n == 0
            || n > 32
            || b.len() != n
            || beta.len() != n
            || c.len() != n * n
            || !finite(&kappa)
            || !finite(&b)
            || !finite(&c)
            || !finite(&beta)
            || kappa.iter().chain(&beta).any(|&a| a < 0.)
        {
            return Err(bad(
                "model shape, finite nonnegative rates/damping required",
            ));
        }
        let mut mu = f64::NEG_INFINITY;
        for i in 0..n {
            let mut row = sub_up(c[i * n + i], kappa[i])?;
            for j in 0..n {
                if j != i {
                    row = add_up(row, c[i * n + j].abs())?;
                }
            }
            mu = mu.max(row);
        }
        if mu > 0. {
            return Err(bad("global infinity logarithmic norm not nonpositive"));
        }
        Ok(Self {
            kappa,
            b,
            c,
            beta,
            mu,
        })
    }
    pub fn dimension(&self) -> usize {
        self.kappa.len()
    }
    pub fn log_norm_upper(&self) -> f64 {
        self.mu
    }
    fn source(&self, y: &[f64], w: &mut Work) -> CoreResult<Vec<f64>> {
        w.source_evals += 1;
        let n = self.dimension();
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let mut v = self.b[i];
            for (&a, &x) in self.c[i * n..(i + 1) * n].iter().zip(y) {
                v = a.mul_add(x, v);
            }
            if self.beta[i] != 0. {
                v -= self.beta[i] * y[i] * y[i] * y[i];
            }
            out.push(v);
        }
        if !finite(&out) {
            return Err(limit("nonfinite predictor source"));
        }
        Ok(out)
    }
    fn validate_path(&self, y: &[f64], p: &[[f64; 3]], h: f64) -> CoreResult<()> {
        if y.len() != self.dimension()
            || p.len() != y.len()
            || !finite(y)
            || p.iter().any(|v| !finite(v))
            || !h.is_finite()
            || h <= 0.
        {
            return Err(bad("path shape or step"));
        }
        Ok(())
    }
}
fn argument(k: f64, h: f64, theta: f64) -> CoreResult<Iv> {
    let x = pt(k)?.mul(pt(h)?)?.mul(pt(theta)?)?;
    Iv::new(x.lo.max(0.), x.hi)
}
/// coefficient of y0 and monomial forcing coefficients. Physical h retained.
/// phi_j(-x) evaluated for the exact represented product k*h*theta.
fn green_weights(
    table: &RetroTable,
    k: f64,
    h: f64,
    theta: f64,
    w: &mut Work,
) -> CoreResult<[Iv; 4]> {
    if theta == 0. {
        return Ok([pt(1.)?, zero(), zero(), zero()]);
    }
    let x = argument(k, h, theta)?;
    let mut a = [zero(); 4];
    a[0] = table.phi_counted(x, 0, w)?;
    let th = pt(theta)?;
    let mut p = pt(h)?;
    for j in 1..=3 {
        p = p.mul(th)?;
        a[j] = p
            .scale([1., 1., 1., 2.][j])?
            .mul(table.phi_counted(x, j, w)?)?;
    }
    Ok(a)
}
fn value(
    table: &RetroTable,
    k: &[f64],
    y: &[f64],
    p: &[[f64; 3]],
    h: f64,
    theta: f64,
    w: &mut Work,
) -> CoreResult<Vec<Iv>> {
    if !theta.is_finite() || !(0. ..=1.).contains(&theta) {
        return Err(bad("evaluation outside slab"));
    }
    if theta == 0. {
        return y.iter().map(|&v| pt(v)).collect();
    }
    (0..y.len())
        .map(|i| {
            let g = green_weights(table, k[i], h, theta, w)?;
            let mut v = g[0].scale(y[i])?;
            for j in 0..3 {
                v = v.add(g[j + 1].scale(p[i][j])?)?;
            }
            Ok(v)
        })
        .collect()
}
fn product(a: &[Iv], b: &[Iv]) -> CoreResult<Vec<Iv>> {
    let mut c = vec![zero(); a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            c[i + j] = c[i + j].add(x.mul(y)?)?;
        }
    }
    Ok(c)
}
fn restriction(p: [f64; 3], l: f64, d: f64) -> CoreResult<[Iv; 3]> {
    let l = pt(l)?;
    let d = pt(d)?;
    let a = pt(p[0])?;
    let b = pt(p[1])?;
    let c = pt(p[2])?;
    Ok([
        a.add(l.mul(b.add(c.mul(l)?)?)?)?,
        d.mul(b.add(c.mul(l)?.scale(2.)?)?)?,
        c.mul(d)?.mul(d)?,
    ])
}
fn cell_bound(model: &Model, y0: &[Iv], pp: &[[Iv; 3]], h: f64, d: f64) -> CoreResult<f64> {
    let n = model.dimension();
    let hd = pt(h)?.mul(pt(d)?)?;
    let mut q = vec![[zero(); M + 1]; n];
    let mut tails = vec![0.; n];
    let mut ranges = vec![0.; n];
    for i in 0..n {
        let b = hd.scale(model.kappa[i])?;
        let b = Iv::new(b.lo.max(0.), b.hi)?;
        let pol = pp[i];
        q[i][0] = y0[i];
        for j in 0..M {
            let g = if j < 3 { hd.mul(pol[j])? } else { zero() };
            q[i][j + 1] = g.sub(b.mul(q[i][j])?)?.div(pt((j + 1) as f64)?)?;
        }
        tails[i] = mul_up(q[i][M].mag(), div_up(b.hi, (M + 1) as f64)?.min(1.))?;
        ranges[i] = add_up(mag_sum(&q[i])?, tails[i])?;
    }
    let mut max = 0.0_f64;
    for i in 0..n {
        let mut defect = [zero(); 3 * M + 1];
        defect[..3].copy_from_slice(&pp[i]);
        defect[0] = defect[0].sub(pt(model.b[i])?)?;
        let mut tail = 0.;
        for j in 0..n {
            let c = model.c[i * n + j];
            if c == 0. {
                continue;
            }
            for (v, &x) in defect.iter_mut().zip(&q[j]) {
                *v = v.sub(x.scale(c)?)?;
            }
            tail = add_up(tail, mul_up(c.abs(), tails[j])?)?;
        }
        if model.beta[i] != 0. {
            let cube = product(&product(&q[i], &q[i])?, &q[i])?;
            for (v, &x) in defect.iter_mut().zip(&cube) {
                *v = v.add(x.scale(model.beta[i])?)?;
            }
            let t = mul_up(
                mul_up(mul_up(3., model.beta[i])?, mul_up(ranges[i], ranges[i])?)?,
                tails[i],
            )?;
            tail = add_up(tail, t)?;
        }
        max = max.max(add_up(mag_sum(&defect)?, tail)?);
    }
    Ok(max)
}

/// A verified local path, relative to its stored initial state, not an
/// arbitrary future value. Only immutable coefficient access is exposed.
#[derive(Clone, Debug)]
pub struct Slab {
    kappa: Vec<f64>,
    y0: Vec<f64>,
    forcing: Vec<[f64; 3]>,
    h: f64,
    ideal_error: f64,
    endpoint_ideal_error: f64,
    gain_upper: f64,
    endpoint_error: f64,
    end: Vec<f64>,
    end_round: f64,
    local_error: f64,
}
impl Slab {
    pub fn h(&self) -> f64 {
        self.h
    }
    pub fn endpoint(&self) -> &[f64] {
        &self.end
    }
    pub fn ideal_error_upper(&self) -> f64 {
        self.ideal_error
    }
    pub fn local_error_upper(&self) -> f64 {
        self.local_error
    }
    pub fn endpoint_error_upper(&self) -> f64 {
        self.endpoint_error
    }
    pub fn endpoint_rounding_upper(&self) -> f64 {
        self.end_round
    }
    pub fn forcing(&self) -> &[[f64; 3]] {
        &self.forcing
    }
    /// Bound includes ideal path error and per-query directed evaluation.
    pub fn evaluate(&self, table: &RetroTable, theta: f64) -> CoreResult<(Vec<f64>, f64)> {
        if theta == 0. {
            return Ok((self.y0.clone(), 0.));
        }
        let v = value(
            table,
            &self.kappa,
            &self.y0,
            &self.forcing,
            self.h,
            theta,
            &mut Work::default(),
        )?;
        let y: Vec<_> = v.iter().copied().map(mid).collect();
        let mut r = 0.0_f64;
        for (&i, &a) in v.iter().zip(&y) {
            r = r.max(rad(i, a)?);
        }
        Ok((
            y,
            add_up(
                if theta == 1. {
                    self.endpoint_ideal_error
                } else {
                    self.ideal_error
                },
                r,
            )?,
        ))
    }
}
fn cells_for(model: &Model, h: f64) -> CoreResult<usize> {
    let max = model.kappa.iter().copied().fold(0., f64::max);
    let a = mul_up(max, h)?;
    let required = mul_up(4., a)?.ceil().max(4.);
    if required > MAX_CELLS as f64 {
        return Err(limit("certificate cell cap"));
    }
    Ok((required as usize).next_power_of_two())
}
// The scalar majorant with constant nonnegative forcing on a cell is
// monotone between its start and end values for mu<=0. Thus a prefix
// maximum of the endpoint majorants bounds the entire ideal path.
fn transport(table: &RetroTable, mu: f64, h: f64, d: f64, w: &mut Work) -> CoreResult<(f64, f64)> {
    let hd = mul_up(h, d)?;
    let x = argument(-mu, h, d)?;
    if x.hi > 16384. {
        return Ok((1., hd));
    }
    let gain = table.phi_counted(x, 0, w)?.hi.min(1.);
    let weight = mul_up(hd, table.phi_counted(x, 1, w)?.hi.min(1.))?;
    Ok((gain, weight))
}
// For the exact ideal Green path, a cheap bound follows directly from
// the positivity of the diagonal semigroup. This is a sufficient test,
// not a heuristic pruning of a small state or residual.
fn coarse_defect(model: &Model, y0: &[f64], p: &[[f64; 3]], h: f64) -> CoreResult<f64> {
    let n = model.dimension();
    let mut range = Vec::with_capacity(n);
    for i in 0..n {
        let pmax = p[i].iter().try_fold(0., |a, b| add_up(a, b.abs()))?;
        let mut r = add_up(y0[i].abs(), mul_up(h, pmax)?)?;
        if model.kappa[i] > 0. {
            // If the quotient overflows, the non-damping bound is still valid.
            if let Ok(v) = div_up(pmax, model.kappa[i]) {
                r = r.min(y0[i].abs().max(v));
            }
        }
        range.push(r);
    }
    let mut bound = 0.0_f64;
    for i in 0..n {
        let mut d = add_up(
            add_up(pt(p[i][0])?.sub(pt(model.b[i])?)?.mag(), p[i][1].abs())?,
            p[i][2].abs(),
        )?;
        for (&a, &r) in model.c[i * n..(i + 1) * n].iter().zip(&range) {
            d = add_up(d, mul_up(a.abs(), r)?)?;
        }
        if model.beta[i] != 0. {
            d = add_up(
                d,
                mul_up(
                    mul_up(mul_up(model.beta[i], range[i])?, range[i])?,
                    range[i],
                )?,
            )?;
        }
        bound = bound.max(d);
    }
    Ok(bound)
}
fn local_green_weights(
    table: &RetroTable,
    k: f64,
    h: f64,
    d: f64,
    w: &mut Work,
) -> CoreResult<[Iv; 4]> {
    let hd = pt(h)?.mul(pt(d)?)?;
    let x = argument(k, h, d)?;
    let mut g = [zero(); 4];
    g[0] = table.phi_counted(x, 0, w)?;
    for j in 1..=3 {
        g[j] = hd
            .scale([1., 1., 1., 2.][j])?
            .mul(table.phi_counted(x, j, w)?)?;
    }
    Ok(g)
}
/// Certifies any finite supplied forcing polynomial against the original
/// model. No assumption that its node residual is small. Nonpositive global
/// mu_inf means propagation of a physical defect has gain <=1.
pub fn certify_polynomial(
    table: &RetroTable,
    model: &Model,
    y0: &[f64],
    p: &[[f64; 3]],
    h: f64,
    w: &mut Work,
) -> CoreResult<Slab> {
    certify_impl(table, model, y0, p, h, None, w)
}
fn certify_impl(
    table: &RetroTable,
    model: &Model,
    y0: &[f64],
    p: &[[f64; 3]],
    h: f64,
    target: Option<f64>,
    w: &mut Work,
) -> CoreResult<Slab> {
    model.validate_path(y0, p, h)?;
    let cells = cells_for(model, h)?;
    let d = 1. / cells as f64;
    let v = value(table, &model.kappa, y0, p, h, 1., w)?;
    let end: Vec<_> = v.iter().copied().map(mid).collect();
    let mut end_round = 0.0_f64;
    for (&i, &x) in v.iter().zip(&end) {
        end_round = end_round.max(rad(i, x)?);
    }
    let (gain_upper, whole_weight) = transport(table, model.mu, h, 1., w)?;
    let (ideal, endpoint_ideal) = if let Some(tol) = target
        && let Ok(defect) = coarse_defect(model, y0, p, h)
        && let Ok(coarse) = mul_up(whole_weight, defect)
        && add_up(coarse, end_round).is_ok_and(|v| v <= tol)
    {
        w.coarse_certificates += 1;
        (coarse, coarse)
    } else {
        let mut ideal = 0.0_f64;
        let mut endpoint_ideal = 0.0_f64;
        let (cell_gain, cell_weight) = transport(table, model.mu, h, d, w)?;
        // All certification cells have the same normalized length. Reuse
        // their directed Green weights and carry an enclosure of the exact
        // ideal path, not a new approximate trajectory.
        let g: Vec<_> = model
            .kappa
            .iter()
            .map(|&k| local_green_weights(table, k, h, d, w))
            .collect::<CoreResult<_>>()?;
        let mut current = y0.iter().map(|&x| pt(x)).collect::<CoreResult<Vec<_>>>()?;
        for j in 0..cells {
            let l = j as f64 * d;
            let pp = p
                .iter()
                .map(|&p| restriction(p, l, d))
                .collect::<CoreResult<Vec<_>>>()?;
            w.certificate_cells += 1;
            let b = cell_bound(model, &current, &pp, h, d)?;
            endpoint_ideal = add_up(mul_up(cell_gain, endpoint_ideal)?, mul_up(cell_weight, b)?)?;
            ideal = ideal.max(endpoint_ideal);
            if j + 1 < cells {
                for ((state, weights), poly) in current.iter_mut().zip(&g).zip(&pp) {
                    let mut v = weights[0].mul(*state)?;
                    for k in 0..3 {
                        v = v.add(weights[k + 1].mul(poly[k])?)?;
                    }
                    *state = v;
                }
            }
        }
        (ideal, endpoint_ideal)
    };
    Ok(Slab {
        kappa: model.kappa.clone(),
        y0: y0.to_vec(),
        forcing: p.to_vec(),
        h,
        ideal_error: ideal,
        endpoint_ideal_error: endpoint_ideal,
        gain_upper,
        endpoint_error: add_up(endpoint_ideal, end_round)?,
        end,
        end_round,
        local_error: add_up(ideal, end_round)?,
    })
}
/// Three-node (0, 1/2, 1) whole-slab Picard predictor. Independent of
/// certificate: the frozen forcing, not the stopping residual, is verified.
pub fn attempt(
    table: &RetroTable,
    model: &Model,
    y0: &[f64],
    h: f64,
    budget: f64,
    forecast: Option<&[Vec<f64>; 2]>,
    w: &mut Work,
) -> CoreResult<Slab> {
    let n = model.dimension();
    if y0.len() != n
        || !finite(y0)
        || !h.is_finite()
        || h <= 0.
        || !budget.is_finite()
        || budget <= 0.
    {
        return Err(bad("attempt input"));
    }
    cells_for(model, h)?;
    let mut g: [Vec<[f64; 4]>; 2] = std::array::from_fn(|_| Vec::with_capacity(n));
    for (a, theta) in [0.5, 1.].into_iter().enumerate() {
        for &k in &model.kappa {
            g[a].push(green_weights(table, k, h, theta, w)?.map(mid));
        }
    }
    let mut future: [Vec<f64>; 2] =
        std::array::from_fn(|a| (0..n).map(|i| g[a][i][0] * y0[i]).collect());
    if let Some(p) = forecast {
        if p.iter().any(|x| x.len() != n || !finite(x)) {
            return Err(bad("forecast shape"));
        }
        future = p.clone();
    }
    let f0 = model.source(y0, w)?;
    let stop = (0.01 * budget).max(2e-14);
    for _ in 0..24 {
        w.picard_sweeps += 1;
        let f1 = model.source(&future[0], w)?;
        let f2 = model.source(&future[1], w)?;
        let p: Vec<_> = (0..n)
            .map(|i| {
                let a = f1[i] - f0[i];
                let b = f2[i] - f0[i];
                [f0[i], 4. * a - b, 2. * b - 4. * a]
            })
            .collect();
        if p.iter().any(|x| !finite(x)) {
            return Err(limit("nonfinite predictor polynomial"));
        }
        let next: [Vec<f64>; 2] = std::array::from_fn(|a| {
            (0..n)
                .map(|i| {
                    let v = g[a][i];
                    v[0] * y0[i] + v[1] * p[i][0] + v[2] * p[i][1] + v[3] * p[i][2]
                })
                .collect()
        });
        if next.iter().any(|v| !finite(v)) {
            return Err(limit("nonfinite predictor path"));
        }
        let diff = next
            .iter()
            .zip(&future)
            .flat_map(|(a, b)| a.iter().zip(b))
            .fold(0.0_f64, |v, (&a, &b)| v.max((a - b).abs()));
        if diff <= stop {
            return certify_impl(table, model, y0, &p, h, Some(budget), w);
        }
        future = next;
    }
    Err(limit("Picard iteration cap"))
}

#[derive(Clone, Debug)]
pub struct Segment {
    start: f64,
    incoming: f64,
    slab: Slab,
}
impl Segment {
    pub fn start(&self) -> f64 {
        self.start
    }
    pub fn slab(&self) -> &Slab {
        &self.slab
    }
    pub fn evaluate(&self, table: &RetroTable, theta: f64) -> CoreResult<(Vec<f64>, f64)> {
        let (x, r) = self.slab.evaluate(table, theta)?;
        Ok((x, add_up(self.incoming, r)?))
    }
}
#[derive(Clone, Debug)]
pub struct Trajectory {
    segments: Vec<Segment>,
    end: Vec<f64>,
    error: f64,
    work: Work,
}
impl Trajectory {
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn endpoint(&self) -> &[f64] {
        &self.end
    }
    pub fn error_upper(&self) -> f64 {
        self.error
    }
    pub fn work(&self) -> &Work {
        &self.work
    }
}
/// Bounded adaptive autonomous IVP driver, hard initial condition and
/// transactional commit. The requested tolerance applies to endpoints;
/// continuous *ideal* error is bounded; dense returned values have a
/// separate per-query rounding-inclusive bound, never an unverified claim.
pub fn integrate(
    table: &RetroTable,
    model: &Model,
    y0: &[f64],
    t_end: f64,
    max_h: f64,
    tol: f64,
) -> CoreResult<Trajectory> {
    if y0.len() != model.dimension()
        || !finite(y0)
        || ![t_end, max_h, tol].iter().all(|x| x.is_finite() && *x > 0.)
    {
        return Err(bad("driver inputs"));
    }
    let mut t = 0.;
    let mut h = max_h.min(t_end);
    let mut y = y0.to_vec();
    let mut e = 0.;
    let mut work = Work::default();
    let mut segments = Vec::new();
    while t < t_end {
        if segments.len() == MAX_STEPS || work.attempts == MAX_ATTEMPTS {
            return Err(limit("driver resource cap"));
        }
        h = h.min(t_end - t);
        if h <= 0. || t + h <= t {
            return Err(limit("time resolution"));
        }
        let sum = Iv::exact_sum(t, h)?;
        if sum.lo != sum.hi || sum.hi > t_end {
            h *= 0.5;
            continue;
        }
        let budget = div_down(mul_down(tol, h)?, t_end)?;
        if budget <= 0. {
            return Err(limit("budget underflow"));
        }
        work.attempts += 1;
        let mut w = Work::default();
        let r = attempt(table, model, &y, h, budget, None, &mut w);
        work.absorb(&w);
        match r {
            Ok(s) if s.local_error <= budget => {
                let next = add_up(mul_up(s.gain_upper, e)?, s.endpoint_error)?;
                if next > tol {
                    return Err(limit("accumulated budget rounding"));
                }
                y = s.end.clone();
                let grow = s.local_error < 0.125 * budget;
                segments.push(Segment {
                    start: t,
                    incoming: e,
                    slab: s,
                });
                t = sum.lo;
                e = next;
                if grow {
                    h = (2. * h).min(max_h);
                }
            }
            Ok(_) | Err(CoreError::NonlinearSolve(_)) => {
                work.rejections += 1;
                h *= 0.5;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(Trajectory {
        segments,
        end: y,
        error: e,
        work,
    })
}
