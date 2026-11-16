//! 2D Viscous Burgers model for nonlinear advective steepening and shock formation.
//!
//! \partial_t u + u \partial_x u + v \partial_y u = \nu \nabla^2 u
//! \partial_t v + u \partial_x v + v \partial_y v = \nu \nabla^2 v

use crate::spatial_grid::{BoundaryCondition, SpatialGridSpec};

/// 2D Viscous Burgers system.
#[derive(Clone, Debug)]
pub struct Burgers2D {
    /// Number of x points.
    pub nx: usize,
    /// Number of y points.
    pub ny: usize,
    /// Grid spacing in x.
    pub hx: f64,
    /// Grid spacing in y.
    pub hy: f64,
    /// Kinematic viscosity $\nu$ (inverse Reynolds number).
    pub nu: f64,
    /// Boundary conditions.
    pub boundary: BoundaryCondition,
}

impl Burgers2D {
    /// Create standard viscous Burgers on $[0, 1]^2$.
    pub fn standard(nx: usize, ny: usize, nu: f64) -> Self {
        let hx = 1.0 / (nx + 1) as f64;
        let hy = 1.0 / (ny + 1) as f64;
        Self {
            nx,
            ny,
            hx,
            hy,
            nu,
            boundary: BoundaryCondition::DirichletZero,
        }
    }

    /// Total degrees of freedom: $2 \cdot nx \cdot ny$ (interleaved $(u, v)$).
    #[inline(always)]
    pub fn n(&self) -> usize {
        2 * self.nx * self.ny
    }

    /// Corresponding grid specification.
    pub fn grid_spec(&self) -> SpatialGridSpec {
        SpatialGridSpec::rect_2d(self.nx, self.ny, (0.0, 1.0), (0.0, 1.0), 2, self.boundary)
    }

    /// Initial Taylor-Green vortex-like velocity field.
    pub fn initial_state(&self) -> Vec<f64> {
        let mut y0 = vec![0.0; self.n()];
        let pi = std::f64::consts::PI;

        for j in 0..self.ny {
            let y = (j + 1) as f64 * self.hy;
            for i in 0..self.nx {
                let x = (i + 1) as f64 * self.hx;
                let idx = 2 * (i + self.nx * j);
                y0[idx] = (pi * x).sin() * (pi * y).cos();
                y0[idx + 1] = -(pi * x).cos() * (pi * y).sin();
            }
        }
        y0
    }

    /// Evaluate RHS in-place with upwind convective discretization.
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        let nx = self.nx;
        let ny = self.ny;
        let nu = self.nu;
        let inv_hx = 1.0 / self.hx;
        let inv_hy = 1.0 / self.hy;
        let inv_hx2 = inv_hx * inv_hx;
        let inv_hy2 = inv_hy * inv_hy;

        for j in 0..ny {
            for i in 0..nx {
                let idx = 2 * (i + nx * j);
                let u = y[idx];
                let v = y[idx + 1];

                let u_l = if i == 0 { 0.0 } else { y[2 * (i - 1 + nx * j)] };
                let u_r = if i + 1 == nx {
                    0.0
                } else {
                    y[2 * (i + 1 + nx * j)]
                };
                let u_d = if j == 0 {
                    0.0
                } else {
                    y[2 * (i + nx * (j - 1))]
                };
                let u_u = if j + 1 == ny {
                    0.0
                } else {
                    y[2 * (i + nx * (j + 1))]
                };

                let v_l = if i == 0 {
                    0.0
                } else {
                    y[2 * (i - 1 + nx * j) + 1]
                };
                let v_r = if i + 1 == nx {
                    0.0
                } else {
                    y[2 * (i + 1 + nx * j) + 1]
                };
                let v_d = if j == 0 {
                    0.0
                } else {
                    y[2 * (i + nx * (j - 1)) + 1]
                };
                let v_u = if j + 1 == ny {
                    0.0
                } else {
                    y[2 * (i + nx * (j + 1)) + 1]
                };

                // Viscous diffusion: nu * Lap(u), nu * Lap(v)
                let lap_u = (u_l - 2.0 * u + u_r) * inv_hx2 + (u_d - 2.0 * u + u_u) * inv_hy2;
                let lap_v = (v_l - 2.0 * v + v_r) * inv_hx2 + (v_d - 2.0 * v + v_u) * inv_hy2;

                // First-order upwind differencing for advection
                let u_ux = if u >= 0.0 {
                    u * (u - u_l) * inv_hx
                } else {
                    u * (u_r - u) * inv_hx
                };
                let v_uy = if v >= 0.0 {
                    v * (u - u_d) * inv_hy
                } else {
                    v * (u_u - u) * inv_hy
                };

                let u_vx = if u >= 0.0 {
                    u * (v - v_l) * inv_hx
                } else {
                    u * (v_r - v) * inv_hx
                };
                let v_vy = if v >= 0.0 {
                    v * (v - v_d) * inv_hy
                } else {
                    v * (v_u - v) * inv_hy
                };

                dy[idx] = nu * lap_u - u_ux - v_uy;
                dy[idx + 1] = nu * lap_v - u_vx - v_vy;
            }
        }
    }

    /// Iterate through non-zero Jacobian entries.
    pub fn jac_entries(&self, y: &[f64], mut push: impl FnMut(usize, usize, f64)) {
        let nx = self.nx;
        let ny = self.ny;
        let nu = self.nu;
        let inv_hx = 1.0 / self.hx;
        let inv_hy = 1.0 / self.hy;
        let inv_hx2 = inv_hx * inv_hx;
        let inv_hy2 = inv_hy * inv_hy;

        for j in 0..ny {
            for i in 0..nx {
                let idx_u = 2 * (i + nx * j);
                let idx_v = idx_u + 1;
                let u = y[idx_u];
                let v = y[idx_v];

                let u_l = if i == 0 { 0.0 } else { y[2 * (i - 1 + nx * j)] };
                let u_r = if i + 1 == nx {
                    0.0
                } else {
                    y[2 * (i + 1 + nx * j)]
                };
                let u_d = if j == 0 {
                    0.0
                } else {
                    y[2 * (i + nx * (j - 1))]
                };
                let u_u = if j + 1 == ny {
                    0.0
                } else {
                    y[2 * (i + nx * (j + 1))]
                };

                let v_l = if i == 0 {
                    0.0
                } else {
                    y[2 * (i - 1 + nx * j) + 1]
                };
                let v_r = if i + 1 == nx {
                    0.0
                } else {
                    y[2 * (i + 1 + nx * j) + 1]
                };
                let v_d = if j == 0 {
                    0.0
                } else {
                    y[2 * (i + nx * (j - 1)) + 1]
                };
                let v_u = if j + 1 == ny {
                    0.0
                } else {
                    y[2 * (i + nx * (j + 1)) + 1]
                };

                // Diffusion diagonal
                let diff_diag = -2.0 * nu * (inv_hx2 + inv_hy2);

                // Upwind advection derivatives for u
                let d_uux_du = if u >= 0.0 {
                    (2.0 * u - u_l) * inv_hx
                } else {
                    (u_r - 2.0 * u) * inv_hx
                };
                let d_vuy_du = if v >= 0.0 { v * inv_hy } else { -v * inv_hy };
                let d_vuy_dv = if v >= 0.0 {
                    (u - u_d) * inv_hy
                } else {
                    (u_u - u) * inv_hy
                };

                push(idx_u, idx_u, diff_diag - d_uux_du - d_vuy_du);
                push(idx_u, idx_v, -d_vuy_dv);

                // Upwind advection derivatives for v
                let d_uvx_du = if u >= 0.0 {
                    (v - v_l) * inv_hx
                } else {
                    (v_r - v) * inv_hx
                };
                let d_uvx_dv = if u >= 0.0 { u * inv_hx } else { -u * inv_hx };
                let d_vvy_dv = if v >= 0.0 {
                    (2.0 * v - v_d) * inv_hy
                } else {
                    (v_u - 2.0 * v) * inv_hy
                };

                push(idx_v, idx_u, -d_uvx_du);
                push(idx_v, idx_v, diff_diag - d_uvx_dv - d_vvy_dv);

                // Spatial neighbors (diffusion + advection upwind fluxes)
                if i > 0 {
                    let adv_l = if u >= 0.0 { u * inv_hx } else { 0.0 };
                    push(idx_u, 2 * (i - 1 + nx * j), nu * inv_hx2 + adv_l);
                    push(idx_v, 2 * (i - 1 + nx * j) + 1, nu * inv_hx2 + adv_l);
                }
                if i + 1 < nx {
                    let adv_r = if u < 0.0 { -u * inv_hx } else { 0.0 };
                    push(idx_u, 2 * (i + 1 + nx * j), nu * inv_hx2 + adv_r);
                    push(idx_v, 2 * (i + 1 + nx * j) + 1, nu * inv_hx2 + adv_r);
                }
                if j > 0 {
                    let adv_d = if v >= 0.0 { v * inv_hy } else { 0.0 };
                    push(idx_u, 2 * (i + nx * (j - 1)), nu * inv_hy2 + adv_d);
                    push(idx_v, 2 * (i + nx * (j - 1)) + 1, nu * inv_hy2 + adv_d);
                }
                if j + 1 < ny {
                    let adv_u = if v < 0.0 { -v * inv_hy } else { 0.0 };
                    push(idx_u, 2 * (i + nx * (j + 1)), nu * inv_hy2 + adv_u);
                    push(idx_v, 2 * (i + nx * (j + 1)) + 1, nu * inv_hy2 + adv_u);
                }
            }
        }
    }
}
