//! 2D Sine-Gordon nonlinear wave model.
//!
//! Models oscillatory soliton dynamics and symplectic Hamiltonian conservation:
//! \partial_t \phi = \psi
//! \partial_t \psi = \nabla^2 \phi - \sin(\phi)
//!
//! Total energy:
//! H = \iint \left[ \frac{1}{2} \psi^2 + \frac{1}{2} |\nabla \phi|^2 + (1 - \cos\phi) \right] dx dy

use crate::spatial_grid::{BoundaryCondition, SpatialGridSpec};

/// 2D Sine-Gordon wave model.
#[derive(Clone, Debug)]
pub struct SineGordon2D {
    /// Number of x points.
    pub nx: usize,
    /// Number of y points.
    pub ny: usize,
    /// Grid spacing in x.
    pub hx: f64,
    /// Grid spacing in y.
    pub hy: f64,
    /// Domain length L.
    pub l: f64,
    /// Boundary conditions.
    pub boundary: BoundaryCondition,
}

impl SineGordon2D {
    /// Create standard Sine-Gordon model on $[-L/2, L/2]^2$.
    pub fn standard(nx: usize, ny: usize, l: f64) -> Self {
        let hx = l / (nx + 1) as f64;
        let hy = l / (ny + 1) as f64;
        Self {
            nx,
            ny,
            hx,
            hy,
            l,
            boundary: BoundaryCondition::NeumannZero,
        }
    }

    /// Create Sine-Gordon model with custom boundary condition.
    pub fn with_boundary(nx: usize, ny: usize, l: f64, boundary: BoundaryCondition) -> Self {
        let (hx, hy) = match boundary {
            BoundaryCondition::Periodic => (l / nx as f64, l / ny as f64),
            _ => (l / (nx + 1) as f64, l / (ny + 1) as f64),
        };
        Self {
            nx,
            ny,
            hx,
            hy,
            l,
            boundary,
        }
    }

    /// Total degrees of freedom: $2 \cdot nx \cdot ny$ (interleaved $(\phi, \psi)$).
    #[inline(always)]
    pub fn n(&self) -> usize {
        2 * self.nx * self.ny
    }

    /// Corresponding grid specification.
    pub fn grid_spec(&self) -> SpatialGridSpec {
        let half = 0.5 * self.l;
        SpatialGridSpec::rect_2d(
            self.nx,
            self.ny,
            (-half, half),
            (-half, half),
            2,
            self.boundary,
        )
    }

    /// Initial ring soliton (kink-antikink ring).
    pub fn initial_state(&self) -> Vec<f64> {
        let mut y0 = vec![0.0; self.n()];
        let half = 0.5 * self.l;
        let r0 = 2.0;

        for j in 0..self.ny {
            let y = -half + (j + 1) as f64 * self.hy;
            for i in 0..self.nx {
                let x = -half + (i + 1) as f64 * self.hx;
                let r = (x * x + y * y).sqrt();
                let idx = 2 * (i + self.nx * j);
                // Circular kink profile
                let phi = 4.0 * (-(r - r0)).exp().atan();
                y0[idx] = phi; // \phi
                y0[idx + 1] = 0.0; // \psi = 0 (initially at rest)
            }
        }
        y0
    }

    /// Total Hamiltonian Energy $H(y)$.
    pub fn total_energy(&self, y: &[f64]) -> f64 {
        let nx = self.nx;
        let ny = self.ny;
        let hx = self.hx;
        let hy = self.hy;
        let mut total_h = 0.0;

        for j in 0..ny {
            let j_next = if j + 1 == ny { 0 } else { j + 1 };
            for i in 0..nx {
                let i_next = if i + 1 == nx { 0 } else { i + 1 };
                let idx = 2 * (i + nx * j);
                let phi = y[idx];
                let psi = y[idx + 1];

                let phi_r = y[2 * (i_next + nx * j)];
                let phi_u = y[2 * (i + nx * j_next)];

                let grad_x = (phi_r - phi) / hx;
                let grad_y = (phi_u - phi) / hy;

                let kinetic = 0.5 * psi * psi;
                let gradient = 0.5 * (grad_x * grad_x + grad_y * grad_y);
                let potential = 1.0 - phi.cos();

                total_h += (kinetic + gradient + potential) * hx * hy;
            }
        }
        total_h
    }

    /// Evaluate RHS in-place.
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        let nx = self.nx;
        let ny = self.ny;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);

        for j in 0..ny {
            let j_up = if j + 1 == ny {
                match self.boundary {
                    BoundaryCondition::Periodic => 0,
                    _ => j,
                }
            } else {
                j + 1
            };
            let j_down = if j == 0 {
                match self.boundary {
                    BoundaryCondition::Periodic => ny - 1,
                    _ => j,
                }
            } else {
                j - 1
            };

            for i in 0..nx {
                let i_right = if i + 1 == nx {
                    match self.boundary {
                        BoundaryCondition::Periodic => 0,
                        _ => i,
                    }
                } else {
                    i + 1
                };
                let i_left = if i == 0 {
                    match self.boundary {
                        BoundaryCondition::Periodic => nx - 1,
                        _ => i,
                    }
                } else {
                    i - 1
                };

                let idx = 2 * (i + nx * j);
                let phi = y[idx];
                let psi = y[idx + 1];

                let phi_l = y[2 * (i_left + nx * j)];
                let phi_r = y[2 * (i_right + nx * j)];
                let phi_d = y[2 * (i + nx * j_down)];
                let phi_u = y[2 * (i + nx * j_up)];

                let lap_phi =
                    (phi_l - 2.0 * phi + phi_r) * inv_hx2 + (phi_d - 2.0 * phi + phi_u) * inv_hy2;

                // d\phi/dt = \psi
                dy[idx] = psi;
                // d\psi/dt = \nabla^2 \phi - \sin(\phi)
                dy[idx + 1] = lap_phi - phi.sin();
            }
        }
    }

    /// Iterate through non-zero Jacobian entries.
    pub fn jac_entries(&self, y: &[f64], mut push: impl FnMut(usize, usize, f64)) {
        let nx = self.nx;
        let ny = self.ny;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);

        for j in 0..ny {
            let j_up = if j + 1 == ny {
                match self.boundary {
                    BoundaryCondition::Periodic => 0,
                    _ => j,
                }
            } else {
                j + 1
            };
            let j_down = if j == 0 {
                match self.boundary {
                    BoundaryCondition::Periodic => ny - 1,
                    _ => j,
                }
            } else {
                j - 1
            };

            for i in 0..nx {
                let i_right = if i + 1 == nx {
                    match self.boundary {
                        BoundaryCondition::Periodic => 0,
                        _ => i,
                    }
                } else {
                    i + 1
                };
                let i_left = if i == 0 {
                    match self.boundary {
                        BoundaryCondition::Periodic => nx - 1,
                        _ => i,
                    }
                } else {
                    i - 1
                };

                let idx_phi = 2 * (i + nx * j);
                let idx_psi = idx_phi + 1;
                let phi = y[idx_phi];

                // d(dy_phi)/dpsi = 1
                push(idx_phi, idx_psi, 1.0);

                // d(dy_psi)/dphi = -2/hx^2 - 2/hy^2 - cos(phi)
                let diag_psi = -2.0 * (inv_hx2 + inv_hy2) - phi.cos();
                push(idx_psi, idx_phi, diag_psi);

                // Neighbors in laplacian
                push(idx_psi, 2 * (i_left + nx * j), inv_hx2);
                push(idx_psi, 2 * (i_right + nx * j), inv_hx2);
                push(idx_psi, 2 * (i + nx * j_down), inv_hy2);
                push(idx_psi, 2 * (i + nx * j_up), inv_hy2);
            }
        }
    }
}
