//! Strip families of loop node V22 (`../../../banded_kernel_route/PREDECLARED.json`): two problems
//! of V04 on a strip of fixed cell size. The number of cells across the strip is the half band
//! width (twice that for the two-species problem); the number of unknowns stays near 8192.
//!
//! Nothing here measures anything.

use std::f64::consts::PI;
use std::sync::Arc;

use crate::problems::{Band, BandFillFn, JvpFn, ProblemDef, RhsFn};

/// Cell size of both families, in both directions.
const CELL: f64 = 1.0 / 64.0;

/// Allen-Cahn `u_t = D lap u + kappa (u - u^3)` on a strip of `w` cells across (fast index) and
/// `ceil(8192 / w)` cells along; zero-flux boundaries, natural ordering. Band: lower = upper = w.
pub fn allen_cahn_strip(w: usize) -> ProblemDef {
    assert!(w >= 1);
    let (d_coeff, kappa) = (0.01, 1.0);
    let my = 8192usize.div_ceil(w);
    let n = w * my;
    let s = d_coeff / (CELL * CELL);

    // `visit(cell, sum (x_nb - x_c), neighbours)`.
    #[inline(always)]
    fn stencil(w: usize, my: usize, x: &[f64], mut visit: impl FnMut(usize, f64)) {
        for j in 0..my {
            let base = j * w;
            for i in 0..w {
                let c = base + i;
                let xc = x[c];
                let mut sum = 0.0;
                if i > 0 {
                    sum += x[c - 1] - xc;
                }
                if i + 1 < w {
                    sum += x[c + 1] - xc;
                }
                if j > 0 {
                    sum += x[c - w] - xc;
                }
                if j + 1 < my {
                    sum += x[c + w] - xc;
                }
                visit(c, sum);
            }
        }
    }

    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        stencil(w, my, y, |c, sum| {
            let u = y[c];
            out[c] = s * sum + kappa * (u - u * u * u);
        });
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, v, out| {
        stencil(w, my, v, |c, sum| {
            let u = y[c];
            out[c] = s * sum + kappa * (1.0 - 3.0 * u * u) * v[c];
        });
        Ok(())
    });
    let band_fill: BandFillFn = Arc::new(move |_t, y, band| {
        let jw = 2 * w + 1;
        for j in 0..my {
            for i in 0..w {
                let c = j * w + i;
                let diag = c * jw + w;
                let mut nb = 0.0;
                if i > 0 {
                    band[diag - 1] = s;
                    nb += 1.0;
                }
                if i + 1 < w {
                    band[diag + 1] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    band[diag - w] = s;
                    nb += 1.0;
                }
                if j + 1 < my {
                    band[diag + w] = s;
                    nb += 1.0;
                }
                let u = y[c];
                band[diag] = -nb * s + kappa * (1.0 - 3.0 * u * u);
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for j in 0..my {
        for i in 0..w {
            let x = (i as f64 + 0.5) * CELL;
            let y = (j as f64 + 0.5) * CELL;
            y0[j * w + i] = 0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos()
                + 0.3 * (5.0 * PI * x).cos() * (PI * y).cos()
                - 0.1;
        }
    }
    ProblemDef {
        family: "acstrip",
        description: format!(
            "Allen-Cahn on a strip, {w} x {my} cells of size 1/64, n = {n}, D = {d_coeff}, kappa = {kappa}, zero-flux boundaries"
        ),
        n,
        y0,
        t_span: (0.0, 10.0),
        rtol: 1.0e-6,
        atol: 1.0e-8,
        initial_step: 1.0e-3,
        rhs,
        dense_fill: None,
        band: Some(Band {
            lower: w,
            upper: w,
            fill: band_fill,
        }),
        jvp: Some(jvp),
    }
}

/// Two-dimensional Brusselator (`A = 1`, `B = 3.4`, `alpha = 0.02`) on a strip of `m` cells
/// across (fast index) and `ceil(4096 / m)` cells along; zero-flux boundaries, species
/// interleaved per cell (index `2 (i + m j) + species`). Band: lower = upper = 2 m.
pub fn brusselator_strip(m: usize) -> ProblemDef {
    assert!(m >= 1);
    let (a_par, b_par, alpha) = (1.0, 3.4, 0.02);
    let my = 4096usize.div_ceil(m);
    let n = 2 * m * my;
    let s = alpha / (CELL * CELL);
    let sy = 2 * m;

    // `visit(a, lap(x_u), lap(x_v), neighbours)` with `a` the index of `u` in the cell.
    #[inline(always)]
    fn stencil(m: usize, my: usize, x: &[f64], mut visit: impl FnMut(usize, f64, f64)) {
        let sy = 2 * m;
        for j in 0..my {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let (xu, xv) = (x[a], x[a + 1]);
                let (mut su, mut sv) = (0.0, 0.0);
                if i > 0 {
                    su += x[a - 2] - xu;
                    sv += x[a - 1] - xv;
                }
                if i + 1 < m {
                    su += x[a + 2] - xu;
                    sv += x[a + 3] - xv;
                }
                if j > 0 {
                    su += x[a - sy] - xu;
                    sv += x[a + 1 - sy] - xv;
                }
                if j + 1 < my {
                    su += x[a + sy] - xu;
                    sv += x[a + 1 + sy] - xv;
                }
                visit(a, su, sv);
            }
        }
    }

    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        stencil(m, my, y, |a, su, sv| {
            let (u, v) = (y[a], y[a + 1]);
            out[a] = a_par + u * u * v - (b_par + 1.0) * u + s * su;
            out[a + 1] = b_par * u - u * u * v + s * sv;
        });
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, x, out| {
        stencil(m, my, x, |a, su, sv| {
            let (u, v) = (y[a], y[a + 1]);
            out[a] = (2.0 * u * v - (b_par + 1.0)) * x[a] + u * u * x[a + 1] + s * su;
            out[a + 1] = (b_par - 2.0 * u * v) * x[a] - u * u * x[a + 1] + s * sv;
        });
        Ok(())
    });
    let band_fill: BandFillFn = Arc::new(move |_t, y, band| {
        let width = 2 * m;
        let jw = 2 * width + 1;
        for j in 0..my {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let (u, v) = (y[a], y[a + 1]);
                let (da, db) = (a * jw + width, (a + 1) * jw + width);
                let mut nb = 0.0;
                if i > 0 {
                    band[da - 2] = s;
                    band[db - 2] = s;
                    nb += 1.0;
                }
                if i + 1 < m {
                    band[da + 2] = s;
                    band[db + 2] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    band[da - sy] = s;
                    band[db - sy] = s;
                    nb += 1.0;
                }
                if j + 1 < my {
                    band[da + sy] = s;
                    band[db + sy] = s;
                    nb += 1.0;
                }
                band[da] = 2.0 * u * v - (b_par + 1.0) - nb * s;
                band[da + 1] = u * u;
                band[db - 1] = b_par - 2.0 * u * v;
                band[db] = -u * u - nb * s;
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for j in 0..my {
        for i in 0..m {
            let x = (i as f64 + 0.5) / m as f64;
            let y = (j as f64 + 0.5) / my as f64;
            let a = 2 * (i + m * j);
            y0[a] = 0.5 + y;
            y0[a + 1] = 1.0 + 5.0 * x;
        }
    }
    ProblemDef {
        family: "brstrip",
        description: format!(
            "2D Brusselator on a strip, {m} x {my} cells of size 1/64, n = {n}, A = 1, B = 3.4, alpha = 0.02, zero-flux boundaries"
        ),
        n,
        y0,
        t_span: (0.0, 10.0),
        rtol: 1.0e-6,
        atol: 1.0e-8,
        initial_step: 1.0e-3,
        rhs,
        dense_fill: None,
        band: Some(Band {
            lower: 2 * m,
            upper: 2 * m,
            fill: band_fill,
        }),
        jvp: Some(jvp),
    }
}
