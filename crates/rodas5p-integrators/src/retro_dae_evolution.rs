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
mod scalar_adjoint_jet;
mod schur_adjoint;
pub use scalar_adjoint_jet::{
    ScalarAdjointJet, ScalarJetWork, scalar_adjoint_jet, scalar_autonomous_source,
};
mod terminal_adjoint;
pub use schur_adjoint::{SchurAdjointWork, SchurAdjointWorkspace};
pub use terminal_adjoint::ConstantTerminalAdjoint;
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
// Degree <= 16 for four affine factors composed with native quartics.
// Preserve trailing zeros and exactly the old arithmetic order. No trimming.
const POLY_SCRATCH: usize = 17;
#[derive(Clone, Copy)]
struct ScratchPoly {
    a: [Iv; POLY_SCRATCH],
    len: usize,
}
impl ScratchPoly {
    fn new(a: &[Iv]) -> CoreResult<Self> {
        if a.len() > POLY_SCRATCH {
            return Err(bad("polynomial scratch capacity"));
        }
        let mut out = Self {
            a: [zero(); POLY_SCRATCH],
            len: a.len(),
        };
        out.a[..a.len()].copy_from_slice(a);
        Ok(out)
    }
    fn slice(&self) -> &[Iv] {
        &self.a[..self.len]
    }
    fn add(&self, b: &[Iv]) -> CoreResult<Self> {
        let mut v = Self::new(&[])?;
        v.len = self.len.max(b.len());
        if v.len > POLY_SCRATCH {
            return Err(bad("polynomial scratch capacity"));
        }
        for (i, &x) in self.slice().iter().enumerate() {
            v.a[i] = v.a[i].add(x)?;
        }
        for (i, &x) in b.iter().enumerate() {
            v.a[i] = v.a[i].add(x)?;
        }
        Ok(v)
    }
    fn scale(&self, b: Iv) -> CoreResult<Self> {
        let mut v = Self::new(&[])?;
        v.len = self.len;
        for (o, &x) in v.a.iter_mut().zip(self.slice()) {
            *o = x.mul(b)?;
        }
        Ok(v)
    }
    fn mul(&self, b: &[Iv]) -> CoreResult<Self> {
        if self.len == 0 || b.is_empty() {
            return Err(bad("empty product polynomial"));
        }
        let mut v = Self::new(&[])?;
        v.len = self.len + b.len() - 1;
        if v.len > POLY_SCRATCH {
            return Err(bad("polynomial scratch capacity"));
        }
        for (i, &x) in self.slice().iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                v.a[i + j] = v.a[i + j].add(x.mul(y)?)?;
            }
        }
        Ok(v)
    }
}
fn bernstein_at(a: &[Iv], i: usize) -> CoreResult<Iv> {
    let n = a.len() - 1;
    (0..=i).try_fold(zero(), |s, j| {
        s.add(a[j].mul(p(choose(i, j) as f64)?.div(p(choose(n, j) as f64)?)?)?)
    })
}
fn range(a: &[Iv]) -> CoreResult<Iv> {
    if a.is_empty() {
        return Err(bad("empty polynomial range"));
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for i in 0..a.len() {
        let b = bernstein_at(a, i)?;
        lo = lo.min(b.lo);
        hi = hi.max(b.hi);
    }
    Iv::new(lo, hi)
}
fn bernstein_magnitude(a: &[Iv], mean: bool) -> CoreResult<f64> {
    if a.is_empty() {
        return Err(bad("empty Bernstein magnitude"));
    }
    let mut out = 0_f64;
    for i in 0..a.len() {
        let m = bernstein_at(a, i)?.mag();
        out = if mean { add_up(out, m)? } else { out.max(m) };
    }
    if mean {
        div_up(out, a.len() as f64)
    } else {
        Ok(out)
    }
}
fn compose(e: &DeclaredPolynomial, t: &[Iv], y: &[Poly]) -> CoreResult<Poly> {
    // Low-level callers may supply longer power rows: retain owned
    // composition for those; only the admitted quartic case is bounded.
    if t.len() > 5 || y.iter().any(|v| v.len() > 5) {
        return compose_owned(e, t, y);
    }
    Ok(compose_bounded(e, t, y)?.slice().to_vec())
}
fn compose_bounded<T: AsRef<[Iv]>>(
    e: &DeclaredPolynomial,
    t: &[Iv],
    y: &[T],
) -> CoreResult<ScratchPoly> {
    let mut out = ScratchPoly::new(&[zero()])?;
    for term in e.terms() {
        let mut v = ScratchPoly::new(&[p(term.coefficient)?])?;
        for f in &term.factors {
            let mut a = ScratchPoly::new(&[p(f.constant)?])?
                .add(ScratchPoly::new(t)?.scale(p(f.time)?)?.slice())?;
            for &(i, w) in &f.state {
                a = a.add(ScratchPoly::new(y[i].as_ref())?.scale(p(w)?)?.slice())?;
            }
            v = v.mul(a.slice())?;
        }
        out = out.add(v.slice())?;
    }
    Ok(out)
}
fn compose_owned(e: &DeclaredPolynomial, t: &[Iv], y: &[Poly]) -> CoreResult<Poly> {
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
/// The residual basis changes the enclosure algorithm, never source or branch authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SlabResidualRoute {
    Bernstein,
    LegendreInterval,
    LegendreCentered,
    HybridInterval,
    HybridCentered,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SlabResidualBounds {
    pub route: SlabResidualRoute,
    pub original_g_upper: f64,
    pub preconditioned_g_upper: f64,
    pub differential_defect: f64,
    pub spectral_products: usize,
    pub dictionary_center_products: usize,
    pub linearization_entries: usize,
}
impl SlabResidualBounds {
    fn new(route: SlabResidualRoute) -> Self {
        Self {
            route,
            original_g_upper: 0.,
            preconditioned_g_upper: 0.,
            differential_defect: 0.,
            spectral_products: 0,
            dictionary_center_products: 0,
            linearization_entries: 0,
        }
    }
}
// Shifted Legendre basis L_j(u)=P_j(2u-1). Exact coefficients of u^k:
// c_k0=1/(k+1), c_k,j+1/c_kj=(2j+3)(k-j)/[(2j+1)(k+j+2)].
// No rounded coefficient is silently treated as exact.
fn power_legendre(a: &[Iv]) -> CoreResult<ScratchPoly> {
    let mut out = ScratchPoly::new(a)?;
    out.a.fill(zero());
    for (k, &v) in a.iter().enumerate() {
        let mut c = p(1.)?.div(p((k + 1) as f64)?)?;
        for j in 0..=k {
            out.a[j] = out.a[j].add(c.mul(v)?)?;
            if j < k {
                c = c
                    .scale(((2 * j + 3) * (k - j)) as f64)?
                    .div(p(((2 * j + 1) * (k + j + 2)) as f64)?)?;
            }
        }
    }
    Ok(out)
}
fn legendre_norm(a: &ScratchPoly) -> CoreResult<f64> {
    a.slice().iter().try_fold(0., |s, v| add_up(s, v.mag()))
}
fn legendre_mul(
    a: &ScratchPoly,
    b: &ScratchPoly,
    d: &Dictionary,
    centered: bool,
    w: &mut SlabResidualBounds,
) -> CoreResult<ScratchPoly> {
    if a.len == 0 || b.len == 0 || a.len > 9 || b.len > 9 {
        return Err(bad("Legendre product operand degree"));
    }
    let len = a.len + b.len - 1;
    let mut out = ScratchPoly::new(&[zero(); 17][..len])?;
    let mut ac = [0.; 9];
    let mut bc = [0.; 9];
    let mut da = [zero(); 9];
    let mut db = [zero(); 9];
    w.spectral_products += 1;
    if centered {
        for i in 0..a.len {
            ac[i] = di::mid(a.a[i]);
            da[i] = a.a[i].sub(p(ac[i])?)?;
        }
        for j in 0..b.len {
            bc[j] = di::mid(b.a[j]);
            db[j] = b.a[j].sub(p(bc[j])?)?;
        }
        out.a = d.legendre_product(&ac[..a.len], &bc[..b.len])?;
        w.dictionary_center_products += 1;
    }
    for t in d.products() {
        if t.i >= a.len || t.j >= b.len {
            continue;
        }
        let (i, j, k) = (t.i, t.j, t.k);
        if k >= len {
            return Err(bad("Legendre dictionary product degree"));
        }
        w.linearization_entries += if centered { 2 } else { 1 };
        let term = if centered {
            da[i]
                .scale(bc[j])?
                .add(db[j].scale(ac[i])?)?
                .add(da[i].mul(db[j])?)?
        } else {
            a.a[i].mul(b.a[j])?
        };
        out.a[k] = out.a[k].add(t.coefficient.enclosure.mul(term)?)?;
    }
    Ok(out)
}
fn compose_legendre(
    e: &DeclaredPolynomial,
    t: &[Iv],
    y: &[Poly],
    d: &Dictionary,
    centered: bool,
    w: &mut SlabResidualBounds,
) -> CoreResult<ScratchPoly> {
    let mut out = ScratchPoly::new(&[zero()])?;
    for term in e.terms() {
        let mut factors = [ScratchPoly::new(&[p(1.)?])?; 4];
        for (slot, f) in factors.iter_mut().zip(&term.factors) {
            let mut power = ScratchPoly::new(&[p(f.constant)?])?
                .add(ScratchPoly::new(t)?.scale(p(f.time)?)?.slice())?;
            for &(i, c) in &f.state {
                power = power.add(ScratchPoly::new(&y[i])?.scale(p(c)?)?.slice())?;
            }
            *slot = power_legendre(power.slice())?;
        }
        let v = match term.factors.len() {
            0 => ScratchPoly::new(&[p(1.)?])?,
            1 => factors[0],
            2 => legendre_mul(&factors[0], &factors[1], d, centered, w)?,
            3 => {
                let a = legendre_mul(&factors[0], &factors[1], d, centered, w)?;
                legendre_mul(&a, &factors[2], d, centered, w)?
            }
            4 => {
                let a = legendre_mul(&factors[0], &factors[1], d, centered, w)?;
                let b = legendre_mul(&factors[2], &factors[3], d, centered, w)?;
                legendre_mul(&a, &b, d, centered, w)?
            }
            _ => return Err(bad("Legendre source degree")),
        };
        out = out.add(v.scale(p(term.coefficient)?)?.slice())?;
    }
    Ok(out)
}
/// Full normalized-slab original polynomial residual bounds, including
/// original C*g and x_u - h*f. Only bounded native degree<=4 input.
/// Success has no heap requests once the shared dictionary is initialized.
/// A residual bound is not a complete physical slab certificate.
pub fn slab_residual_bounds(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    c: &[f64],
    route: SlabResidualRoute,
) -> CoreResult<SlabResidualBounds> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let nx = src.nx();
    let nz = src.nz();
    if y.len() != nx + nz
        || y.iter().any(|r| r.is_empty() || r.len() > 5)
        || c.len() != nz * nz
        || !c.iter().all(|x| x.is_finite())
    {
        return Err(bad("bounded slab residual shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo <= 0. {
        return Err(bad("clock"));
    }
    let clock = [p(times[0])?, h];
    let hybrid = matches!(
        route,
        SlabResidualRoute::HybridCentered | SlabResidualRoute::HybridInterval
    );
    let spectral = route != SlabResidualRoute::Bernstein;
    let centered = matches!(
        route,
        SlabResidualRoute::LegendreCentered | SlabResidualRoute::HybridCentered
    );
    let mut w = SlabResidualBounds::new(route);
    let modes = if hybrid { 2 } else { 1 };
    for k in 0..modes {
        let leg = spectral && k == 0;
        let d = if leg { Some(Dictionary::get()?) } else { None };
        let mut g = [ScratchPoly::new(&[zero()])?; 32];
        let mut gu = 0_f64;
        let mut cgu = 0_f64;
        let mut dx = 0_f64;
        let norm = |a: &ScratchPoly| {
            if leg {
                legendre_norm(a)
            } else {
                range(a.slice()).map(|v| v.mag())
            }
        };
        for (o, eq) in g[..nz].iter_mut().zip(&src.g) {
            *o = if let Some(d) = d {
                compose_legendre(eq, &clock, y, d, centered, &mut w)?
            } else {
                compose_bounded(eq, &clock, y)?
            };
            gu = gu.max(norm(o)?);
        }
        for i in 0..nz {
            let mut v = ScratchPoly::new(&[zero()])?;
            for (a, row) in c[i * nz..(i + 1) * nz].iter().zip(&g[..nz]) {
                v = v.add(row.scale(p(*a)?)?.slice())?;
            }
            cgu = cgu.max(norm(&v)?);
        }
        for (i, eq) in src.f.iter().enumerate() {
            let mut du = ScratchPoly::new(&[])?;
            du.len = y[i].len() - 1;
            for (j, &v) in y[i].iter().enumerate().skip(1) {
                du.a[j - 1] = v.scale(j as f64)?;
            }
            let f = if let Some(d) = d {
                du = power_legendre(du.slice())?;
                compose_legendre(eq, &clock, y, d, centered, &mut w)?
            } else {
                compose_bounded(eq, &clock, y)?
            };
            let r = du.add(f.scale(-h)?.slice())?;
            dx = dx.max(norm(&r)?);
        }
        if k == 0 {
            w.original_g_upper = gu;
            w.preconditioned_g_upper = cgu;
            w.differential_defect = dx;
        } else {
            w.original_g_upper = w.original_g_upper.min(gu);
            w.preconditioned_g_upper = w.preconditioned_g_upper.min(cgu);
            w.differential_defect = w.differential_defect.min(dx);
        }
    }
    Ok(w)
}
fn pol_norm(p: &[Poly]) -> CoreResult<f64> {
    p.iter()
        .map(|r| range(r).map(|v| v.mag()))
        .try_fold(0.0_f64, |m, v| Ok(m.max(v?)))
}
fn c_polys(c: &DenseMatrix, v: &[Poly]) -> CoreResult<Vec<Poly>> {
    (0..c.nrows())
        .map(|i| {
            if v.iter().all(|r| r.len() <= POLY_SCRATCH) {
                let mut out = ScratchPoly::new(&[zero()])?;
                for (&a, r) in c.row(i).iter().zip(v) {
                    out = out.add(ScratchPoly::new(r)?.scale(p(a)?)?.slice())?;
                }
                Ok(out.slice().to_vec())
            } else {
                c.row(i)
                    .iter()
                    .zip(v)
                    .try_fold(vec![zero()], |s, (&a, v)| add(&s, &scale(v, p(a)?)?))
            }
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
/// Same represented power coefficients without owning native stage rows.
pub fn dense_power_rows(
    y0: &[f64],
    y1: &[f64],
    stages: &[&[f64]],
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
/// Directed native coefficients into preallocated rows. Compute all
/// coefficients in bounded scratch before mutating caller output.
pub fn dense_power_rows_into(
    y0: &[f64],
    y1: &[f64],
    stages: &[&[f64]],
    basis: DenseBasis,
    out: &mut [Poly],
) -> CoreResult<()> {
    let n = y0.len();
    if n == 0
        || n > 32
        || y1.len() != n
        || stages.len() != 8
        || stages.iter().any(|v| v.len() != n)
        || out.len() != n
        || out.iter().any(|r| r.len() != 5)
    {
        return Err(bad("dense packet output shape"));
    }
    let h = basis.coefficients()?;
    let mut scratch = [[zero(); 5]; 32];
    for i in 0..n {
        let mut d = [zero(); 3];
        for (k, v) in d.iter_mut().enumerate() {
            for (j, u) in stages.iter().enumerate() {
                *v = v.add(p(h[(k, j)])?.scale(u[i])?)?;
            }
        }
        let a = p(y0[i])?;
        let b = p(y1[i])?;
        scratch[i] = [
            a,
            b.sub(a)?.add(d[0])?,
            d[1].sub(d[0])?,
            d[2].sub(d[1])?,
            -d[2],
        ];
    }
    for (o, v) in out.iter_mut().zip(&scratch[..n]) {
        o.copy_from_slice(v);
    }
    Ok(())
}
/// Actual fast native component adapter into already allocated rows.
pub fn dense_power_components_into(
    s: &crate::FastDenseSegments,
    k: usize,
    out: &mut [Poly],
) -> CoreResult<()> {
    let n = s.dimension();
    if n == 0 || n > 32 || k >= s.len() || out.len() != n || out.iter().any(|r| r.len() != 5) {
        return Err(bad("dense component output shape"));
    }
    let mut scratch = [[zero(); 5]; 32];
    for (i, v) in scratch[..n].iter_mut().enumerate() {
        *v = crate::retro_physical_event::power(&s.represented_component(k, i)?)?;
    }
    for (o, v) in out.iter_mut().zip(&scratch[..n]) {
        o.copy_from_slice(v);
    }
    Ok(())
}
/// Reuse the existing original physical-event component adapter and
/// directed coefficient kernel on the actual fast native store.
pub fn dense_power_components(s: &crate::FastDenseSegments, k: usize) -> CoreResult<Vec<Poly>> {
    (0..s.dimension())
        .map(|i| {
            crate::retro_physical_event::power(&s.represented_component(k, i)?).map(|p| p.to_vec())
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
    project_recorded(src, t, x, seed, branch, policy, route, warp, None)
}
/// A projection boundary result with initialization work even on refusal.
#[allow(clippy::too_many_arguments)]
pub fn project_audited(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
) -> (CoreResult<InitReport>, di::InitAttemptWork) {
    let mut a = di::InitAttemptWork::default();
    let r = project_recorded(src, t, x, seed, branch, policy, route, warp, Some(&mut a));
    (r, a)
}
#[allow(clippy::too_many_arguments)]
fn project_recorded(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
    audit: Option<&mut di::InitAttemptWork>,
) -> CoreResult<InitReport> {
    if branch
        .iter()
        .any(|b| !b[0].is_finite() || !b[1].is_finite() || b[0] >= b[1])
    {
        return Err(bad("branch shape"));
    }
    let mut scratch = [0.; 32];
    let mut predicted;
    let seed = if let Some(w) = warp {
        predicted = w.seed(src, t, x)?;
        predicted.as_mut_slice()
    } else {
        if seed.len() > 32 {
            return Err(bad("seed shape"));
        }
        scratch[..seed.len()].copy_from_slice(seed);
        &mut scratch[..seed.len()]
    };
    if seed.len() != branch.len() {
        return Err(bad("seed shape"));
    }
    for (v, b) in seed.iter_mut().zip(branch) {
        *v = v.clamp(b[0], b[1]);
    }
    let policy = InitPolicy {
        mode: InitMode::ProjectAlgebraic,
        ..policy
    };
    if let Some(a) = audit {
        let (r, w) = src.initialize_audited(t, x, seed, branch, policy, route);
        *a = w;
        r
    } else {
        src.initialize(t, x, seed, branch, policy, route)
    }
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
    reconstruct_recorded(
        src, times, x, seed, branch, policy, route, warp, nodes, None,
    )
}
/// Actual completed and failed node projections are retained on Err.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_audited(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
    nodes: usize,
) -> (
    CoreResult<(Vec<Poly>, Vec<InitWork>)>,
    di::ProjectionAttemptWork,
) {
    let mut w = di::ProjectionAttemptWork::default();
    let r = reconstruct_recorded(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        route,
        warp,
        nodes,
        Some(&mut w),
    );
    (r, w)
}
/// Seed choices for the exact same scalar Reuse projection at dictionary nodes.
/// Continuation is opt-in; a failed node is never silently retried.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ReconstructionSeedRoute {
    Fixed,
    PreviousNode,
}
pub struct ScalarReconstructionWorkspace {
    power: [Iv; 7],
    work: [InitWork; 7],
}
impl ScalarReconstructionWorkspace {
    pub fn new() -> Self {
        Self {
            power: [zero(); 7],
            work: std::array::from_fn(|_| InitWork::default()),
        }
    }
}
impl Default for ScalarReconstructionWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(clippy::too_many_arguments)]
fn reconstruct_scalar_inner(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    nodes: usize,
    seed_route: ReconstructionSeedRoute,
    route: InitRoute,
    mut audit: Option<&mut di::ProjectionAttemptWork>,
    scratch: &mut ScalarReconstructionWorkspace,
) -> CoreResult<()> {
    let b = Dictionary::get()?.basis(nodes)?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    if x.len() != src.nx() || src.nz() != 1 {
        return Err(bad("reconstruction differential shape"));
    }
    let mut v = [0.; 32];
    scratch.power[..nodes].fill(zero());
    let mut last = None;
    for (j, node) in b.nodes.iter().enumerate() {
        let t = di::mid(p(times[0])?.add(h.scale(node.value)?)?);
        for (v, a) in v[..x.len()].iter_mut().zip(x) {
            *v = di::mid(eval(a, p(node.value)?)?);
        }
        let (r, w) = if branch.len() != 1
            || !branch[0][0].is_finite()
            || !branch[0][1].is_finite()
            || branch[0][0] >= branch[0][1]
        {
            (Err(bad("branch shape")), di::InitAttemptWork::default())
        } else if seed.len() != 1 {
            (Err(bad("seed shape")), di::InitAttemptWork::default())
        } else {
            let z = if seed_route == ReconstructionSeedRoute::PreviousNode {
                last.unwrap_or(seed[0])
            } else {
                seed[0]
            };
            let guess = [z.clamp(branch[0][0], branch[0][1])];
            let policy = InitPolicy {
                mode: InitMode::ProjectAlgebraic,
                ..policy
            };
            if matches!(
                route,
                InitRoute::BoundedSturmScalar | InitRoute::BoundedSturmNewtonScalar
            ) {
                src.initialize_scalar_bounded_sturm_audited(
                    t,
                    &v[..x.len()],
                    &guess,
                    branch,
                    policy,
                    route == InitRoute::BoundedSturmNewtonScalar,
                )
            } else if route == InitRoute::BoundedAffineScalar {
                src.initialize_scalar_bounded_affine_audited(
                    t,
                    &v[..x.len()],
                    &guess,
                    branch,
                    policy,
                )
            } else if route == InitRoute::BoundedPreparedScalar {
                src.initialize_scalar_bounded_prepared_audited(
                    t,
                    &v[..x.len()],
                    &guess,
                    branch,
                    policy,
                )
            } else {
                src.initialize_scalar_reuse_audited(t, &v[..x.len()], &guess, branch, policy)
            }
        };
        if let Some(a) = audit.as_deref_mut() {
            a.record(&w, r.is_ok());
        }
        let r = r?;
        scratch.work[j] = r.work().clone();
        last = Some(r.z());
        for (k, a) in scratch.power[..nodes].iter_mut().enumerate() {
            *a = a.add(b.monomial[j][k].enclosure.scale(r.z())?)?;
        }
    }
    for v in &mut scratch.power[..nodes] {
        *v = p(di::mid(*v))?;
    }
    Ok(())
}
/// Caller output committed only after every original-source node projection
/// and coefficient operation succeeds. Audit keeps failed work. Native
/// bounded scalar Reuse only; this is a candidate, not a slab certificate.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_scalar_reuse_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    seed_route: ReconstructionSeedRoute,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    scratch: &mut ScalarReconstructionWorkspace,
) -> CoreResult<()> {
    if out.len() != work.len() {
        return Err(bad("scalar reconstruction output shape"));
    }
    let nodes = out.len();
    reconstruct_scalar_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        seed_route,
        InitRoute::Reuse,
        Some(audit),
        scratch,
    )?;
    out.copy_from_slice(&scratch.power[..nodes]);
    work.clone_from_slice(&scratch.work[..nodes]);
    Ok(())
}
/// Bounded prepared scalar projection at each dictionary node, retaining
/// the same transactional output boundary and all original-source receipts.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_scalar_bounded_prepared_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    seed_route: ReconstructionSeedRoute,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    scratch: &mut ScalarReconstructionWorkspace,
) -> CoreResult<()> {
    if out.len() != work.len() {
        return Err(bad("scalar reconstruction output shape"));
    }
    let nodes = out.len();
    reconstruct_scalar_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        seed_route,
        InitRoute::BoundedPreparedScalar,
        Some(audit),
        scratch,
    )?;
    out.copy_from_slice(&scratch.power[..nodes]);
    work.clone_from_slice(&scratch.work[..nodes]);
    Ok(())
}
/// Caller-owned bounded affine reconstruction, with no hidden root fallback.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_scalar_bounded_affine_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    seed_route: ReconstructionSeedRoute,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    scratch: &mut ScalarReconstructionWorkspace,
) -> CoreResult<()> {
    if out.len() != work.len() {
        return Err(bad("scalar reconstruction output shape"));
    }
    let nodes = out.len();
    reconstruct_scalar_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        seed_route,
        InitRoute::BoundedAffineScalar,
        Some(audit),
        scratch,
    )?;
    out.copy_from_slice(&scratch.power[..nodes]);
    work.clone_from_slice(&scratch.work[..nodes]);
    Ok(())
}
/// Caller-owned certified nonlinear scalar root reconstruction.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_scalar_bounded_sturm_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    seed_route: ReconstructionSeedRoute,
    newton: bool,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    scratch: &mut ScalarReconstructionWorkspace,
) -> CoreResult<()> {
    if out.len() != work.len() {
        return Err(bad("scalar reconstruction output shape"));
    }
    let nodes = out.len();
    reconstruct_scalar_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        seed_route,
        if newton {
            InitRoute::BoundedSturmNewtonScalar
        } else {
            InitRoute::BoundedSturmScalar
        },
        Some(audit),
        scratch,
    )?;
    out.copy_from_slice(&scratch.power[..nodes]);
    work.clone_from_slice(&scratch.work[..nodes]);
    Ok(())
}
/// Caller-owned vector dictionary reconstruction; fixed scalar workspace
/// remains separate. Every node retains original vector projection proof.
pub struct VectorReconstructionWorkspace {
    power: [[Iv; 7]; 32],
    work: [InitWork; 7],
    projection: di::VectorProjectionWorkspace,
}
impl VectorReconstructionWorkspace {
    pub fn new() -> Self {
        Self {
            power: [[zero(); 7]; 32],
            work: std::array::from_fn(|_| InitWork::default()),
            projection: di::VectorProjectionWorkspace::new(),
        }
    }
}
impl Default for VectorReconstructionWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(clippy::too_many_arguments)]
fn reconstruct_vector_inner(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    nodes: usize,
    route: InitRoute,
    seed_route: ReconstructionSeedRoute,
    mut audit: Option<&mut di::ProjectionAttemptWork>,
    w: &mut VectorReconstructionWorkspace,
) -> CoreResult<()> {
    let b = Dictionary::get()?.basis(nodes)?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    let nx = src.nx();
    let nz = src.nz();
    if x.len() != nx
        || seed.len() != nz
        || branch.len() != nz
        || branch
            .iter()
            .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] >= v[1])
    {
        return Err(bad("vector reconstruction shape"));
    }
    let mut guess = [0.; 32];
    for i in 0..nz {
        guess[i] = seed[i].clamp(branch[i][0], branch[i][1]);
        w.power[i][..nodes].fill(zero());
    }
    let mut v = [0.; 32];
    for (j, node) in b.nodes.iter().enumerate() {
        let t = di::mid(p(times[0])?.add(h.scale(node.value)?)?);
        for (v, a) in v[..nx].iter_mut().zip(x) {
            *v = di::mid(eval(a, p(node.value)?)?);
        }
        let policy = InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            ..policy
        };
        let (r, receipt) = if matches!(
            route,
            InitRoute::BoundedVectorPivot | InitRoute::BoundedVectorEquilibrated
        ) {
            src.initialize_pivoted_vector_audited(
                t,
                &v[..nx],
                &guess[..nz],
                branch,
                policy,
                route == InitRoute::BoundedVectorEquilibrated,
                &mut w.projection,
            )
        } else {
            src.initialize_bounded_vector_audited(
                t,
                &v[..nx],
                &guess[..nz],
                branch,
                policy,
                route == InitRoute::BoundedVectorRefined,
                &mut w.projection,
            )
        };
        if let Some(a) = audit.as_deref_mut() {
            a.record(&receipt, r.is_ok());
        }
        let r = r?;
        w.work[j] = r.work().clone();
        for i in 0..nz {
            if seed_route == ReconstructionSeedRoute::PreviousNode {
                guess[i] = r.z()[i];
            }
            for (k, v) in w.power[i][..nodes].iter_mut().enumerate() {
                *v = v.add(b.monomial[j][k].enclosure.scale(r.z()[i])?)?;
            }
        }
    }
    for row in &mut w.power[..nz] {
        for v in &mut row[..nodes] {
            *v = p(di::mid(*v))?;
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_bounded_vector_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    refine: bool,
    seed_route: ReconstructionSeedRoute,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    w: &mut VectorReconstructionWorkspace,
) -> CoreResult<()> {
    let nodes = work.len();
    if out.len() != src.nz() * nodes {
        return Err(bad("vector reconstruction output shape"));
    }
    reconstruct_vector_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        if refine {
            InitRoute::BoundedVectorRefined
        } else {
            InitRoute::BoundedVector
        },
        seed_route,
        Some(audit),
        w,
    )?;
    for (o, v) in out.chunks_exact_mut(nodes).zip(&w.power[..src.nz()]) {
        o.copy_from_slice(&v[..nodes]);
    }
    work.clone_from_slice(&w.work[..nodes]);
    Ok(())
}
/// Caller-owned nondominant vector dictionary reconstruction.
#[allow(clippy::too_many_arguments)]
pub fn reconstruct_pivoted_vector_into(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    equilibrate: bool,
    seed_route: ReconstructionSeedRoute,
    out: &mut [Iv],
    work: &mut [InitWork],
    audit: &mut di::ProjectionAttemptWork,
    w: &mut VectorReconstructionWorkspace,
) -> CoreResult<()> {
    let nodes = work.len();
    if out.len() != src.nz() * nodes {
        return Err(bad("vector reconstruction output shape"));
    }
    reconstruct_vector_inner(
        src,
        times,
        x,
        seed,
        branch,
        policy,
        nodes,
        if equilibrate {
            InitRoute::BoundedVectorEquilibrated
        } else {
            InitRoute::BoundedVectorPivot
        },
        seed_route,
        Some(audit),
        w,
    )?;
    for (o, v) in out.chunks_exact_mut(nodes).zip(&w.power[..src.nz()]) {
        o.copy_from_slice(&v[..nodes]);
    }
    work.clone_from_slice(&w.work[..nodes]);
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn reconstruct_recorded(
    src: &DaeSource,
    times: [f64; 2],
    x: &[Poly],
    seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    warp: Option<&WarpSeed>,
    nodes: usize,
    mut audit: Option<&mut di::ProjectionAttemptWork>,
) -> CoreResult<(Vec<Poly>, Vec<InitWork>)> {
    if matches!(
        route,
        InitRoute::BoundedVector
            | InitRoute::BoundedVectorRefined
            | InitRoute::BoundedVectorPivot
            | InitRoute::BoundedVectorEquilibrated
    ) && warp.is_none()
    {
        let mut w = VectorReconstructionWorkspace::new();
        reconstruct_vector_inner(
            src,
            times,
            x,
            seed,
            branch,
            policy,
            nodes,
            route,
            ReconstructionSeedRoute::Fixed,
            audit,
            &mut w,
        )?;
        return Ok((
            w.power[..src.nz()]
                .iter()
                .map(|r| r[..nodes].to_vec())
                .collect(),
            w.work[..nodes].to_vec(),
        ));
    }
    if src.nz() == 1
        && matches!(
            route,
            InitRoute::Reuse
                | InitRoute::BoundedPreparedScalar
                | InitRoute::BoundedAffineScalar
                | InitRoute::BoundedSturmScalar
                | InitRoute::BoundedSturmNewtonScalar
        )
        && warp.is_none()
    {
        let mut scratch = ScalarReconstructionWorkspace::new();
        reconstruct_scalar_inner(
            src,
            times,
            x,
            seed,
            branch,
            policy,
            nodes,
            ReconstructionSeedRoute::Fixed,
            route,
            audit,
            &mut scratch,
        )?;
        return Ok((
            vec![scratch.power[..nodes].to_vec()],
            scratch.work[..nodes].to_vec(),
        ));
    }
    let b = Dictionary::get()?.basis(nodes)?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    let mut out = vec![vec![zero(); nodes]; src.nz()];
    if x.len() != src.nx() {
        return Err(bad("reconstruction differential shape"));
    }
    let mut work = Vec::with_capacity(nodes);
    let mut v = [0.; 32];
    for (j, node) in b.nodes.iter().enumerate() {
        let t = di::mid(p(times[0])?.add(h.scale(node.value)?)?);
        for (v, a) in v[..x.len()].iter_mut().zip(x) {
            *v = di::mid(eval(a, p(node.value)?)?);
        }
        let r = if let Some(a) = audit.as_deref_mut() {
            let (r, w) = project_audited(src, t, &v[..x.len()], seed, branch, policy, route, warp);
            a.record(&w, r.is_ok());
            r
        } else {
            project(src, t, &v[..x.len()], seed, branch, policy, route, warp)
        }?;
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
/// Endpoint-only source linear transport: no replacement for the physical tube.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PicardEndpointRoute {
    OrderedWords,
    Polynomial3,
    Polynomial5,
    Polynomial7,
    Adaptive7,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PicardEndpointBounds {
    pub route: PicardEndpointRoute,
    pub candidate_error: f64,
    pub majorant: f64,
    pub tail_upper: f64,
    pub coefficient_perturbation: f64,
    pub initial_error: f64,
    pub iterations: usize,
    pub matrix_actions: usize,
    pub scalar_products: usize,
}
// Extract exactly the original declared differential source, with normalized
// clock and augmented constant component. Reject nonlinear state dependence,
// z dependence, and unsupported clock degree (no numerical fitting).
fn linear_source_matrices(
    src: &DaeSource,
    times: [f64; 2],
) -> CoreResult<([[Iv; 32 * 32]; 3], usize)> {
    let nx = src.nx();
    let n = nx + 1;
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo <= 0. {
        return Err(bad("Picard clock"));
    }
    let mut a = [[zero(); 32 * 32]; 3];
    for (i, eq) in src.f.iter().enumerate() {
        for term in eq.terms() {
            let mut poly = ScratchPoly::new(&[p(term.coefficient)?])?;
            let mut state_factor = None;
            for f in &term.factors {
                if f.state.iter().any(|&(k, w)| k >= nx && w != 0.) {
                    return Err(bad("Picard source depends on algebraic variable"));
                }
                if f.state.iter().any(|&(_, w)| w != 0.) {
                    if state_factor.is_some() {
                        return Err(bad("Picard nonlinear state product"));
                    }
                    state_factor = Some(f);
                } else {
                    let factor = [
                        p(f.constant)?.add(p(times[0])?.scale(f.time)?)?,
                        h.scale(f.time)?,
                    ];
                    poly = poly.mul(&factor)?;
                }
            }
            for j in 0..n {
                let v = if let Some(f) = state_factor {
                    if j == nx {
                        poly.mul(&[
                            p(f.constant)?.add(p(times[0])?.scale(f.time)?)?,
                            h.scale(f.time)?,
                        ])?
                    } else {
                        let c = f
                            .state
                            .iter()
                            .filter(|&&(k, _)| k == j)
                            .try_fold(zero(), |s, &(_, w)| s.add(p(w)?))?;
                        poly.scale(c)?
                    }
                } else if j == nx {
                    poly
                } else {
                    continue;
                };
                if v.slice().iter().skip(3).any(|v| v.lo != 0. || v.hi != 0.) {
                    return Err(bad("Picard normalized clock degree exceeds two"));
                }
                for (k, &v) in v.slice().iter().take(3).enumerate() {
                    a[k][i * n + j] = a[k][i * n + j].add(v.mul(h)?)?;
                }
            }
        }
    }
    Ok((a, n))
}
fn picard_majorant(a: &[[Iv; 32 * 32]; 3], n: usize) -> CoreResult<f64> {
    let mut m = 0.;
    for (k, ak) in a.iter().enumerate() {
        let mut r = 0_f64;
        for row in ak[..n * n].chunks_exact(n) {
            r = r.max(row.iter().try_fold(0., |s, v| add_up(s, v.mag()))?);
        }
        m = add_up(m, div_up(r, (k + 1) as f64)?)?;
    }
    Ok(m)
}
fn picard_tail(norm: f64, m: f64, order: usize, gain: f64) -> CoreResult<f64> {
    let mut t = norm;
    for k in 1..=order + 1 {
        t = div_up(mul_up(t, m)?, k as f64)?;
    }
    mul_up(t, gain)
}
/// Independent endpoint enclosure from the original linear differential
/// source. The caller still needs the original branch and tube certificate.
/// Successful polynomial variants use fixed storage, not a heap word tree.
pub fn source_picard_endpoint(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    incoming: f64,
    route: PicardEndpointRoute,
) -> CoreResult<PicardEndpointBounds> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let nx = src.nx();
    if y.len() != nx + src.nz()
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !incoming.is_finite()
        || incoming < 0.
    {
        return Err(bad("Picard endpoint shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let (a, n) = linear_source_matrices(src, times)?;
    let m = picard_majorant(&a, n)?;
    if m > 8. {
        return Err(bad("Picard majorant too large"));
    }
    let gain = exp_interval(m)?.hi;
    let mut initial = [0.; 32];
    initial[nx] = 1.;
    let mut left_error = 0_f64;
    for (i, v) in y[..nx].iter().enumerate() {
        initial[i] = di::mid(v[0]);
        left_error = left_error.max(v[0].sub(p(initial[i])?)?.mag());
    }
    let norm = initial[..n].iter().fold(0_f64, |s, v| s.max(v.abs()));
    let initial_error = mul_up(gain, add_up(incoming, left_error)?)?;
    let mut out = PicardEndpointBounds {
        route,
        candidate_error: 0.,
        majorant: m,
        tail_upper: 0.,
        coefficient_perturbation: 0.,
        initial_error,
        iterations: 0,
        matrix_actions: 0,
        scalar_products: 0,
    };
    let mut end = [zero(); 32];
    let mut residual_radius = 0.;
    if route == PicardEndpointRoute::OrderedWords {
        let point = a
            .each_ref()
            .map(|a| DenseMatrix::new(n, n, a[..n * n].iter().copied().map(di::mid).collect()));
        let [a0, a1, a2] = point;
        let matrices = [a0?, a1?, a2?];
        let mut delta = 0.;
        for (k, ak) in a.iter().enumerate() {
            let mut r = 0_f64;
            for i in 0..n {
                let mut s = 0.;
                for j in 0..n {
                    s = add_up(s, ak[i * n + j].sub(p(matrices[k][(i, j)])?)?.mag())?;
                }
                r = r.max(s);
            }
            delta = add_up(delta, div_up(r, (k + 1) as f64)?)?;
        }
        // Both exact and midpoint generators share the same pointwise
        // majorant sum ||A_k|| u^k. Duhamel: exp(M)*||Y0||*int||Delta A||.
        out.coefficient_perturbation = mul_up(gain, mul_up(norm, delta)?)?;
        let r = crate::retro_foundation::ordered_picard(&matrices, &initial[..n])?;
        out.iterations = 5;
        out.matrix_actions = r.words;
        out.scalar_products = r.words * n * n;
        out.tail_upper = r.tail_upper;
        residual_radius = add_up(r.error_upper, out.coefficient_perturbation)?;
        for (v, &x) in end[..n].iter_mut().zip(&r.values) {
            *v = p(x)?;
        }
    } else {
        let cap = match route {
            PicardEndpointRoute::Polynomial3 => 3,
            PicardEndpointRoute::Polynomial5 => 5,
            _ => 7,
        };
        let mut prev = [[zero(); 32]; 22];
        let mut next = [[zero(); 32]; 22];
        for i in 0..n {
            prev[0][i] = p(initial[i])?;
            end[i] = prev[0][i];
        }
        let mut degree = 0;
        let mut terminated = false;
        for order in 1..=cap {
            let nd = degree + 3;
            next[..=nd].fill([zero(); 32]);
            for (k, v) in prev[..=degree].iter().enumerate() {
                if v[..n].iter().all(|x| x.lo == 0. && x.hi == 0.) {
                    continue;
                }
                for (j, aj) in a.iter().enumerate() {
                    if aj[..n * n].iter().all(|x| x.lo == 0. && x.hi == 0.) {
                        continue;
                    }
                    out.matrix_actions += 1;
                    for i in 0..n {
                        let mut s = zero();
                        for l in 0..n {
                            let c = aj[i * n + l];
                            let x = v[l];
                            if (c.lo == 0. && c.hi == 0.) || (x.lo == 0. && x.hi == 0.) {
                                continue;
                            }
                            s = s.add(c.mul(x)?)?;
                            out.scalar_products += 1;
                        }
                        next[k + j + 1][i] = next[k + j + 1][i].add(s)?;
                    }
                }
            }
            for (k, v) in next[..=nd].iter_mut().enumerate().skip(1) {
                for i in 0..n {
                    v[i] = v[i].div(p(k as f64)?)?;
                    end[i] = end[i].add(v[i])?;
                }
            }
            out.iterations = order;
            if route == PicardEndpointRoute::Adaptive7
                && next[..=nd]
                    .iter()
                    .all(|v| v[..n].iter().all(|x| x.lo == 0. && x.hi == 0.))
            {
                terminated = true;
                break;
            }
            std::mem::swap(&mut prev, &mut next);
            degree = nd;
        }
        out.tail_upper = if terminated {
            0.
        } else {
            picard_tail(norm, m, cap, gain)?
        };
        residual_radius = out.tail_upper;
    }
    for (v, row) in end[..nx].iter().zip(&y[..nx]) {
        out.candidate_error = out.candidate_error.max(v.sub(eval(row, p(1.)?)?)?.mag());
    }
    out.candidate_error = add_up(out.candidate_error, add_up(initial_error, residual_radius)?)?;
    Ok(out)
}
/// Re-certify the original slab, then tighten ONLY the endpoint transport
/// radius. Full tube and algebraic certificates remain untouched.
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_with_picard(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    route: PicardEndpointRoute,
) -> CoreResult<SlabCertificate> {
    let mut c = certify_slab(src, times, y, branch, tube, incoming, green, condition_cap)?;
    if c.global_qualified {
        let r = source_picard_endpoint(src, times, y, c.incoming, route)?;
        c.outgoing = Some(
            c.outgoing
                .ok_or_else(|| bad("Picard original outgoing"))?
                .min(r.candidate_error),
        );
        c.picard_endpoint = Some(r);
    }
    Ok(c)
}
/// Linear scalar homogeneous IVP proposal, independently checked against
/// the original dissipative source. Archived quadrature is proposal only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LinearLsEndpointRoute {
    NormalEquations,
    BoundedNormal,
    QrProposal,
    RationalResponse,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct LinearLsEndpointBounds {
    pub route: LinearLsEndpointRoute,
    pub order: usize,
    pub rate: [f64; 2],
    pub candidate_error: f64,
    pub response: [f64; 2],
    pub ivp_residual: Option<f64>,
    pub normal_residual: Option<f64>,
    pub coefficient_error: Option<f64>,
    pub dictionary_gap: Option<f64>,
    pub initial_error: f64,
    pub analytic_error: Option<f64>,
}
fn bounded_ls_normal(
    l: &rodas5p_core::retro_dictionary::Legendre,
    z: f64,
    n: usize,
) -> CoreResult<[f64; 6]> {
    let mut a = [[0.; 6]; 6];
    let mut b = [0.; 6];
    for i in 0..n {
        b[i] = z * l.d[i].value - z * z * l.v[i].value;
        for j in 0..n {
            a[i][j] =
                l.g[i][j].value - z * (l.c[i][j].value + l.c[j][i].value) + z * z * l.m[i][j].value;
        }
    }
    // Pivoted elimination only proposes coefficients. No stability claim
    // from the normal equations; the independent residual decides validity.
    for k in 0..n {
        let mut pivot = k;
        for i in k + 1..n {
            if a[i][k].abs() > a[pivot][k].abs() {
                pivot = i;
            }
        }
        a.swap(k, pivot);
        b.swap(k, pivot);
        let d = a[k][k];
        if d == 0. || !d.is_finite() {
            return Err(bad("bounded normal pivot"));
        }
        for i in k + 1..n {
            let q = a[i][k] / d;
            for j in k + 1..n {
                a[i][j] -= q * a[k][j];
            }
            b[i] -= q * b[k];
        }
    }
    for i in (0..n).rev() {
        for j in i + 1..n {
            b[i] -= a[i][j] * b[j];
        }
        b[i] /= a[i][i];
    }
    if b[..n].iter().any(|v| !v.is_finite()) {
        return Err(bad("bounded normal nonfinite"));
    }
    Ok(b)
}
/// The LS polynomial q(0)=1 is constructed from dictionary integrals.
/// Normal residual certifies distance to the exact LS minimizer, NOT
/// distance to the IVP. A second, original ODE residual certifies the IVP.
pub fn source_linear_ls_endpoint(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    incoming: f64,
    order: usize,
    route: LinearLsEndpointRoute,
) -> CoreResult<LinearLsEndpointBounds> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    if src.nx() != 1
        || y.len() != 1 + src.nz()
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !incoming.is_finite()
        || incoming < 0.
    {
        return Err(bad("linear LS shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let (a, n) = linear_source_matrices(src, times)?;
    if n != 2
        || a[0][0].hi > 0.
        || a[0][1..4]
            .iter()
            .chain(&a[1][..4])
            .chain(&a[2][..4])
            .any(|v| v.lo != 0. || v.hi != 0.)
    {
        return Err(bad(
            "linear LS requires homogeneous constant nonpositive rate",
        ));
    }
    let rate = a[0][0];
    let zm = di::mid(rate);
    let d = Dictionary::get()?;
    let l = d.legendre(order)?;
    let exact_response = l.endpoint_response(zm)?;
    let initial = di::mid(y[0][0]);
    let gain = exp_interval(rate.hi)?.hi;
    let initial_error = mul_up(gain, add_up(incoming, y[0][0].sub(p(initial)?)?.mag())?)?;
    let right = eval(&y[0], p(1.)?)?;
    let mut out = LinearLsEndpointBounds {
        route,
        order,
        rate: rate.as_pair(),
        candidate_error: 0.,
        response: exact_response.as_pair(),
        ivp_residual: None,
        normal_residual: None,
        coefficient_error: None,
        dictionary_gap: None,
        initial_error,
        analytic_error: None,
    };
    if route == LinearLsEndpointRoute::RationalResponse {
        // Independent exact scalar flow comparator. The dictionary rational
        // response is an LS approximation, not the true exponential.
        let exact = Iv::new(exp_interval(rate.lo)?.lo, gain)?;
        let e = exact_response.sub(exact)?.mag();
        out.analytic_error = Some(e);
        out.candidate_error = add_up(
            exact_response.scale(initial)?.sub(right)?.mag(),
            add_up(mul_up(initial.abs(), e)?, initial_error)?,
        )?;
        return Ok(out);
    }
    let mut coeff = [0.; 6];
    match route {
        LinearLsEndpointRoute::NormalEquations => {
            let (a, b) = l.normal_equations(zm)?;
            let x = rodas5p_core::direct_solve(&a, &b)?;
            coeff[..order].copy_from_slice(&x);
        }
        LinearLsEndpointRoute::BoundedNormal => coeff = bounded_ls_normal(l, zm, order)?,
        LinearLsEndpointRoute::QrProposal => {
            let m = l.gauss_nodes.len();
            let mut a = DenseMatrix::zeros(m, order);
            let mut b = vec![0.; m];
            for (i, (&u, &weight)) in l.gauss_nodes.iter().zip(&l.gauss_weights).enumerate() {
                let w = weight.sqrt();
                b[i] = w * zm;
                for j in 0..order {
                    let value = |v: &[rodas5p_core::retro_dictionary::Rational]| {
                        v.iter().rev().fold(0_f64, |s, c| s.mul_add(u, c.value))
                    };
                    a[(i, j)] = w * (value(&l.l[j]) - zm * value(&l.integral[j]));
                }
            }
            coeff[..order].copy_from_slice(&rodas5p_krylov::least_squares(&a, &b)?);
        }
        _ => unreachable!(),
    }
    let zz = p(zm)?;
    let z2 = zz.mul(zz)?;
    let mut residual = 0_f64;
    for i in 0..order {
        let b = zz.mul(l.d[i].enclosure)?.sub(z2.mul(l.v[i].enclosure)?)?;
        let mut v = b;
        for j in 0..order {
            let a = l.g[i][j]
                .enclosure
                .sub(zz.mul(l.c[i][j].enclosure.add(l.c[j][i].enclosure)?)?)?
                .add(z2.mul(l.m[i][j].enclosure)?)?;
            v = v.sub(a.scale(coeff[j])?)?;
        }
        residual = residual.max(v.mag());
    }
    // ||q'-z q||² >= ||q'||² for z<=0 and q(0)=0.
    // lambda_min >=1/(2p-1), use sqrt(p)<=p conservatively.
    let ce = mul_up((order * (2 * order - 1)) as f64, residual)?;
    out.normal_residual = Some(residual);
    out.coefficient_error = Some(ce);
    let mut q = [zero(); 7];
    let mut dq = [zero(); 7];
    q[0] = p(1.)?;
    for j in 0..order {
        for (v, c) in q.iter_mut().zip(&l.integral[j]) {
            *v = v.add(c.enclosure.scale(coeff[j])?)?;
        }
        for (v, c) in dq.iter_mut().zip(&l.l[j]) {
            *v = v.add(c.enclosure.scale(coeff[j])?)?;
        }
    }
    let endpoint = eval(&q[..=order], p(1.)?)?;
    let mut end_ls = p(1.)?;
    let mut ell_norm = 0.;
    for j in 0..order {
        end_ls = end_ls.add(l.ell[j].enclosure.scale(coeff[j])?)?;
        ell_norm = add_up(ell_norm, l.ell[j].enclosure.mag())?;
    }
    let ee = mul_up(ce, ell_norm)?;
    let candidate = end_ls.add(Iv::new(-ee, ee)?)?;
    // The rational table is an independent representation of this LS
    // minimizer. Refuse disagreement, not a substitution for the IVP proof.
    if candidate.lo > exact_response.hi || candidate.hi < exact_response.lo {
        return Err(bad("normal residual disagrees with rational LS table"));
    }
    out.dictionary_gap = Some(end_ls.sub(exact_response)?.mag());
    for j in 0..=order {
        dq[j] = dq[j].sub(rate.mul(q[j])?)?;
    }
    let r = range(&dq[..=order])?.mag();
    let damp = if rate.hi < 0. {
        1_f64.min(div_up(1., -rate.hi)?)
    } else {
        1.
    };
    out.ivp_residual = Some(r);
    out.response = endpoint.as_pair();
    out.candidate_error = add_up(
        endpoint.scale(initial)?.sub(right)?.mag(),
        add_up(mul_up(initial.abs(), mul_up(damp, r)?)?, initial_error)?,
    )?;
    Ok(out)
}
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_with_linear_ls(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    order: usize,
    route: LinearLsEndpointRoute,
) -> CoreResult<SlabCertificate> {
    let mut c = certify_slab(src, times, y, branch, tube, incoming, green, condition_cap)?;
    if c.global_qualified {
        let r = source_linear_ls_endpoint(src, times, y, c.incoming, order, route)?;
        c.outgoing = Some(
            c.outgoing
                .ok_or_else(|| bad("linear LS original outgoing"))?
                .min(r.candidate_error),
        );
        c.linear_ls_endpoint = Some(r);
    }
    Ok(c)
}
/// Verified complex response for source-owned constant damped rotation
/// with polynomial forcing. Dictionary output is a proposal, never authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ComplexEndpointRoute {
    DictionaryDirected,
    Taylor48,
    ScaledRecurrence,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ComplexEndpointBounds {
    pub route: ComplexEndpointRoute,
    pub rate: [f64; 2],
    pub frequency: [f64; 2],
    pub candidate_error: f64,
    pub initial_error: f64,
    pub proposal_error: f64,
    pub response: [[f64; 2]; 2],
    pub series_terms: usize,
    pub squarings: usize,
    pub dictionary_taylor: Option<bool>,
}
type Ci = [Iv; 2];
fn ci_add(a: Ci, b: Ci) -> CoreResult<Ci> {
    Ok([a[0].add(b[0])?, a[1].add(b[1])?])
}
fn ci_mul(a: Ci, b: Ci) -> CoreResult<Ci> {
    Ok([
        a[0].mul(b[0])?.sub(a[1].mul(b[1])?)?,
        a[0].mul(b[1])?.add(a[1].mul(b[0])?)?,
    ])
}
fn ci_div(a: Ci, b: Ci) -> CoreResult<Ci> {
    let square = |v: Iv| -> CoreResult<Iv> {
        let lo = if v.contains_zero() {
            0.
        } else {
            v.lo.abs().min(v.hi.abs())
        };
        let hi = v.mag();
        Ok(Iv::new(p(lo)?.mul(p(lo)?)?.lo, p(hi)?.mul(p(hi)?)?.hi)?)
    };
    let den = square(b[0])?.add(square(b[1])?)?;
    let v = ci_mul(a, [b[0], -b[1]])?;
    Ok([v[0].div(den)?, v[1].div(den)?])
}
fn ci_norm(a: Ci) -> CoreResult<f64> {
    add_up(a[0].mag(), a[1].mag())
}
fn complex_series(z: Ci, j: usize, kmax: usize, terms: &mut usize) -> CoreResult<Ci> {
    let mut first = p(1.)?;
    for k in 1..=j {
        first = first.div(p(k as f64)?)?;
    }
    let mut term = [first, zero()];
    let mut sum = term;
    let r = ci_norm(z)?;
    for k in 1..=kmax {
        term = ci_mul(term, z)?;
        term = [
            term[0].div(p((k + j) as f64)?)?,
            term[1].div(p((k + j) as f64)?)?,
        ];
        sum = ci_add(sum, term)?;
        *terms += 1;
    }
    // Absolute tail: next term majorant then geometric ratio.
    let next = div_up(mul_up(ci_norm(term)?, r)?, (kmax + j + 1) as f64)?;
    let ratio = div_up(r, (kmax + j + 2) as f64)?;
    let gap = sub_down(1., ratio)?;
    if gap <= 0. {
        return Err(bad("complex series tail cap"));
    }
    let tail = div_up(next, gap)?;
    let e = Iv::new(-tail, tail)?;
    Ok([sum[0].add(e)?, sum[1].add(e)?])
}
fn verified_complex_phi(
    z: Ci,
    scaled: bool,
    terms: &mut usize,
    squares: &mut usize,
) -> CoreResult<[Ci; 4]> {
    let r = ci_norm(z)?;
    if r > 64. {
        return Err(bad("complex response norm cap"));
    }
    let mut out = [[zero(); 2]; 4];
    if !scaled || r <= 1. {
        if r > 16. {
            return Err(bad("complex direct series norm cap"));
        }
        for (j, v) in out.iter_mut().enumerate() {
            *v = complex_series(z, j, 48, terms)?;
        }
    } else {
        let mut a = z;
        while ci_norm(a)? > 0.5 {
            a = [a[0].scale(0.5)?, a[1].scale(0.5)?];
            *squares += 1;
        }
        let mut e = complex_series(a, 0, 24, terms)?;
        for _ in 0..*squares {
            e = ci_mul(e, e)?;
        }
        out[0] = e;
        let mut inv_fact = p(1.)?;
        for j in 1..4 {
            if j > 1 {
                inv_fact = inv_fact.div(p((j - 1) as f64)?)?;
            }
            out[j] = ci_div([out[j - 1][0].sub(inv_fact)?, out[j - 1][1]], z)?;
        }
    }
    Ok(out)
}
/// Endpoint proof for Y'= (a+i*b)Y+sum f_j u^j in normalized time.
/// Independent real source extraction proves the physical block identities
/// before normalization. Full slab proof remains the caller's authority.
pub fn source_complex_endpoint(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    incoming: f64,
    route: ComplexEndpointRoute,
) -> CoreResult<ComplexEndpointBounds> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    if src.nx() != 2
        || y.len() != 2 + src.nz()
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !incoming.is_finite()
        || incoming < 0.
    {
        return Err(bad("complex endpoint shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let (physical, n) = linear_source_matrices(src, [0., 1.])?;
    let q = &physical[0];
    let iszero = |v: Iv| v.lo == 0. && v.hi == 0.;
    if n != 3
        || q[0].hi > 0.
        || !iszero(q[0].sub(q[4])?)
        || !iszero(q[1].add(q[3])?)
        || physical[1..]
            .iter()
            .any(|a| [0, 1, 3, 4].iter().any(|&k| !iszero(a[k])))
    {
        return Err(bad("complex source is not proved constant damped rotation"));
    }
    let h = p(times[1])?.sub(p(times[0])?)?;
    let z = [q[0].mul(h)?, q[3].mul(h)?];
    let (a, _) = linear_source_matrices(src, times)?;
    let mut terms = 0;
    let mut squares = 0;
    let phi = verified_complex_phi(
        z,
        route != ComplexEndpointRoute::Taylor48,
        &mut terms,
        &mut squares,
    )?;
    let initial = [p(di::mid(y[0][0]))?, p(di::mid(y[1][0]))?];
    let left_error = y[0][0]
        .sub(initial[0])?
        .mag()
        .max(y[1][0].sub(initial[1])?.mag());
    let initial_error = mul_up(ci_norm(phi[0])?, add_up(incoming, left_error)?)?;
    let endpoint = |v: &[Ci; 4]| -> CoreResult<Ci> {
        let mut e = ci_mul(v[0], initial)?;
        for k in 0..3 {
            let f = [a[k][2], a[k][5]];
            let term = ci_mul(v[k + 1], f)?;
            let scale = if k == 2 { 2. } else { 1. };
            e = ci_add(e, [term[0].scale(scale)?, term[1].scale(scale)?])?;
        }
        Ok(e)
    };
    let rigorous = endpoint(&phi)?;
    let mut proposed = rigorous;
    let mut proposal_error = 0_f64;
    let mut dictionary_taylor = None;
    if route == ComplexEndpointRoute::DictionaryDirected {
        let d = Dictionary::get()?.complex_phi(di::mid(z[0]), di::mid(z[1]))?;
        let mut v = [[zero(); 2]; 4];
        for (i, w) in v.iter_mut().enumerate() {
            *w = [p(d.values[i][0])?, p(d.values[i][1])?];
        }
        proposed = endpoint(&v)?;
        for i in 0..2 {
            proposal_error = proposal_error.max(rigorous[i].sub(proposed[i])?.mag());
        }
        dictionary_taylor = Some(d.used_taylor);
    }
    let mut error = 0_f64;
    for i in 0..2 {
        error = error.max(proposed[i].sub(eval(&y[i], p(1.)?)?)?.mag());
    }
    Ok(ComplexEndpointBounds {
        route,
        rate: z[0].as_pair(),
        frequency: z[1].as_pair(),
        candidate_error: add_up(error, add_up(initial_error, proposal_error)?)?,
        initial_error,
        proposal_error,
        response: proposed.map(|v| v.as_pair()),
        series_terms: terms,
        squarings: squares,
        dictionary_taylor,
    })
}
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_with_complex(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    route: ComplexEndpointRoute,
) -> CoreResult<SlabCertificate> {
    let mut c = certify_slab(src, times, y, branch, tube, incoming, green, condition_cap)?;
    if c.global_qualified {
        let r = source_complex_endpoint(src, times, y, c.incoming, route)?;
        c.outgoing = Some(
            c.outgoing
                .ok_or_else(|| bad("complex original outgoing"))?
                .min(r.candidate_error),
        );
        c.complex_endpoint = Some(r);
    }
    Ok(c)
}
/// Source-bound aperiodic transport. The owning recurrence verifies the
/// rounded transition chain; the physical receipt additionally verifies
/// transition truncation, source coefficients and incoming uncertainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AperiodicEndpointRoute {
    Sequential,
    OrderedScan,
    CenteredStream,
    IntervalStream,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct AperiodicEndpointBounds {
    pub route: AperiodicEndpointRoute,
    pub cells: usize,
    pub order: usize,
    pub candidate_error: f64,
    pub initial_error: f64,
    pub propagated_error: f64,
    pub discrete_error: Option<f64>,
    pub max_tail: f64,
    pub coefficient_steps: usize,
    pub scalar_products: usize,
    pub nonzero_actions: usize,
    pub threads: usize,
}
const AP_DIM: usize = 8;
const AP_MAT: usize = AP_DIM * AP_DIM;
const AP_DEG: usize = 36;
fn interval_matrix_transition(
    src: &DaeSource,
    times: [f64; 2],
    order: usize,
    steps: &mut usize,
    products: &mut usize,
    actions: &mut usize,
) -> CoreResult<([Iv; AP_MAT], f64)> {
    let (a, n) = linear_source_matrices(src, times)?;
    if n > AP_DIM || !(7..=12).contains(&order) {
        return Err(bad("aperiodic transition cap"));
    }
    let mut d = 0;
    for j in 1..3 {
        if a[j][..n * n].iter().any(|v| v.lo != 0. || v.hi != 0.) {
            d = j;
        }
    }
    let m = picard_majorant(&a, n)?;
    if m > 8. {
        return Err(bad("aperiodic transition majorant"));
    }
    // Direct time-power recurrence rather than enumerating ordered words.
    // Terms omitted above degree (d+1)*order contain >order generators.
    // Their absolute-coefficient sum is bounded by the Peano-Baker tail.
    let limit = (d + 1) * order;
    let mut t = [[zero(); AP_MAT]; AP_DEG + 1];
    let mut sum = [zero(); AP_MAT];
    for i in 0..n {
        t[0][i * n + i] = p(1.)?;
        sum[i * n + i] = p(1.)?;
    }
    let mut zero_run = 0;
    let mut exact = false;
    for k in 1..=limit {
        for j in 0..=d.min(k - 1) {
            let prev = t[k - 1 - j];
            if prev[..n * n].iter().all(|v| v.lo == 0. && v.hi == 0.)
                || a[j][..n * n].iter().all(|v| v.lo == 0. && v.hi == 0.)
            {
                continue;
            }
            *actions += 1;
            for i in 0..n {
                for l in 0..n {
                    let c = a[j][i * n + l];
                    if c.lo == 0. && c.hi == 0. {
                        continue;
                    }
                    for col in 0..n {
                        let v = prev[l * n + col];
                        if v.lo == 0. && v.hi == 0. {
                            continue;
                        }
                        let idx = i * n + col;
                        t[k][idx] = t[k][idx].add(c.mul(v)?)?;
                        *products += 1;
                    }
                }
            }
        }
        for i in 0..n * n {
            t[k][i] = t[k][i].div(p(k as f64)?)?;
            sum[i] = sum[i].add(t[k][i])?;
        }
        *steps += 1;
        if t[k][..n * n].iter().all(|v| v.lo == 0. && v.hi == 0.) {
            zero_run += 1;
        } else {
            zero_run = 0;
        }
        if zero_run > d {
            exact = true;
            break;
        }
    }
    let tail = if exact {
        0.
    } else {
        picard_tail(1., m, order, exp_interval(m)?.hi)?
    };
    let e = Iv::new(-tail, tail)?;
    for v in &mut sum[..(n - 1) * n] {
        *v = v.add(e)?;
    }
    // The augmented coordinate is identically constant, not a free IVP
    // error component. This identity is structural in linear_source_matrices.
    for j in 0..n {
        sum[(n - 1) * n + j] = p(if j + 1 == n { 1. } else { 0. })?;
    }
    Ok((sum, tail))
}
fn ap_action(a: &[Iv], v: &[Iv], n: usize) -> CoreResult<[Iv; AP_DIM]> {
    let mut out = [zero(); AP_DIM];
    for i in 0..n {
        out[i] = a[i * n..(i + 1) * n]
            .iter()
            .zip(v)
            .try_fold(zero(), |s, (&a, &x)| s.add(a.mul(x)?))?;
    }
    Ok(out)
}
fn ap_gain(a: &[Iv], n: usize) -> CoreResult<f64> {
    let mut gain = 0_f64;
    for i in 0..n - 1 {
        gain = gain.max(
            a[i * n..i * n + n - 1]
                .iter()
                .try_fold(0., |s, v| add_up(s, v.mag()))?,
        );
    }
    Ok(gain)
}
/// Nonuniform partition uses represented clock endpoints, not an assumed
/// exact equal step. Transition matrices enclose the exact local flow.
pub fn source_aperiodic_endpoint(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    incoming: f64,
    cells: usize,
    order: usize,
    route: AperiodicEndpointRoute,
    exec: &ParallelExecution,
) -> CoreResult<AperiodicEndpointBounds> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    let nx = src.nx();
    let n = nx + 1;
    if n > AP_DIM
        || y.len() != nx + src.nz()
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !incoming.is_finite()
        || incoming < 0.
        || !matches!(cells, 2 | 4 | 8)
        || !matches!(order, 7 | 12)
    {
        return Err(bad("aperiodic endpoint shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo <= 0. {
        return Err(bad("aperiodic clock"));
    }
    let mut clock = [0.; 9];
    clock[0] = times[0];
    clock[cells] = times[1];
    // First cell half a regular cell, final cell one and a half.
    for i in 1..cells {
        let u = (i as f64 - 0.5) / cells as f64;
        clock[i] = di::mid(p(times[0])?.add(h.scale(u)?)?);
    }
    if clock[..=cells].windows(2).any(|v| v[0] >= v[1]) {
        return Err(bad("aperiodic partition resolution"));
    }
    let mut t = [[zero(); AP_MAT]; 8];
    let mut max_tail = 0_f64;
    let mut coefficient_steps = 0;
    let mut scalar_products = 0;
    let mut nonzero_actions = 0;
    for i in 0..cells {
        let (v, e) = interval_matrix_transition(
            src,
            [clock[i], clock[i + 1]],
            order,
            &mut coefficient_steps,
            &mut scalar_products,
            &mut nonzero_actions,
        )?;
        t[i] = v;
        max_tail = max_tail.max(e);
    }
    let mut center = [0.; AP_DIM];
    center[nx] = 1.;
    let mut initial_error = incoming;
    for i in 0..nx {
        center[i] = di::mid(y[i][0]);
        initial_error = initial_error.max(add_up(incoming, y[i][0].sub(p(center[i])?)?.mag())?);
    }
    let mut discrete_error = None;
    let mut propagated_error = initial_error;
    let mut final_box = [zero(); AP_DIM];
    if route == AperiodicEndpointRoute::IntervalStream {
        for i in 0..nx {
            final_box[i] = y[i][0].add(Iv::new(-incoming, incoming)?)?;
        }
        final_box[nx] = p(1.)?;
        for a in &t[..cells] {
            final_box = ap_action(&a[..n * n], &final_box[..n], n)?;
        }
        propagated_error = 0.;
        for v in &final_box[..nx] {
            propagated_error = propagated_error.max(v.sub(p(di::mid(*v))?)?.mag());
        }
    } else {
        let own = matches!(
            route,
            AperiodicEndpointRoute::Sequential | AperiodicEndpointRoute::OrderedScan
        );
        let report = if own {
            let g = t[..cells]
                .iter()
                .map(|a| DenseMatrix::new(n, n, a[..n * n].iter().copied().map(di::mid).collect()))
                .collect::<CoreResult<Vec<_>>>()?;
            let owner = crate::retro_foundation::Aperiodic::new(g)?;
            let b = vec![vec![0.; n]; cells];
            let mode = if route == AperiodicEndpointRoute::Sequential {
                crate::retro_foundation::TimeMode::Sequential
            } else {
                crate::retro_foundation::TimeMode::OrderedScan
            };
            let r = owner.solve(&center[..n], &b, mode, exec)?;
            discrete_error = Some(r.error_upper);
            Some(r)
        } else {
            None
        };
        for (i, a) in t[..cells].iter().enumerate() {
            let mut point = [zero(); AP_DIM];
            for j in 0..n {
                point[j] = p(center[j])?;
            }
            let v = ap_action(&a[..n * n], &point[..n], n)?;
            let mut next = [0.; AP_DIM];
            next[nx] = 1.;
            for j in 0..nx {
                next[j] = if let Some(r) = &report {
                    r.values[i][j]
                } else {
                    di::mid(v[j])
                };
            }
            if report.as_ref().is_some_and(|r| r.values[i][nx] != 1.) {
                return Err(bad("aperiodic augmented coordinate"));
            }
            let mut residual = 0_f64;
            for j in 0..nx {
                residual = residual.max(v[j].sub(p(next[j])?)?.mag());
            }
            propagated_error = add_up(
                mul_up(ap_gain(&a[..n * n], n)?, propagated_error)?,
                residual,
            )?;
            center = next;
        }
        for i in 0..nx {
            final_box[i] = p(center[i])?.add(Iv::new(-propagated_error, propagated_error)?)?;
        }
    }
    let mut candidate_error = 0_f64;
    for i in 0..nx {
        candidate_error = candidate_error.max(final_box[i].sub(eval(&y[i], p(1.)?)?)?.mag());
    }
    Ok(AperiodicEndpointBounds {
        route,
        cells,
        order,
        candidate_error,
        initial_error,
        propagated_error,
        discrete_error,
        max_tail,
        coefficient_steps,
        scalar_products,
        nonzero_actions,
        threads: if route == AperiodicEndpointRoute::OrderedScan {
            exec.threads()
        } else {
            1
        },
    })
}
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_with_aperiodic(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    cells: usize,
    order: usize,
    route: AperiodicEndpointRoute,
    exec: &ParallelExecution,
) -> CoreResult<SlabCertificate> {
    let mut c = certify_slab(src, times, y, branch, tube, incoming, green, condition_cap)?;
    if c.global_qualified {
        let r = source_aperiodic_endpoint(src, times, y, c.incoming, cells, order, route, exec)?;
        c.outgoing = Some(
            c.outgoing
                .ok_or_else(|| bad("aperiodic original outgoing"))?
                .min(r.candidate_error),
        );
        c.aperiodic_endpoint = Some(r);
    }
    Ok(c)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_variant: Option<SlabResidualBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picard_endpoint: Option<PicardEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_ls_endpoint: Option<LinearLsEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complex_endpoint: Option<ComplexEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aperiodic_endpoint: Option<AperiodicEndpointBounds>,
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
    certify_slab_with_residual(
        src,
        times,
        y,
        branch,
        tube,
        incoming,
        green,
        condition_cap,
        SlabResidualRoute::Bernstein,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_with_residual(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    residual_route: SlabResidualRoute,
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
    let (original_g_upper, az, dx, residual_variant) =
        if residual_route == SlabResidualRoute::Bernstein {
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
            (original_g_upper, az, dx, None)
        } else {
            let r = slab_residual_bounds(src, times, y, c.as_slice(), residual_route)?;
            (
                r.original_g_upper,
                div_up(r.preconditioned_g_upper, gap)?,
                r.differential_defect,
                Some(r),
            )
        };
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
        residual_variant,
        picard_endpoint: None,
        linear_ls_endpoint: None,
        complex_endpoint: None,
        aperiodic_endpoint: None,
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
/// Explicit candidate, not a change to the legacy general or conservation
/// proof. Every candidate uses the original source interval defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SlabWorkspaceRoute {
    Pivot,
    Equilibrated,
    Refined,
    PackedGreen,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SlabWorkspaceWork {
    pub source_rows: usize,
    pub inverse_calls: usize,
    pub local_trials: usize,
    pub column_solves: usize,
    pub refinement: di::AttemptCount,
    pub directed_green_calls: usize,
    pub packed_green_calls: usize,
}
/// Scratch only. Does not retain a source-specific inverse or certificate.
/// The active prefixes are overwritten on each invocation.
pub struct SlabCertificateWorkspace {
    jac: [[Iv; 33]; 32],
    other: [[Iv; 33]; 32],
    values: [Iv; 32],
    a: [f64; 1024],
    c: [f64; 1024],
    defect: [Iv; 1024],
    local_defect: [Iv; 1024],
    zx: [Iv; 256],
    inverse: GuardInverseWorkspace,
    work: SlabWorkspaceWork,
}
impl SlabCertificateWorkspace {
    pub const fn new() -> Self {
        const Z: Iv = Iv { lo: 0., hi: 0. };
        Self {
            jac: [[Z; 33]; 32],
            other: [[Z; 33]; 32],
            values: [Z; 32],
            a: [0.; 1024],
            c: [0.; 1024],
            defect: [Z; 1024],
            local_defect: [Z; 1024],
            zx: [Z; 256],
            inverse: GuardInverseWorkspace::new(),
            work: SlabWorkspaceWork {
                source_rows: 0,
                inverse_calls: 0,
                local_trials: 0,
                column_solves: 0,
                refinement: di::AttemptCount {
                    started: 0,
                    returned: 0,
                },
                directed_green_calls: 0,
                packed_green_calls: 0,
            },
        }
    }
    pub fn work(&self) -> SlabWorkspaceWork {
        self.work
    }
}
impl Default for SlabCertificateWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
impl SlabCertificate {
    /// Reserve the six retained output arrays once. This object is not a
    /// qualified certificate until the proof has succeeded.
    pub fn workspace_output(nx: usize, nz: usize) -> CoreResult<Self> {
        if nx == 0 || nz == 0 || nx.checked_add(nz).is_none_or(|n| n > 32) {
            return Err(bad("certificate output shape"));
        }
        Ok(Self {
            times: [0.; 2],
            tube_radius: 0.,
            x_range: Vec::with_capacity(nx),
            z_range: Vec::with_capacity(nz),
            branch_root_box: Vec::with_capacity(nz),
            self_map_box: Vec::with_capacity(nz),
            self_map_center: Vec::with_capacity(nz),
            preconditioner: Vec::with_capacity(nz * nz),
            branch_defect: 0.,
            condition_upper: 0.,
            original_g_upper: 0.,
            residual_variant: None,
            picard_endpoint: None,
            linear_ls_endpoint: None,
            complex_endpoint: None,
            aperiodic_endpoint: None,
            algebraic_defect: 0.,
            differential_defect: 0.,
            normalized_reduced_defect: 0.,
            reduced_mu_upper: 0.,
            zx_norm_upper: 0.,
            incoming: 0.,
            outgoing: None,
            whole_x_error: None,
            whole_z_error: None,
            green_used: false,
            global_qualified: false,
            global_reason: None,
            conservation: None,
        })
    }
}
fn slab_fixed_image(
    cg: &[Iv],
    zc: &[f64],
    local: &[Iv],
    e: &[Iv],
    out: &mut [Iv],
) -> CoreResult<()> {
    let n = zc.len();
    let mut delta = [zero(); 32];
    for i in 0..n {
        delta[i] = local[i].sub(p(zc[i])?)?;
    }
    for i in 0..n {
        out[i] = p(zc[i])?
            .sub(cg[i])?
            .add(guard_dot(&e[i * n..(i + 1) * n], &delta[..n])?)?;
    }
    Ok(())
}
/// Upper Green factors are monotone in the nonnegative damping argument.
/// The packed lane evaluates the existing certified table at the left
/// endpoint. For the other candidate, keep Hybrid's original upper bounds,
/// without reconstructing the legacy RetroTable owner each slab.
fn slab_workspace_green(
    mu: f64,
    h: Iv,
    ein: f64,
    rnorm: f64,
    route: SlabWorkspaceRoute,
    work: &mut SlabWorkspaceWork,
) -> CoreResult<f64> {
    let u = h.scale(-mu)?;
    let u = Iv::new(u.lo.max(0.), u.hi)?;
    if u.hi > 16384. {
        return Err(bad("workspace Green argument cap"));
    }
    let d = Dictionary::get()?;
    let (gain, ph) = if route == SlabWorkspaceRoute::PackedGreen {
        let v = d.phi_all_fast(u.lo)?;
        work.packed_green_calls += 1;
        (v[0].hi.min(1.), v[1].hi)
    } else {
        let gain = d.phi(u, 0, RealMode::Hybrid)?.hi.min(1.);
        let ph = d.phi(u, 1, RealMode::Hybrid)?.hi;
        work.directed_green_calls += 2;
        (gain, ph)
    };
    add_up(mul_up(gain, ein)?, mul_up(ph, rnorm)?)
}
/// Prove into caller-owned retained storage. Hard failures leave out
/// unchanged; scratch may be modified. Successful fixed-shape warmed
/// calls allocate no new output or scratch. Unresolved/rejected physical
/// bootstrap reasons remain ordinary owning diagnostics.
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_workspace_into(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    route: SlabWorkspaceRoute,
    w: &mut SlabCertificateWorkspace,
    out: &mut SlabCertificate,
) -> CoreResult<()> {
    crate::affine_event_certificate::check_affine_event_arithmetic()?;
    w.work = SlabWorkspaceWork::default();
    let nx = src.nx();
    let nz = src.nz();
    let n = nx + nz;
    let m = nz * nz;
    if y.len() != n
        || branch.len() != nz
        || y.iter().any(|v| v.is_empty() || v.len() > 5)
        || !tube.is_finite()
        || tube <= 0.
        || !condition_cap.is_finite()
        || condition_cap <= 0.
        || incoming.is_some_and(|v| !v.is_finite() || v < 0.)
    {
        return Err(bad("slab shape"));
    }
    for v in y.iter().flatten() {
        Iv::new(v.lo, v.hi)?;
    }
    let t = Iv::new(times[0], times[1])?;
    let h = p(times[1])?.sub(p(times[0])?)?;
    if h.lo <= 0. {
        return Err(bad("clock"));
    }
    let mut ranges = [zero(); 32];
    let mut b = [zero(); 32];
    let mut all = [zero(); 32];
    let mut center = [zero(); 32];
    let mut zc = [0.; 32];
    let mut fix = [zero(); 32];
    for (v, row) in ranges[..n].iter_mut().zip(y) {
        *v = range(row)?;
    }
    for i in 0..nz {
        b[i] = Iv::new(branch[i][0], branch[i][1])?;
    }
    if !di::subset(&ranges[nx..n], &b[..nz]) {
        return Err(bad("algebraic polynomial leaves branch"));
    }
    for i in 0..nx {
        all[i] = ranges[i].add(Iv::new(-tube, tube)?)?;
        center[i] = p(di::mid(all[i]))?;
        fix[i] = all[i];
    }
    for i in 0..nz {
        all[nx + i] = b[i];
        zc[i] = di::mid(ranges[nx + i]);
        center[nx + i] = p(zc[i])?;
        fix[nx + i] = center[nx + i];
    }
    guard_source_rows(
        &src.g,
        p(di::mid(t))?,
        &center[..n],
        &mut w.values[..nz],
        &mut w.other[..nz],
    )?;
    w.work.source_rows += nz;
    for i in 0..nz {
        for j in 0..nz {
            w.a[i * nz + j] = di::mid(w.other[i][1 + nx + j]);
        }
    }
    w.work.inverse_calls += 1;
    w.inverse.invert_pivoted(
        nz,
        &w.a[..m],
        &mut w.c[..m],
        route == SlabWorkspaceRoute::Equilibrated,
    )?;
    let c = GuardMatrix::new(nz, &w.c[..m])?;
    guard_source_rows(&src.g, t, &all[..n], &mut w.values[..nz], &mut w.jac[..nz])?;
    w.work.source_rows += nz;
    let q = guard_defect_into(c, &w.jac[..nz], nx, &mut w.defect[..m])?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("uniform branch contraction not proved"));
    }
    guard_source_rows(
        &src.g,
        t,
        &fix[..n],
        &mut w.values[..nz],
        &mut w.other[..nz],
    )?;
    w.work.source_rows += nz;
    let mut cg = [zero(); 32];
    for (i, o) in cg[..nz].iter_mut().enumerate() {
        *o = c
            .row(i)
            .iter()
            .zip(&w.values[..nz])
            .try_fold(zero(), |s, (&a, &v)| s.add(v.scale(a)?))?;
    }
    let mut local = b;
    let mut root = [zero(); 32];
    slab_fixed_image(
        &cg[..nz],
        &zc[..nz],
        &local[..nz],
        &w.defect[..m],
        &mut root[..nz],
    )?;
    if !di::subset(&root[..nz], &local[..nz]) {
        let eta = di::norm(&cg[..nz]);
        let mut admitted = false;
        for factor in [1.25, 2., 4., 8., 16., 32.] {
            w.work.local_trials += 1;
            let rad = mul_up(factor, eta)?;
            for i in 0..nz {
                local[i] = p(zc[i])?.add(Iv::new(-rad, rad)?)?;
            }
            if !di::subset(&local[..nz], &b[..nz]) {
                continue;
            }
            let mut box_y = all;
            box_y[nx..n].copy_from_slice(&local[..nz]);
            guard_source_rows(
                &src.g,
                t,
                &box_y[..n],
                &mut w.values[..nz],
                &mut w.other[..nz],
            )?;
            w.work.source_rows += nz;
            guard_defect_into(c, &w.other[..nz], nx, &mut w.local_defect[..m])?;
            slab_fixed_image(
                &cg[..nz],
                &zc[..nz],
                &local[..nz],
                &w.local_defect[..m],
                &mut root[..nz],
            )?;
            if di::subset(&root[..nz], &local[..nz]) {
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(bad("uniform self-map leaves branch"));
        }
    }
    let mut cnorm = 0_f64;
    let mut jnorm = 0_f64;
    for i in 0..nz {
        cnorm = cnorm.max(c.row(i).iter().try_fold(0., |s, v| add_up(s, v.abs()))?);
        jnorm = jnorm.max(guard_row_norm(&w.jac[i][1 + nx..=n])?);
    }
    let condition_upper = mul_up(div_up(cnorm, gap)?, jnorm)?;
    if condition_upper > condition_cap {
        return Err(bad("uniform condition cap"));
    }
    let residual = slab_residual_bounds(src, times, y, &w.c[..m], SlabResidualRoute::Bernstein)?;
    let az = div_up(residual.preconditioned_g_upper, gap)?;
    let dx = residual.differential_defect;
    guard_source_rows(
        &src.f,
        t,
        &all[..n],
        &mut w.values[..nx],
        &mut w.other[..nx],
    )?;
    w.work.source_rows += nx;
    let mut fz = 0_f64;
    for row in &w.other[..nx] {
        fz = fz.max(guard_row_norm(&row[1 + nx..=n])?);
    }
    let rnorm = add_up(dx, mul_up(h.hi, mul_up(fz, az)?)?)?;
    let refine = if route == SlabWorkspaceRoute::Refined {
        GuardColumnRefinement::AdaptiveGaussSeidel2
    } else {
        GuardColumnRefinement::None
    };
    let mut rhs = [zero(); 32];
    let mut sol = [zero(); 32];
    for k in 0..nx {
        for i in 0..nz {
            rhs[i] = -w.jac[i][k + 1];
        }
        w.work.column_solves += 1;
        guard_solve_into(
            c,
            &rhs[..nz],
            q,
            gap,
            &mut sol[..nz],
            &w.defect[..m],
            refine,
            &mut w.work.refinement,
        )?;
        for i in 0..nz {
            w.zx[i * nx + k] = sol[i];
        }
    }
    let mut lip = 0_f64;
    for row in w.zx[..nx * nz].chunks_exact(nx) {
        lip = lip.max(guard_row_norm(row)?);
    }
    let mut mu = f64::NEG_INFINITY;
    for (i, row) in w.other[..nx].iter().enumerate() {
        let mut sum = 0.;
        for k in 0..nx {
            let v = (0..nz).try_fold(row[k + 1], |s, l| {
                s.add(row[1 + nx + l].mul(w.zx[l * nx + k])?)
            })?;
            sum = add_up(sum, if i == k { v.hi } else { v.mag() })?;
        }
        mu = mu.max(sum);
    }
    // Stage all fallible physical decisions before committing the report.
    let mut outgoing = None;
    let mut whole_x = None;
    let mut whole_z = None;
    let mut used = false;
    let mut qualified = false;
    let mut reason = None;
    if let Some(ein) = incoming {
        let propagate = (|| -> CoreResult<(f64, bool)> {
            if green && mu < 0. {
                Ok((
                    slab_workspace_green(mu, h, ein, rnorm, route, &mut w.work)?,
                    true,
                ))
            } else {
                let g = if mu > 0. {
                    exp_interval(mul_up(mu, h.hi)?)?.hi
                } else {
                    1.
                };
                Ok((mul_up(g, add_up(ein, rnorm)?)?, false))
            }
        })();
        match propagate {
            Ok((e, u)) => {
                let r = e.max(ein);
                outgoing = Some(e);
                whole_x = Some(r);
                whole_z = Some(add_up(az, mul_up(lip, r)?)?);
                used = u;
                qualified = r < tube;
                if !qualified {
                    reason = Some("physical bootstrap tube exceeded".into());
                }
            }
            Err(e) => reason = Some(e.to_string()),
        }
    } else {
        reason = Some("prior physical tube unresolved".into());
    }
    out.times = times;
    out.tube_radius = tube;
    out.x_range.clear();
    out.x_range.extend(all[..nx].iter().map(|v| v.as_pair()));
    out.z_range.clear();
    out.z_range
        .extend(ranges[nx..n].iter().map(|v| v.as_pair()));
    out.branch_root_box.clear();
    out.branch_root_box
        .extend(root[..nz].iter().map(|v| v.as_pair()));
    out.self_map_box.clear();
    out.self_map_box
        .extend(local[..nz].iter().map(|v| v.as_pair()));
    out.self_map_center.clear();
    out.self_map_center.extend_from_slice(&zc[..nz]);
    out.preconditioner.clear();
    out.preconditioner.extend_from_slice(&w.c[..m]);
    out.branch_defect = q;
    out.condition_upper = condition_upper;
    out.original_g_upper = residual.original_g_upper;
    out.residual_variant = None;
    out.picard_endpoint = None;
    out.linear_ls_endpoint = None;
    out.complex_endpoint = None;
    out.aperiodic_endpoint = None;
    out.conservation = None;
    out.algebraic_defect = az;
    out.differential_defect = dx;
    out.normalized_reduced_defect = rnorm;
    out.reduced_mu_upper = mu;
    out.zx_norm_upper = lip;
    out.incoming = incoming.unwrap_or(0.);
    out.outgoing = outgoing;
    out.whole_x_error = whole_x;
    out.whole_z_error = whole_z;
    out.green_used = used;
    out.global_qualified = qualified;
    out.global_reason = reason;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn certify_slab_workspace(
    src: &DaeSource,
    times: [f64; 2],
    y: &[Poly],
    branch: &[[f64; 2]],
    tube: f64,
    incoming: Option<f64>,
    green: bool,
    condition_cap: f64,
    route: SlabWorkspaceRoute,
    w: &mut SlabCertificateWorkspace,
) -> CoreResult<SlabCertificate> {
    let mut out = SlabCertificate::workspace_output(src.nx(), src.nz())?;
    certify_slab_workspace_into(
        src,
        times,
        y,
        branch,
        tube,
        incoming,
        green,
        condition_cap,
        route,
        w,
        &mut out,
    )?;
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

/// Evaluate retained interval pairs with the identical directed Horner
/// operation order as query_x_error/conservation_query_full. The fixed
/// row candidate stages coefficients; streaming reads each pair directly.
pub fn query_error_from_pairs(
    power: &[Vec<[f64; 2]>],
    times: [f64; 2],
    t: f64,
    state: &[f64],
    ideal: f64,
    conserved: bool,
    stream: bool,
) -> CoreResult<f64> {
    query_error_from_pair_rows(power, times, t, state, ideal, conserved, stream)
}
/// Generic borrowed rows preserve the original directed Horner operations.
pub fn query_error_from_pair_rows<R: AsRef<[[f64; 2]]>>(
    power: &[R],
    times: [f64; 2],
    t: f64,
    state: &[f64],
    ideal: f64,
    conserved: bool,
    stream: bool,
) -> CoreResult<f64> {
    if !t.is_finite()
        || t < times[0]
        || t > times[1]
        || power.is_empty()
        || (if conserved {
            power.len() != 2 || state.len() != 3
        } else {
            state.len() != power.len()
        })
        || power
            .iter()
            .any(|r| r.as_ref().is_empty() || r.as_ref().len() > 5)
    {
        return Err(bad("query pair shape"));
    }
    let h = p(times[1])?.sub(p(times[0])?)?;
    let u = p(t)?.sub(p(times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let row = |v: &[[f64; 2]]| -> CoreResult<Iv> {
        if stream {
            v.iter()
                .rev()
                .try_fold(zero(), |s, b| s.mul(u)?.add(Iv::new(b[0], b[1])?))
        } else {
            let mut a = [zero(); 5];
            for (o, b) in a.iter_mut().zip(v) {
                *o = Iv::new(b[0], b[1])?;
            }
            eval(&a[..v.len()], u)
        }
    };
    let correction = if conserved {
        let a = row(power[0].as_ref())?;
        let b = row(power[1].as_ref())?;
        let lift = p(1.)?.sub(a)?.sub(b)?;
        [a, b, lift]
            .iter()
            .zip(state)
            .try_fold(0_f64, |s, (&a, &b)| {
                Ok::<f64, CoreError>(s.max(a.sub(p(b)?)?.mag()))
            })?
    } else {
        power.iter().zip(state).try_fold(0_f64, |m, (v, &x)| {
            Ok::<f64, CoreError>(m.max(row(v.as_ref())?.sub(p(x)?)?.mag()))
        })?
    };
    add_up(ideal, correction)
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
    /// Opt-in interval fixed-point intersection, one synchronous sweep.
    SchurRefine1,
    /// Opt-in interval fixed-point intersection, two synchronous sweeps.
    SchurRefine2,
    PreparedSchurRefine2,
    ConservedSchurRefine2,
    /// Directed diagonal isolation with one forward interval sweep.
    SchurGs1,
    SchurGs2,
    SchurGsAdaptive2,
    PreparedSchurGs2,
    ConservedSchurGsAdaptive2,
    SchurCentered,
    SchurCenteredGrouped,
    SchurCenteredGs2,
    PreparedSchurCenteredGs2,
    ConservedSchurCenteredGs2,
    SchurDualGrouped,
    SchurDualCentered,
    SchurDualCenteredGs2,
    BoundedPreparedSchur,
    BoundedPreparedDualCenteredGs2,
    ConservedSchurDualCentered,
    /// Exact affine constraint graph candidate, admitted only by identity and branch proof.
    SchurAffineGraph,
    ConservedSchurAffineGraph,
}
/// Conditional interval linear refinement; never an independent slab certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GuardColumnRefinement {
    None,
    Jacobi1,
    Jacobi2,
    GaussSeidel1,
    GaussSeidel2,
    AdaptiveGaussSeidel2,
}
impl GuardColumnRefinement {
    fn jacobi(sweeps: usize) -> CoreResult<Self> {
        match sweeps {
            0 => Ok(Self::None),
            1 => Ok(Self::Jacobi1),
            2 => Ok(Self::Jacobi2),
            _ => Err(bad("guard column workspace shape")),
        }
    }
    fn budget(self) -> usize {
        match self {
            Self::None => 0,
            Self::Jacobi1 | Self::GaussSeidel1 => 1,
            _ => 2,
        }
    }
    fn gs(self) -> bool {
        matches!(
            self,
            Self::GaussSeidel1 | Self::GaussSeidel2 | Self::AdaptiveGaussSeidel2
        )
    }
}
impl GuardDerivativeRoute {
    fn refinement(self) -> GuardColumnRefinement {
        use GuardColumnRefinement as R;
        match self {
            Self::SchurRefine1 => R::Jacobi1,
            Self::SchurRefine2 | Self::PreparedSchurRefine2 | Self::ConservedSchurRefine2 => {
                R::Jacobi2
            }
            Self::SchurGs1 => R::GaussSeidel1,
            Self::SchurGs2 | Self::PreparedSchurGs2 => R::GaussSeidel2,
            Self::SchurGsAdaptive2 | Self::ConservedSchurGsAdaptive2 => R::AdaptiveGaussSeidel2,
            _ => R::None,
        }
    }
    fn prepared(self) -> bool {
        matches!(
            self,
            Self::PreparedSchur
                | Self::PreparedSchurRefine2
                | Self::PreparedSchurGs2
                | Self::PreparedSchurCenteredGs2
        )
    }
    fn conserved(self) -> bool {
        matches!(
            self,
            Self::ConservedSchur
                | Self::ConservedSchurRefine2
                | Self::ConservedSchurGsAdaptive2
                | Self::ConservedSchurCenteredGs2
                | Self::ConservedSchurDualCentered
                | Self::ConservedSchurAffineGraph
        )
    }
    fn affine_graph(self) -> bool {
        matches!(
            self,
            Self::SchurAffineGraph | Self::ConservedSchurAffineGraph
        )
    }
    fn bounded_prepared(self) -> bool {
        matches!(
            self,
            Self::BoundedPreparedSchur | Self::BoundedPreparedDualCenteredGs2
        )
    }
    fn dual(self) -> Option<bool> {
        match self {
            Self::SchurDualGrouped => Some(false),
            Self::SchurDualCentered
            | Self::SchurDualCenteredGs2
            | Self::BoundedPreparedDualCenteredGs2
            | Self::ConservedSchurDualCentered => Some(true),
            _ => None,
        }
    }
    fn centered(self) -> Option<(bool, GuardColumnRefinement)> {
        use GuardColumnRefinement as R;
        match self {
            Self::SchurCentered => Some((false, R::None)),
            Self::SchurCenteredGrouped => Some((true, R::None)),
            Self::SchurCenteredGs2
            | Self::PreparedSchurCenteredGs2
            | Self::ConservedSchurCenteredGs2
            | Self::SchurDualCenteredGs2
            | Self::BoundedPreparedDualCenteredGs2 => Some((true, R::GaussSeidel2)),
            _ => None,
        }
    }
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

// Bounded native guard storage. Long low-level power rows keep the old
// owned arithmetic and representation; no implicit degree truncation.
enum GuardPoly {
    Bounded(ScratchPoly),
    Owned(Poly),
}
impl std::ops::Deref for GuardPoly {
    type Target = [Iv];
    fn deref(&self) -> &[Iv] {
        match self {
            Self::Bounded(v) => v.slice(),
            Self::Owned(v) => v,
        }
    }
}
enum GuardCg {
    Bounded { rows: [ScratchPoly; 32], len: usize },
    Owned(Vec<Poly>),
}
impl GuardCg {
    fn norm_at(&self, u: Iv) -> CoreResult<f64> {
        match self {
            Self::Bounded { rows, len } => rows[..*len]
                .iter()
                .try_fold(0_f64, |s, v| Ok(s.max(eval(v.slice(), u)?.mag()))),
            Self::Owned(rows) => rows
                .iter()
                .try_fold(0_f64, |s, v| Ok(s.max(eval(v, u)?.mag()))),
        }
    }
}
fn guard_lift(y: &[Poly]) -> CoreResult<GuardPoly> {
    if y[0].len() <= 5 && y[1].len() <= 5 {
        Ok(GuardPoly::Bounded(ScratchPoly::new(&[p(1.)?])?.add(
            ScratchPoly::new(&y[0])?.add(&y[1])?.scale(p(-1.)?)?.slice(),
        )?))
    } else {
        Ok(GuardPoly::Owned(add(
            &[p(1.)?],
            &scale(&add(&y[0], &y[1])?, p(-1.)?)?,
        )?))
    }
}
fn guard_polynomial(
    guard: &DeclaredPolynomial,
    clock: &[Iv],
    y: &[Poly],
    lifted: bool,
) -> CoreResult<GuardPoly> {
    if lifted {
        let lift = guard_lift(y)?;
        if y[0].len() <= 5 && y[1].len() <= 5 {
            Ok(GuardPoly::Bounded(compose_bounded(
                guard,
                clock,
                &[y[0].as_slice(), y[1].as_slice(), &*lift],
            )?))
        } else {
            Ok(GuardPoly::Owned(compose(
                guard,
                clock,
                &[y[0].clone(), y[1].clone(), lift.to_vec()],
            )?))
        }
    } else if y.iter().all(|v| v.len() <= 5) {
        Ok(GuardPoly::Bounded(compose_bounded(guard, clock, y)?))
    } else {
        Ok(GuardPoly::Owned(compose(guard, clock, y)?))
    }
}
#[derive(Clone, Copy)]
struct GuardMatrix<'a> {
    n: usize,
    data: &'a [f64],
}
impl<'a> GuardMatrix<'a> {
    fn new(n: usize, data: &'a [f64]) -> CoreResult<Self> {
        if n.checked_mul(n) != Some(data.len()) {
            return Err(CoreError::Dimension(format!(
                "matrix shape {n}x{n} does not match {} entries",
                data.len()
            )));
        }
        if !data.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite("matrix contains NaN/Inf".into()));
        }
        Ok(Self { n, data })
    }
    fn row(self, i: usize) -> &'a [f64] {
        &self.data[i * self.n..(i + 1) * self.n]
    }
}
/// Fixed-size scratch for an opt-in midpoint inverse candidate. It retains
/// no matrix identity or accepted physical certificate. Every call starts
/// afresh and returns a candidate only after a directed residual check.
pub struct GuardInverseWorkspace {
    a: [f64; 32 * 32],
    inverse: [f64; 32 * 32],
}
impl GuardInverseWorkspace {
    pub const fn new() -> Self {
        Self {
            a: [0.; 32 * 32],
            inverse: [0.; 32 * 32],
        }
    }
    /// Original midpoint strict row dominance and residual verification,
    /// followed by caller's mandatory full interval defect check.
    /// Output untouched on any failure.
    pub fn invert(&mut self, n: usize, center: &[f64], out: &mut [f64]) -> CoreResult<f64> {
        self.invert_impl(n, center, out, false, false)
    }
    /// Bounded pivoted approximate inverse, optionally row-equilibrated.
    /// The candidate is certified against the unscaled original midpoint
    /// by ||I-C A||_inf<1. Returns a bound on ||A^-1-C||_inf.
    /// Full source interval branch/local tests remain mandatory.
    pub fn invert_pivoted(
        &mut self,
        n: usize,
        center: &[f64],
        out: &mut [f64],
        equilibrate: bool,
    ) -> CoreResult<f64> {
        self.invert_impl(n, center, out, true, equilibrate)
    }
    fn invert_impl(
        &mut self,
        n: usize,
        center: &[f64],
        out: &mut [f64],
        pivoted: bool,
        equilibrate: bool,
    ) -> CoreResult<f64> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        if n == 0 || n > 32 || out.len() != n * n {
            return Err(bad("bounded inverse shape"));
        }
        let a = GuardMatrix::new(n, center)?;
        let mut gap = f64::INFINITY;
        for i in 0..n {
            let mut g = a.row(i)[i].abs();
            for j in 0..n {
                if i != j {
                    g = sub_down(g, a.row(i)[j].abs())?;
                }
            }
            gap = gap.min(g);
        }
        if !pivoted && (gap <= 0. || !gap.is_finite()) {
            return Err(bad("bounded inverse row dominance"));
        }
        let m = n * n;
        self.a[..m].copy_from_slice(center);
        self.inverse[..m].fill(0.);
        for i in 0..n {
            self.inverse[i * n + i] = 1.;
        }
        if equilibrate {
            for i in 0..n {
                let scale = center[i * n..(i + 1) * n]
                    .iter()
                    .fold(0_f64, |m, v| m.max(v.abs()));
                if scale == 0. || !scale.is_finite() {
                    return Err(bad("bounded inverse zero row"));
                }
                for j in 0..n {
                    self.a[i * n + j] /= scale;
                }
                self.inverse[i * n + i] = 1. / scale;
                if !self.inverse[i * n + i].is_finite() {
                    return Err(bad("bounded inverse row scale"));
                }
            }
        }
        // One point Gauss-Jordan factor/elimination, all identity RHS.
        for k in 0..n {
            let mut pivot = k;
            for i in k + 1..n {
                if self.a[i * n + k].abs() > self.a[pivot * n + k].abs() {
                    pivot = i;
                }
            }
            let d = self.a[pivot * n + k];
            if d == 0. || !d.is_finite() {
                return Err(bad("bounded inverse pivot"));
            }
            if pivot != k {
                for j in 0..n {
                    self.a.swap(k * n + j, pivot * n + j);
                    self.inverse.swap(k * n + j, pivot * n + j);
                }
            }
            for j in 0..n {
                self.a[k * n + j] /= d;
                self.inverse[k * n + j] /= d;
            }
            for i in 0..n {
                if i == k {
                    continue;
                }
                let r = self.a[i * n + k];
                for j in 0..n {
                    self.a[i * n + j] -= r * self.a[k * n + j];
                    self.inverse[i * n + j] -= r * self.inverse[k * n + j];
                }
            }
            if !self.a[..m]
                .iter()
                .chain(&self.inverse[..m])
                .all(|x| x.is_finite())
            {
                return Err(bad("bounded inverse nonfinite"));
            }
        }
        if pivoted {
            let mut q = 0_f64;
            let mut cnorm = 0_f64;
            for i in 0..n {
                let mut row = 0.;
                let mut cr = 0.;
                for k in 0..n {
                    let mut r = p(if i == k { 1. } else { 0. })?;
                    for j in 0..n {
                        r = r.sub(p(center[j * n + k])?.scale(self.inverse[i * n + j])?)?;
                    }
                    row = add_up(row, r.mag())?;
                    cr = add_up(cr, self.inverse[i * n + k].abs())?;
                }
                q = q.max(row);
                cnorm = cnorm.max(cr);
            }
            let gap = sub_down(1., q)?;
            if gap <= 0. {
                return Err(bad("bounded pivot inverse defect not proved"));
            }
            let e = div_up(mul_up(q, cnorm)?, gap)?;
            out.copy_from_slice(&self.inverse[..m]);
            return Ok(e);
        }
        // Check every original midpoint residual (same row/column order).
        // Skip only exact zero support when old PreparedRhs would select it.
        let sparse = center.iter().filter(|&&v| v != 0.).count() <= m / 2;
        let mut residual = 0_f64;
        for k in 0..n {
            for i in 0..n {
                let mut r = p(if i == k { 1. } else { 0. })?;
                for j in 0..n {
                    let v = a.row(i)[j];
                    if !sparse || v != 0. {
                        r = r.sub(p(v)?.scale(self.inverse[j * n + k])?)?;
                    }
                }
                residual = residual.max(r.mag());
            }
        }
        let error = div_up(residual, gap)?;
        out.copy_from_slice(&self.inverse[..m]);
        Ok(error)
    }
}
impl Default for GuardInverseWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
fn guard_cg(
    src: &DaeSource,
    clock: &[Iv],
    y: &[Poly],
    original: GuardMatrix<'_>,
) -> CoreResult<GuardCg> {
    if y.iter().all(|v| v.len() <= 5) {
        let empty = ScratchPoly::new(&[])?;
        let mut gp = [empty; 32];
        let n = src.nz();
        for (o, g) in gp[..n].iter_mut().zip(&src.g) {
            *o = compose_bounded(g, clock, y)?;
        }
        let mut rows = [empty; 32];
        for (i, o) in rows[..n].iter_mut().enumerate() {
            let mut acc = ScratchPoly::new(&[zero()])?;
            for (&a, r) in original.row(i).iter().zip(&gp[..n]) {
                acc = acc.add(r.scale(p(a)?)?.slice())?;
            }
            *o = acc;
        }
        Ok(GuardCg::Bounded { rows, len: n })
    } else {
        let gp = src
            .g
            .iter()
            .map(|g| compose(g, clock, y))
            .collect::<CoreResult<Vec<_>>>()?;
        Ok(GuardCg::Owned(c_polys(
            &DenseMatrix::new(original.n, original.n, original.data.to_vec())?,
            &gp,
        )?))
    }
}
fn guard_pairs(v: &[[f64; 2]]) -> CoreResult<[Iv; 32]> {
    let mut out = [zero(); 32];
    if v.len() > out.len() {
        return Err(bad("guard scratch shape"));
    }
    for (o, a) in out.iter_mut().zip(v) {
        *o = Iv::new(a[0], a[1])?;
    }
    Ok(out)
}
/// Conditional on an authentic certificate for the same original source,
/// polynomial and slab. Only the facade constructs event authority.
pub struct GuardProofCore<'a> {
    guard: &'a DeclaredPolynomial,
    y: &'a [Poly],
    nx: usize,
    times: [f64; 2],
    x_range: [Iv; 32],
    z_range: [Iv; 32],
    root_box: [Iv; 32],
    h_poly: GuardPoly,
    cg_poly: GuardCg,
    gap: f64,
    zx_lip: f64,
    whole_x: f64,
    evidence: GuardInlineEvidence,
}
// Public owning report remains unchanged. The numerical core needs only
// bounded arrays and scalar receipts. Materialization is an explicit boundary.
struct GuardInlineEvidence {
    times: [f64; 2],
    derivative_route: GuardDerivativeRoute,
    value_route: GuardValueRoute,
    velocity: [f64; 2],
    reduced_gradient: [[f64; 2]; 32],
    nx: usize,
    reduced_clock: [f64; 2],
    inverse_defect: f64,
    reduced_lip: f64,
    algebraic_guard_lip: f64,
    prepared_work: InitWork,
    conserved_derivative_box: Option<[[f64; 2]; 32]>,
    n: usize,
}
impl GuardInlineEvidence {
    fn owned(&self) -> GuardSlabEvidence {
        GuardSlabEvidence {
            evaluations: Vec::new(),
            times: self.times,
            derivative_route: self.derivative_route,
            value_route: self.value_route,
            velocity: self.velocity,
            reduced_gradient: self.reduced_gradient[..self.nx].to_vec(),
            reduced_clock: self.reduced_clock,
            inverse_defect: self.inverse_defect,
            reduced_lip: self.reduced_lip,
            algebraic_guard_lip: self.algebraic_guard_lip,
            prepared_work: self.prepared_work.clone(),
            conserved_derivative_box: self
                .conserved_derivative_box
                .as_ref()
                .map(|v| v[..self.n].to_vec()),
        }
    }
}
/// Compatibility owner; legacy callers can still own/mutate their diagnostic report.
pub struct GuardProof<'a> {
    core: GuardProofCore<'a>,
    pub evidence: GuardSlabEvidence,
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
        Self::new_recorded(
            src,
            guard,
            y,
            cert,
            derivative,
            value,
            &mut GuardKernelAudit::default(),
        )
    }
    pub fn new_recorded(
        src: &DaeSource,
        guard: &'a DeclaredPolynomial,
        y: &'a [Poly],
        cert: &SlabCertificate,
        derivative: GuardDerivativeRoute,
        value: GuardValueRoute,
        audit: &mut GuardKernelAudit,
    ) -> CoreResult<Self> {
        let core = GuardProofCore::new_recorded(src, guard, y, cert, derivative, value, audit)?;
        let evidence = core.owned_evidence();
        Ok(Self { core, evidence })
    }
    pub fn hint_coefficients(&self) -> Vec<f64> {
        self.core.hint_coefficients()
    }
    pub fn value_at(&self, t: f64, error: f64) -> CoreResult<Iv> {
        self.core.value_at_with(
            t,
            error,
            self.evidence.value_route,
            self.evidence.reduced_lip,
            self.evidence.algebraic_guard_lip,
        )
    }
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
macro_rules! observed_reset {
    ($a:expr,$field:ident,$e:expr) => {{
        let r = (|| $e)();
        $a.$field.record(r.is_ok());
        r
    }};
}
// The source dimension and factor caps are already validated by DaeSource.
// All rows (including unused derivatives) are evaluated, preserving errors.
fn guard_source_rows(
    polys: &[DeclaredPolynomial],
    time: Iv,
    state: &[Iv],
    values: &mut [Iv],
    jac: &mut [[Iv; 33]],
) -> CoreResult<()> {
    for ((eq, v), row) in polys.iter().zip(values).zip(jac) {
        *v = di::evaluate_row_bounded(eq, time, state, &mut row[..=state.len()])?;
    }
    Ok(())
}
fn guard_defect_into(
    c: GuardMatrix<'_>,
    g: &[[Iv; 33]],
    nx: usize,
    defect: &mut [Iv],
) -> CoreResult<f64> {
    let n = c.n;
    for i in 0..n {
        for k in 0..n {
            defect[i * n + k] = (0..n).try_fold(p(if i == k { 1. } else { 0. })?, |s, l| {
                s.sub(g[l][1 + nx + k].scale(c.row(i)[l])?)
            })?;
        }
    }
    // Evaluate the complete defect before the row norm, as in the old route.
    defect
        .chunks_exact(n)
        .map(guard_row_norm)
        .try_fold(0.0_f64, |s, v| Ok(s.max(v?)))
}
// X contains every admissible solution and W contains Cb. Since x=Cb+Ex,
// intersecting X with W+E X preserves inclusion. Jacobi (not Gauss-Seidel)
// means every next component reads the same previous box.
fn guard_refine(w: &[Iv], defect: &[Iv], out: &mut [Iv], next: &mut [Iv]) -> CoreResult<()> {
    let n = out.len();
    for i in 0..n {
        let ex = guard_dot(&defect[i * n..(i + 1) * n], out)?;
        next[i] = intersect(out[i], w[i].add(ex)?)?;
    }
    out.copy_from_slice(next);
    Ok(())
}
// The exact fixed point obeys (1-E_ii)v_i=w_i+sum_{j!=i}E_ij v_j.
// The certified q<1 implies positive denominator. Each component is
// intersected before using it for the next row; inclusion is inductive.
fn guard_gs_refine(w: &[Iv], defect: &[Iv], out: &mut [Iv]) -> CoreResult<()> {
    let n = out.len();
    for i in 0..n {
        let row = &defect[i * n..(i + 1) * n];
        let mut sum = zero();
        for (j, (&e, &v)) in row.iter().zip(out.iter()).enumerate() {
            if i != j {
                sum = sum.add(e.mul(v)?)?;
            }
        }
        let diag = p(1.)?.sub(row[i])?;
        if diag.lo <= 0. {
            return Err(bad("guard GS diagonal"));
        }
        out[i] = intersect(out[i], w[i].add(sum)?.div(diag)?)?;
    }
    Ok(())
}
// Same row-major dot and defect-product rounding as di::matvec followed
// by product_bound. Output is local scratch, not accepted physical state.
fn guard_solve_into(
    c: GuardMatrix<'_>,
    rhs: &[Iv],
    q: f64,
    gap: f64,
    out: &mut [Iv],
    defect: &[Iv],
    refinement: GuardColumnRefinement,
    sweep_work: &mut di::AttemptCount,
) -> CoreResult<()> {
    let sweeps = refinement.budget();
    for (i, o) in out.iter_mut().enumerate() {
        *o = c
            .row(i)
            .iter()
            .zip(rhs)
            .try_fold(zero(), |s, (&a, &v)| s.add(v.scale(a)?))?;
    }
    let mut w = [zero(); 32];
    let n = out.len();
    if sweeps != 0 {
        w[..n].copy_from_slice(out);
    }
    let e = div_up(mul_up(q, di::norm(out))?, gap)?;
    for v in out.iter_mut() {
        *v = v.add(Iv::new(-e, e)?)?;
    }
    let adaptive = refinement == GuardColumnRefinement::AdaptiveGaussSeidel2;
    if sweeps != 0 && !(adaptive && q == 0.) {
        let mut next = [zero(); 32];
        for _ in 0..sweeps {
            if adaptive {
                next[..n].copy_from_slice(out);
            }
            let r = if refinement.gs() {
                guard_gs_refine(&w[..n], defect, out)
            } else {
                guard_refine(&w[..n], defect, out, &mut next[..n])
            };
            sweep_work.record(r.is_ok());
            r?;
            if adaptive
                && out.iter().zip(&next[..n]).all(|(a, b)| {
                    a.lo.to_bits() == b.lo.to_bits() && a.hi.to_bits() == b.hi.to_bits()
                })
            {
                break;
            }
        }
    }
    Ok(())
}
// Directional residual r=-D_(e_k,center)g; all coefficients are directed.
// Grouping is exact algebra before interval evaluation. Repeated remaining
// factors are compared in declared order; affine terms are collected by variable.
fn guard_directional_residual(
    eq: &DeclaredPolynomial,
    time: Iv,
    state: &[Iv],
    nx: usize,
    k: usize,
    center: &[f64],
    grouped: bool,
) -> CoreResult<Iv> {
    use crate::retro_event_poly::AffineFactor;
    fn equal(a: &AffineFactor, b: &AffineFactor) -> bool {
        a.constant == b.constant && a.time == b.time && a.state == b.state
    }
    fn factor(f: &AffineFactor, t: Iv, y: &[Iv]) -> CoreResult<Iv> {
        f.state
            .iter()
            .try_fold(p(f.constant)?.add(t.scale(f.time)?)?, |s, &(i, a)| {
                s.add(y[i].scale(a)?)
            })
    }
    let terms = eq.terms();
    let mut entries: [Option<(usize, usize, Iv)>; 64] = [None; 64];
    let mut count = 0;
    let mut affine = [zero(); 34]; // constant, clock, then 32 state variables
    let mut sum = zero();
    for (ti, term) in terms.iter().enumerate() {
        for (fi, f) in term.factors.iter().enumerate() {
            let mut df = if k == 0 { p(f.time)? } else { zero() };
            for &(j, a) in &f.state {
                if j < nx {
                    if j + 1 == k {
                        df = df.add(p(a)?)?;
                    }
                } else {
                    df = df.add(p(center[j - nx])?.scale(a)?)?;
                }
            }
            let coef = df.scale(-term.coefficient)?;
            let rest = term
                .factors
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != fi)
                .map(|(_, f)| f);
            if !grouped {
                let mut v = coef;
                for f in rest {
                    v = v.mul(factor(f, time, state)?)?;
                }
                sum = sum.add(v)?;
            } else if term.factors.len() <= 2 {
                if let Some(f) = rest.clone().next() {
                    affine[0] = affine[0].add(coef.scale(f.constant)?)?;
                    affine[1] = affine[1].add(coef.scale(f.time)?)?;
                    for &(j, a) in &f.state {
                        affine[2 + j] = affine[2 + j].add(coef.scale(a)?)?;
                    }
                } else {
                    affine[0] = affine[0].add(coef)?;
                }
            } else {
                let existing = entries[..count].iter().position(|entry| {
                    let (ot, of, _) = entry.unwrap();
                    let other = &terms[ot];
                    other.factors.len() == term.factors.len()
                        && rest
                            .clone()
                            .zip(
                                other
                                    .factors
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, _)| *i != of)
                                    .map(|(_, f)| f),
                            )
                            .all(|(a, b)| equal(a, b))
                });
                if let Some(i) = existing {
                    let (ot, of, old) = entries[i].unwrap();
                    entries[i] = Some((ot, of, old.add(coef)?));
                } else {
                    if count == entries.len() {
                        return Err(bad("guard residual factor cap"));
                    }
                    entries[count] = Some((ti, fi, coef));
                    count += 1;
                }
            }
        }
    }
    if grouped {
        sum = affine[0].add(affine[1].mul(time)?)?;
        for (a, v) in affine[2..].iter().zip(state) {
            sum = sum.add(a.mul(*v)?)?;
        }
        for entry in &entries[..count] {
            let (ti, fi, mut v) = entry.unwrap();
            for (i, f) in terms[ti].factors.iter().enumerate() {
                if i != fi {
                    v = v.mul(factor(f, time, state)?)?;
                }
            }
            sum = sum.add(v)?;
        }
    }
    Ok(sum)
}
// One exact weighted directional derivative of the original source
// expression H - lambda^T g. Group before interval evaluation across rows,
// including identical ordered remaining factors of higher-degree terms.
fn guard_dual_direction(
    src: &DaeSource,
    guard: &DeclaredPolynomial,
    lambda: &[f64],
    time: Iv,
    state: &[Iv],
    direction: &[f64],
) -> CoreResult<Iv> {
    use crate::retro_event_poly::{AffineFactor, EventTerm};
    fn same(a: &AffineFactor, b: &AffineFactor) -> bool {
        a.constant == b.constant && a.time == b.time && a.state == b.state
    }
    fn factor(f: &AffineFactor, time: Iv, state: &[Iv]) -> CoreResult<Iv> {
        f.state
            .iter()
            .try_fold(p(f.constant)?.add(time.scale(f.time)?)?, |s, &(i, a)| {
                s.add(state[i].scale(a)?)
            })
    }
    // nx+nz<=32, nx>=1 => at most 32 polynomials of 16 terms*4 factors.
    let mut groups: [Option<(&EventTerm, usize, Iv)>; 2048] = [None; 2048];
    let mut count = 0;
    let mut affine = [zero(); 34];
    for (eq, weight) in
        std::iter::once((guard, 1.)).chain(src.g.iter().zip(lambda).map(|(g, &l)| (g, -l)))
    {
        for term in eq.terms() {
            let c = p(term.coefficient)?.scale(weight)?;
            for (fi, f) in term.factors.iter().enumerate() {
                let mut df = p(direction[0])?.scale(f.time)?;
                for &(j, a) in &f.state {
                    df = df.add(p(direction[1 + j])?.scale(a)?)?;
                }
                let coef = c.mul(df)?;
                if coef.lo == 0. && coef.hi == 0. {
                    continue;
                }
                let rest = term
                    .factors
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != fi)
                    .map(|(_, f)| f);
                if term.factors.len() <= 2 {
                    if let Some(f) = rest.clone().next() {
                        affine[0] = affine[0].add(coef.scale(f.constant)?)?;
                        affine[1] = affine[1].add(coef.scale(f.time)?)?;
                        for &(j, a) in &f.state {
                            affine[2 + j] = affine[2 + j].add(coef.scale(a)?)?;
                        }
                    } else {
                        affine[0] = affine[0].add(coef)?;
                    }
                } else {
                    let prior = groups[..count].iter().position(|entry| {
                        let (t, of, _) = entry.unwrap();
                        t.factors.len() == term.factors.len()
                            && rest
                                .clone()
                                .zip(
                                    t.factors
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| *i != of)
                                        .map(|(_, f)| f),
                                )
                                .all(|(a, b)| same(a, b))
                    });
                    if let Some(i) = prior {
                        let (t, of, old) = groups[i].unwrap();
                        groups[i] = Some((t, of, old.add(coef)?));
                    } else {
                        if count == groups.len() {
                            return Err(bad("guard dual factor capacity"));
                        }
                        groups[count] = Some((term, fi, coef));
                        count += 1;
                    }
                }
            }
        }
    }
    let mut sum = affine[0].add(affine[1].mul(time)?)?;
    for (&a, &v) in affine[2..].iter().zip(state) {
        sum = sum.add(a.mul(v)?)?;
    }
    for entry in &groups[..count] {
        let (term, fi, mut v) = entry.unwrap();
        for (j, f) in term.factors.iter().enumerate() {
            if j != fi {
                v = v.mul(factor(f, time, state)?)?;
            }
        }
        sum = sum.add(v)?;
    }
    Ok(sum)
}
// Bounded sparse multivariate polynomial identity proof. Index 0 denotes
// no variable, 1 denotes clock, 2..nx+1 denote differential coordinates.
// Each key is the sorted multiset of at most four variable indices.
// Interval coefficients are outward; only exact [0,0] is deleted.
#[derive(Clone, Copy)]
struct AffineMonomial {
    key: [u8; 4],
    coefficient: Iv,
}
const AFFINE_MONOMIAL_CAP: usize = 256;
const EMPTY_MONOMIAL: AffineMonomial = AffineMonomial {
    key: [0; 4],
    coefficient: Iv { lo: 0., hi: 0. },
};
fn affine_monomial_add(
    out: &mut [AffineMonomial; AFFINE_MONOMIAL_CAP],
    len: &mut usize,
    key: [u8; 4],
    v: Iv,
) -> CoreResult<()> {
    if v.lo == 0. && v.hi == 0. {
        return Ok(());
    }
    if let Some(i) = out[..*len].iter().position(|e| e.key == key) {
        let w = out[i].coefficient.add(v)?;
        if w.lo == 0. && w.hi == 0. {
            *len -= 1;
            out[i] = out[*len];
        } else {
            out[i].coefficient = w;
        }
    } else {
        if *len == out.len() {
            return Err(bad("affine graph monomial capacity"));
        }
        out[*len] = AffineMonomial {
            key,
            coefficient: v,
        };
        *len += 1;
    }
    Ok(())
}
struct GuardAffineGraph {
    coefficients: [[f64; 34]; 32],
}
impl GuardAffineGraph {
    fn identity(&self, src: &DaeSource) -> CoreResult<()> {
        let nx = src.nx();
        for eq in &src.g {
            let mut sum = [EMPTY_MONOMIAL; AFFINE_MONOMIAL_CAP];
            let mut sl = 0;
            let mut term = [EMPTY_MONOMIAL; AFFINE_MONOMIAL_CAP];
            let mut next = [EMPTY_MONOMIAL; AFFINE_MONOMIAL_CAP];
            for t in eq.terms() {
                let mut tl = 0;
                affine_monomial_add(&mut term, &mut tl, [0; 4], p(t.coefficient)?)?;
                for f in &t.factors {
                    let mut a = [zero(); 34];
                    a[0] = p(f.constant)?;
                    a[1] = p(f.time)?;
                    for &(j, w) in &f.state {
                        if j < nx {
                            a[j + 2] = a[j + 2].add(p(w)?)?;
                        } else {
                            for (v, &c) in a[..nx + 2]
                                .iter_mut()
                                .zip(&self.coefficients[j - nx][..nx + 2])
                            {
                                *v = v.add(p(c)?.scale(w)?)?;
                            }
                        }
                    }
                    let mut nl = 0;
                    for e in &term[..tl] {
                        for (k, &v) in a[..nx + 2].iter().enumerate() {
                            if v.lo == 0. && v.hi == 0. {
                                continue;
                            }
                            let mut key = e.key;
                            if k != 0 {
                                let i = key
                                    .iter()
                                    .position(|&x| x == 0)
                                    .ok_or_else(|| bad("affine graph degree cap"))?;
                                key[i] = k as u8;
                                key.sort_unstable();
                            }
                            affine_monomial_add(&mut next, &mut nl, key, e.coefficient.mul(v)?)?;
                        }
                    }
                    std::mem::swap(&mut term, &mut next);
                    tl = nl;
                }
                for e in &term[..tl] {
                    affine_monomial_add(&mut sum, &mut sl, e.key, e.coefficient)?;
                }
            }
            if sl != 0 {
                return Err(bad("affine graph identity not proved"));
            }
        }
        Ok(())
    }
    fn prove(src: &DaeSource, cert: &SlabCertificate) -> CoreResult<Self> {
        let nx = src.nx();
        let nz = src.nz();
        let n = nx + nz;
        if !cert.global_qualified
            || cert.x_range.len() != nx
            || cert.self_map_box.len() != nz
            || !cert.branch_defect.is_finite()
            || cert.branch_defect < 0.
            || cert.branch_defect >= 1.
        {
            return Err(bad("affine graph original branch certificate"));
        }
        // An origin jet is only a numerical proposal. It need not be a
        // physical point, and it does not confer source or root authority.
        let mut values = [zero(); 32];
        let mut jac = [[zero(); 33]; 32];
        guard_source_rows(
            &src.g,
            zero(),
            &[zero(); 32][..n],
            &mut values[..nz],
            &mut jac[..nz],
        )?;
        let mut center = [0.; 32 * 32];
        for i in 0..nz {
            for j in 0..nz {
                center[i * nz + j] = di::mid(jac[i][1 + nx + j]);
            }
        }
        let mut inv = [0.; 32 * 32];
        GuardInverseWorkspace::new().invert(nz, &center[..nz * nz], &mut inv[..nz * nz])?;
        let mut out = Self {
            coefficients: [[0.; 34]; 32],
        };
        for i in 0..nz {
            for k in 0..nx + 2 {
                let mut v = 0.;
                for j in 0..nz {
                    let b = if k == 0 { values[j] } else { jac[j][k - 1] };
                    v -= inv[i * nz + j] * di::mid(b);
                }
                if !v.is_finite() {
                    return Err(bad("affine graph nonfinite proposal"));
                }
                out.coefficients[i][k] = v;
            }
        }
        out.identity(src)?;
        // The polynomial identity holds globally. The original slab's
        // whole-branch uniqueness selects this root only if the whole graph
        // is inside its certified self-map box, for ALL t,x in the tube.
        let t = Iv::new(cert.times[0], cert.times[1])?;
        for (i, b) in cert.self_map_box.iter().enumerate() {
            let a = &out.coefficients[i];
            let mut v = p(a[0])?.add(t.scale(a[1])?)?;
            for (x, &k) in cert.x_range.iter().zip(&a[2..nx + 2]) {
                v = v.add(Iv::new(x[0], x[1])?.scale(k)?)?;
            }
            let box_ = Iv::new(b[0], b[1])?;
            if v.lo < box_.lo || v.hi > box_.hi {
                return Err(bad("affine graph leaves certified branch self-map"));
            }
        }
        Ok(out)
    }
}
fn guard_centered_column(
    src: &DaeSource,
    time: Iv,
    physical: &[Iv],
    k: usize,
    c: GuardMatrix<'_>,
    q: f64,
    gap: f64,
    defect: &[Iv],
    grouped: bool,
    refinement: GuardColumnRefinement,
    out: &mut [Iv],
    audit: &mut GuardKernelAudit,
) -> CoreResult<()> {
    let n = out.len();
    let mut center = [0.; 32];
    let mut residual = [zero(); 32];
    let mut correction = [zero(); 32];
    for (v, a) in center.iter_mut().zip(out.iter()) {
        *v = di::mid(*a);
    }
    observed_reset!(audit, centered_residuals, {
        for (o, g) in residual[..n].iter_mut().zip(&src.g) {
            *o = guard_directional_residual(g, time, physical, src.nx(), k, &center[..n], grouped)?;
        }
        Ok::<(), CoreError>(())
    })?;
    observed_reset!(
        audit,
        centered_corrections,
        guard_solve_into(
            c,
            &residual[..n],
            q,
            gap,
            &mut correction[..n],
            defect,
            refinement,
            &mut audit.refinement_sweeps
        )
    )?;
    for i in 0..n {
        out[i] = intersect(out[i], p(center[i])?.add(correction[i])?)?;
    }
    Ok(())
}
fn guard_row_norm(row: &[Iv]) -> CoreResult<f64> {
    Ok(0.0_f64.max(row.iter().try_fold(0., |s, v| add_up(s, v.mag()))?))
}
/// Bounded storage for the original-source interval linear kernel.
/// This is a conditional linear enclosure, not a source-bound slab certificate.
/// It does not retain a cached preconditioner or confer physical event authority.
pub struct GuardColumnWorkspace {
    values: [Iv; 32],
    jacobian: [[Iv; 33]; 32],
    defect: [Iv; 32 * 32],
}
impl GuardColumnWorkspace {
    pub const fn new() -> Self {
        const ZERO: Iv = Iv { lo: 0., hi: 0. };
        Self {
            values: [ZERO; 32],
            jacobian: [[ZERO; 33]; 32],
            defect: [ZERO; 32 * 32],
        }
    }
    /// Re-evaluate the declared constraint source on the supplied box and
    /// certify the supplied RHS for every admissible g_z. The entire output
    /// slice is committed only on success. Up to two optional refinements.
    /// The caller is responsible for any physical interpretation of this box.
    #[allow(clippy::too_many_arguments)]
    pub fn enclose(
        &mut self,
        src: &DaeSource,
        time: Iv,
        state: &[Iv],
        c: &DenseMatrix,
        rhs: &[Iv],
        sweeps: usize,
        out: &mut [Iv],
        work: &mut di::AttemptCount,
    ) -> CoreResult<f64> {
        self.enclose_refined(
            src,
            time,
            state,
            c,
            rhs,
            GuardColumnRefinement::jacobi(sweeps)?,
            out,
            work,
        )
    }
    /// Same conditional source-bound linear kernel with a typed refinement policy.
    #[allow(clippy::too_many_arguments)]
    pub fn enclose_refined(
        &mut self,
        src: &DaeSource,
        time: Iv,
        state: &[Iv],
        c: &DenseMatrix,
        rhs: &[Iv],
        refinement: GuardColumnRefinement,
        out: &mut [Iv],
        work: &mut di::AttemptCount,
    ) -> CoreResult<f64> {
        let nx = src.nx();
        let nz = src.nz();
        if state.len() != nx + nz
            || c.nrows() != nz
            || c.ncols() != nz
            || rhs.len() != nz
            || out.len() != nz
        {
            return Err(bad("guard column workspace shape"));
        }
        for v in state.iter().chain(rhs).chain(std::iter::once(&time)) {
            Iv::new(v.lo, v.hi)?;
        }
        if !c.as_slice().iter().all(|v| v.is_finite()) {
            return Err(bad("guard column preconditioner nonfinite"));
        }
        let gj = &mut self.jacobian[..nz];
        guard_source_rows(&src.g, time, state, &mut self.values[..nz], gj)?;
        let defect = &mut self.defect[..nz * nz];
        let c = GuardMatrix::new(nz, c.as_slice())?;
        let q = guard_defect_into(c, gj, nx, defect)?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("guard inverse defect"));
        }
        let mut staged = [zero(); 32];
        guard_solve_into(c, rhs, q, gap, &mut staged[..nz], defect, refinement, work)?;
        out.copy_from_slice(&staged[..nz]);
        Ok(q)
    }
}
impl Default for GuardColumnWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> GuardProofCore<'a> {
    pub fn new(
        src: &DaeSource,
        guard: &'a DeclaredPolynomial,
        y: &'a [Poly],
        cert: &SlabCertificate,
        derivative: GuardDerivativeRoute,
        value: GuardValueRoute,
    ) -> CoreResult<Self> {
        Self::new_recorded(
            src,
            guard,
            y,
            cert,
            derivative,
            value,
            &mut GuardKernelAudit::default(),
        )
    }
    /// Counts actual guard construction boundaries, even on failure.
    pub fn new_recorded(
        src: &DaeSource,
        guard: &'a DeclaredPolynomial,
        y: &'a [Poly],
        cert: &SlabCertificate,
        derivative: GuardDerivativeRoute,
        value: GuardValueRoute,
        audit: &mut GuardKernelAudit,
    ) -> CoreResult<Self> {
        let r = Self::build_recorded(src, guard, y, cert, derivative, value, audit);
        audit.builds.record(r.is_ok());
        r
    }
    fn build_recorded(
        src: &DaeSource,
        guard: &'a DeclaredPolynomial,
        y: &'a [Poly],
        cert: &SlabCertificate,
        derivative: GuardDerivativeRoute,
        value: GuardValueRoute,
        audit: &mut GuardKernelAudit,
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
        let mut xb = guard_pairs(&cert.x_range)?;
        let zb = guard_pairs(&cert.z_range)?;
        let mut rb = guard_pairs(&cert.branch_root_box)?;
        let structural = cert.conservation.is_some() && nx == 2 && nz == 1;
        let tight = derivative.conserved();
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
            let lift = guard_lift(y)?;
            // Exact affine lift: |delta z| <= ||delta x||_1, not 2r.
            rb[0] = intersect(rb[0], range(&lift)?.add(err)?)?;
        }
        let mut physical_storage = [zero(); 32];
        let physical = &mut physical_storage[..nx + nz];
        physical[..nx].copy_from_slice(&xb[..nx]);
        physical[nx..].copy_from_slice(&rb[..nz]);
        let mut fv_storage = [zero(); 32];
        let fv = &mut fv_storage[..nx];
        let mut unused_row = [zero(); 33];
        // f derivatives are checked but need not be retained.
        observed_reset!(audit, evaluations, {
            for (eq, v) in src.f.iter().zip(fv.iter_mut()) {
                *v = di::evaluate_row_bounded(eq, time, physical, &mut unused_row[..=nx + nz])?;
            }
            Ok::<(), CoreError>(())
        })?;
        let mut linear = GuardColumnWorkspace::new();
        let gj = &mut linear.jacobian[..nz];
        observed_reset!(
            audit,
            evaluations,
            guard_source_rows(&src.g, time, physical, &mut linear.values[..nz], gj)
        )?;
        let mut h_storage = [zero(); 33];
        let h = &mut h_storage[..=nx + nz];
        observed_reset!(
            audit,
            evaluations,
            di::evaluate_row_bounded(guard, time, physical, h)
        )?;
        let original = GuardMatrix::new(nz, &cert.preconditioner)?;
        let mut work = InitWork::default();
        let prepared;
        let mut bounded_inverse = [0.; 32 * 32];
        let c = if derivative.bounded_prepared() {
            let mut center = [0.; 32 * 32];
            for (i, row) in gj.iter().enumerate() {
                for j in 0..nz {
                    center[i * nz + j] = di::mid(row[1 + nx + j]);
                }
            }
            audit.rhs_requested += nz;
            work.linear_batches += 1;
            work.rhs_count += nz;
            let r = GuardInverseWorkspace::new().invert(
                nz,
                &center[..nz * nz],
                &mut bounded_inverse[..nz * nz],
            );
            audit.preconditioners.record(r.is_ok());
            audit.bounded_inverse.record(r.is_ok());
            if r.is_ok() {
                audit.rhs_returned += nz;
                let nnz = center[..nz * nz].iter().filter(|&&x| x != 0.).count();
                audit.bounded_residual_terms += nz * if nnz <= nz * nz / 2 { nnz } else { nz * nz };
            }
            r?;
            GuardMatrix::new(nz, &bounded_inverse[..nz * nz])?
        } else if derivative.prepared() {
            let center = DenseMatrix::new(
                nz,
                nz,
                gj.iter()
                    .flat_map(|r| r[1 + nx..=nx + nz].iter().copied().map(di::mid))
                    .collect(),
            )?;
            audit.rhs_requested += nz;
            let r = di::preconditioner(
                &center,
                InitRoute::PreparedBatch,
                &ParallelExecution::sequential(),
                &mut work,
            );
            audit.preconditioners.record(r.is_ok());
            if r.is_ok() {
                audit.rhs_returned += nz;
            }
            prepared = r?;
            GuardMatrix::new(nz, prepared.as_slice())?
        } else {
            original
        };
        let defect = &mut linear.defect[..nz * nz];
        let q = guard_defect_into(c, gj, nx, defect)?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("guard inverse defect"));
        }
        // DaeSource owns nx+nz<=32. Keep all directed operations and their
        // order, but reuse bounded column storage rather than three Vecs per
        // solve. GuardSlabEvidence still owns its public output.
        let mut rhs_storage = [zero(); 32];
        let mut solved_storage = [zero(); 32];
        let mut reduced_storage = [zero(); 33];
        let rhs = &mut rhs_storage[..nz];
        let solved = &mut solved_storage[..nz];
        let reduced = &mut reduced_storage[..=nx];
        let solve = |rhs: &[Iv],
                     out: &mut [Iv],
                     k: Option<usize>,
                     audit: &mut GuardKernelAudit|
         -> CoreResult<()> {
            observed_reset!(
                audit,
                reduced_columns,
                guard_solve_into(
                    c,
                    rhs,
                    q,
                    gap,
                    out,
                    defect,
                    derivative.refinement(),
                    &mut audit.refinement_sweeps
                )
            )?;
            if let (Some(k), Some((grouped, refinement))) = (k, derivative.centered()) {
                guard_centered_column(
                    src, time, physical, k, c, q, gap, defect, grouped, refinement, out, audit,
                )?;
            }
            Ok(())
        };
        let hz = &h[1 + nx..];
        let graph = if derivative.affine_graph() {
            Some(observed_reset!(
                audit,
                affine_graph_proofs,
                GuardAffineGraph::prove(src, cert)
            )?)
        } else {
            None
        };
        let dual = derivative.dual();
        let mut lambda = [0.; 32];
        let mut rz = [zero(); 32];
        let mut direction = [0.; 33];
        if dual.is_some() {
            // lambda is only a finite point candidate; it need not be a
            // certified adjoint solution. Exact identity holds for any lambda.
            for j in 0..nz {
                let mut v = 0.;
                for i in 0..nz {
                    v += c.row(i)[j] * di::mid(hz[i]);
                }
                if !v.is_finite() {
                    return Err(bad("guard dual center nonfinite"));
                }
                lambda[j] = v;
            }
            for j in 0..nz {
                direction[1 + nx + j] = 1.;
                rz[j] = observed_reset!(
                    audit,
                    dual_source_evals,
                    guard_dual_direction(
                        src,
                        guard,
                        &lambda[..nz],
                        time,
                        physical,
                        &direction[..=nx + nz]
                    )
                )?;
                direction[1 + nx + j] = 0.;
            }
        }
        for (k, &hk) in h[..=nx].iter().enumerate() {
            for (v, r) in rhs.iter_mut().zip(gj.iter()) {
                *v = -r[k];
            }
            solve(rhs, solved, Some(k), audit)?;
            reduced[k] = hk.add(guard_dot(hz, solved)?)?;
            if let Some(graph) = &graph {
                let mut slope = [0.; 32];
                for (i, v) in slope[..nz].iter_mut().enumerate() {
                    *v = graph.coefficients[i][k + 1];
                }
                // r=-D H, evaluated on the same full certified physical box.
                let r = observed_reset!(
                    audit,
                    affine_graph_directions,
                    guard_directional_residual(guard, time, physical, nx, k, &slope[..nz], true)
                )?;
                reduced[k] = intersect(reduced[k], -r)?;
            }
            if let Some(centered) = dual {
                direction.fill(0.);
                direction[k] = 1.;
                let mut delta = [zero(); 32];
                for j in 0..nz {
                    let m = if centered { di::mid(solved[j]) } else { 0. };
                    direction[1 + nx + j] = m;
                    delta[j] = solved[j].sub(p(m)?)?;
                }
                let r = observed_reset!(
                    audit,
                    dual_source_evals,
                    guard_dual_direction(
                        src,
                        guard,
                        &lambda[..nz],
                        time,
                        physical,
                        &direction[..=nx + nz]
                    )
                )?;
                reduced[k] = observed_reset!(
                    audit,
                    dual_intersections,
                    intersect(reduced[k], r.add(guard_dot(&rz[..nz], &delta[..nz])?)?)
                )?;
            }
        }
        let velocity = if derivative == GuardDerivativeRoute::Tangent {
            for (v, r) in rhs.iter_mut().zip(gj.iter()) {
                *v = -r[0].add(guard_dot(&r[1..1 + nx], fv)?)?;
            }
            let partial = h[0].add(guard_dot(&h[1..1 + nx], fv)?)?;
            solve(rhs, solved, None, audit)?;
            partial.add(guard_dot(hz, solved)?)?
        } else {
            reduced[0].add(guard_dot(&reduced[1..], fv)?)?
        };
        // The structural receipt owns a reduced L1 radius. Its dual
        // norm is the maximum absolute reduced gradient component.
        let reduced_lip = if structural {
            reduced[1..].iter().map(|v| v.mag()).fold(0., f64::max)
        } else {
            guard_row_norm(&reduced[1..])?
        };
        // The straight segment between represented V and true Z(t,U)
        // lies in this hull; use it for the algebraic correction only.
        let mut hull_storage = [zero(); 32];
        let hull = &mut hull_storage[..nx + nz];
        hull[..nx].copy_from_slice(&xb[..nx]);
        for (o, (&v, &r)) in hull[nx..].iter_mut().zip(zb.iter().zip(&rb)) {
            *o = Iv::new(v.lo.min(r.lo), v.hi.max(r.hi))?;
        }
        let mut hh = [zero(); 33];
        observed_reset!(
            audit,
            evaluations,
            di::evaluate_row_bounded(guard, time, hull, &mut hh[..=nx + nz])
        )?;
        let algebraic_guard_lip = guard_row_norm(&hh[1 + nx..=nx + nz])?;
        let clock = [p(cert.times[0])?, p(cert.times[1])?.sub(p(cert.times[0])?)?];
        let (h_poly, algebraic_guard_lip) = if structural && value == GuardValueRoute::Transport {
            // The source-owned Robertson constraint is exact affine
            // elimination. The comparison polynomial lives on the same
            // constraint manifold, so no separate algebraic defect term.
            (guard_polynomial(guard, &clock, y, true)?, 0.)
        } else {
            (
                guard_polynomial(guard, &clock, y, false)?,
                algebraic_guard_lip,
            )
        };
        let cg_poly = guard_cg(src, &clock, y, original)?;
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
            evidence: GuardInlineEvidence {
                times: cert.times,
                derivative_route: derivative,
                value_route: value,
                velocity: velocity.as_pair(),
                nx,
                n: nx + nz,
                reduced_gradient: {
                    let mut a = [[0.; 2]; 32];
                    for (o, v) in a.iter_mut().zip(&reduced[1..]) {
                        *o = v.as_pair();
                    }
                    a
                },
                reduced_clock: reduced[0].as_pair(),
                inverse_defect: q,
                reduced_lip,
                algebraic_guard_lip,
                prepared_work: work,
                conserved_derivative_box: tight.then(|| {
                    let mut a = [[0.; 2]; 32];
                    for (o, v) in a.iter_mut().zip(physical.iter()) {
                        *o = v.as_pair();
                    }
                    a
                }),
            },
        })
    }
    /// Allocating compatibility report, separated from the core computation.
    pub fn owned_evidence(&self) -> GuardSlabEvidence {
        self.evidence.owned()
    }
    pub fn velocity(&self) -> [f64; 2] {
        self.evidence.velocity
    }
    fn algebraic_radius(&self, u: Iv) -> CoreResult<f64> {
        let n = self.cg_poly.norm_at(u)?;
        div_up(n, self.gap)
    }
    fn box_value(&self, u: Iv, clock: Iv, error: f64, az: f64) -> CoreResult<Iv> {
        let n = self.y.len();
        if n > 32 {
            return Err(bad("guard scratch shape"));
        }
        let ez = add_up(az, mul_up(self.zx_lip, error)?)?;
        let mut values = [zero(); 32];
        for (i, v) in self.y.iter().enumerate() {
            let a = eval(v, u)?;
            values[i] = if i < self.nx {
                intersect(a.add(Iv::new(-error, error)?)?, self.x_range[i])?
            } else {
                let k = i - self.nx;
                let a = intersect(a, self.z_range[k])?;
                intersect(a.add(Iv::new(-ez, ez)?)?, self.root_box[k])?
            };
        }
        // Preserve all derivative evaluation, including errors, even
        // though this query only consumes the value.
        let mut row = [zero(); 33];
        di::evaluate_row_bounded(self.guard, clock, &values[..n], &mut row[..=n])
    }
    /// Only an advisory polynomial, never an authority on physical roots.
    pub fn hint_coefficients(&self) -> Vec<f64> {
        self.h_poly.iter().map(|&v| di::mid(v)).collect()
    }
    /// At a physical time in the certified slab; error must be an externally
    /// established differential radius (endpoint or whole-slab).
    pub fn value_at(&self, t: f64, error: f64) -> CoreResult<Iv> {
        self.value_at_with(
            t,
            error,
            self.evidence.value_route,
            self.evidence.reduced_lip,
            self.evidence.algebraic_guard_lip,
        )
    }
    fn value_at_with(
        &self,
        t: f64,
        error: f64,
        value_route: GuardValueRoute,
        reduced_lip: f64,
        algebraic_guard_lip: f64,
    ) -> CoreResult<Iv> {
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
        let az = self.algebraic_radius(u)?;
        match value_route {
            GuardValueRoute::Transport => {
                let r = add_up(
                    mul_up(reduced_lip, error)?,
                    mul_up(algebraic_guard_lip, az)?,
                )?;
                eval(&self.h_poly, u)?.add(Iv::new(-r, r)?)
            }
            GuardValueRoute::Box => self.box_value(u, p(t)?, error, az),
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
    let proof = GuardProofCore::new(src, reset, y, cert, derivative, value)?;
    let h = p(cert.times[1])?.sub(p(cert.times[0])?)?;
    let clock = Iv::new(times[0], times[1])?;
    let normalize = |t: Iv| -> CoreResult<Iv> {
        let u = t.sub(p(cert.times[0])?)?.div(h)?;
        Iv::new(u.lo.max(0.), u.hi.min(1.))
    };
    let u = normalize(clock)?;
    let ex = proof.whole_x;
    let az = proof.algebraic_radius(u)?;
    let er = add_up(
        mul_up(proof.evidence.reduced_lip, ex)?,
        mul_up(proof.evidence.algebraic_guard_lip, az)?,
    )?;
    let r = if value == GuardValueRoute::Box {
        proof.box_value(u, clock, ex, az)?
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
        let mut shifted = proof.h_poly.to_vec();
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
/// Observed Result boundaries, not internal arithmetic or physical authority.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GuardKernelAudit {
    pub builds: di::AttemptCount,
    pub evaluations: di::AttemptCount,
    pub preconditioners: di::AttemptCount,
    pub rhs_requested: usize,
    pub rhs_returned: usize,
    pub reduced_columns: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub refinement_sweeps: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub centered_residuals: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub centered_corrections: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub bounded_inverse: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_usize_zero")]
    pub bounded_residual_terms: usize,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub dual_source_evals: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub dual_intersections: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub affine_graph_proofs: di::AttemptCount,
    #[serde(skip_serializing_if = "guard_attempt_empty")]
    pub affine_graph_directions: di::AttemptCount,
}
fn guard_usize_zero(v: &usize) -> bool {
    *v == 0
}
fn guard_attempt_empty(w: &di::AttemptCount) -> bool {
    w.started == 0 && w.returned == 0
}
impl GuardKernelAudit {
    pub fn merge(&mut self, b: &Self) {
        self.builds.merge(b.builds);
        self.evaluations.merge(b.evaluations);
        self.preconditioners.merge(b.preconditioners);
        self.rhs_requested += b.rhs_requested;
        self.rhs_returned += b.rhs_returned;
        self.reduced_columns.merge(b.reduced_columns);
        self.refinement_sweeps.merge(b.refinement_sweeps);
        self.centered_residuals.merge(b.centered_residuals);
        self.centered_corrections.merge(b.centered_corrections);
        self.bounded_inverse.merge(b.bounded_inverse);
        self.bounded_residual_terms += b.bounded_residual_terms;
        self.dual_source_evals.merge(b.dual_source_evals);
        self.dual_intersections.merge(b.dual_intersections);
        self.affine_graph_proofs.merge(b.affine_graph_proofs);
        self.affine_graph_directions
            .merge(b.affine_graph_directions);
    }
}
/// Retained separately from an owning vector certificate, even on Err.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ResetKernelAudit {
    pub components: di::AttemptCount,
    pub component_guards: GuardKernelAudit,
    pub branches: di::AttemptCount,
    pub branch_initialization: di::ProjectionAttemptWork,
    pub branch_evaluations: di::AttemptCount,
    pub branch_preconditioners: di::AttemptCount,
    pub branch_rhs_requested: usize,
    pub branch_rhs_returned: usize,
    pub branch_columns: di::AttemptCount,
    pub tubes: di::AttemptCount,
    pub picard_images: di::AttemptCount,
    pub strict_images: usize,
    pub synchronizations: di::AttemptCount,
    pub sync_components: di::AttemptCount,
    pub green: di::AttemptCount,
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
    vector_reset_component_recorded(
        src,
        reset,
        y,
        c,
        times,
        route,
        derivative,
        &mut ResetKernelAudit::default(),
    )
}
pub fn vector_reset_component_recorded(
    src: &DaeSource,
    reset: &DeclaredPolynomial,
    y: &[Poly],
    c: &SlabCertificate,
    times: [f64; 2],
    route: VectorSyncRoute,
    derivative: GuardDerivativeRoute,
    audit: &mut ResetKernelAudit,
) -> CoreResult<VectorResetComponent> {
    let r = vector_reset_component_inner(src, reset, y, c, times, route, derivative, audit);
    audit.components.record(r.is_ok());
    r
}
fn vector_reset_component_inner(
    src: &DaeSource,
    reset: &DeclaredPolynomial,
    y: &[Poly],
    c: &SlabCertificate,
    times: [f64; 2],
    route: VectorSyncRoute,
    derivative: GuardDerivativeRoute,
    audit: &mut ResetKernelAudit,
) -> CoreResult<VectorResetComponent> {
    if !(c.times[0] <= times[0] && times[0] <= times[1] && times[1] <= c.times[1]) {
        return Err(bad("vector reset slab domain"));
    }
    let value = if route == VectorSyncRoute::BoxPicard {
        GuardValueRoute::Box
    } else {
        GuardValueRoute::Transport
    };
    let proof = GuardProofCore::new_recorded(
        src,
        reset,
        y,
        c,
        derivative,
        value,
        &mut audit.component_guards,
    )?;
    let clock = Iv::new(times[0], times[1])?;
    let h = p(c.times[1])?.sub(p(c.times[0])?)?;
    let u = clock.sub(p(c.times[0])?)?.div(h)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let az = proof.algebraic_radius(u)?;
    let ex = proof.whole_x;
    let er = add_up(
        mul_up(proof.evidence.reduced_lip, ex)?,
        mul_up(proof.evidence.algebraic_guard_lip, az)?,
    )?;
    let r = if value == GuardValueRoute::Box {
        proof.box_value(u, clock, ex, az)?
    } else {
        eval(&proof.h_poly, u)?.add(Iv::new(-er, er)?)?
    };
    Ok(VectorResetComponent {
        enclosure: r.as_pair(),
        transport_error: er,
        reduced_lip: proof.evidence.reduced_lip,
        prepared_work: proof.evidence.prepared_work,
        power: proof.h_poly.to_vec(),
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
    certify_vector_branch_recorded(
        src,
        times,
        xb,
        center,
        z_seed,
        branch,
        policy,
        route,
        &mut ResetKernelAudit::default(),
    )
}
#[allow(clippy::too_many_arguments)]
pub fn certify_vector_branch_recorded(
    src: &DaeSource,
    times: [f64; 2],
    xb: &[[f64; 2]],
    center: &[f64],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    audit: &mut ResetKernelAudit,
) -> CoreResult<VectorBranchProof> {
    let r =
        certify_vector_branch_inner(src, times, xb, center, z_seed, branch, policy, route, audit);
    audit.branches.record(r.is_ok());
    r
}
fn certify_vector_branch_inner(
    src: &DaeSource,
    times: [f64; 2],
    xb: &[[f64; 2]],
    center: &[f64],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    audit: &mut ResetKernelAudit,
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
            InitRoute::Reuse
                | InitRoute::PreparedBatch
                | InitRoute::ScalarRoot
                | InitRoute::AffineExact
        )
    {
        return Err(bad("vector uniform branch contract"));
    }
    let (initial, w) = src.initialize_audited(times[1], center, z_seed, branch, policy, route);
    audit.branch_initialization.record(&w, initial.is_ok());
    let initial = initial?;
    let b = iv_pairs(branch)?;
    let zc = di::points(&initial.z)?;
    let mut point = di::points(center)?;
    point.extend_from_slice(&zc);
    let jp = jz(
        &observed_reset!(
            audit,
            branch_evaluations,
            di::evaluate(&src.g, p(times[1])?, &point)
        )?,
        nx,
    );
    let mut work = InitWork::default();
    let a = di::center_matrix(&jp)?;
    audit.branch_rhs_requested += nz;
    let c = di::preconditioner(&a, route, &ParallelExecution::sequential(), &mut work);
    audit.branch_preconditioners.record(c.is_ok());
    if c.is_ok() {
        audit.branch_rhs_returned += nz;
    }
    let c = c?;
    let mut full = x.clone();
    full.extend_from_slice(&b);
    let j = jz(
        &observed_reset!(audit, branch_evaluations, di::evaluate(&src.g, t, &full))?,
        nx,
    );
    let err = di::defect(&c, &j)?;
    let q = di::rows_norm(&err)?;
    let gap = sub_down(1., q)?;
    if gap <= 0. {
        return Err(bad("vector uniform branch contraction not proved"));
    }
    let mut base = x.clone();
    base.extend_from_slice(&zc);
    let cg = di::matvec(
        &c,
        &observed_reset!(audit, branch_evaluations, di::evaluate(&src.g, t, &base))?.v,
    )?;
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
            let el = di::defect(
                &c,
                &jz(
                    &observed_reset!(audit, branch_evaluations, di::evaluate(&src.g, t, &v))?,
                    nx,
                ),
            )?;
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
    let g = observed_reset!(audit, branch_evaluations, di::evaluate(&src.g, t, &y))?;
    let f = observed_reset!(audit, branch_evaluations, di::evaluate(&src.f, t, &y))?;
    let qr = di::rows_norm(&di::defect(&c, &jz(&g, nx))?)?;
    let gr = sub_down(1., qr)?;
    if gr <= 0. {
        return Err(bad("vector reduced inverse defect"));
    }
    let mut columns = Vec::with_capacity(nx + 1);
    for k in 0..=nx {
        let rhs = g.j.iter().map(|r| -r[k]).collect::<Vec<_>>();
        columns.push(observed_reset!(
            audit,
            branch_columns,
            product_bound(qr, gr, &di::matvec(&c, &rhs)?)
        )?);
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
    let f0 = observed_reset!(
        audit,
        branch_evaluations,
        di::evaluate(&src.f, p(times[1])?, &nominal)
    )?
    .v;
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
    certify_vector_tube_recorded(
        src,
        times,
        r,
        z_seed,
        branch,
        policy,
        route,
        &mut ResetKernelAudit::default(),
    )
}
pub fn certify_vector_tube_audited(
    src: &DaeSource,
    times: [f64; 2],
    r: &[[f64; 2]],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
) -> (CoreResult<VectorTubeProof>, ResetKernelAudit) {
    let mut a = ResetKernelAudit::default();
    let r = certify_vector_tube_recorded(src, times, r, z_seed, branch, policy, route, &mut a);
    (r, a)
}
pub fn certify_vector_tube_recorded(
    src: &DaeSource,
    times: [f64; 2],
    r: &[[f64; 2]],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    audit: &mut ResetKernelAudit,
) -> CoreResult<VectorTubeProof> {
    let r = certify_vector_tube_inner(src, times, r, z_seed, branch, policy, route, audit);
    audit.tubes.record(r.is_ok());
    r
}
fn certify_vector_tube_inner(
    src: &DaeSource,
    times: [f64; 2],
    r: &[[f64; 2]],
    z_seed: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    audit: &mut ResetKernelAudit,
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
        let proof = certify_vector_branch_recorded(
            src, times, &xb, &center, z_seed, branch, policy, route, audit,
        )?;
        let field = iv_pairs(&proof.field)?;
        let image = observed_reset!(
            audit,
            picard_images,
            reset
                .iter()
                .zip(&field)
                .map(|(&r, &f)| r.add(elapsed.mul(f)?))
                .collect::<CoreResult<Vec<_>>>()
        )?;
        let admitted = image
            .iter()
            .zip(&b)
            .all(|(a, b)| b.lo < a.lo && a.hi < b.hi);
        audit.strict_images += usize::from(admitted);
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
    // Evaluate derivative coefficients in the old forward order, then
    // use the identical reverse Horner recurrence. Longer inputs retain
    // the prior owned derivative path.
    let mut d = [zero(); POLY_SCRATCH];
    let owned;
    let d = if poly.len() <= POLY_SCRATCH {
        for (k, v) in poly.iter().enumerate().skip(1) {
            d[k - 1] = v.scale(k as f64)?;
        }
        &d[..poly.len().saturating_sub(1)]
    } else {
        owned = poly
            .iter()
            .enumerate()
            .skip(1)
            .map(|(k, v)| v.scale(k as f64))
            .collect::<CoreResult<Vec<_>>>()?;
        owned.as_slice()
    };
    let tight = eval(poly, center)?.add(u.sub(center)?.mul(eval(d, u)?)?)?;
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
    synchronize_vector_recorded(
        reset,
        times,
        s,
        tube,
        route,
        table,
        &mut ResetKernelAudit::default(),
    )
}
pub fn synchronize_vector_recorded(
    reset: &[VectorResetComponent],
    times: [f64; 2],
    s: f64,
    tube: &VectorTubeProof,
    route: VectorSyncRoute,
    table: &RetroTable,
    audit: &mut ResetKernelAudit,
) -> CoreResult<VectorSyncEvidence> {
    let r = synchronize_vector_inner(reset, times, s, tube, route, table, audit);
    audit.synchronizations.record(r.is_ok());
    r
}
fn synchronize_vector_inner(
    reset: &[VectorResetComponent],
    times: [f64; 2],
    s: f64,
    tube: &VectorTubeProof,
    route: VectorSyncRoute,
    table: &RetroTable,
    audit: &mut ResetKernelAudit,
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
        let result = (|| -> CoreResult<Iv> {
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
                    let mut residual =
                        Iv::new(proof.nominal_field[i][0], proof.nominal_field[i][1])?
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
                    let gb = observed_reset!(audit, green, table.phi(db, 0))?;
                    let yb = e.add(gb.mul(eval(&shifted, ub)?)?)?;
                    let d = b.sub(clock)?;
                    let d = Iv::new(d.lo.max(0.), d.hi)?;
                    let weight = gb
                        .mul(d)?
                        .mul(observed_reset!(audit, green, table.phi(d, 1))?)?;
                    let ge = observed_reset!(audit, green, table.phi(elapsed, 0))?;
                    let err = mul_up(ge.hi, r.transport_error)?;
                    let forcing = elapsed
                        .mul(observed_reset!(audit, green, table.phi(elapsed, 1))?)?
                        .mul(remainder)?;
                    out.green_calls += 4;
                    let corrected = yb
                        .sub(weight.mul(q)?)?
                        .add(Iv::new(-err, err)?)?
                        .add(forcing)?;
                    intersect(rect, corrected)?
                }
            };
            intersect(v, xb[i])
        })();
        audit.sync_components.record(result.is_ok());
        out.synchronized.push(result?.as_pair());
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
                let vminus = (-range(conservation_restrict(&y[1], l, w)?.slice())?.lo).max(0.);
                let zminus = (-range(conservation_restrict(&lift, l, w)?.slice())?.lo).max(0.);
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
                    let q = conservation_restrict(d, l, w)?;
                    let m = bernstein_magnitude(q.slice(), route != ConservationRoute::Max)?;
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
fn conservation_restrict(a: &[Iv], l: f64, w: f64) -> CoreResult<ScratchPoly> {
    let mut out = ScratchPoly::new(&[*a
        .last()
        .ok_or_else(|| bad("empty conservation polynomial"))?])?;
    let v = [p(l)?, p(w)?];
    for &c in a[..a.len() - 1].iter().rev() {
        out = out.mul(&v)?.add(&[c])?;
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
        .try_fold(0_f64, |s, (&a, &b)| {
            Ok::<f64, CoreError>(s.max(a.sub(p(b)?)?.mag()))
        })?;
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

#[cfg(test)]
mod bounded_polynomial_scratch_tests {
    use super::*;
    use crate::retro_event_poly::{AffineFactor as F, EventTerm as T};
    fn same(a: &[Iv], b: &[Iv]) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x.lo.to_bits(), y.lo.to_bits());
            assert_eq!(x.hi.to_bits(), y.hi.to_bits());
        }
    }
    fn old_restrict(a: &[Iv], l: f64, w: f64) -> CoreResult<Poly> {
        let mut out = vec![*a.last().unwrap()];
        let v = [p(l)?, p(w)?];
        for &c in a[..a.len() - 1].iter().rev() {
            out = add(&mul(&out, &v)?, &[c])?;
        }
        Ok(out)
    }
    fn old_bernstein(a: &[Iv]) -> CoreResult<Poly> {
        let n = a.len() - 1;
        (0..=n)
            .map(|i| {
                (0..=i).try_fold(zero(), |s, j| {
                    s.add(a[j].mul(p(choose(i, j) as f64)?.div(p(choose(n, j) as f64)?)?)?)
                })
            })
            .collect()
    }
    #[test]
    fn exact_operation_order_and_capacity() {
        let a = F {
            constant: 0.125,
            time: -0.5,
            state: vec![(0, 0.75), (1, -0.25), (0, 0.125)],
        };
        let q = DeclaredPolynomial::new(vec![
            T {
                coefficient: 1. / 3.,
                factors: vec![a.clone(), a.clone(), a.clone(), a],
            },
            T {
                coefficient: -0.25,
                factors: vec![],
            },
        ])
        .unwrap();
        q.validate(2).unwrap();
        let t = [p(0.125).unwrap(), p(0.25).unwrap()];
        let mut checked = 0;
        // Includes non-degenerate interval coefficients, all native
        // polynomial lengths, and the old longer-row fallback.
        for n in 1..=6 {
            for k in 0..8 {
                let y = (0..2)
                    .map(|j| {
                        (0..n)
                            .map(|i| {
                                let x = ((k + 1) * (i + 2 + j)) as f64 / 17. - 0.5;
                                Iv::new(x.next_down(), x.next_up()).unwrap()
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let b = compose_owned(&q, &t, &y).unwrap();
                let c = compose(&q, &t, &y).unwrap();
                same(&b, &c);
                checked += b.len() * 2;
                let old = old_bernstein(&b).unwrap();
                let r = range(&c).unwrap();
                same(
                    &[r],
                    &[Iv::new(
                        old.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min),
                        old.iter().map(|v| v.hi).fold(f64::NEG_INFINITY, f64::max),
                    )
                    .unwrap()],
                );
                let max = old.iter().map(Iv::mag).fold(0_f64, f64::max);
                let mean = div_up(
                    old.iter()
                        .try_fold(0_f64, |s, v| add_up(s, v.mag()))
                        .unwrap(),
                    old.len() as f64,
                )
                .unwrap();
                assert_eq!(
                    bernstein_magnitude(&c, false).unwrap().to_bits(),
                    max.to_bits()
                );
                assert_eq!(
                    bernstein_magnitude(&c, true).unwrap().to_bits(),
                    mean.to_bits()
                );
                if c.len() <= POLY_SCRATCH {
                    for m in [1, 4, 16] {
                        let w = 1. / m as f64;
                        for i in 0..m {
                            let l = i as f64 * w;
                            same(
                                conservation_restrict(&c, l, w).unwrap().slice(),
                                &old_restrict(&b, l, w).unwrap(),
                            );
                            checked += 2 * b.len();
                        }
                    }
                }
                let u = Iv::new(0.25, 0.75).unwrap();
                let center = p(di::mid(u)).unwrap();
                let d = c
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(k, v)| v.scale(k as f64))
                    .collect::<CoreResult<Vec<_>>>()
                    .unwrap();
                let tight = eval(&c, center)
                    .unwrap()
                    .add(u.sub(center).unwrap().mul(eval(&d, u).unwrap()).unwrap())
                    .unwrap();
                same(
                    &[reset_centered_eval(&c, u).unwrap()],
                    &[intersect(eval(&c, u).unwrap(), tight).unwrap()],
                );
            }
        }
        assert!(ScratchPoly::new(&[zero(); 18]).is_err());
        assert!(conservation_restrict(&[], 0., 1.).is_err());
        assert!(range(&[]).is_err());
        println!("POLYNOMIAL_BIT_CHECKS {checked}");
    }
}

#[cfg(test)]
mod guard_defect_scratch_contracts {
    use super::*;
    use crate::retro_event_poly::{AffineFactor as F, EventTerm as T};
    fn same(a: Iv, b: Iv) {
        assert_eq!(
            (a.lo.to_bits(), a.hi.to_bits()),
            (b.lo.to_bits(), b.hi.to_bits())
        );
    }
    #[test]
    fn directed_source_defect_refinement_equivalence() {
        let t = Iv::new(0., 0.25).unwrap();
        let y = [Iv::new(-0.25, 0.25).unwrap(); 3];
        let tr = |coefficient, factors| T {
            coefficient,
            factors,
        };
        let g = [
            DeclaredPolynomial::new(vec![
                tr(1., vec![F::component(1, 0.)]),
                tr(1., vec![F::clock(0.), F::component(2, 0.)]),
            ])
            .unwrap(),
            DeclaredPolynomial::new(vec![
                tr(1., vec![F::component(2, 0.)]),
                tr(-0.5, vec![F::clock(0.), F::component(1, 0.)]),
            ])
            .unwrap(),
        ];
        for e in &g {
            e.validate(3).unwrap();
        }
        let c = DenseMatrix::identity(2);
        let old = di::evaluate(&g, t, &y).unwrap();
        let j = jz(&old, 1);
        let defect = di::defect(&c, &j).unwrap();
        let q0 = di::rows_norm(&defect).unwrap();
        let rhs = [p(1.).unwrap(), p(2.).unwrap()];
        let expected = product_bound(
            q0,
            sub_down(1., q0).unwrap(),
            &di::matvec(&c, &rhs).unwrap(),
        )
        .unwrap();
        let mut v = [zero(); 2];
        let mut jac = [[zero(); 33]; 2];
        let mut e = [zero(); 4];
        let mut base = [zero(); 2];
        let mut one = [zero(); 2];
        let mut two = [zero(); 2];
        let mut audit = [di::AttemptCount::default(); 3];
        let q = {
            guard_source_rows(&g, t, &y, &mut v, &mut jac).unwrap();
            let q = guard_defect_into(GuardMatrix::new(2, c.as_slice()).unwrap(), &jac, 1, &mut e)
                .unwrap();
            let gap = sub_down(1., q).unwrap();
            for (sweeps, out) in [(0, &mut base), (1, &mut one), (2, &mut two)] {
                guard_solve_into(
                    GuardMatrix::new(2, c.as_slice()).unwrap(),
                    &rhs,
                    q,
                    gap,
                    out,
                    &e,
                    GuardColumnRefinement::jacobi(sweeps).unwrap(),
                    &mut audit[sweeps],
                )
                .unwrap();
            }
            q
        };
        assert_eq!(q.to_bits(), q0.to_bits());
        for i in 0..2 {
            same(v[i], old.v[i]);
            for k in 0..4 {
                same(jac[i][k], old.j[i][k]);
            }
            for k in 0..2 {
                same(e[i * 2 + k], defect[i][k]);
            }
            same(base[i], expected[i]);
            assert!(base[i].lo <= one[i].lo && one[i].hi <= base[i].hi);
            assert!(one[i].lo <= two[i].lo && two[i].hi <= one[i].hi);
        }
        assert!(
            two.iter()
                .zip(base)
                .any(|(x, y)| x.lo > y.lo || x.hi < y.hi)
        );
        for k in 0..=64 {
            let t = k as f64 / 256.;
            let exact = [
                (1. - 2. * t) / (1. + 0.5 * t * t),
                (2. + 0.5 * t) / (1. + 0.5 * t * t),
            ];
            for i in 0..2 {
                assert!(two[i].lo <= exact[i] && exact[i] <= two[i].hi);
            }
        }
        for (k, a) in audit.iter().enumerate() {
            assert_eq!((a.started, a.returned), (k, k));
        }
        println!(
            "GUARD_KERNEL_EQUIVALENCE q={q} base={:?} one={:?} two={:?}",
            base, one, two
        );
    }
}

#[cfg(test)]
mod centered_source_residual_contracts {
    use super::*;
    use crate::retro_event_poly::{AffineFactor as F, EventTerm as T};

    #[test]
    fn repeated_degree_four_factors_preserve_exact_cancellation() {
        // g=(1+t^3)(z-x), expanded in the original declaration.
        // Independent dyadic point formula is exact for this small grid.
        let t = F::clock(0.);
        let x = F::component(0, 0.);
        let z = F::component(1, 0.);
        let g = DeclaredPolynomial::new(vec![
            T {
                coefficient: 1.,
                factors: vec![z.clone()],
            },
            T {
                coefficient: -1.,
                factors: vec![x.clone()],
            },
            T {
                coefficient: 1.,
                factors: vec![z, t.clone(), t.clone(), t.clone()],
            },
            T {
                coefficient: -1.,
                factors: vec![x, t.clone(), t.clone(), t],
            },
        ])
        .unwrap();
        g.validate(2).unwrap();
        let time = Iv::new(0., 0.5).unwrap();
        let state = [Iv::new(-0.25, 0.25).unwrap(), Iv::new(-0.5, 0.5).unwrap()];
        let independent = guard_directional_residual(&g, time, &state, 1, 1, &[1.], false).unwrap();
        let grouped = guard_directional_residual(&g, time, &state, 1, 1, &[1.], true).unwrap();
        assert!(independent.lo < 0. && independent.hi > 0.);
        assert_eq!(grouped.as_pair(), [0., 0.]);
        let mut checks = 0;
        for k in [0, 1] {
            for grouped in [false, true] {
                let r =
                    guard_directional_residual(&g, time, &state, 1, k, &[0.75], grouped).unwrap();
                for ti in 0..=8 {
                    let t = ti as f64 / 16.;
                    for xi in -4..=4 {
                        let x = xi as f64 / 16.;
                        for zi in -8..=8 {
                            let z = zi as f64 / 16.;
                            let exact = if k == 0 {
                                -3. * t * t * (z - x) - 0.75 * (1. + t * t * t)
                            } else {
                                0.25 * (1. + t * t * t)
                            };
                            assert!(
                                r.lo <= exact && exact <= r.hi,
                                "k={k} grouped={grouped} exact={exact} box={r:?}"
                            );
                            checks += 1;
                        }
                    }
                }
            }
        }
        println!(
            "CENTERED_SOURCE_DEGREE4 checks={checks} independent={independent:?} grouped={grouped:?}"
        );
    }
}

#[cfg(test)]
mod guard_dual_inverse_contracts {
    use super::*;
    use crate::retro_event_poly::{AffineFactor as F, EventTerm as T};
    use num_rational::BigRational as Q;

    #[test]
    fn bounded_inverse_exact_rational_dimension_lattice() {
        let mut ws = GuardInverseWorkspace::new();
        let mut a = [0.; 32 * 32];
        let mut inv = [0.; 32 * 32];
        let mut checks = 0;
        for n in [1, 2, 4, 8, 16, 31, 32] {
            // A=2I+J/64, A^-1=I/2 - J/[2(128+n)].
            for i in 0..n {
                for j in 0..n {
                    a[i * n + j] = 1. / 64. + if i == j { 2. } else { 0. };
                }
            }
            let err = ws.invert(n, &a[..n * n], &mut inv[..n * n]).unwrap();
            let err = Q::from_float(err).unwrap();
            let corr = Q::new(1.into(), (2 * (128 + n) as i64).into());
            for i in 0..n {
                for j in 0..n {
                    let exact = if i == j {
                        Q::new(1.into(), 2.into()) - corr.clone()
                    } else {
                        -corr.clone()
                    };
                    let diff = Q::from_float(inv[i * n + j]).unwrap() - exact;
                    assert!(diff <= err && diff >= -err.clone(), "n={n} i={i} j={j}");
                    checks += 1;
                }
            }
        }
        println!("BOUNDED_INVERSE_RATIONAL_CHECKS {checks}");
    }

    #[test]
    fn dual_source_cross_row_degree_four_cancellation() {
        let t = F::clock(0.);
        let x = F::component(0, 0.);
        let z = F::component(1, 0.);
        let w = F::component(2, 0.);
        let tr = |coefficient, factors| T {
            coefficient,
            factors,
        };
        let g1 = vec![
            tr(1., vec![z.clone()]),
            tr(-1., vec![x.clone()]),
            tr(1., vec![z, t.clone(), t.clone(), t.clone()]),
            tr(-1., vec![x.clone(), t.clone(), t.clone(), t.clone()]),
        ];
        let g2 = vec![
            tr(1., vec![w.clone()]),
            tr(-2., vec![x.clone()]),
            tr(1., vec![w, x.clone(), x.clone()]),
            tr(-2., vec![x.clone(), x.clone(), x.clone()]),
        ];
        let mut ht = Vec::new();
        for mut term in g1.clone() {
            term.coefficient *= 2.;
            ht.push(term);
        }
        for mut term in g2.clone() {
            term.coefficient *= -1.;
            ht.push(term);
        }
        ht.push(tr(0.25, vec![t]));
        ht.push(tr(0.5, vec![x]));
        let guard = DeclaredPolynomial::new(ht).unwrap();
        let src = DaeSource::new(
            vec![DeclaredPolynomial::new(vec![tr(1., vec![])]).unwrap()],
            vec![
                DeclaredPolynomial::new(g1).unwrap(),
                DeclaredPolynomial::new(g2).unwrap(),
            ],
        )
        .unwrap();
        guard.validate(3).unwrap();
        let time = Iv::new(0., 0.5).unwrap();
        let state = [Iv::new(-0.25, 0.25).unwrap(); 3];
        let mut checks = 0;
        for direction in [
            [1., 0., 0.5, -0.25],
            [0., 1., 1., 2.],
            [0.5, -0.25, 0.75, -0.5],
        ] {
            let [dt, dx, dz, dw] = direction;
            let exact = 0.25 * dt + 0.5 * dx;
            let r =
                guard_dual_direction(&src, &guard, &[2., -1.], time, &state, &direction).unwrap();
            assert_eq!(r.as_pair(), [exact, exact]);
            let r =
                guard_dual_direction(&src, &guard, &[1., -0.5], time, &state, &direction).unwrap();
            for ti in 0..=8 {
                for xi in -4..=4 {
                    for zi in -4..=4 {
                        for wi in -4..=4 {
                            let t = ti as f64 / 16.;
                            let x = xi as f64 / 16.;
                            let z = zi as f64 / 16.;
                            let w = wi as f64 / 16.;
                            let v = 3. * t * t * dt * (z - x) + (1. + t * t * t) * (dz - dx)
                                - 0.5
                                    * (2. * x * dx * (w - 2. * x) + (1. + x * x) * (dw - 2. * dx))
                                + exact;
                            assert!(r.lo <= v && v <= r.hi, "{v} in {r:?}");
                            checks += 1;
                        }
                    }
                }
            }
        }
        println!(
            "DUAL_SOURCE_DYADIC_CHECKS {checks}, exact cross-row cancellation for 3 directions"
        );
    }
}
