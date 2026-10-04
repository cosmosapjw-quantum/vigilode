//! Native FFT predictor right side for the Fourier client (RVJ DAG node
//! PP06, research node `research/pp06_fourier_fft_candidate_20261004`).
//!
//! Source: `code/fft_bridge.py` of the Loop 11 ZIP. Axis 0 of the grid is
//! the harmonic index, axis 1 the formal polynomial degree (not physical
//! time). Conjugation is applied to coefficients, `(k, j) -> (-k, j)`,
//! before transforming. The grid is padded to powers of two of at least
//! `2 (3K + S) + 1` by `3P + Q + 1` so the cyclic product equals the full
//! noncyclic one. The result is only a candidate: acceptance stays with
//! [`crate::fourier_path_certificate::certificate`], which recomputes the
//! noncyclic `lambda = 1` right side and never sees these values.

use rodas5p_core::{CoreError, CoreResult};

use crate::fourier_path_certificate::{Cplx, FourierModel, FourierPath};

fn invalid(message: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("Fourier FFT predictor: {message}"))
}

/// Work of the FFT predictor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FftWork {
    pub rhs_calls: usize,
    pub forward_2d: usize,
    pub inverse_2d: usize,
    pub padded_grid_points: usize,
    pub max_grid_points: usize,
}

fn fft_in_place(data: &mut [Cplx], inverse: bool) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let angle = sign * 2.0 * std::f64::consts::PI / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (angle * k as f64).sin_cos();
                let w = Cplx::new(c, s);
                let u = data[start + k];
                let t = data[start + k + len / 2];
                let v = Cplx::new(t.re * w.re - t.im * w.im, t.re * w.im + t.im * w.re);
                data[start + k] = Cplx::new(u.re + v.re, u.im + v.im);
                data[start + k + len / 2] = Cplx::new(u.re - v.re, u.im - v.im);
            }
        }
        len <<= 1;
    }
    if inverse {
        let scale = 1.0 / n as f64;
        for x in data.iter_mut() {
            *x = Cplx::new(x.re * scale, x.im * scale);
        }
    }
}

/// Row-major `nk x nj` grid.
struct Grid {
    nk: usize,
    nj: usize,
    v: Vec<Cplx>,
}

impl Grid {
    fn from_path(path: &FourierPath, nk: usize, nj: usize) -> CoreResult<Self> {
        let mut v = vec![Cplx::ZERO; nk * nj];
        for (&(k, j), &c) in path.coefficients() {
            if !(c.re.is_finite() && c.im.is_finite()) {
                return Err(invalid("nonfinite FFT input"));
            }
            let row = k.rem_euclid(nk as i32) as usize;
            let col = j as usize;
            if col >= nj {
                return Err(invalid("degree outside the grid"));
            }
            let e = &mut v[row * nj + col];
            *e = Cplx::new(e.re + c.re, e.im + c.im);
        }
        Ok(Self { nk, nj, v })
    }
    fn transform(&mut self, inverse: bool) {
        for row in self.v.chunks_mut(self.nj) {
            fft_in_place(row, inverse);
        }
        let mut col = vec![Cplx::ZERO; self.nk];
        for j in 0..self.nj {
            for (k, x) in col.iter_mut().enumerate() {
                *x = self.v[k * self.nj + j];
            }
            fft_in_place(&mut col, inverse);
            for (k, x) in col.iter().enumerate() {
                self.v[k * self.nj + j] = *x;
            }
        }
    }
}

fn mul(a: Cplx, b: Cplx) -> Cplx {
    Cplx::new(a.re * b.re - a.im * b.im, a.re * b.im + a.im * b.re)
}

fn conj_coefficients(p: &FourierPath) -> FourierPath {
    FourierPath::from_coefficients(
        p.coefficients()
            .iter()
            .map(|((k, j), v)| ((-k, *j), Cplx::new(v.re, -v.im))),
    )
}

/// `(band, degree, nk, nj)` of the full product (source `required_shape`,
/// powers of two instead of `next_fast_len`).
pub fn required_shape(
    path: &[FourierPath; 2],
    phase: &FourierPath,
) -> (usize, usize, usize, usize) {
    let kmax = |p: &FourierPath| {
        p.coefficients()
            .keys()
            .map(|(k, _)| k.unsigned_abs() as usize)
            .max()
            .unwrap_or(0)
    };
    let jmax = |p: &FourierPath| {
        p.coefficients()
            .keys()
            .map(|(_, j)| *j as usize)
            .max()
            .unwrap_or(0)
    };
    let k = kmax(&path[0]).max(kmax(&path[1]));
    let p = jmax(&path[0]).max(jmax(&path[1]));
    let band = 3 * k + kmax(phase);
    let degree = 3 * p + jmax(phase);
    (
        band,
        degree,
        (2 * band + 1).next_power_of_two(),
        (degree + 1).next_power_of_two(),
    )
}

/// FFT right side times `h` at `lambda = 1` (source `fft_rhs`). `shape`
/// overrides the grid; a grid smaller than the full product is refused
/// unless `allow_alias` (negative-control use only).
pub fn fft_rhs_h(
    path: &[FourierPath; 2],
    model: &FourierModel,
    phase: &FourierPath,
    h: f64,
    shape: Option<(usize, usize)>,
    allow_alias: bool,
    work: &mut FftWork,
) -> CoreResult<[FourierPath; 2]> {
    let (band, degree, mut nk, mut nj) = required_shape(path, phase);
    if let Some((k, j)) = shape {
        if !(k.is_power_of_two() && j.is_power_of_two()) {
            return Err(invalid("grid sizes must be powers of two"));
        }
        if (k < 2 * band + 1 || j < degree + 1) && !allow_alias {
            return Err(invalid("insufficient full-product padding"));
        }
        nk = k;
        nj = j;
    }
    let mut grids = Vec::with_capacity(5);
    for p in [
        &path[0],
        &conj_coefficients(&path[0]),
        &path[1],
        phase,
        &conj_coefficients(phase),
    ] {
        let mut g = Grid::from_path(p, nk, nj)?;
        g.transform(false);
        grids.push(g);
    }
    let (av, ac, bv, pv, pc) = (
        &grids[0].v,
        &grids[1].v,
        &grids[2].v,
        &grids[3].v,
        &grids[4].v,
    );
    let q = model.q();
    let mut fa = Grid {
        nk,
        nj,
        v: vec![Cplx::ZERO; nk * nj],
    };
    let mut fb = Grid {
        nk,
        nj,
        v: vec![Cplx::ZERO; nk * nj],
    };
    for i in 0..nk * nj {
        let s = mul(pv[i], av[i]);
        let t = mul(pc[i], ac[i]);
        let c = Cplx::new(
            model.g + model.eps * 0.5 * (s.re + t.re),
            model.eps * 0.5 * (s.im + t.im),
        );
        fa.v[i] = mul(mul(Cplx::new(0.0, h * q), c), mul(ac[i], bv[i]));
        fb.v[i] = mul(mul(Cplx::new(0.0, h * q / 2.0), c), mul(av[i], av[i]));
    }
    let mut out = [FourierPath::default(), FourierPath::default()];
    for (slot, mut g) in out.iter_mut().zip([fa, fb]) {
        g.transform(true);
        let mut entries = Vec::new();
        let kb = band.min(nk / 2) as i32;
        for k in -kb..=kb {
            let row = k.rem_euclid(nk as i32) as usize;
            for j in 0..=degree.min(nj - 1) {
                let x = g.v[row * nj + j];
                if !(x.re.is_finite() && x.im.is_finite()) {
                    return Err(invalid("nonfinite FFT output"));
                }
                entries.push(((k, j as u32), x));
            }
        }
        *slot = FourierPath::from_coefficients(entries);
    }
    work.rhs_calls += 1;
    work.forward_2d += 5;
    work.inverse_2d += 2;
    work.padded_grid_points += nk * nj;
    work.max_grid_points = work.max_grid_points.max(nk * nj);
    Ok(out)
}

/// The wrong order (conjugating grid values instead of coefficients), for
/// the negative test that a transform roundtrip is not a correctness
/// argument: on a grid of a polynomial axis it reverses the degree.
pub fn conj_on_grid_values(path: &FourierPath, nk: usize, nj: usize) -> CoreResult<FourierPath> {
    let mut g = Grid::from_path(path, nk, nj)?;
    g.transform(false);
    for x in g.v.iter_mut() {
        *x = Cplx::new(x.re, -x.im);
    }
    g.transform(true);
    let mut entries = Vec::new();
    for row in 0..nk {
        let k = if row > nk / 2 {
            row as i32 - nk as i32
        } else {
            row as i32
        };
        for j in 0..nj {
            entries.push(((k, j as u32), g.v[row * nj + j]));
        }
    }
    Ok(FourierPath::from_coefficients(entries))
}

/// Max over coefficients of `|x - y|` divided by the max `|y|` (both
/// complex moduli); the comparison of the FFT and direct right sides.
pub fn relative_difference(x: &FourierPath, y: &FourierPath) -> f64 {
    let mut keys: Vec<(i32, u32)> = x.coefficients().keys().copied().collect();
    keys.extend(y.coefficients().keys().copied());
    let get = |p: &FourierPath, k| p.coefficients().get(&k).copied().unwrap_or(Cplx::ZERO);
    let scale = y
        .coefficients()
        .values()
        .map(|v| v.re.hypot(v.im))
        .fold(0.0_f64, f64::max);
    let diff = keys
        .into_iter()
        .map(|k| {
            let (a, b) = (get(x, k), get(y, k));
            (a.re - b.re).hypot(a.im - b.im)
        })
        .fold(0.0_f64, f64::max);
    if scale == 0.0 { diff } else { diff / scale }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_conjugation_order() {
        let p = FourierPath::from_coefficients([
            ((0, 0), Cplx::new(1.0, 0.5)),
            ((1, 2), Cplx::new(-0.25, 0.75)),
            ((-1, 1), Cplx::new(0.5, -0.125)),
        ]);
        let mut g = Grid::from_path(&p, 8, 8).unwrap();
        g.transform(false);
        g.transform(true);
        let back = FourierPath::from_coefficients((0..8).flat_map(|row| {
            let v = &g.v;
            (0..8).map(move |j| {
                let k = if row > 4 { row as i32 - 8 } else { row as i32 };
                ((k, j as u32), v[row * 8 + j])
            })
        }));
        assert!(relative_difference(&back, &p) < 1e-15);
        // Conjugating grid values reverses the degree axis: not conj(p).
        let wrong = conj_on_grid_values(&p, 8, 8).unwrap();
        assert!(relative_difference(&wrong, &conj_coefficients(&p)) > 0.5);
    }
}
