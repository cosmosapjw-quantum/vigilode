//! 2D Spherical surface diffusion model with pole singularity regularization.
//!
//! Models heat/matter diffusion on the unit sphere $S^2$:
//! \partial_t u = \frac{1}{\sin\theta} \partial_\theta (\sin\theta \partial_\theta u) + \frac{1}{\sin^2\theta} \partial_{\phi\phi} u
//!
//! Uses conservative finite-volume metric weighting:
//! \Delta \Omega_{j, k} = (\cos\theta_{j-1/2} - \cos\theta_{j+1/2}) \Delta\phi
//! Total integral \iint_{S^2} u d\Omega is analytically conserved.

use crate::spatial_grid::{BoundaryCondition, SpatialGridSpec};

/// 2D Spherical surface diffusion model.
#[derive(Clone, Debug)]
pub struct SphericalDiffusion2D {
    /// Number of polar angle cells $\theta \in (0, \pi)$.
    pub n_theta: usize,
    /// Number of azimuthal angle cells $\phi \in [0, 2\pi)$.
    pub n_phi: usize,
    /// Polar cell boundaries.
    pub theta_edges: Vec<f64>,
    /// Polar cell centers.
    pub theta_centers: Vec<f64>,
    /// Azimuthal step size $d\phi$.
    pub dphi: f64,
    /// Cell solid angle weights $\Delta\Omega_{j}$.
    pub cell_areas: Vec<f64>,
    /// Diffusion coefficient.
    pub kappa: f64,
}

impl SphericalDiffusion2D {
    /// Create standard spherical diffusion on $S^2$.
    pub fn standard(n_theta: usize, n_phi: usize) -> Self {
        let dtheta = std::f64::consts::PI / n_theta as f64;
        let dphi = 2.0 * std::f64::consts::PI / n_phi as f64;

        let mut theta_edges = Vec::with_capacity(n_theta + 1);
        for j in 0..=n_theta {
            theta_edges.push(j as f64 * dtheta);
        }

        let mut theta_centers = Vec::with_capacity(n_theta);
        let mut cell_areas = Vec::with_capacity(n_theta);
        for j in 0..n_theta {
            let th_c = 0.5 * (theta_edges[j] + theta_edges[j + 1]);
            theta_centers.push(th_c);
            // Area = (\cos\theta_j - \cos\theta_{j+1}) * d\phi
            let area = (theta_edges[j].cos() - theta_edges[j + 1].cos()) * dphi;
            cell_areas.push(area.max(1e-15));
        }

        Self {
            n_theta,
            n_phi,
            theta_edges,
            theta_centers,
            dphi,
            cell_areas,
            kappa: 0.1,
        }
    }

    /// Total degrees of freedom: $n_\theta \cdot n_\phi$.
    #[inline(always)]
    pub fn n(&self) -> usize {
        self.n_theta * self.n_phi
    }

    /// Corresponding grid specification.
    pub fn grid_spec(&self) -> SpatialGridSpec {
        SpatialGridSpec::rect_2d(
            self.n_theta,
            self.n_phi,
            (0.0, std::f64::consts::PI),
            (0.0, 2.0 * std::f64::consts::PI),
            1,
            BoundaryCondition::Periodic,
        )
    }

    /// Initial dipole distribution on sphere: $u_0(\theta, \phi) = 1 + \cos\theta + \sin\theta \cos\phi$.
    pub fn initial_state(&self) -> Vec<f64> {
        let mut y0 = vec![0.0; self.n()];
        for j in 0..self.n_theta {
            let th = self.theta_centers[j];
            for k in 0..self.n_phi {
                let phi = (k as f64 + 0.5) * self.dphi;
                let val = 1.0 + th.cos() + 0.5 * th.sin() * phi.cos();
                y0[k + self.n_phi * j] = val;
            }
        }
        y0
    }

    /// Compute total integrated mass $\int_{S^2} u \, d\Omega$.
    pub fn total_mass(&self, y: &[f64]) -> f64 {
        let mut sum = 0.0;
        for j in 0..self.n_theta {
            let area_j = self.cell_areas[j];
            for k in 0..self.n_phi {
                sum += y[k + self.n_phi * j] * area_j;
            }
        }
        sum
    }

    /// Evaluate RHS in-place using conservative flux balancing.
    #[inline]
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        let nt = self.n_theta;
        let np = self.n_phi;
        let dphi = self.dphi;
        let kappa = self.kappa;

        for j in 0..nt {
            let area_j = self.cell_areas[j];
            let inv_area = 1.0 / area_j;
            let th_c = self.theta_centers[j];
            let sin_th = th_c.sin().max(1e-6);

            // Interface metric terms: sin(\theta_{j+1/2})
            let sin_north = self.theta_edges[j].sin();
            let sin_south = self.theta_edges[j + 1].sin();
            let dtheta_j = self.theta_edges[j + 1] - self.theta_edges[j];

            for k in 0..np {
                let idx = k + np * j;
                let u = y[idx];

                let k_left = if k == 0 { np - 1 } else { k - 1 };
                let k_right = if k + 1 == np { 0 } else { k + 1 };
                let u_l = y[k_left + np * j];
                let u_r = y[k_right + np * j];

                // Polar fluxes (theta direction)
                let flux_north = if j == 0 {
                    0.0 // Pole boundary has zero surface area: sin(0) = 0
                } else {
                    let dth = th_c - self.theta_centers[j - 1];
                    kappa * sin_north * (u - y[k + np * (j - 1)]) / dth * dphi
                };

                let flux_south = if j + 1 == nt {
                    0.0 // South pole boundary: sin(pi) = 0
                } else {
                    let dth = self.theta_centers[j + 1] - th_c;
                    kappa * sin_south * (y[k + np * (j + 1)] - u) / dth * dphi
                };

                // Azimuthal fluxes (phi direction)
                let flux_phi = kappa * (u_l - 2.0 * u + u_r) / (dphi * dphi) * (dtheta_j / sin_th);

                // Conservative balance: du/dt = (flux_south - flux_north + flux_phi) / Area
                dy[idx] = (flux_south - flux_north + flux_phi) * inv_area;
            }
        }
    }

    /// Iterate through non-zero Jacobian entries.
    pub fn jac_entries(&self, _y: &[f64], mut push: impl FnMut(usize, usize, f64)) {
        let nt = self.n_theta;
        let np = self.n_phi;
        let dphi = self.dphi;
        let kappa = self.kappa;

        for j in 0..nt {
            let area_j = self.cell_areas[j];
            let inv_area = 1.0 / area_j;
            let th_c = self.theta_centers[j];
            let sin_th = th_c.sin().max(1e-6);

            let sin_north = self.theta_edges[j].sin();
            let sin_south = self.theta_edges[j + 1].sin();
            let dtheta_j = self.theta_edges[j + 1] - self.theta_edges[j];

            let c_phi = kappa * (dtheta_j / sin_th) / (dphi * dphi) * inv_area;

            for k in 0..np {
                let idx = k + np * j;
                let mut diag = -2.0 * c_phi;

                if j > 0 {
                    let dth = th_c - self.theta_centers[j - 1];
                    let c_north = kappa * sin_north / dth * dphi * inv_area;
                    push(idx, k + np * (j - 1), c_north);
                    diag -= c_north;
                }
                if j + 1 < nt {
                    let dth = self.theta_centers[j + 1] - th_c;
                    let c_south = kappa * sin_south / dth * dphi * inv_area;
                    push(idx, k + np * (j + 1), c_south);
                    diag -= c_south;
                }

                let k_left = if k == 0 { np - 1 } else { k - 1 };
                let k_right = if k + 1 == np { 0 } else { k + 1 };
                push(idx, k_left + np * j, c_phi);
                push(idx, k_right + np * j, c_phi);

                push(idx, idx, diag);
            }
        }
    }
}
