//! Full immutable RETRO-WARP dictionary. All ten source files are byte-bound.
//! Real tables inherit the source polynomial error bounds. Complex and LS
//! data are separate contracts, not a global IVP error guarantee.
use crate::{
    CoreError, CoreResult, DenseMatrix,
    directed::{Interval, add_up, exp_interval, sub_down},
    sha256_hex,
};
use serde_json::Value;
use std::sync::OnceLock;
type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro dictionary: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn zero() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn array(v: &Value) -> CoreResult<&[Value]> {
    v.as_array().map(Vec::as_slice).ok_or_else(|| bad("array"))
}
fn number(v: &Value) -> CoreResult<f64> {
    v.as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| bad("number"))
}
fn count(v: &Value) -> CoreResult<usize> {
    v.as_u64()
        .and_then(|x| usize::try_from(x).ok())
        .ok_or_else(|| bad("integer"))
}
fn string(v: &Value) -> CoreResult<&str> {
    v.as_str().ok_or_else(|| bad("string"))
}
fn values(v: &Value) -> CoreResult<Vec<f64>> {
    array(v)?.iter().map(number).collect()
}
fn nonnegative(v: &Value) -> CoreResult<f64> {
    let x = number(v)?;
    if x < 0. {
        Err(bad("negative bound"))
    } else {
        Ok(x)
    }
}
fn length<T>(v: &[T], n: usize) -> CoreResult<()> {
    if v.len() != n {
        Err(bad("shape"))
    } else {
        Ok(())
    }
}
fn pow(mut x: Iv, n: usize) -> CoreResult<Iv> {
    let mut y = pt(1.)?;
    let mut k = n;
    while k > 0 {
        if k % 2 == 1 {
            y = y.mul(x)?;
        }
        k /= 2;
        if k > 0 {
            x = x.mul(x)?;
        }
    }
    Ok(y)
}
/// Immutable source metadata. Bytes remain available for exact-rational audit.
#[derive(Clone, Copy)]
pub struct Source {
    pub name: &'static str,
    pub sha256: &'static str,
    pub bytes: &'static [u8],
}
impl Source {
    pub fn verify(&self, bytes: &[u8]) -> CoreResult<()> {
        if sha256_hex(bytes) == self.sha256 {
            Ok(())
        } else {
            Err(bad("source identity"))
        }
    }
}
include!("retro_dictionary_sources.rs");
/// Rational coefficient with point approximation and outward enclosure.
#[derive(Clone, Copy, Debug)]
pub struct Rational {
    pub value: f64,
    pub enclosure: Iv,
}
fn integer_iv(s: &str) -> CoreResult<Iv> {
    let v = s.parse::<f64>().map_err(|_| bad("rational integer"))?;
    if !v.is_finite() {
        return Err(bad("rational overflow"));
    }
    if let Ok(i) = s.parse::<i64>()
        && i.unsigned_abs() <= 1u64 << 53
    {
        return pt(v);
    }
    Iv::new(v.next_down(), v.next_up())
}
fn rat(v: &Value) -> CoreResult<Rational> {
    let s = string(v)?;
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let ai = integer_iv(a)?;
    let bi = integer_iv(b)?;
    if bi.lo <= 0. {
        return Err(bad("rational denominator"));
    }
    let enclosure = ai.div(bi)?;
    let value = a.parse::<f64>().map_err(|_| bad("rational numerator"))?
        / b.parse::<f64>().map_err(|_| bad("rational denominator"))?;
    if !enclosure.contains(value) {
        return Err(bad("rational conversion"));
    }
    Ok(Rational { value, enclosure })
}
fn rats(v: &Value) -> CoreResult<Vec<Rational>> {
    array(v)?.iter().map(rat).collect()
}
fn rat_rows(v: &Value) -> CoreResult<Vec<Vec<Rational>>> {
    array(v)?.iter().map(rats).collect()
}
fn horner(c: &[Rational], x: Iv) -> CoreResult<Iv> {
    c.iter()
        .rev()
        .try_fold(zero(), |a, b| a.mul(x)?.add(b.enclosure))
}
fn finite_slice(x: &[f64]) -> bool {
    x.iter().all(|v| v.is_finite())
}
const NC: usize = 57;
const NF: usize = 45;
const DEG: usize = 20;
const MAX_X: f64 = 16384.;
const FACT: [f64; 9] = [1., 1., 2., 6., 24., 120., 720., 5040., 40320.];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealMode {
    TableFast,
    TableDirected,
    Scaled,
    Hybrid,
}
struct Cell {
    end: f64,
    center: f64,
    radius: f64,
    // Degree-major, functions contiguous for vectorization.
    coeff: [[f64; NF]; DEG + 1],
    poly: [f64; NF],
    total: [f64; NF],
}
impl Cell {
    #[inline]
    fn fast<const N: usize>(&self, x: f64, off: usize) -> [f64; N] {
        let u = (-x - self.center) / self.radius;
        let mut y = [0.; N];
        y.copy_from_slice(&self.coeff[DEG][off..off + N]);
        for c in self.coeff[..DEG].iter().rev() {
            for (v, &a) in y.iter_mut().zip(&c[off..off + N]) {
                *v = *v * u + a;
            }
        }
        y
    }
    fn fast_one(&self, x: f64, j: usize) -> CoreResult<Iv> {
        let u = (-x - self.center) / self.radius;
        let mut v = self.coeff[DEG][j];
        for c in self.coeff[..DEG].iter().rev() {
            v = v * u + c[j];
        }
        self.fast_enclose(v, j)
    }
    fn fast_enclose(&self, v: f64, j: usize) -> CoreResult<Iv> {
        Iv::new(
            sub_down(v, self.total[j])?.max(0.),
            add_up(v, self.total[j])?.max(0.),
        )
    }
    fn directed(&self, x: f64, j: usize) -> CoreResult<Iv> {
        let u = pt(-x)?.sub(pt(self.center)?)?.div(pt(self.radius)?)?;
        let mut v = pt(self.coeff[DEG][j])?;
        for c in self.coeff[..DEG].iter().rev() {
            v = v.mul(u)?.add(pt(c[j])?)?;
        }
        let e = self.poly[j];
        let v = v.add(Iv::new(-e, e)?)?;
        Iv::new(v.lo.max(0.), v.hi.max(0.))
    }
}
fn moment_index(p: usize, k: usize) -> CoreResult<usize> {
    if p > 7 || k > p {
        Err(bad("moment degree"))
    } else {
        Ok(p * (p + 1) / 2 + k)
    }
}
struct ScaledMoment {
    inverse_power: usize,
    coeff: Vec<f64>,
    tail: f64,
    total: f64,
}
impl ScaledMoment {
    fn evaluate(&self, x: f64) -> CoreResult<Iv> {
        if !(64. ..=MAX_X).contains(&x) {
            return Err(bad("scaled domain"));
        }
        let u = pt(1.)?.div(pt(x)?)?;
        let v = self
            .coeff
            .iter()
            .rev()
            .try_fold(zero(), |a, &b| a.mul(u)?.add(pt(b)?))?;
        let v = v
            .add(Iv::new(-self.tail, self.tail)?)?
            .mul(pow(u, self.inverse_power)?)?;
        Iv::new(v.lo.max(0.), v.hi.max(0.))
    }
    fn evaluate_fast(&self, x: f64) -> CoreResult<Iv> {
        if !(64. ..=MAX_X).contains(&x) {
            return Err(bad("scaled domain"));
        }
        let u = 1. / x;
        let mut v = 0.;
        for &c in self.coeff.iter().rev() {
            v = v * u + c;
        }
        for _ in 0..self.inverse_power {
            v *= u;
        }
        let e = pt(self.total)?
            .mul(pow(pt(1.)?.div(pt(x)?)?, self.inverse_power)?)?
            .hi;
        Iv::new(sub_down(v, e)?.max(0.), add_up(v, e)?.max(0.))
    }
}
/// Complete collocation basis record (node count 3, 5 or 7).
#[non_exhaustive]
pub struct Basis {
    pub nodes: Vec<Rational>,
    pub monomial: Vec<Vec<Rational>>,
    pub bernstein: Vec<Vec<Rational>>,
    pub restriction_binomial: Vec<Vec<usize>>,
    pub damping_lift: Vec<Vec<Rational>>,
}
impl Basis {
    pub fn restrict(&self, row: usize, theta: f64) -> CoreResult<Vec<Iv>> {
        if row >= self.nodes.len() || !theta.is_finite() || !(0. ..=1.).contains(&theta) {
            return Err(bad("basis restriction"));
        }
        let t = pt(theta)?;
        let s = pt(1.)?.sub(t)?;
        (0..self.nodes.len())
            .map(|i| {
                (0..=i).try_fold(zero(), |v, k| {
                    v.add(
                        self.bernstein[row][k]
                            .enclosure
                            .mul(pow(t, k)?)?
                            .mul(pow(s, i - k)?)?
                            .scale(self.restriction_binomial[i][k] as f64)?,
                    )
                })
            })
            .collect()
    }
    /// Scalar Green coefficient multiplying a forcing node and E^l in the
    /// carrier Z=-aI+E. The physical slab length h is not included.
    pub fn green_weight(
        &self,
        d: &Dictionary,
        row: usize,
        a: f64,
        theta: f64,
        l: usize,
        mode: RealMode,
    ) -> CoreResult<Iv> {
        if l >= self.damping_lift.len() || !a.is_finite() || a < 0. {
            return Err(bad("damping lift degree/domain"));
        }
        let b = self.restrict(row, theta)?;
        if theta == 0. {
            return pt(0.);
        }
        let x = pt(a)?.mul(pt(theta)?)?;
        let p = self.nodes.len() - 1;
        let sum = b.iter().enumerate().try_fold(zero(), |s, (i, &v)| {
            s.add(
                v.mul(self.damping_lift[l][i].enclosure)?
                    .mul(d.moment(x, p + l, i, mode)?)?,
            )
        })?;
        sum.mul(pow(pt(theta)?, l + 1)?)?.div(pt(FACT[l])?)
    }
}
/// Original exact-rational basis and real-scalar least-squares response.
/// Gauss nodes/weights are archived binary64 quadrature, not intervals.
#[non_exhaustive]
pub struct Legendre {
    pub l: Vec<Vec<Rational>>,
    pub integral: Vec<Vec<Rational>>,
    pub g: Vec<Vec<Rational>>,
    pub c: Vec<Vec<Rational>>,
    pub m: Vec<Vec<Rational>>,
    pub ell: Vec<Rational>,
    pub d: Vec<Rational>,
    pub v: Vec<Rational>,
    pub gauss_nodes: Vec<f64>,
    pub gauss_weights: Vec<f64>,
    pub numerator: Vec<Rational>,
    pub denominator: Vec<Rational>,
    pub first_nonzero_error_power: usize,
    pub leading_error_coefficient: Rational,
    pub stiff_limit: Rational,
}
impl Legendre {
    pub fn endpoint_response(&self, z: f64) -> CoreResult<Iv> {
        if !z.is_finite() || z > 0. {
            return Err(bad("real LS domain"));
        }
        if z.abs() <= 1. {
            return horner(&self.numerator, pt(z)?)?.div(horner(&self.denominator, pt(z)?)?);
        }
        let x = pt(1.)?.div(pt(z)?)?;
        let mut n = zero();
        let mut den = zero();
        for c in &self.numerator {
            n = n.mul(x)?.add(c.enclosure)?;
        }
        for c in &self.denominator {
            den = den.mul(x)?.add(c.enclosure)?;
        }
        n.mul(pow(x, self.denominator.len() - self.numerator.len())?)?
            .div(den)
    }
    pub fn normal_equations(&self, z: f64) -> CoreResult<(DenseMatrix, Vec<f64>)> {
        if !z.is_finite() || z > 0. {
            return Err(bad("LS domain"));
        }
        let p = self.d.len();
        let mut a = DenseMatrix::zeros(p, p);
        let mut b = vec![0.; p];
        for i in 0..p {
            b[i] = z * self.d[i].value - z * z * self.v[i].value;
            for j in 0..p {
                a[(i, j)] = self.g[i][j].value - z * (self.c[i][j].value + self.c[j][i].value)
                    + z * z * self.m[i][j].value;
            }
        }
        if !finite_slice(a.as_slice()) || !finite_slice(&b) {
            return Err(bad("LS overflow"));
        }
        Ok((a, b))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ProductEntry {
    pub i: usize,
    pub j: usize,
    pub k: usize,
    pub coefficient: Rational,
}
#[derive(Clone, Debug)]
pub struct PicardWord {
    pub word: Vec<usize>,
    pub degree: usize,
    pub coefficient: Rational,
}
/// Numerical candidate only. Tail at four is an analytic Taylor tail, NOT a
/// full complex floating point or transcendental enclosure.
pub struct ComplexCandidate {
    pub values: [[f64; 2]; 9],
    pub tail_at_four: [f64; 9],
    pub used_taylor: bool,
}
pub struct Dictionary {
    cells: Vec<Cell>,
    scaled: Vec<ScaledMoment>,
    basis: Vec<Basis>,
    legendre: Vec<Legendre>,
    product: Vec<ProductEntry>,
    picard: Vec<PicardWord>,
    complex: Vec<Vec<Rational>>,
    complex_tail: Vec<Rational>,
}
static DICT: OnceLock<Result<Dictionary, String>> = OnceLock::new();
impl Dictionary {
    pub fn get() -> CoreResult<&'static Self> {
        DICT.get_or_init(|| Self::load().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| bad(e))
    }
    /// Byte digest of actual packed coefficients, geometry and both real bounds.
    pub fn packed_digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        for c in &self.cells {
            for v in [c.end, c.center, c.radius]
                .iter()
                .chain(c.coeff.iter().flatten())
                .chain(&c.poly)
                .chain(&c.total)
            {
                h.update(v.to_bits().to_le_bytes());
            }
        }
        hex::encode(h.finalize())
    }
    pub fn source_count(&self) -> usize {
        SOURCES.len()
    }
    pub fn sources(&self) -> &'static [Source] {
        &SOURCES
    }
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
    pub fn cell_edges(&self) -> impl Iterator<Item = f64> + '_ {
        self.cells.iter().map(|c| c.end)
    }
    pub fn products(&self) -> &[ProductEntry] {
        &self.product
    }
    pub fn picard_words(&self) -> &[PicardWord] {
        &self.picard
    }
    pub fn basis(&self, n: usize) -> CoreResult<&Basis> {
        match n {
            3 => Ok(&self.basis[0]),
            5 => Ok(&self.basis[1]),
            7 => Ok(&self.basis[2]),
            _ => Err(bad("basis nodes")),
        }
    }
    pub fn legendre(&self, p: usize) -> CoreResult<&Legendre> {
        if !(1..=6).contains(&p) {
            Err(bad("LS degree"))
        } else {
            Ok(&self.legendre[p - 1])
        }
    }
    pub fn complex_coefficients(&self) -> &[Vec<Rational>] {
        &self.complex
    }
    fn load() -> CoreResult<Self> {
        let mut docs = Vec::new();
        for s in SOURCES {
            s.verify(s.bytes)?;
            docs.push(serde_json::from_slice::<Value>(s.bytes).map_err(|e| bad(&e.to_string()))?);
        }
        let doc = |name: &str| -> CoreResult<&Value> {
            SOURCES
                .iter()
                .position(|x| x.name == name)
                .map(|i| &docs[i])
                .ok_or_else(|| bad("source name"))
        };
        let phi = doc("PHI_TABLE.json")?;
        let bern = doc("BERNSTEIN_MOMENTS.json")?;
        let cert = doc("TABLE_CERTIFICATE.json")?;
        for v in [phi, bern, cert] {
            if count(&v["degree"])? != DEG {
                return Err(bad("table degree"));
            }
            length(array(&v["cells"])?, NC)?;
        }
        let pp = array(&phi["cells"])?;
        let bb = array(&bern["cells"])?;
        let cc = array(&cert["cells"])?;
        let mut cells = Vec::with_capacity(NC);
        let mut end = 0.;
        for i in 0..NC {
            let a = -number(&pp[i]["hi"])?;
            let b = -number(&pp[i]["lo"])?;
            let center = number(&pp[i]["center"])?;
            let radius = number(&pp[i]["radius"])?;
            if a != end
                || b <= a
                || radius <= 0.
                || -center - radius != a
                || -center + radius != b
                || number(&cc[i]["a"])? != a
                || number(&cc[i]["b"])? != b
            {
                return Err(bad("cell geometry"));
            }
            for field in ["center", "radius", "lo", "hi"] {
                if number(&pp[i][field])? != number(&bb[i][field])? {
                    return Err(bad("moment cell geometry"));
                }
            }
            let mut c = Cell {
                end: b,
                center,
                radius,
                coeff: [[0.; NF]; DEG + 1],
                poly: [0.; NF],
                total: [0.; NF],
            };
            for (off, n, data, bounds) in [
                (0, 9, &pp[i]["coefficients"], &cc[i]["phi"]),
                (9, 36, &bb[i]["coefficients"], &cc[i]["bernstein"]),
            ] {
                let co = array(data)?;
                let bo = array(bounds)?;
                length(co, n)?;
                length(bo, n)?;
                for j in 0..n {
                    let v = values(&co[j])?;
                    length(&v, DEG + 1)?;
                    for (k, x) in v.into_iter().enumerate() {
                        c.coeff[k][off + j] = x;
                    }
                    c.poly[off + j] = nonnegative(&bo[j]["polynomial_error_upper"])?;
                    c.total[off + j] = nonnegative(&bo[j]["total_error_upper"])?;
                    if c.total[off + j] < c.poly[off + j] {
                        return Err(bad("bound ordering"));
                    }
                }
            }
            cells.push(c);
            end = b;
        }
        if end != MAX_X {
            return Err(bad("cell domain"));
        }
        for v in [bern, cert] {
            let keys = array(&v["keys"])?;
            length(keys, 36)?;
            for p in 0..8 {
                for k in 0..=p {
                    let a = array(&keys[moment_index(p, k)?])?;
                    if a.len() != 2 || count(&a[0])? != p || count(&a[1])? != k {
                        return Err(bad("moment keys"));
                    }
                }
            }
        }
        let sc = array(&doc("SCALED_MOMENTS.json")?["rows"])?;
        length(sc, 36)?;
        let mut scaled = Vec::new();
        for p in 0..8 {
            for k in 0..=p {
                let row = &sc[moment_index(p, k)?];
                let c = values(&row["coefficients"])?;
                length(&c, k + 1)?;
                if count(&row["p"])? != p
                    || count(&row["k"])? != k
                    || count(&row["inverse_power"])? != p - k + 1
                {
                    return Err(bad("scaled row"));
                }
                scaled.push(ScaledMoment {
                    inverse_power: p - k + 1,
                    coeff: c,
                    tail: nonnegative(&row["scaled_tail_upper"])?,
                    total: nonnegative(&row["scaled_total_upper"])?,
                });
            }
        }
        let lag = doc("LAGRANGE_TABLE.json")?;
        let bas = doc("BASIS_OPERATOR_TABLE.json")?;
        let mut basis = Vec::new();
        for n in [3, 5, 7] {
            let key = n.to_string();
            let a = &lag["records"][&key];
            let b = &bas["records"][&key];
            if a["nodes"] != b["nodes"] || a["coefficients_ascending"] != b["monomial"] {
                return Err(bad("basis cross-source"));
            }
            let nodes = rats(&a["nodes"])?;
            let monomial = rat_rows(&a["coefficients_ascending"])?;
            let bernstein = rat_rows(&b["bernstein"])?;
            let restriction_binomial = array(&b["restriction_binomial"])?
                .iter()
                .map(|r| array(r)?.iter().map(count).collect::<CoreResult<Vec<_>>>())
                .collect::<CoreResult<Vec<_>>>()?;
            let damping_lift = (0..=8 - n)
                .map(|l| rats(&b["damping_lift_ratios"][l.to_string()]))
                .collect::<CoreResult<Vec<_>>>()?;
            length(&nodes, n)?;
            length(&monomial, n)?;
            length(&bernstein, n)?;
            length(&restriction_binomial, n)?;
            for r in monomial.iter().chain(&bernstein).chain(&damping_lift) {
                length(r, n)?;
            }
            for r in &restriction_binomial {
                length(r, n)?;
            }
            basis.push(Basis {
                nodes,
                monomial,
                bernstein,
                restriction_binomial,
                damping_lift,
            });
        }
        let ldoc = doc("UNIVERSAL_LEGENDRE.json")?;
        let mut legendre = Vec::new();
        for p in 1..=6 {
            let key = p.to_string();
            let r = &ldoc["records"][&key];
            let e = &ldoc["linear_real_endpoint_rational"][&key];
            let item = Legendre {
                l: rat_rows(&r["L"])?,
                integral: rat_rows(&r["I"])?,
                g: rat_rows(&r["G"])?,
                c: rat_rows(&r["C"])?,
                m: rat_rows(&r["M"])?,
                ell: rats(&r["ell"])?,
                d: rats(&r["d"])?,
                v: rats(&r["v"])?,
                gauss_nodes: values(&r["gauss_nodes"])?,
                gauss_weights: values(&r["gauss_weights"])?,
                numerator: rats(&e["num_ascending"])?,
                denominator: rats(&e["den_ascending"])?,
                first_nonzero_error_power: count(&e["first_nonzero_error_power"])?,
                leading_error_coefficient: rat(&e["leading_error_coefficient"])?,
                stiff_limit: rat(&e["stiff_limit"])?,
            };
            for q in [&item.ell, &item.d, &item.v] {
                length(q, p)?;
            }
            for q in [&item.g, &item.c, &item.m] {
                length(q, p)?;
                for x in q {
                    length(x, p)?;
                }
            }
            length(&item.l, p)?;
            length(&item.integral, p)?;
            length(&item.gauss_nodes, 3 * p + 1)?;
            length(&item.gauss_weights, 3 * p + 1)?;
            if item.numerator.is_empty() || item.denominator.len() < item.numerator.len() {
                return Err(bad("LS rational shape"));
            }
            legendre.push(item);
        }
        let mut product = Vec::new();
        for r in array(&doc("LEGENDRE_PRODUCT_TABLE.json")?["entries"])? {
            let (i, j, k) = (count(&r["i"])?, count(&r["j"])?, count(&r["k"])?);
            if i > 8 || j > 8 || k > 16 || k < i.abs_diff(j) || k > i + j || (i + j - k) % 2 != 0 {
                return Err(bad("product support"));
            }
            product.push(ProductEntry {
                i,
                j,
                k,
                coefficient: rat(&r["value"])?,
            });
        }
        length(&product, 285)?;
        let mut picard = Vec::new();
        for r in array(&doc("ORDERED_PICARD_TABLE.json")?["records"])? {
            let word = array(&r["word"])?
                .iter()
                .map(count)
                .collect::<CoreResult<Vec<_>>>()?;
            let degree = count(&r["degree"])?;
            if word.is_empty()
                || word.len() > 5
                || word.iter().any(|&j| j > 2)
                || degree != word.iter().sum::<usize>() + word.len()
            {
                return Err(bad("ordered word"));
            }
            picard.push(PicardWord {
                word,
                degree,
                coefficient: rat(&r["coefficient"])?,
            });
        }
        length(&picard, 363)?;
        let cd = doc("COMPLEX_PHI_SERIES.json")?;
        let complex = rat_rows(&cd["coefficients"])?;
        let complex_tail = rats(&cd["remainder_at_4"])?;
        length(&complex, 9)?;
        length(&complex_tail, 9)?;
        for r in &complex {
            length(r, 41)?;
        }
        Ok(Self {
            cells,
            scaled,
            basis,
            legendre,
            product,
            picard,
            complex,
            complex_tail,
        })
    }
    fn cell(&self, x: f64) -> CoreResult<&Cell> {
        if !x.is_finite() || !(0. ..=MAX_X).contains(&x) {
            return Err(bad("real argument domain"));
        }
        Ok(&self.cells[self.cells.partition_point(|c| c.end < x)])
    }
    fn point(&self, x: f64, j: usize, mode: RealMode) -> CoreResult<Iv> {
        let c = self.cell(x)?;
        if x == 0. {
            return if j < 9 {
                pt(1.)?.div(pt(FACT[j])?)
            } else {
                let k = j - 9;
                let p = (0..8)
                    .find(|p| k < (p + 1) * (p + 2) / 2)
                    .ok_or_else(|| bad("moment"))?;
                pt(1.)?.div(pt((p + 1) as f64)?)
            };
        }
        if mode == RealMode::Scaled || (mode == RealMode::Hybrid && x >= 64.) {
            if x < 64. {
                return Err(bad("scaled domain"));
            }
            if j == 0 {
                return exp_interval(-x);
            }
            if j < 9 {
                return self.scaled[moment_index(j - 1, j - 1)?]
                    .evaluate(x)?
                    .div(pt(FACT[j - 1])?);
            }
            return self.scaled[j - 9].evaluate(x);
        }
        match mode {
            RealMode::TableFast => c.fast_one(x, j),
            _ => c.directed(x, j),
        }
    }
    fn interval(&self, x: Iv, j: usize, mode: RealMode) -> CoreResult<Iv> {
        Iv::new(x.lo, x.hi)?;
        let a = self.point(x.lo, j, mode)?;
        if x.lo == x.hi {
            return Ok(a);
        }
        let b = self.point(x.hi, j, mode)?;
        Iv::new(b.lo.max(0.), a.hi.max(0.))
    }
    pub fn phi(&self, x: Iv, j: usize, mode: RealMode) -> CoreResult<Iv> {
        if j > 8 {
            return Err(bad("phi index"));
        }
        self.interval(x, j, mode)
    }
    pub fn moment(&self, x: Iv, p: usize, k: usize, mode: RealMode) -> CoreResult<Iv> {
        self.interval(x, 9 + moment_index(p, k)?, mode)
    }
    pub fn scaled_moment_fast(&self, x: f64, p: usize, k: usize) -> CoreResult<Iv> {
        self.scaled[moment_index(p, k)?].evaluate_fast(x)
    }
    /// Packed coefficient lanes: one lookup, one Horner loop, no heap scratch.
    pub fn phi_all_fast(&self, x: f64) -> CoreResult<[Iv; 9]> {
        let c = self.cell(x)?;
        let v = c.fast::<9>(x, 0);
        let mut r = [zero(); 9];
        for j in 0..9 {
            r[j] = if x == 0. {
                pt(1.)?.div(pt(FACT[j])?)?
            } else {
                c.fast_enclose(v[j], j)?
            };
        }
        Ok(r)
    }
    pub fn moments_all_fast(&self, x: f64) -> CoreResult<[Iv; 36]> {
        let c = self.cell(x)?;
        let v = c.fast::<36>(x, 9);
        let mut r = [zero(); 36];
        for p in 0..8 {
            for k in 0..=p {
                let j = moment_index(p, k)?;
                r[j] = if x == 0. {
                    pt(1.)?.div(pt((p + 1) as f64)?)?
                } else {
                    c.fast_enclose(v[j], j + 9)?
                };
            }
        }
        Ok(r)
    }
    pub fn legendre_product(&self, a: &[f64], b: &[f64]) -> CoreResult<[Iv; 17]> {
        if a.len() > 9 || b.len() > 9 || !finite_slice(a) || !finite_slice(b) {
            return Err(bad("product input"));
        }
        let mut out = [zero(); 17];
        for t in &self.product {
            if t.i < a.len() && t.j < b.len() {
                out[t.k] = out[t.k].add(t.coefficient.enclosure.scale(a[t.i])?.scale(b[t.j])?)?;
            }
        }
        Ok(out)
    }
    pub fn complex_phi(&self, re: f64, im: f64) -> CoreResult<ComplexCandidate> {
        let r = re.hypot(im);
        if !re.is_finite() || !im.is_finite() || re > 0. || !r.is_finite() {
            return Err(bad("complex domain"));
        }
        let mut out = [[0.; 2]; 9];
        let e = re.exp();
        let (s, c) = im.sin_cos();
        out[0] = [e * c, e * s];
        let taylor = r <= 4.;
        if taylor {
            for (j, value) in out.iter_mut().enumerate().skip(1) {
                let mut v = [self.complex[j][40].value, 0.];
                for a in self.complex[j][..40].iter().rev() {
                    v = [v[0] * re - v[1] * im + a.value, v[0] * im + v[1] * re];
                }
                *value = v;
            }
        } else {
            let scale = re.abs().max(im.abs());
            let a = re / scale;
            let b = im / scale;
            let den = a * a + b * b;
            for j in 1..9 {
                let u = (out[j - 1][0] - self.complex[j - 1][0].value) / scale;
                let v = out[j - 1][1] / scale;
                out[j] = [(u * a + v * b) / den, (v * a - u * b) / den];
            }
        }
        if out.iter().flatten().any(|x| !x.is_finite()) {
            return Err(bad("complex arithmetic"));
        }
        Ok(ComplexCandidate {
            values: out,
            tail_at_four: std::array::from_fn(|j| self.complex_tail[j].enclosure.hi),
            used_taylor: taylor,
        })
    }
}
