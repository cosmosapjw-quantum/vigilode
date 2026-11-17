//! The problems of the V04 cost anatomy (loop L1, `../../PREDECLARED.json`).
//!
//! Every problem is autonomous, has the identity mass matrix and gives its right-hand side, an
//! analytic Jacobian in the storage of its structure class (dense fill, band fill) and an
//! analytic Jacobian-vector product. Nothing here measures anything.

use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreResult, DenseMatrix};

pub type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
/// Writes every entry of the fixed pattern of the dense Jacobian at `y`.
pub type DenseFillFn = Arc<dyn Fn(&[f64], &mut DenseMatrix) + Send + Sync>;
/// The band layout of `rodas5p_integrators::BandedJacobianFn`: row `i`, column `j` at
/// `i (lower + upper + 1) + (j + lower - i)`, into a zeroed buffer.
pub type BandFillFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

pub struct Band {
    pub lower: usize,
    pub upper: usize,
    pub fill: BandFillFn,
}

pub struct ProblemDef {
    pub family: &'static str,
    pub description: String,
    pub n: usize,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub rtol: f64,
    pub atol: f64,
    pub initial_step: f64,
    pub rhs: RhsFn,
    pub dense_fill: Option<DenseFillFn>,
    pub band: Option<Band>,
    pub jvp: Option<JvpFn>,
}

// ---------------------------------------------------------------------------------------------
// HIRES (n = 8): plant-physiology kinetics, the stiff test set problem.
// ---------------------------------------------------------------------------------------------

pub fn hires() -> ProblemDef {
    let rhs: RhsFn = Arc::new(|_t, y, out| {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    });
    let dense_fill: DenseFillFn = Arc::new(|y, j| {
        j[(0, 0)] = -1.71;
        j[(0, 1)] = 0.43;
        j[(0, 2)] = 8.32;
        j[(1, 0)] = 1.71;
        j[(1, 1)] = -8.75;
        j[(2, 2)] = -10.03;
        j[(2, 3)] = 0.43;
        j[(2, 4)] = 0.035;
        j[(3, 1)] = 8.32;
        j[(3, 2)] = 1.71;
        j[(3, 3)] = -1.12;
        j[(4, 4)] = -1.745;
        j[(4, 5)] = 0.43;
        j[(4, 6)] = 0.43;
        j[(5, 3)] = 0.69;
        j[(5, 4)] = 1.71;
        j[(5, 5)] = -280.0 * y[7] - 0.43;
        j[(5, 6)] = 0.69;
        j[(5, 7)] = -280.0 * y[5];
        j[(6, 5)] = 280.0 * y[7];
        j[(6, 6)] = -1.81;
        j[(6, 7)] = 280.0 * y[5];
        j[(7, 5)] = -280.0 * y[7];
        j[(7, 6)] = 1.81;
        j[(7, 7)] = -280.0 * y[5];
    });
    ProblemDef {
        family: "hires",
        description: "HIRES plant-physiology kinetics, n = 8".into(),
        n: 8,
        y0: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
        t_span: (0.0, 321.8122),
        rtol: 1.0e-6,
        atol: 1.0e-10,
        initial_step: 1.0e-6,
        rhs,
        dense_fill: Some(dense_fill),
        band: None,
        jvp: None,
    }
}

// ---------------------------------------------------------------------------------------------
// 1D Brusselator, method of lines, species interleaved (u_i, v_i), Dirichlet u = 1, v = 3.
// u_t = 1 + u^2 v - 4 u + c (u_l - 2 u + u_r), v_t = 3 u - u^2 v + c (v_l - 2 v + v_r),
// c = (cells + 1)^2 / 50. Band: lower = upper = 2.
// ---------------------------------------------------------------------------------------------

pub fn bruss1d(cells: usize) -> ProblemDef {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    });
    let dense_fill: DenseFillFn = Arc::new(move |y, j| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = -u * u - 2.0 * c;
            if i > 0 {
                j[(a, a - 2)] = c;
                j[(b, b - 2)] = c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = c;
                j[(b, b + 2)] = c;
            }
        }
    });
    let band_fill: BandFillFn = Arc::new(move |_t, y, band| {
        // Row r, column q at r * 5 + (q + 2 - r).
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            let (da, db) = (a * 5 + 2, b * 5 + 2);
            band[da] = 2.0 * u * v - 4.0 - 2.0 * c;
            band[da + 1] = u * u;
            band[db - 1] = 3.0 - 2.0 * u * v;
            band[db] = -u * u - 2.0 * c;
            if i > 0 {
                band[da - 2] = c;
                band[db - 2] = c;
            }
            if i + 1 < cells {
                band[da + 2] = c;
                band[db + 2] = c;
            }
        }
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, w, out| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            let (wl_u, wl_v) = if i == 0 {
                (0.0, 0.0)
            } else {
                (w[a - 2], w[b - 2])
            };
            let (wr_u, wr_v) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (w[a + 2], w[b + 2])
            };
            out[a] = (2.0 * u * v - 4.0 - 2.0 * c) * w[a] + u * u * w[b] + c * (wl_u + wr_u);
            out[b] = (3.0 - 2.0 * u * v) * w[a] + (-u * u - 2.0 * c) * w[b] + c * (wl_v + wr_v);
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    ProblemDef {
        family: "bruss1d",
        description: format!(
            "1D Brusselator, method of lines, {cells} cells, n = {n}, diffusion (cells+1)^2/50"
        ),
        n,
        y0,
        t_span: (0.0, 10.0),
        rtol: 1.0e-6,
        atol: 1.0e-6,
        initial_step: 1.0e-6,
        rhs,
        dense_fill: Some(dense_fill),
        band: Some(Band {
            lower: 2,
            upper: 2,
            fill: band_fill,
        }),
        jvp: Some(jvp),
    }
}

// ---------------------------------------------------------------------------------------------
// Radiative-conductive lumped thermal network on a ring (genuinely dense Jacobian).
//
//   C_i T_i' = sum_j R_ij (T_j^4 - T_i^4) + g0 (T_{i-1} - 2 T_i + T_{i+1}) + ge (1 - T_i) + q_i
//
// R_ij = (1/n) / (1 + (d_ij / 0.1)^2), d_ij the ring distance in [0, 1/2] (every entry nonzero);
// capacities C_i = 10^(-4 frac(i phi)), phi the golden ratio conjugate, span four decades; the
// physical model is a radiosity network, its parameters are generated by these formulas.
// ---------------------------------------------------------------------------------------------

struct RadNet {
    n: usize,
    r: Vec<f64>,
    rowsum: Vec<f64>,
    inv_c: Vec<f64>,
    q: Vec<f64>,
    g0: f64,
    ge: f64,
}

/// A dot product with four accumulators in a fixed order (deterministic).
fn dot4(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = [0.0_f64; 4];
    let (ca, cb) = (a.chunks_exact(4), b.chunks_exact(4));
    let (ra, rb) = (ca.remainder(), cb.remainder());
    for (x, y) in ca.zip(cb) {
        acc[0] += x[0] * y[0];
        acc[1] += x[1] * y[1];
        acc[2] += x[2] * y[2];
        acc[3] += x[3] * y[3];
    }
    let mut tail = 0.0;
    for (x, y) in ra.iter().zip(rb) {
        tail += x * y;
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3]) + tail
}

pub fn radnet(n: usize) -> ProblemDef {
    assert!(n >= 4, "the ring needs at least four nodes");
    let golden = 0.618_033_988_749_894_9_f64;
    let mut r = vec![0.0; n * n];
    let mut rowsum = vec![0.0; n];
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..n {
            let k = i.abs_diff(j);
            let d = k.min(n - k) as f64 / n as f64;
            let value = (1.0 / n as f64) / (1.0 + (d / 0.1) * (d / 0.1));
            r[i * n + j] = value;
            sum += value;
        }
        rowsum[i] = sum;
    }
    let inv_c: Vec<f64> = (0..n)
        .map(|i| 10.0_f64.powf(4.0 * (i as f64 * golden).fract()))
        .collect();
    let q: Vec<f64> = (0..n)
        .map(|i| 0.5 * (1.0 + (2.0 * PI * i as f64 / n as f64).sin()))
        .collect();
    let net = Arc::new(RadNet {
        n,
        r,
        rowsum,
        inv_c,
        q,
        g0: 1.0,
        ge: 0.1,
    });
    let scratch_rhs = Mutex::new(vec![0.0; n]);
    let net_rhs = Arc::clone(&net);
    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        let net = &net_rhs;
        let n = net.n;
        let mut t4 = scratch_rhs.lock().unwrap_or_else(|e| e.into_inner());
        for (p, v) in t4.iter_mut().zip(y) {
            let s = v * v;
            *p = s * s;
        }
        for i in 0..n {
            let exchange = dot4(&net.r[i * n..(i + 1) * n], &t4) - t4[i] * net.rowsum[i];
            let left = y[if i == 0 { n - 1 } else { i - 1 }];
            let right = y[if i + 1 == n { 0 } else { i + 1 }];
            out[i] = net.inv_c[i]
                * (exchange
                    + net.g0 * (left - 2.0 * y[i] + right)
                    + net.ge * (1.0 - y[i])
                    + net.q[i]);
        }
        Ok(())
    });
    let scratch_jac = Mutex::new(vec![0.0; n]);
    let net_jac = Arc::clone(&net);
    let dense_fill: DenseFillFn = Arc::new(move |y, j| {
        let net = &net_jac;
        let n = net.n;
        let mut t3 = scratch_jac.lock().unwrap_or_else(|e| e.into_inner());
        for (p, v) in t3.iter_mut().zip(y) {
            *p = 4.0 * v * v * v;
        }
        let data = j.as_mut_slice();
        for i in 0..n {
            let ci = net.inv_c[i];
            let row = &mut data[i * n..(i + 1) * n];
            for ((entry, rij), t3j) in row.iter_mut().zip(&net.r[i * n..(i + 1) * n]).zip(&*t3) {
                *entry = ci * rij * t3j;
            }
            row[i] += ci * (-t3[i] * net.rowsum[i] - 2.0 * net.g0 - net.ge);
            row[if i == 0 { n - 1 } else { i - 1 }] += ci * net.g0;
            row[if i + 1 == n { 0 } else { i + 1 }] += ci * net.g0;
        }
    });
    let y0 = (0..n)
        .map(|i| 1.0 + 0.5 * (2.0 * PI * 3.0 * i as f64 / n as f64).sin())
        .collect();
    ProblemDef {
        family: "radnet",
        description: format!(
            "radiative-conductive lumped thermal network on a ring, n = {n}, all-to-all T^4 exchange, capacities over four decades"
        ),
        n,
        y0,
        t_span: (0.0, 2.0),
        rtol: 1.0e-6,
        atol: 1.0e-8,
        initial_step: 1.0e-6,
        rhs,
        dense_fill: Some(dense_fill),
        band: None,
        jvp: None,
    }
}

// ---------------------------------------------------------------------------------------------
// Allen-Cahn u_t = D lap u + kappa (u - u^3) on the unit square or cube, cell-centred second-order
// finite volumes, zero-flux boundaries, natural ordering (x fastest). Band: lower = upper = m in
// 2D and m^2 in 3D.
// ---------------------------------------------------------------------------------------------

pub fn allen_cahn(dim: usize, m: usize) -> ProblemDef {
    assert!(dim == 2 || dim == 3);
    let (d_coeff, kappa) = if dim == 2 { (0.01, 1.0) } else { (0.04, 1.0) };
    let n = m.pow(dim as u32);
    let s = d_coeff * (m as f64) * (m as f64);
    let mz = if dim == 3 { m } else { 1 };
    let (sy, sz) = (m, m * m);
    let width = if dim == 2 { m } else { m * m };

    // The discrete zero-flux Laplacian of `x` at every cell, scaled by `s`, visited as
    // `visit(cell, s * sum (x_nb - x_c), neighbours)`.
    #[inline(always)]
    fn stencil(m: usize, mz: usize, x: &[f64], mut visit: impl FnMut(usize, f64, f64)) {
        let (sy, sz) = (m, m * m);
        for k in 0..mz {
            for j in 0..m {
                let base = k * sz + j * sy;
                for i in 0..m {
                    let c = base + i;
                    let xc = x[c];
                    let mut sum = 0.0;
                    let mut nb = 0.0;
                    if i > 0 {
                        sum += x[c - 1] - xc;
                        nb += 1.0;
                    }
                    if i + 1 < m {
                        sum += x[c + 1] - xc;
                        nb += 1.0;
                    }
                    if j > 0 {
                        sum += x[c - sy] - xc;
                        nb += 1.0;
                    }
                    if j + 1 < m {
                        sum += x[c + sy] - xc;
                        nb += 1.0;
                    }
                    if k > 0 {
                        sum += x[c - sz] - xc;
                        nb += 1.0;
                    }
                    if k + 1 < mz {
                        sum += x[c + sz] - xc;
                        nb += 1.0;
                    }
                    visit(c, sum, nb);
                }
            }
        }
    }

    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        stencil(m, mz, y, |c, sum, _| {
            let u = y[c];
            out[c] = s * sum + kappa * (u - u * u * u);
        });
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, w, out| {
        stencil(m, mz, w, |c, sum, _| {
            let u = y[c];
            out[c] = s * sum + kappa * (1.0 - 3.0 * u * u) * w[c];
        });
        Ok(())
    });
    let band_fill: BandFillFn = Arc::new(move |_t, y, band| {
        let jw = 2 * width + 1;
        for k in 0..mz {
            for j in 0..m {
                for i in 0..m {
                    let c = k * sz + j * sy + i;
                    let diag = c * jw + width;
                    let mut nb = 0.0;
                    if i > 0 {
                        band[diag - 1] = s;
                        nb += 1.0;
                    }
                    if i + 1 < m {
                        band[diag + 1] = s;
                        nb += 1.0;
                    }
                    if j > 0 {
                        band[diag - sy] = s;
                        nb += 1.0;
                    }
                    if j + 1 < m {
                        band[diag + sy] = s;
                        nb += 1.0;
                    }
                    if k > 0 {
                        band[diag - sz] = s;
                        nb += 1.0;
                    }
                    if k + 1 < mz {
                        band[diag + sz] = s;
                        nb += 1.0;
                    }
                    let u = y[c];
                    band[diag] = -nb * s + kappa * (1.0 - 3.0 * u * u);
                }
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for k in 0..mz {
        for j in 0..m {
            for i in 0..m {
                let x = (i as f64 + 0.5) / m as f64;
                let y = (j as f64 + 0.5) / m as f64;
                let z = (k as f64 + 0.5) / m as f64;
                let value = if dim == 2 {
                    0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos()
                        + 0.3 * (5.0 * PI * x).cos() * (PI * y).cos()
                        - 0.1
                } else {
                    0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos() * (PI * z).cos()
                        + 0.3 * (3.0 * PI * x).cos() * (PI * y).cos() * (2.0 * PI * z).cos()
                        - 0.1
                };
                y0[k * sz + j * sy + i] = value;
            }
        }
    }
    ProblemDef {
        family: if dim == 2 { "ac2d" } else { "ac3d" },
        description: format!(
            "Allen-Cahn, {dim}D, {m}^{dim} cells, n = {n}, D = {d_coeff}, kappa = {kappa}, zero-flux boundaries"
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
            lower: width,
            upper: width,
            fill: band_fill,
        }),
        jvp: Some(jvp),
    }
}

// ---------------------------------------------------------------------------------------------
// 2D Brusselator u_t = A + u^2 v - (B + 1) u + alpha lap u, v_t = B u - u^2 v + alpha lap v on the
// unit square, A = 1, B = 3.4, alpha = 0.02, zero-flux boundaries, species interleaved per cell
// (index 2 (i + m j) + species). Band: lower = upper = 2 m.
// ---------------------------------------------------------------------------------------------

pub fn bruss2d(m: usize) -> ProblemDef {
    let (a_par, b_par, alpha) = (1.0, 3.4, 0.02);
    let n = 2 * m * m;
    let s = alpha * (m as f64) * (m as f64);
    let sy = 2 * m;

    // `visit(a, s * lap(x_u), s * lap(x_v), neighbours)` with `a` the index of `u` in the cell.
    #[inline(always)]
    fn stencil(m: usize, x: &[f64], mut visit: impl FnMut(usize, f64, f64, f64)) {
        let sy = 2 * m;
        for j in 0..m {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let (xu, xv) = (x[a], x[a + 1]);
                let (mut su, mut sv, mut nb) = (0.0, 0.0, 0.0);
                if i > 0 {
                    su += x[a - 2] - xu;
                    sv += x[a - 1] - xv;
                    nb += 1.0;
                }
                if i + 1 < m {
                    su += x[a + 2] - xu;
                    sv += x[a + 3] - xv;
                    nb += 1.0;
                }
                if j > 0 {
                    su += x[a - sy] - xu;
                    sv += x[a + 1 - sy] - xv;
                    nb += 1.0;
                }
                if j + 1 < m {
                    su += x[a + sy] - xu;
                    sv += x[a + 1 + sy] - xv;
                    nb += 1.0;
                }
                visit(a, su, sv, nb);
            }
        }
    }

    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        stencil(m, y, |a, su, sv, _| {
            let (u, v) = (y[a], y[a + 1]);
            out[a] = a_par + u * u * v - (b_par + 1.0) * u + s * su;
            out[a + 1] = b_par * u - u * u * v + s * sv;
        });
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, w, out| {
        stencil(m, w, |a, su, sv, _| {
            let (u, v) = (y[a], y[a + 1]);
            out[a] = (2.0 * u * v - (b_par + 1.0)) * w[a] + u * u * w[a + 1] + s * su;
            out[a + 1] = (b_par - 2.0 * u * v) * w[a] - u * u * w[a + 1] + s * sv;
        });
        Ok(())
    });
    let band_fill: BandFillFn = Arc::new(move |_t, y, band| {
        let width = 2 * m;
        let jw = 2 * width + 1;
        for j in 0..m {
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
                if j + 1 < m {
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
    for j in 0..m {
        for i in 0..m {
            let x = (i as f64 + 0.5) / m as f64;
            let y = (j as f64 + 0.5) / m as f64;
            let a = 2 * (i + m * j);
            y0[a] = 0.5 + y;
            y0[a + 1] = 1.0 + 5.0 * x;
        }
    }
    ProblemDef {
        family: "bruss2d",
        description: format!(
            "2D Brusselator, {m}x{m} cells, n = {n}, A = 1, B = 3.4, alpha = 0.02, zero-flux boundaries"
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

/// The dense Jacobian of a problem with a JVP, column by column (the consistency self-test).
pub fn dense_from_jvp(def: &ProblemDef, y: &[f64]) -> CoreResult<DenseMatrix> {
    let n = def.n;
    let jvp = def.jvp.as_ref().expect("problem with a JVP");
    let mut matrix = DenseMatrix::zeros(n, n);
    let (mut basis, mut column) = (vec![0.0; n], vec![0.0; n]);
    for j in 0..n {
        basis[j] = 1.0;
        jvp(def.t_span.0, y, &basis, &mut column)?;
        basis[j] = 0.0;
        for i in 0..n {
            matrix[(i, j)] = column[i];
        }
    }
    Ok(matrix)
}
