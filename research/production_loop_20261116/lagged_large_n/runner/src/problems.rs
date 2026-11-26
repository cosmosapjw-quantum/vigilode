//! The problem families of loop L1 node V09_LAGGED_LARGE_N (`../../PREDECLARED.json`).
//!
//! Every problem is autonomous, has the identity mass matrix and gives its right-hand side, an
//! analytic Jacobian written into a dense matrix and an analytic Jacobian-vector product. The
//! dense fill writes every entry of the problem's fixed pattern on every call and nothing else;
//! the drivers keep their Jacobian buffer between calls, so the entries outside the pattern stay
//! zero (the pattern of V04's runner, `../../cost_anatomy/runner/src/problems.rs`). Nothing here
//! measures anything.

use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreResult, DenseMatrix};

pub type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
/// Writes every entry of the fixed pattern of the dense Jacobian at `y`.
pub type DenseFillFn = Arc<dyn Fn(&[f64], &mut DenseMatrix) + Send + Sync>;

pub struct ProblemDef {
    pub family: &'static str,
    /// `genuinely-dense`, `dense-storage-coupled` or `dense-storage-stencil`.
    pub structure: &'static str,
    pub description: String,
    /// The size parameter of the family: n (radnet, acspec, vdpnet), cells per axis (ac2d,
    /// bruss2d).
    pub size: usize,
    pub n: usize,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub rhs: RhsFn,
    pub dense_fill: DenseFillFn,
    pub jvp: JvpFn,
}

/// The families of the node and their size parameters.
pub const FAMILIES: [&str; 5] = ["radnet", "acspec", "vdpnet", "ac2d", "bruss2d"];

pub fn sizes(family: &str) -> &'static [usize] {
    match family {
        "radnet" | "acspec" | "vdpnet" => &[256, 512, 1024],
        "ac2d" => &[32, 45],
        "bruss2d" => &[23, 32],
        _ => &[],
    }
}

pub fn definition(family: &str, size: usize) -> Option<ProblemDef> {
    match family {
        "radnet" => Some(radnet(size)),
        "acspec" => Some(acspec(size)),
        "vdpnet" => Some(vdpnet(size)),
        "ac2d" => Some(ac2d(size)),
        "bruss2d" => Some(bruss2d(size)),
        _ => None,
    }
}

/// A dot product with four accumulators in a fixed order (deterministic).
pub fn dot4(a: &[f64], b: &[f64]) -> f64 {
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

// ---------------------------------------------------------------------------------------------
// radnet: radiative-conductive lumped thermal network on a ring (genuinely dense Jacobian),
// node V04's definition unchanged (`../../cost_anatomy/runner/src/problems.rs`):
//
//   C_i T_i' = sum_j R_ij (T_j^4 - T_i^4) + g0 (T_{i-1} - 2 T_i + T_{i+1}) + ge (1 - T_i) + q_i
//
// R_ij = (1/n) / (1 + (d_ij / 0.1)^2), d_ij the ring distance in [0, 1/2]; 1/C_i =
// 10^(4 frac(i phi)), phi the golden-ratio conjugate; g0 = 1, ge = 0.1,
// q_i = (1 + sin(2 pi i / n)) / 2, T_i(0) = 1 + sin(6 pi i / n) / 2, t in [0, 2].
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
    // (J v)_i = (1/C_i) (sum_j R_ij 4 T_j^3 v_j - (4 T_i^3 rowsum_i + 2 g0 + ge) v_i
    //            + g0 (v_{i-1} + v_{i+1})).
    let scratch_jvp = Mutex::new(vec![0.0; n]);
    let net_jvp = Arc::clone(&net);
    let jvp: JvpFn = Arc::new(move |_t, y, v, out| {
        let net = &net_jvp;
        let n = net.n;
        let mut w = scratch_jvp.lock().unwrap_or_else(|e| e.into_inner());
        for ((p, yv), vv) in w.iter_mut().zip(y).zip(v) {
            *p = 4.0 * yv * yv * yv * vv;
        }
        for i in 0..n {
            let t3i = 4.0 * y[i] * y[i] * y[i];
            let left = v[if i == 0 { n - 1 } else { i - 1 }];
            let right = v[if i + 1 == n { 0 } else { i + 1 }];
            out[i] = net.inv_c[i]
                * (dot4(&net.r[i * n..(i + 1) * n], &w)
                    - (t3i * net.rowsum[i] + 2.0 * net.g0 + net.ge) * v[i]
                    + net.g0 * (left + right));
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|i| 1.0 + 0.5 * (2.0 * PI * 3.0 * i as f64 / n as f64).sin())
        .collect();
    ProblemDef {
        family: "radnet",
        structure: "genuinely-dense",
        description: format!(
            "radiative-conductive lumped thermal network on a ring, n = {n}, all-to-all T^4 exchange, capacities over four decades (node V04's definition)"
        ),
        size: n,
        n,
        y0,
        t_span: (0.0, 2.0),
        rhs,
        dense_fill,
        jvp,
    }
}

// ---------------------------------------------------------------------------------------------
// acspec: Allen-Cahn u_t = nu u_xx + u - u^3 on [0, 2 pi) with periodic boundaries, Fourier
// collocation on n equispaced nodes x_j = 2 pi j / n (n even). The second-derivative matrix is
// dense (every entry nonzero):
//   (D2)_jj = -pi^2 / (3 h^2) - 1/6,  (D2)_jk = -(-1)^(j-k) / (2 sin^2((j - k) h / 2)), h = 2 pi / n.
// nu = 0.05; u(0) = 0.4 sin x + 0.3 cos(3x + 0.5) + 0.2 sin(5x + 1) - 0.05; t in [0, 10].
// ---------------------------------------------------------------------------------------------

pub const ACSPEC_NU: f64 = 0.05;

/// `nu D2` in row-major order.
fn acspec_matrix(n: usize, nu: f64) -> Vec<f64> {
    assert!(n >= 4 && n % 2 == 0, "Fourier collocation needs an even n >= 4");
    let h = 2.0 * PI / n as f64;
    let diagonal = -PI * PI / (3.0 * h * h) - 1.0 / 6.0;
    let mut a = vec![0.0; n * n];
    for j in 0..n {
        for k in 0..n {
            let value = if j == k {
                diagonal
            } else {
                let d = j as f64 - k as f64;
                let sign = if (j + n - k) % 2 == 0 { 1.0 } else { -1.0 };
                let s = (d * h / 2.0).sin();
                -sign / (2.0 * s * s)
            };
            a[j * n + k] = nu * value;
        }
    }
    a
}

pub fn acspec(n: usize) -> ProblemDef {
    let a = Arc::new(acspec_matrix(n, ACSPEC_NU));
    let a_rhs = Arc::clone(&a);
    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        for j in 0..n {
            let u = y[j];
            out[j] = dot4(&a_rhs[j * n..(j + 1) * n], y) + u - u * u * u;
        }
        Ok(())
    });
    let a_jac = Arc::clone(&a);
    let dense_fill: DenseFillFn = Arc::new(move |y, m| {
        let data = m.as_mut_slice();
        data.copy_from_slice(&a_jac);
        for j in 0..n {
            data[j * n + j] += 1.0 - 3.0 * y[j] * y[j];
        }
    });
    let a_jvp = Arc::clone(&a);
    let jvp: JvpFn = Arc::new(move |_t, y, v, out| {
        for j in 0..n {
            out[j] = dot4(&a_jvp[j * n..(j + 1) * n], v) + (1.0 - 3.0 * y[j] * y[j]) * v[j];
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|j| {
            let x = 2.0 * PI * j as f64 / n as f64;
            0.4 * x.sin() + 0.3 * (3.0 * x + 0.5).cos() + 0.2 * (5.0 * x + 1.0).sin() - 0.05
        })
        .collect();
    ProblemDef {
        family: "acspec",
        structure: "genuinely-dense",
        description: format!(
            "Allen-Cahn u_t = {ACSPEC_NU} u_xx + u - u^3 on [0, 2 pi), periodic, Fourier collocation, n = {n} (dense second-derivative matrix)"
        ),
        size: n,
        n,
        y0,
        t_span: (0.0, 10.0),
        rhs,
        dense_fill,
        jvp,
    }
}

// ---------------------------------------------------------------------------------------------
// vdpnet: m = n/2 relaxation (Lienard / Van der Pol) oscillators with mean-field coupling,
// states interleaved (x_i, y_i) at (2i, 2i + 1):
//   x_i' = (y_i - x_i^3/3 + x_i) / eps,   y_i' = -omega_i x_i + kappa (xbar - x_i),
//   xbar = (1/m) sum_j x_j,  eps = 1e-3, kappa = 0.05, omega_i = 1 + 0.02 (frac(i phi) - 1/2).
// Oscillator i belongs to phase cluster g = i mod 8 and starts on the slow manifold
// y = x^3/3 - x at phase g/8 of the singular limit cycle (right branch x in [1, 2] for g < 4,
// left branch mirrored): the time to the fold, T_b (1 - 2 p) with T_b = 3/2 - ln 2 and
// p = (g mod 4)/8, fixes x by (x^2 - 1)/2 - ln x = T_b (1 - 2 p). A fold (stiff eigenvalue
// (1 - x^2)/eps through zero) and a jump of some cluster occur about every T_b / 4 / 1.05;
// t in [0, 0.4] (two such events). The coupling rows hold m entries each (Jacobian density
// about 1/4).
// ---------------------------------------------------------------------------------------------

pub const VDPNET_EPS: f64 = 1.0e-3;
pub const VDPNET_KAPPA: f64 = 0.05;
pub const VDPNET_CLUSTERS: usize = 8;

/// The x in [1, 2] with (x^2 - 1)/2 - ln x = r, by 200 bisection steps (deterministic).
fn branch_point(r: f64) -> f64 {
    let g = |x: f64| (x * x - 1.0) / 2.0 - x.ln() - r;
    let (mut lo, mut hi) = (1.0_f64, 2.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if g(mid) <= 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

pub fn vdpnet(n: usize) -> ProblemDef {
    assert!(n >= 2 * VDPNET_CLUSTERS && n % 2 == 0);
    let m = n / 2;
    let golden = 0.618_033_988_749_894_9_f64;
    let (eps, kappa) = (VDPNET_EPS, VDPNET_KAPPA);
    let omega: Arc<Vec<f64>> = Arc::new(
        (0..m)
            .map(|i| 1.0 + 0.02 * ((i as f64 * golden).fract() - 0.5))
            .collect(),
    );
    let t_branch = 1.5 - 2.0_f64.ln();
    let mut y0 = vec![0.0; n];
    for i in 0..m {
        let g = i % VDPNET_CLUSTERS;
        let p = (g % (VDPNET_CLUSTERS / 2)) as f64 / VDPNET_CLUSTERS as f64;
        let magnitude = branch_point(t_branch * (1.0 - 2.0 * p));
        let x = if g < VDPNET_CLUSTERS / 2 {
            magnitude
        } else {
            -magnitude
        };
        y0[2 * i] = x;
        y0[2 * i + 1] = x * x * x / 3.0 - x;
    }
    let om = Arc::clone(&omega);
    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        let mut sum = 0.0;
        for i in 0..m {
            sum += y[2 * i];
        }
        let xbar = sum / m as f64;
        for i in 0..m {
            let (x, v) = (y[2 * i], y[2 * i + 1]);
            out[2 * i] = (v - x * x * x / 3.0 + x) / eps;
            out[2 * i + 1] = -om[i] * x + kappa * (xbar - x);
        }
        Ok(())
    });
    let om = Arc::clone(&omega);
    let dense_fill: DenseFillFn = Arc::new(move |y, j| {
        let coupling = kappa / m as f64;
        for i in 0..m {
            let x = y[2 * i];
            j[(2 * i, 2 * i)] = (1.0 - x * x) / eps;
            j[(2 * i, 2 * i + 1)] = 1.0 / eps;
            for k in 0..m {
                j[(2 * i + 1, 2 * k)] = coupling;
            }
            j[(2 * i + 1, 2 * i)] = -om[i] - kappa + coupling;
        }
    });
    let om = Arc::clone(&omega);
    let jvp: JvpFn = Arc::new(move |_t, y, v, out| {
        let mut sum = 0.0;
        for i in 0..m {
            sum += v[2 * i];
        }
        let vbar = sum / m as f64;
        for i in 0..m {
            let x = y[2 * i];
            out[2 * i] = ((1.0 - x * x) * v[2 * i] + v[2 * i + 1]) / eps;
            out[2 * i + 1] = -(om[i] + kappa) * v[2 * i] + kappa * vbar;
        }
        Ok(())
    });
    ProblemDef {
        family: "vdpnet",
        structure: "dense-storage-coupled",
        description: format!(
            "{m} mean-field coupled relaxation oscillators (Lienard form, eps = {VDPNET_EPS}, kappa = {VDPNET_KAPPA}), 8 phase clusters, n = {n}; fast-changing Jacobian (negative control)"
        ),
        size: n,
        n,
        y0,
        t_span: (0.0, 0.4),
        rhs,
        dense_fill,
        jvp,
    }
}

// ---------------------------------------------------------------------------------------------
// ac2d: Allen-Cahn u_t = D lap u + kappa (u - u^3) on the unit square, cell-centred second-order
// finite volumes on m x m cells, zero-flux boundaries, natural ordering (x fastest); node V04's
// discretisation and initial state with D = 0.1, kappa = 1; t in [0, 5]. Stored densely
// (Jacobian density about 5/n).
// ---------------------------------------------------------------------------------------------

pub const AC2D_D: f64 = 0.1;
pub const AC2D_KAPPA: f64 = 1.0;

/// The zero-flux Laplacian of `x` on m x m cells: `visit(cell, sum (x_nb - x_c), neighbours)`.
#[inline(always)]
fn stencil_2d(m: usize, x: &[f64], mut visit: impl FnMut(usize, f64, f64)) {
    for j in 0..m {
        let base = j * m;
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
                sum += x[c - m] - xc;
                nb += 1.0;
            }
            if j + 1 < m {
                sum += x[c + m] - xc;
                nb += 1.0;
            }
            visit(c, sum, nb);
        }
    }
}

pub fn ac2d(m: usize) -> ProblemDef {
    let (d_coeff, kappa) = (AC2D_D, AC2D_KAPPA);
    let n = m * m;
    let s = d_coeff * (m as f64) * (m as f64);
    let rhs: RhsFn = Arc::new(move |_t, y, out| {
        stencil_2d(m, y, |c, sum, _| {
            let u = y[c];
            out[c] = s * sum + kappa * (u - u * u * u);
        });
        Ok(())
    });
    let jvp: JvpFn = Arc::new(move |_t, y, w, out| {
        stencil_2d(m, w, |c, sum, _| {
            let u = y[c];
            out[c] = s * sum + kappa * (1.0 - 3.0 * u * u) * w[c];
        });
        Ok(())
    });
    let dense_fill: DenseFillFn = Arc::new(move |y, jac| {
        for j in 0..m {
            for i in 0..m {
                let c = j * m + i;
                let mut nb = 0.0;
                if i > 0 {
                    jac[(c, c - 1)] = s;
                    nb += 1.0;
                }
                if i + 1 < m {
                    jac[(c, c + 1)] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    jac[(c, c - m)] = s;
                    nb += 1.0;
                }
                if j + 1 < m {
                    jac[(c, c + m)] = s;
                    nb += 1.0;
                }
                let u = y[c];
                jac[(c, c)] = -nb * s + kappa * (1.0 - 3.0 * u * u);
            }
        }
    });
    let mut y0 = vec![0.0; n];
    for j in 0..m {
        for i in 0..m {
            let x = (i as f64 + 0.5) / m as f64;
            let y = (j as f64 + 0.5) / m as f64;
            y0[j * m + i] = 0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos()
                + 0.3 * (5.0 * PI * x).cos() * (PI * y).cos()
                - 0.1;
        }
    }
    ProblemDef {
        family: "ac2d",
        structure: "dense-storage-stencil",
        description: format!(
            "Allen-Cahn 2D, {m}x{m} cells, n = {n}, D = {AC2D_D}, kappa = {AC2D_KAPPA}, zero-flux boundaries, stored densely"
        ),
        size: m,
        n,
        y0,
        t_span: (0.0, 5.0),
        rhs,
        dense_fill,
        jvp,
    }
}

// ---------------------------------------------------------------------------------------------
// bruss2d: u_t = A + u^2 v - (B + 1) u + alpha lap u, v_t = B u - u^2 v + alpha lap v on the unit
// square, A = 1, B = 3.4, alpha = 0.1, m x m cells, zero-flux boundaries, species interleaved
// per cell (index 2 (i + m j) + species); node V04's discretisation and initial state with
// alpha = 0.1; t in [0, 10]. Stored densely (Jacobian density about 6/n).
// ---------------------------------------------------------------------------------------------

pub const BRUSS2D_ALPHA: f64 = 0.1;

pub fn bruss2d(m: usize) -> ProblemDef {
    let (a_par, b_par, alpha) = (1.0, 3.4, BRUSS2D_ALPHA);
    let n = 2 * m * m;
    let s = alpha * (m as f64) * (m as f64);
    let sy = 2 * m;

    // `visit(a, lap(x_u), lap(x_v), neighbours)` with `a` the index of `u` in the cell.
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
    let dense_fill: DenseFillFn = Arc::new(move |y, jac| {
        for j in 0..m {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let b = a + 1;
                let (u, v) = (y[a], y[b]);
                let mut nb = 0.0;
                if i > 0 {
                    jac[(a, a - 2)] = s;
                    jac[(b, b - 2)] = s;
                    nb += 1.0;
                }
                if i + 1 < m {
                    jac[(a, a + 2)] = s;
                    jac[(b, b + 2)] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    jac[(a, a - sy)] = s;
                    jac[(b, b - sy)] = s;
                    nb += 1.0;
                }
                if j + 1 < m {
                    jac[(a, a + sy)] = s;
                    jac[(b, b + sy)] = s;
                    nb += 1.0;
                }
                jac[(a, a)] = 2.0 * u * v - (b_par + 1.0) - nb * s;
                jac[(a, b)] = u * u;
                jac[(b, a)] = b_par - 2.0 * u * v;
                jac[(b, b)] = -u * u - nb * s;
            }
        }
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
        structure: "dense-storage-stencil",
        description: format!(
            "2D Brusselator, {m}x{m} cells, n = {n}, A = 1, B = 3.4, alpha = {BRUSS2D_ALPHA}, zero-flux boundaries, stored densely"
        ),
        size: m,
        n,
        y0,
        t_span: (0.0, 10.0),
        rhs,
        dense_fill,
        jvp,
    }
}

/// The dense Jacobian of a problem by its JVP, column by column (self-test only).
pub fn dense_from_jvp(def: &ProblemDef, y: &[f64]) -> CoreResult<DenseMatrix> {
    let n = def.n;
    let mut matrix = DenseMatrix::zeros(n, n);
    let (mut basis, mut column) = (vec![0.0; n], vec![0.0; n]);
    for j in 0..n {
        basis[j] = 1.0;
        (def.jvp)(def.t_span.0, y, &basis, &mut column)?;
        basis[j] = 0.0;
        for i in 0..n {
            matrix[(i, j)] = column[i];
        }
    }
    Ok(matrix)
}
