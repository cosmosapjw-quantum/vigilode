//! Candidate-only PP06 cyclic/negacyclic quotient reconstruction.
//!
//! Harmonic padding remains noncyclic. Formal degree is NOT physical time:
//! the Volterra primitive may lower a high degree, so no coefficient of the
//! full RHS is discarded. With full degree < 2N, reduce modulo t^N-1 and
//! t^N+1, then recover low/high bands by the half-sum and half-difference.
//! A shifted formal DFT makes the negacyclic product ordinary cyclic product.
//! The coefficient conjugation (k,j)->(-k,j), conj is obtained from the same
//! transform with frequency j' = shift-j mod N, shift=0 or 1.
//! The original PP05 noncyclic interval certificate remains the authority.
use super::{Cplx, FourierModel, FourierPath, invalid, mul};
use rodas5p_core::CoreResult;
use std::f64::consts::PI;

/// Cap on logical (full-product padded) grid points, per call.
pub const MAX_GRID_POINTS: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Full degree grid, shared conjugate transforms and precomputed twiddles.
    Full,
    /// Two half-width formal quotient grids (or Full for a constant degree).
    Crt,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub rhs_calls: usize,
    pub forward_2d: usize,
    pub inverse_2d: usize,
    pub sectors: usize,
    /// Sum of grid points over sectors, not over individual transforms.
    pub grid_points: usize,
    /// Largest single active grid.
    pub max_grid_points: usize,
    /// Sum of grid points over all transforms, not FLOPs.
    pub transformed_points: usize,
}

fn finite(z: Cplx) -> bool {
    z.re.is_finite() && z.im.is_finite()
}
fn add(a: Cplx, b: Cplx) -> Cplx {
    Cplx::new(a.re + b.re, a.im + b.im)
}
fn sub(a: Cplx, b: Cplx) -> Cplx {
    Cplx::new(a.re - b.re, a.im - b.im)
}
fn scale(a: Cplx, s: f64) -> Cplx {
    Cplx::new(a.re * s, a.im * s)
}
fn conj(a: Cplx) -> Cplx {
    Cplx::new(a.re, -a.im)
}

#[derive(Clone, Copy)]
struct Shape {
    band: usize,
    degree: usize,
    nk: usize,
    nj: usize,
}

fn shape(path: &[FourierPath; 2], phase: &FourierPath) -> CoreResult<Shape> {
    let support = |p: &FourierPath| -> CoreResult<(usize, usize)> {
        let mut k = 0;
        let mut j = 0;
        for (&(ki, ji), &v) in p.coefficients() {
            if !finite(v) {
                return Err(invalid("nonfinite WARP input"));
            }
            k = k.max(ki.unsigned_abs() as usize);
            j = j.max(ji as usize);
        }
        Ok((k, j))
    };
    let (ak, aj) = support(&path[0])?;
    let (bk, bj) = support(&path[1])?;
    let (pk, pj) = support(phase)?;
    let band = ak
        .max(bk)
        .checked_mul(3)
        .and_then(|x| x.checked_add(pk))
        .ok_or_else(|| invalid("WARP support overflow"))?;
    let degree = aj
        .max(bj)
        .checked_mul(3)
        .and_then(|x| x.checked_add(pj))
        .ok_or_else(|| invalid("WARP support overflow"))?;
    let nk = band
        .checked_mul(2)
        .and_then(|x| x.checked_add(1))
        .and_then(usize::checked_next_power_of_two)
        .ok_or_else(|| invalid("WARP harmonic size overflow"))?;
    let nj = degree
        .checked_add(1)
        .and_then(usize::checked_next_power_of_two)
        .ok_or_else(|| invalid("WARP formal size overflow"))?;
    if nk.checked_mul(nj).is_none_or(|x| x > MAX_GRID_POINTS) {
        return Err(invalid("WARP grid cap exceeded"));
    }
    Ok(Shape {
        band,
        degree,
        nk,
        nj,
    })
}

/// Same radix-2 butterfly arithmetic as PP06, with one table of roots per
/// axis rather than trigonometric evaluation inside every butterfly.
pub(crate) struct Plan {
    n: usize,
    swaps: Vec<(usize, usize)>,
    roots: Vec<Cplx>,
}
impl Plan {
    pub(crate) fn new(n: usize) -> Self {
        debug_assert!(n.is_power_of_two());
        let mut swaps = Vec::new();
        if n > 1 {
            for i in 1..n {
                let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
                if i < j {
                    swaps.push((i, j));
                }
            }
        }
        let roots = (0..n / 2)
            .map(|k| {
                let (s, c) = (-2.0 * PI * k as f64 / n as f64).sin_cos();
                Cplx::new(c, s)
            })
            .collect();
        Self { n, swaps, roots }
    }
    pub(crate) fn apply(&self, v: &mut [Cplx], inverse: bool) {
        debug_assert_eq!(v.len(), self.n);
        for &(i, j) in &self.swaps {
            v.swap(i, j);
        }
        let mut len = 2;
        while len <= self.n {
            let stride = self.n / len;
            for chunk in v.chunks_exact_mut(len) {
                let (lo, hi) = chunk.split_at_mut(len / 2);
                for (k, (a, b)) in lo.iter_mut().zip(hi).enumerate() {
                    let mut w = self.roots[k * stride];
                    if inverse {
                        w = conj(w);
                    }
                    let u = *a;
                    let t = mul(*b, w);
                    *a = add(u, t);
                    *b = sub(u, t);
                }
            }
            len *= 2;
        }
        if inverse {
            let s = 1.0 / self.n as f64;
            for x in v {
                *x = scale(*x, s);
            }
        }
    }
}
struct Sector {
    k: Plan,
    j: Plan,
    // Positive unit phase on the formal axis; rho^N = -1 in twisted sector.
    twist: Vec<Cplx>,
    twisted: bool,
    column: Vec<Cplx>,
}
impl Sector {
    fn new(nk: usize, nj: usize, twisted: bool) -> Self {
        let twist = (0..nj)
            .map(|j| {
                if twisted {
                    let (s, c) = (PI * j as f64 / nj as f64).sin_cos();
                    Cplx::new(c, s)
                } else {
                    Cplx::new(1., 0.)
                }
            })
            .collect();
        Self {
            k: Plan::new(nk),
            j: Plan::new(nj),
            twist,
            twisted,
            column: vec![Cplx::ZERO; nk],
        }
    }
    fn transform(&mut self, v: &mut [Cplx], inverse: bool) {
        let nj = self.j.n;
        for row in v.chunks_exact_mut(nj) {
            self.j.apply(row, inverse);
        }
        for j in 0..nj {
            for (k, x) in self.column.iter_mut().enumerate() {
                *x = v[k * nj + j];
            }
            self.k.apply(&mut self.column, inverse);
            for (k, x) in self.column.iter().enumerate() {
                v[k * nj + j] = *x;
            }
        }
    }
    fn input(&mut self, p: &FourierPath) -> Vec<Cplx> {
        let (nk, nj) = (self.k.n, self.j.n);
        let mut v = vec![Cplx::ZERO; nk * nj];
        for (&(k, j), &z) in p.coefficients() {
            let j = j as usize;
            let col = j % nj;
            let row = k.rem_euclid(nk as i32) as usize;
            let carry = if self.twisted && (j / nj) % 2 == 1 {
                -1.0
            } else {
                1.0
            };
            let x = scale(mul(z, self.twist[col]), carry);
            v[row * nj + col] = add(v[row * nj + col], x);
        }
        self.transform(&mut v, false);
        v
    }
    fn rhs(
        &mut self,
        path: &[FourierPath; 2],
        model: &FourierModel,
        phase: &FourierPath,
        h: f64,
    ) -> CoreResult<[Vec<Cplx>; 2]> {
        let (nk, nj) = (self.k.n, self.j.n);
        let a = self.input(&path[0]);
        let b = self.input(&path[1]);
        let p = self.input(phase);
        let mut fa = vec![Cplx::ZERO; nk * nj];
        let mut fb = vec![Cplx::ZERO; nk * nj];
        let shift = usize::from(self.twisted);
        for row in 0..nk {
            for j in 0..nj {
                let i = row * nj + j;
                let reflected = row * nj + (nj + shift - j) % nj;
                // Conjugate harmonic reversal cancels the frequency reversal
                // in k, but leaves the formal-degree frequency reflected.
                let ac = conj(a[reflected]);
                let pc = conj(p[reflected]);
                let s = mul(p[i], a[i]);
                let t = mul(pc, ac);
                let c = Cplx::new(
                    model.g + model.eps * 0.5 * (s.re + t.re),
                    model.eps * 0.5 * (s.im + t.im),
                );
                fa[i] = mul(mul(Cplx::new(0., h * model.q()), c), mul(ac, b[i]));
                fb[i] = mul(mul(Cplx::new(0., h * model.q() / 2.), c), mul(a[i], a[i]));
            }
        }
        for v in [&mut fa, &mut fb] {
            self.transform(v, true);
            for row in v.chunks_exact_mut(nj) {
                for (x, &w) in row.iter_mut().zip(&self.twist) {
                    *x = mul(*x, conj(w));
                    if !finite(*x) {
                        return Err(invalid("nonfinite WARP output"));
                    }
                }
            }
        }
        Ok([fa, fb])
    }
}

/// Full noncyclic PP06 RHS, returned in original coefficient coordinates.
/// In Crt mode no single quotient is interpreted as the full solution.
/// Work is charged only after a successful complete call (as in old FftWork).
pub fn rhs_h(
    path: &[FourierPath; 2],
    model: &FourierModel,
    phase: &FourierPath,
    h: f64,
    mode: Mode,
    work: &mut Work,
) -> CoreResult<[FourierPath; 2]> {
    if !h.is_finite()
        || h < 0.0
        || !model.omega.is_finite()
        || !model.g.is_finite()
        || !model.eps.is_finite()
        || !matches!(model.sigma, -1 | 1)
        || !(h * model.q()).is_finite()
    {
        return Err(invalid("invalid WARP model or step"));
    }
    let s = shape(path, phase)?;
    let folded = mode == Mode::Crt && s.nj > 1;
    let nj = if folded { s.nj / 2 } else { s.nj };
    let plus = Sector::new(s.nk, nj, false).rhs(path, model, phase, h)?;
    let minus = if folded {
        Some(Sector::new(s.nk, nj, true).rhs(path, model, phase, h)?)
    } else {
        None
    };
    let mut out = [FourierPath::default(), FourierPath::default()];
    for (component, slot) in out.iter_mut().enumerate() {
        let mut entries = Vec::new();
        for k in -(s.band as i32)..=s.band as i32 {
            let row = k.rem_euclid(s.nk as i32) as usize;
            for j in 0..=s.degree {
                let i = row * nj + j % nj;
                let z = if let Some(m) = &minus {
                    // Scale before sum to avoid an unnecessary intermediate overflow.
                    let p = scale(plus[component][i], 0.5);
                    let q = scale(m[component][i], 0.5);
                    if j < nj { add(p, q) } else { sub(p, q) }
                } else {
                    plus[component][i]
                };
                if !finite(z) {
                    return Err(invalid("nonfinite WARP reconstructed coefficient"));
                }
                entries.push(((k, j as u32), z));
            }
        }
        *slot = FourierPath::from_coefficients(entries);
    }
    let sectors = if folded { 2 } else { 1 };
    let points = s.nk * nj;
    // Do not silently wrap externally accumulated counters.
    let inc = |v: usize, x: usize| {
        v.checked_add(x)
            .ok_or_else(|| invalid("WARP work counter overflow"))
    };
    *work = Work {
        rhs_calls: inc(work.rhs_calls, 1)?,
        forward_2d: inc(work.forward_2d, 3 * sectors)?,
        inverse_2d: inc(work.inverse_2d, 2 * sectors)?,
        sectors: inc(work.sectors, sectors)?,
        grid_points: inc(work.grid_points, points * sectors)?,
        max_grid_points: work.max_grid_points.max(points),
        transformed_points: inc(work.transformed_points, 5 * sectors * points)?,
    };
    Ok(out)
}
