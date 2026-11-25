//! Problem families of loop node V25 (confirmation of the adaptive
//! matrix-free stage solve P1 on families no earlier node used, owner
//! decision OD-L1-15), included by the `mf_confirm_*` test targets with
//! `#[path]`.
//!
//! The declared instances are those of
//! `research/production_loop_20261116/mf_p1_confirm/PREDECLARED.json`
//! ([`declared`]): `ac3d` (m = 16, n = 4096), `robertson_rd1d` (the V06
//! generator with 400 cells, n = 1200), `schnakenberg2d` (m = 32, n = 2048)
//! and `porous2d` (m = 48, n = 2304). Every problem is JVP-only, autonomous
//! and allocation-free in its callbacks and comes with the analytic diagonal
//! of its Jacobian (the [`Family`] record needs one; no run of the node uses
//! a preconditioner). The independent numpy implementation of the same
//! right-hand sides is `mf_p1_confirm/reference/families.py`.
#![allow(dead_code, unused_imports)]

#[path = "stage_clients_krylov_v17_problems.rs"]
pub mod v17;

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::OdeProblem;
use rodas5p_integrators::stage_clients::krylov::JacobianDiagonalFn;

pub use v17::v06::Family;
pub use v17::{AuditSummary, ProtectedAudit, adaptive};

fn jvp_only(
    name: String,
    n: usize,
    rhs: impl Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static,
    jvp: impl Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static,
) -> CoreResult<OdeProblem> {
    OdeProblem::new(
        name,
        n,
        Arc::new(rhs),
        None,
        None,
        Some(Arc::new(jvp)),
        None,
        true,
        None,
        None,
    )
}

/// The cell centres `(i + 1/2)/m` of a uniform grid on `[0, 1]`.
fn centre(i: usize, m: usize) -> f64 {
    (i as f64 + 0.5) / m as f64
}

/// 3D Allen-Cahn equation on the unit cube with zero-flux boundaries:
///
/// ```text
/// u_t = D lap(u) + lam (u - u^3),  D = 0.1, lam = 10
/// ```
///
/// `m^3` cell-centred finite volumes, 7-point Laplacian
/// `m^2 (sum of the six neighbours - 6 u)` where a neighbour outside the
/// cube is the cell itself; index `i + m (j + m k)`, cell centre
/// `((i + 1/2)/m, (j + 1/2)/m, (k + 1/2)/m)`; start
/// `u = 0.4 cos(2 pi x) cos(pi y) + 0.3 cos(3 pi x) cos(pi z) + 0.05`.
/// JVP: `D lap(v) + lam (1 - 3 u^2) v`.
pub fn ac3d(m: usize) -> CoreResult<Family> {
    if m < 2 {
        return Err(CoreError::InvalidInput("invalid Allen-Cahn grid".into()));
    }
    const D: f64 = 0.1;
    const LAM: f64 = 10.0;
    let n = m * m * m;
    let scale = (m * m) as f64;
    // `m^2 (xm + xp + ym + yp + zm + zp - 6 w)` with mirrored ghost cells.
    let laplace = move |w: &[f64], i: usize, j: usize, k: usize| {
        let c = i + m * (j + m * k);
        let centre = w[c];
        let xm = if i > 0 { w[c - 1] } else { centre };
        let xp = if i + 1 < m { w[c + 1] } else { centre };
        let ym = if j > 0 { w[c - m] } else { centre };
        let yp = if j + 1 < m { w[c + m] } else { centre };
        let zm = if k > 0 { w[c - m * m] } else { centre };
        let zp = if k + 1 < m { w[c + m * m] } else { centre };
        scale * (xm + xp + ym + yp + zm + zp - 6.0 * centre)
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    let u = y[c];
                    out[c] = D * laplace(y, i, j, k) + LAM * (u - u * u * u);
                }
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    let u = y[c];
                    out[c] = D * laplace(v, i, j, k) + LAM * (1.0 - 3.0 * u * u) * v[c];
                }
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    let neighbours = f64::from(
                        u8::from(i > 0)
                            + u8::from(i + 1 < m)
                            + u8::from(j > 0)
                            + u8::from(j + 1 < m)
                            + u8::from(k > 0)
                            + u8::from(k + 1 < m),
                    );
                    let u = y[c];
                    out[c] = -D * scale * neighbours + LAM * (1.0 - 3.0 * u * u);
                }
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for k in 0..m {
        for j in 0..m {
            for i in 0..m {
                let (x, y, z) = (centre(i, m), centre(j, m), centre(k, m));
                y0[i + m * (j + m * k)] = 0.4 * (2.0 * PI * x).cos() * (PI * y).cos()
                    + 0.3 * (3.0 * PI * x).cos() * (PI * z).cos()
                    + 0.05;
            }
        }
    }
    Ok(Family {
        id: format!("ac3d-{m}"),
        problem: jvp_only(format!("ac3d-{m}"), n, rhs, jvp)?,
        y0,
        span: SPAN_AC3D,
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// Robertson kinetics with diffusion: the generator
/// [`v17::v06::robertson_rd1d`] of node V06 unchanged (rates, diffusion
/// `1e-5 / dx^2`, start), with `atol = rtol` as the frozen tolerances of
/// node V25 require for every family (the V06 generator carries
/// `atol_over_rtol = 1e-4`, a protocol choice of node V06).
pub fn robertson_rd1d(cells: usize) -> CoreResult<Family> {
    let mut family = v17::v06::robertson_rd1d(cells)?;
    family.atol_over_rtol = 1.0;
    family.span = SPAN_ROBERTSON_RD1D;
    Ok(family)
}

/// The Schnakenberg reaction-diffusion system on the unit square with
/// zero-flux boundaries:
///
/// ```text
/// u_t = du lap(u) + gam (a - u + u^2 v)
/// v_t = dv lap(v) + gam (b - u^2 v)
/// ```
///
/// `du = 0.01`, `dv = 0.4`, `gam = 100`, `a = 0.1`, `b = 0.9`; `m^2`
/// cell-centred finite volumes, 5-point Laplacian
/// `m^2 (xm + xp + ym + yp - 4 w)` where a neighbour outside the square is
/// the cell itself; species interleaved per cell, index `2 (i + m j) + s`;
/// start `u = a + b + 0.05 cos(2 pi x) cos(2 pi y)`,
/// `v = b / (a + b)^2 + 0.05 sin(2 pi x) sin(3 pi y)`.
pub fn schnakenberg2d(m: usize) -> CoreResult<Family> {
    if m < 2 {
        return Err(CoreError::InvalidInput("invalid Schnakenberg grid".into()));
    }
    const DU: f64 = 0.01;
    const DV: f64 = 0.4;
    const GAM: f64 = 100.0;
    const A: f64 = 0.1;
    const B: f64 = 0.9;
    let cells = m * m;
    let n = 2 * cells;
    let scale = (m * m) as f64;
    // `m^2 (xm + xp + ym + yp - 4 w)` for species `s` of cell `(i, j)`.
    let laplace = move |w: &[f64], i: usize, j: usize, s: usize| {
        let c = 2 * (i + m * j) + s;
        let centre = w[c];
        let xm = if i > 0 { w[c - 2] } else { centre };
        let xp = if i + 1 < m { w[c + 2] } else { centre };
        let ym = if j > 0 { w[c - 2 * m] } else { centre };
        let yp = if j + 1 < m { w[c + 2 * m] } else { centre };
        scale * (xm + xp + ym + yp - 4.0 * centre)
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = 2 * (i + m * j);
                let (u, v) = (y[c], y[c + 1]);
                let uuv = u * u * v;
                out[c] = DU * laplace(y, i, j, 0) + GAM * (A - u + uuv);
                out[c + 1] = DV * laplace(y, i, j, 1) + GAM * (B - uuv);
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], d: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = 2 * (i + m * j);
                let (u, v) = (y[c], y[c + 1]);
                let (du, dv) = (d[c], d[c + 1]);
                let uv2 = 2.0 * u * v;
                let uu = u * u;
                out[c] = DU * laplace(d, i, j, 0) + GAM * ((uv2 - 1.0) * du + uu * dv);
                out[c + 1] = DV * laplace(d, i, j, 1) - GAM * (uv2 * du + uu * dv);
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = 2 * (i + m * j);
                let neighbours = f64::from(
                    u8::from(i > 0) + u8::from(i + 1 < m) + u8::from(j > 0) + u8::from(j + 1 < m),
                );
                let (u, v) = (y[c], y[c + 1]);
                out[c] = -DU * scale * neighbours + GAM * (2.0 * u * v - 1.0);
                out[c + 1] = -DV * scale * neighbours - GAM * u * u;
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for j in 0..m {
        for i in 0..m {
            let (x, y) = (centre(i, m), centre(j, m));
            let c = 2 * (i + m * j);
            y0[c] = A + B + 0.05 * (2.0 * PI * x).cos() * (2.0 * PI * y).cos();
            y0[c + 1] =
                B / ((A + B) * (A + B)) + 0.05 * (2.0 * PI * x).sin() * (3.0 * PI * y).sin();
        }
    }
    Ok(Family {
        id: format!("schnakenberg2d-{m}"),
        problem: jvp_only(format!("schnakenberg2d-{m}"), n, rhs, jvp)?,
        y0,
        span: SPAN_SCHNAKENBERG2D,
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// The porous medium equation `u_t = lap(u^2)` on the unit square with
/// zero-flux boundaries: `m^2` cell-centred finite volumes, 5-point
/// Laplacian `m^2 (xm + xp + ym + yp - 4 w)` of `w = u^2`, where a neighbour
/// outside the square is the cell itself; index `i + m j`, cell centre
/// `((i + 1/2)/m, (j + 1/2)/m)`; start
/// `u = 0.1 + exp(-50 ((x - 1/2)^2 + (y - 1/2)^2))`. JVP: `lap(2 u v)`.
pub fn porous2d(m: usize) -> CoreResult<Family> {
    if m < 2 {
        return Err(CoreError::InvalidInput("invalid porous-medium grid".into()));
    }
    let n = m * m;
    let scale = (m * m) as f64;
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let centre = y[c] * y[c];
                let xm = if i > 0 { y[c - 1] * y[c - 1] } else { centre };
                let xp = if i + 1 < m {
                    y[c + 1] * y[c + 1]
                } else {
                    centre
                };
                let ym = if j > 0 { y[c - m] * y[c - m] } else { centre };
                let yp = if j + 1 < m {
                    y[c + m] * y[c + m]
                } else {
                    centre
                };
                out[c] = scale * (xm + xp + ym + yp - 4.0 * centre);
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let w = |q: usize| 2.0 * y[q] * v[q];
                let centre = w(c);
                let xm = if i > 0 { w(c - 1) } else { centre };
                let xp = if i + 1 < m { w(c + 1) } else { centre };
                let ym = if j > 0 { w(c - m) } else { centre };
                let yp = if j + 1 < m { w(c + m) } else { centre };
                out[c] = scale * (xm + xp + ym + yp - 4.0 * centre);
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let neighbours = f64::from(
                    u8::from(i > 0) + u8::from(i + 1 < m) + u8::from(j > 0) + u8::from(j + 1 < m),
                );
                out[c] = -scale * neighbours * 2.0 * y[c];
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for j in 0..m {
        for i in 0..m {
            let (x, y) = (centre(i, m), centre(j, m));
            y0[i + m * j] = 0.1 + (-50.0 * ((x - 0.5) * (x - 0.5) + (y - 0.5) * (y - 0.5))).exp();
        }
    }
    Ok(Family {
        id: format!("porous2d-{m}"),
        problem: jvp_only(format!("porous2d-{m}"), n, rhs, jvp)?,
        y0,
        span: SPAN_POROUS2D,
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// The provisional time spans of `PREDECLARED.json`; the final spans are
/// those of `PROTOCOL.json` (sizing pilot) and reach the measurement target
/// as its end-time variable.
pub const SPAN_AC3D: (f64, f64) = (0.0, 1.0);
pub const SPAN_ROBERTSON_RD1D: (f64, f64) = (0.0, 40.0);
pub const SPAN_SCHNAKENBERG2D: (f64, f64) = (0.0, 1.0);
pub const SPAN_POROUS2D: (f64, f64) = (0.0, 0.1);

/// The declared family ids of the node.
pub const DECLARED: [&str; 4] = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"];

/// The declared instances of `PREDECLARED.json` by id, and reduced
/// instances (`dev_*`) for the contract tests.
pub fn declared(id: &str) -> CoreResult<Family> {
    let mut family = match id {
        "ac3d" => ac3d(16)?,
        "robertson_rd1d" => robertson_rd1d(400)?,
        "schnakenberg2d" => schnakenberg2d(32)?,
        "porous2d" => porous2d(48)?,
        "dev_ac3d" => ac3d(4)?,
        "dev_robertson_rd1d" => robertson_rd1d(8)?,
        "dev_schnakenberg2d" => schnakenberg2d(6)?,
        "dev_porous2d" => porous2d(6)?,
        _ => {
            return Err(CoreError::InvalidInput(format!("unknown V25 family {id}")));
        }
    };
    family.id = id.to_string();
    Ok(family)
}

/// The family `id` integrated over `[t0, t1]` instead of its provisional
/// span.
pub fn with_end(mut family: Family, t1: f64) -> CoreResult<Family> {
    if !(t1.is_finite() && t1 > family.span.0) {
        return Err(CoreError::InvalidInput(format!(
            "invalid end time {t1} for {}",
            family.id
        )));
    }
    family.span.1 = t1;
    Ok(family)
}

/// SplitMix64: the deterministic generator of the consistency states.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniform number in `[-1, 1)` from the top 53 bits.
    pub fn symmetric(&mut self) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        2.0 * unit - 1.0
    }
}

/// The seed of the consistency states.
pub const CONSISTENCY_SEED: u64 = 0x0025_c0de_5eed_0001;
/// The number of consistency states (`PREDECLARED.json`).
pub const CONSISTENCY_STATES: usize = 20;

/// State `k` and direction `k` of the consistency check of `family`:
/// `y_i = y0_i (1 + 0.2 r) + 0.05 r'` and `v_i = r''` with `r, r', r''`
/// uniform in `[-1, 1)` from [`SplitMix64`] seeded with
/// [`CONSISTENCY_SEED`] + `k`, drawn in the order `r, r', r''` per
/// component.
pub fn consistency_state(family: &Family, k: usize) -> (Vec<f64>, Vec<f64>) {
    let mut rng = SplitMix64(CONSISTENCY_SEED.wrapping_add(k as u64));
    let n = family.y0.len();
    let (mut y, mut v) = (vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        let r = rng.symmetric();
        let r2 = rng.symmetric();
        let r3 = rng.symmetric();
        y[i] = family.y0[i] * (1.0 + 0.2 * r) + 0.05 * r2;
        v[i] = r3;
    }
    (y, v)
}
