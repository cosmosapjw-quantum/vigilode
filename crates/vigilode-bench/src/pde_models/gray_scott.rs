//! 2D Gray-Scott reaction-diffusion pattern formation model.
//!
//! Models autocatalytic spot replication and intense localized stiffness:
//! \partial_t u = D_u \nabla^2 u - u v^2 + F(1 - u)
//! \partial_t v = D_v \nabla^2 v + u v^2 - (F + k) v

use crate::spatial_grid::{BoundaryCondition, SpatialGridSpec};

/// 2D Gray-Scott model.
#[derive(Clone, Debug)]
pub struct GrayScott2D {
    /// Number of x points.
    pub nx: usize,
    /// Number of y points.
    pub ny: usize,
    /// Grid spacing in x.
    pub hx: f64,
    /// Grid spacing in y.
    pub hy: f64,
    /// Diffusion coefficient for u.
    pub du: f64,
    /// Diffusion coefficient for v.
    pub dv: f64,
    /// Feed rate.
    pub f: f64,
    /// Kill rate.
    pub k: f64,
    /// Boundary condition.
    pub boundary: BoundaryCondition,
}

impl GrayScott2D {
    /// Create standard spot-dividing Gray-Scott model on $[0, 1]^2$.
    pub fn standard(nx: usize, ny: usize) -> Self {
        let hx = 1.0 / (nx + 1) as f64;
        let hy = 1.0 / (ny + 1) as f64;
        Self {
            nx,
            ny,
            hx,
            hy,
            du: 2.0e-5,
            dv: 1.0e-5,
            f: 0.030,
            k: 0.062,
            boundary: BoundaryCondition::NeumannZero,
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

    /// Initial state with central perturbation triggering spot mitosis.
    pub fn initial_state(&self) -> Vec<f64> {
        let n = self.n();
        let mut y0 = vec![0.0; n];
        let (cx, cy) = (0.5, 0.5);
        let r = 0.15;

        for j in 0..self.ny {
            let y = (j + 1) as f64 * self.hy;
            for i in 0..self.nx {
                let x = (i + 1) as f64 * self.hx;
                let idx = 2 * (i + self.nx * j);
                let dist_sq = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                if dist_sq < r * r {
                    y0[idx] = 0.5; // u depleted
                    y0[idx + 1] = 0.25; // v excited
                } else {
                    y0[idx] = 1.0; // u uniform
                    y0[idx + 1] = 0.0; // v quiescent
                }
            }
        }
        y0
    }

    /// Evaluate RHS in-place.
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        let nx = self.nx;
        let ny = self.ny;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);
        let du = self.du;
        let dv = self.dv;
        let f = self.f;
        let k = self.k;

        for j in 0..ny {
            let j_up = if j + 1 == ny {
                if self.boundary == BoundaryCondition::Periodic {
                    0
                } else {
                    j
                }
            } else {
                j + 1
            };
            let j_down = if j == 0 {
                if self.boundary == BoundaryCondition::Periodic {
                    ny - 1
                } else {
                    j
                }
            } else {
                j - 1
            };

            for i in 0..nx {
                let i_right = if i + 1 == nx {
                    if self.boundary == BoundaryCondition::Periodic {
                        0
                    } else {
                        i
                    }
                } else {
                    i + 1
                };
                let i_left = if i == 0 {
                    if self.boundary == BoundaryCondition::Periodic {
                        nx - 1
                    } else {
                        i
                    }
                } else {
                    i - 1
                };

                let idx = 2 * (i + nx * j);
                let u = y[idx];
                let v = y[idx + 1];

                let u_l = y[2 * (i_left + nx * j)];
                let u_r = y[2 * (i_right + nx * j)];
                let u_d = y[2 * (i + nx * j_down)];
                let u_u = y[2 * (i + nx * j_up)];

                let v_l = y[2 * (i_left + nx * j) + 1];
                let v_r = y[2 * (i_right + nx * j) + 1];
                let v_d = y[2 * (i + nx * j_down) + 1];
                let v_u = y[2 * (i + nx * j_up) + 1];

                let lap_u = (u_l - 2.0 * u + u_r) * inv_hx2 + (u_d - 2.0 * u + u_u) * inv_hy2;
                let lap_v = (v_l - 2.0 * v + v_r) * inv_hx2 + (v_d - 2.0 * v + v_u) * inv_hy2;

                let uv2 = u * v * v;
                dy[idx] = du * lap_u - uv2 + f * (1.0 - u);
                dy[idx + 1] = dv * lap_v + uv2 - (f + k) * v;
            }
        }
    }

    /// Iterate through non-zero Jacobian entries for sparse or banded assemblers.
    pub fn jac_entries(&self, y: &[f64], mut push: impl FnMut(usize, usize, f64)) {
        let nx = self.nx;
        let ny = self.ny;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);
        let du = self.du;
        let dv = self.dv;
        let f = self.f;
        let k = self.k;

        for j in 0..ny {
            let j_up = if j + 1 == ny {
                if self.boundary == BoundaryCondition::Periodic {
                    0
                } else {
                    j
                }
            } else {
                j + 1
            };
            let j_down = if j == 0 {
                if self.boundary == BoundaryCondition::Periodic {
                    ny - 1
                } else {
                    j
                }
            } else {
                j - 1
            };

            for i in 0..nx {
                let i_right = if i + 1 == nx {
                    if self.boundary == BoundaryCondition::Periodic {
                        0
                    } else {
                        i
                    }
                } else {
                    i + 1
                };
                let i_left = if i == 0 {
                    if self.boundary == BoundaryCondition::Periodic {
                        nx - 1
                    } else {
                        i
                    }
                } else {
                    i - 1
                };

                let idx_u = 2 * (i + nx * j);
                let idx_v = idx_u + 1;
                let u = y[idx_u];
                let v = y[idx_v];

                // d(dy_u)/du diagonal: -2 du (inv_hx2 + inv_hy2) - v^2 - f
                push(idx_u, idx_u, -2.0 * du * (inv_hx2 + inv_hy2) - v * v - f);
                // d(dy_u)/dv cross: -2 u v
                push(idx_u, idx_v, -2.0 * u * v);

                // d(dy_v)/du cross: v^2
                push(idx_v, idx_u, v * v);
                // d(dy_v)/dv diagonal: -2 dv (inv_hx2 + inv_hy2) + 2 u v - (f + k)
                push(
                    idx_v,
                    idx_v,
                    -2.0 * dv * (inv_hx2 + inv_hy2) + 2.0 * u * v - (f + k),
                );

                // Diffusion neighbors for u
                push(idx_u, 2 * (i_left + nx * j), du * inv_hx2);
                push(idx_u, 2 * (i_right + nx * j), du * inv_hx2);
                push(idx_u, 2 * (i + nx * j_down), du * inv_hy2);
                push(idx_u, 2 * (i + nx * j_up), du * inv_hy2);

                // Diffusion neighbors for v
                push(idx_v, 2 * (i_left + nx * j) + 1, dv * inv_hx2);
                push(idx_v, 2 * (i_right + nx * j) + 1, dv * inv_hx2);
                push(idx_v, 2 * (i + nx * j_down) + 1, dv * inv_hy2);
                push(idx_v, 2 * (i + nx * j_up) + 1, dv * inv_hy2);
            }
        }
    }
}
