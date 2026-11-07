//! 2D Allen-Cahn phase field model for sharp moving interfacial boundaries.
//!
//! Models diffuse interface phase separation:
//! \partial_t u = \epsilon^2 \nabla^2 u - (u^3 - u)

use crate::spatial_grid::{BoundaryCondition, SpatialGridSpec};

/// 2D Allen-Cahn phase field model.
#[derive(Clone, Debug)]
pub struct AllenCahn2D {
    /// Number of x points.
    pub nx: usize,
    /// Number of y points.
    pub ny: usize,
    /// Grid spacing in x.
    pub hx: f64,
    /// Grid spacing in y.
    pub hy: f64,
    /// Interfacial width parameter $\epsilon$.
    pub epsilon: f64,
    /// Boundary conditions.
    pub boundary: BoundaryCondition,
}

impl AllenCahn2D {
    /// Create standard Allen-Cahn model on $[-1, 1]^2$.
    pub fn standard(nx: usize, ny: usize) -> Self {
        let hx = 2.0 / (nx + 1) as f64;
        let hy = 2.0 / (ny + 1) as f64;
        Self {
            nx,
            ny,
            hx,
            hy,
            epsilon: 0.05,
            boundary: BoundaryCondition::NeumannZero,
        }
    }

    /// Create Allen-Cahn model with custom boundary condition.
    pub fn with_boundary(nx: usize, ny: usize, boundary: BoundaryCondition) -> Self {
        let (hx, hy) = match boundary {
            BoundaryCondition::Periodic => (2.0 / nx as f64, 2.0 / ny as f64),
            _ => (2.0 / (nx + 1) as f64, 2.0 / (ny + 1) as f64),
        };
        Self {
            nx,
            ny,
            hx,
            hy,
            epsilon: 0.05,
            boundary,
        }
    }

    /// Total degrees of freedom: $nx \cdot ny$.
    #[inline(always)]
    pub fn n(&self) -> usize {
        self.nx * self.ny
    }

    /// Corresponding grid specification.
    pub fn grid_spec(&self) -> SpatialGridSpec {
        SpatialGridSpec::rect_2d(self.nx, self.ny, (-1.0, 1.0), (-1.0, 1.0), 1, self.boundary)
    }

    /// Initial state: a shrinking circular bubble of phase +1 in sea of -1.
    pub fn initial_state(&self) -> Vec<f64> {
        let mut y0 = vec![0.0; self.n()];
        let r0 = 0.5;
        let eps = self.epsilon;

        for j in 0..self.ny {
            let y = -1.0 + (j + 1) as f64 * self.hy;
            for i in 0..self.nx {
                let x = -1.0 + (i + 1) as f64 * self.hx;
                let dist = (x * x + y * y).sqrt();
                // Hyperbolic tangent profile for equilibrium diffuse interface
                let val = (-(dist - r0) / (std::f64::consts::SQRT_2 * eps)).tanh();
                y0[i + self.nx * j] = val;
            }
        }
        y0
    }

    /// Evaluate RHS in-place.
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        let nx = self.nx;
        let ny = self.ny;
        let eps2 = self.epsilon * self.epsilon;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);

        for j in 0..ny {
            for i in 0..nx {
                let k = i + nx * j;
                let u = y[k];

                let left = if i == 0 {
                    match self.boundary {
                        BoundaryCondition::NeumannZero => y[k],
                        BoundaryCondition::Periodic => y[k + nx - 1],
                        BoundaryCondition::DirichletZero => 0.0,
                    }
                } else {
                    y[k - 1]
                };

                let right = if i + 1 == nx {
                    match self.boundary {
                        BoundaryCondition::NeumannZero => y[k],
                        BoundaryCondition::Periodic => y[k - (nx - 1)],
                        BoundaryCondition::DirichletZero => 0.0,
                    }
                } else {
                    y[k + 1]
                };

                let down = if j == 0 {
                    match self.boundary {
                        BoundaryCondition::NeumannZero => y[k],
                        BoundaryCondition::Periodic => y[k + nx * (ny - 1)],
                        BoundaryCondition::DirichletZero => 0.0,
                    }
                } else {
                    y[k - nx]
                };

                let up = if j + 1 == ny {
                    match self.boundary {
                        BoundaryCondition::NeumannZero => y[k],
                        BoundaryCondition::Periodic => y[k - nx * (ny - 1)],
                        BoundaryCondition::DirichletZero => 0.0,
                    }
                } else {
                    y[k + nx]
                };

                let lap = (left - 2.0 * u + right) * inv_hx2 + (down - 2.0 * u + up) * inv_hy2;
                let reaction = -(u * u * u - u);

                dy[k] = eps2 * lap + reaction;
            }
        }
    }

    /// Iterate through non-zero Jacobian entries.
    pub fn jac_entries(&self, y: &[f64], mut push: impl FnMut(usize, usize, f64)) {
        let nx = self.nx;
        let ny = self.ny;
        let eps2 = self.epsilon * self.epsilon;
        let inv_hx2 = 1.0 / (self.hx * self.hx);
        let inv_hy2 = 1.0 / (self.hy * self.hy);

        for j in 0..ny {
            for i in 0..nx {
                let k = i + nx * j;
                let u = y[k];

                let cx_diag = match self.boundary {
                    BoundaryCondition::NeumannZero => {
                        let mut c = -2.0;
                        if i == 0 {
                            c += 1.0;
                        }
                        if i + 1 == nx {
                            c += 1.0;
                        }
                        c
                    }
                    _ => -2.0,
                };
                let cy_diag = match self.boundary {
                    BoundaryCondition::NeumannZero => {
                        let mut c = -2.0;
                        if j == 0 {
                            c += 1.0;
                        }
                        if j + 1 == ny {
                            c += 1.0;
                        }
                        c
                    }
                    _ => -2.0,
                };

                let diag = eps2 * (cx_diag * inv_hx2 + cy_diag * inv_hy2) - (3.0 * u * u - 1.0);
                push(k, k, diag);

                if i > 0 {
                    push(k, k - 1, eps2 * inv_hx2);
                } else if self.boundary == BoundaryCondition::Periodic {
                    push(k, k + nx - 1, eps2 * inv_hx2);
                }

                if i + 1 < nx {
                    push(k, k + 1, eps2 * inv_hx2);
                } else if self.boundary == BoundaryCondition::Periodic {
                    push(k, k - (nx - 1), eps2 * inv_hx2);
                }

                if j > 0 {
                    push(k, k - nx, eps2 * inv_hy2);
                } else if self.boundary == BoundaryCondition::Periodic {
                    push(k, k + nx * (ny - 1), eps2 * inv_hy2);
                }

                if j + 1 < ny {
                    push(k, k + nx, eps2 * inv_hy2);
                } else if self.boundary == BoundaryCondition::Periodic {
                    push(k, k - nx * (ny - 1), eps2 * inv_hy2);
                }
            }
        }
    }
}
