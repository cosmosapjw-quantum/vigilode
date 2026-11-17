//! Runtime-configurable spatial grid specifications and zero-allocation pre-allocated workspaces.
//!
//! Provides the architectural bridge between continuous spatial PDEs and discrete
//! ODE systems (Method of Lines). Enforces `VG-1` (Argon Zero-Allocation in Hot Loops)
//! through a two-phase lifecycle:
//! 1. Phase 1 (Init): Single-chunk contiguous memory arena allocation sized to runtime dimensions.
//! 2. Phase 2 (Solve): Zero-allocation in-place slice mutation with strided band and Krylov accessors.

use std::fmt;

/// Spatial domain topology.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridTopology {
    /// 1D line domain with `nx` interior points and span `[x_min, x_max]`.
    Line1D {
        /// Number of interior grid points.
        nx: usize,
        /// Domain spatial boundaries.
        span: (f64, f64),
    },
    /// 2D rectangular Cartesian domain with `(nx, ny)` interior points.
    Rect2D {
        /// Number of points in x direction.
        nx: usize,
        /// Number of points in y direction.
        ny: usize,
        /// Domain boundaries in x.
        x_span: (f64, f64),
        /// Domain boundaries in y.
        y_span: (f64, f64),
    },
    /// 3D rectangular box domain with `(nx, ny, nz)` interior points.
    Box3D {
        /// Number of points in x direction.
        nx: usize,
        /// Number of points in y direction.
        ny: usize,
        /// Number of points in z direction.
        nz: usize,
        /// Domain boundaries in x.
        x_span: (f64, f64),
        /// Domain boundaries in y.
        y_span: (f64, f64),
        /// Domain boundaries in z.
        z_span: (f64, f64),
    },
    /// 2D spherical coordinates on `(theta, phi)` or `(q, mu)` with pole regularization.
    Sphere2D {
        /// Number of radial or log-momentum cells.
        nq: usize,
        /// Number of angular cells (cos theta).
        nmu: usize,
    },
    /// 3D spherical coordinates on `(q, mu, phi)`.
    Sphere3D {
        /// Number of radial/momentum cells.
        nq: usize,
        /// Number of polar angle cells.
        nmu: usize,
        /// Number of azimuthal angle cells.
        nphi: usize,
    },
}

/// Boundary condition specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryCondition {
    /// Homogeneous Dirichlet: $u = 0$ on boundary.
    DirichletZero,
    /// Homogeneous Neumann: $\partial_n u = 0$ on boundary.
    NeumannZero,
    /// Periodic boundary conditions: $u(x + L) = u(x)$.
    Periodic,
}

/// Full specification of a spatial discretization.
#[derive(Clone, Debug, PartialEq)]
pub struct SpatialGridSpec {
    /// Domain topology and resolution.
    pub topology: GridTopology,
    /// Number of physical fields per grid point (e.g. 1 for scalar, 2 for Gray-Scott or Sine-Gordon).
    pub fields_per_node: usize,
    /// Boundary conditions.
    pub boundary: BoundaryCondition,
}

impl SpatialGridSpec {
    /// Create a 1D grid specification.
    pub fn line_1d(nx: usize, span: (f64, f64), fields: usize, bc: BoundaryCondition) -> Self {
        Self {
            topology: GridTopology::Line1D { nx, span },
            fields_per_node: fields,
            boundary: bc,
        }
    }

    /// Create a 2D rectangular grid specification.
    pub fn rect_2d(
        nx: usize,
        ny: usize,
        x_span: (f64, f64),
        y_span: (f64, f64),
        fields: usize,
        bc: BoundaryCondition,
    ) -> Self {
        Self {
            topology: GridTopology::Rect2D {
                nx,
                ny,
                x_span,
                y_span,
            },
            fields_per_node: fields,
            boundary: bc,
        }
    }

    /// Create a 3D rectangular box specification.
    #[allow(clippy::too_many_arguments)]
    pub fn box_3d(
        nx: usize,
        ny: usize,
        nz: usize,
        x_span: (f64, f64),
        y_span: (f64, f64),
        z_span: (f64, f64),
        fields: usize,
        bc: BoundaryCondition,
    ) -> Self {
        Self {
            topology: GridTopology::Box3D {
                nx,
                ny,
                nz,
                x_span,
                y_span,
                z_span,
            },
            fields_per_node: fields,
            boundary: bc,
        }
    }

    /// Total number of spatial grid points / cells.
    pub fn total_nodes(&self) -> usize {
        match self.topology {
            GridTopology::Line1D { nx, .. } => nx,
            GridTopology::Rect2D { nx, ny, .. } => nx * ny,
            GridTopology::Box3D { nx, ny, nz, .. } => nx * ny * nz,
            GridTopology::Sphere2D { nq, nmu } => nq * nmu,
            GridTopology::Sphere3D { nq, nmu, nphi } => nq * nmu * nphi,
        }
    }

    /// Total number of ODE state variables (degrees of freedom $N$).
    pub fn total_dofs(&self) -> usize {
        self.total_nodes() * self.fields_per_node
    }

    /// Spatial step sizes $(h_x, h_y, h_z)$ for finite difference stencils.
    pub fn step_sizes(&self) -> (f64, f64, f64) {
        match self.topology {
            GridTopology::Line1D { nx, span } => {
                let hx = (span.1 - span.0) / (nx + 1) as f64;
                (hx, 1.0, 1.0)
            }
            GridTopology::Rect2D {
                nx,
                ny,
                x_span,
                y_span,
            } => {
                let hx = (x_span.1 - x_span.0) / (nx + 1) as f64;
                let hy = (y_span.1 - y_span.0) / (ny + 1) as f64;
                (hx, hy, 1.0)
            }
            GridTopology::Box3D {
                nx,
                ny,
                nz,
                x_span,
                y_span,
                z_span,
            } => {
                let hx = (x_span.1 - x_span.0) / (nx + 1) as f64;
                let hy = (y_span.1 - y_span.0) / (ny + 1) as f64;
                let hz = (z_span.1 - z_span.0) / (nz + 1) as f64;
                (hx, hy, hz)
            }
            GridTopology::Sphere2D { nq, nmu } => (1.0 / nq as f64, 2.0 / nmu as f64, 1.0),
            GridTopology::Sphere3D { nq, nmu, nphi } => (
                1.0 / nq as f64,
                2.0 / nmu as f64,
                2.0 * std::f64::consts::PI / nphi as f64,
            ),
        }
    }

    /// Bandwidth $(lower, upper)$ under optimal lexicographical ordering.
    pub fn bandwidth(&self) -> (usize, usize) {
        let f = self.fields_per_node;
        match self.topology {
            GridTopology::Line1D { .. } => (f, f),
            GridTopology::Rect2D { nx, ny, .. } => {
                // Ordering along the smaller dimension minimizes bandwidth
                let min_dim = nx.min(ny);
                (min_dim * f, min_dim * f)
            }
            GridTopology::Box3D { nx, ny, nz, .. } => {
                // In 3D, bandwidth scales with the minimum cross-section plane
                let plane = (nx * ny).min(ny * nz).min(nx * nz);
                (plane * f, plane * f)
            }
            GridTopology::Sphere2D { nmu, .. } => (nmu * f + 1, nmu * f + 1),
            GridTopology::Sphere3D { nmu, nphi, .. } => {
                let plane = nmu * nphi;
                (plane * f + 1, plane * f + 1)
            }
        }
    }

    /// Pitch for LINPACK / LAPACK DGBFA format: `2 * lower + upper + 1`.
    pub fn lu_pitch(&self) -> usize {
        let (lower, upper) = self.bandwidth();
        2 * lower + upper + 1
    }

    /// Storage width for un-factorized Jacobian band: `lower + upper + 1`.
    pub fn jac_band_width(&self) -> usize {
        let (lower, upper) = self.bandwidth();
        lower + upper + 1
    }
}

/// Pre-allocated single-arena workspace guaranteeing `VG-1` zero-allocation during time integration.
pub struct SpatialWorkspace {
    /// Contiguous heap block holding all vectors and matrices.
    arena: Box<[f64]>,
    /// Total degrees of freedom $N$.
    pub n: usize,
    /// Number of Runge-Kutta stages (6 for RODAS5P, 7 for DOPRI5).
    pub stages: usize,
    /// Jacobian band width `lower + upper + 1`.
    pub jac_width: usize,
    /// LU factorization pitch `2 * lower + upper + 1`.
    pub lu_pitch: usize,
    /// Dimension of Krylov subspace basis.
    pub krylov_m: usize,
    // Arena byte offsets
    off_y: usize,
    off_dy: usize,
    off_stages: usize,
    off_jac_band: usize,
    off_lu_band: usize,
    off_krylov: usize,
    off_scratch: usize,
}

impl fmt::Debug for SpatialWorkspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpatialWorkspace")
            .field("n", &self.n)
            .field("stages", &self.stages)
            .field("jac_width", &self.jac_width)
            .field("lu_pitch", &self.lu_pitch)
            .field("krylov_m", &self.krylov_m)
            .field("total_bytes", &(self.arena.len() * 8))
            .finish()
    }
}

/// Disjoint mutable slice views of workspace buffers during an integration step.
pub struct SpatialStepViews<'a> {
    /// In-place slice for state vector $y$.
    pub y: &'a mut [f64],
    /// In-place slice for derivative vector $dy$.
    pub dy: &'a mut [f64],
    /// In-place slice for stage $k$.
    pub stage: &'a mut [f64],
    /// In-place slice for Jacobian band.
    pub jac_band: &'a mut [f64],
    /// In-place slice for LU factorization buffer.
    pub lu_band: &'a mut [f64],
    /// In-place slice for Krylov basis vector $j$.
    pub krylov: &'a mut [f64],
}

impl SpatialWorkspace {
    /// Phase 1: Allocate single contiguous memory block sized exactly to grid specification.
    pub fn new(spec: &SpatialGridSpec, stages: usize, krylov_m: usize) -> Self {
        let n = spec.total_dofs();
        let (kl, ku) = spec.bandwidth();
        let jac_width = kl + ku + 1;
        let lu_pitch = 2 * kl + ku + 1;

        // Calculate offsets
        let mut cur = 0;
        let off_y = cur;
        cur += n;

        let off_dy = cur;
        cur += n;

        let off_stages = cur;
        cur += n * stages;

        let off_jac_band = cur;
        cur += n * jac_width;

        let off_lu_band = cur;
        cur += n * lu_pitch;

        let off_krylov = cur;
        cur += n * (krylov_m + 1);

        let off_scratch = cur;
        cur += n * 4; // 4 work scratch vectors

        let total_doubles = cur;
        let arena = vec![0.0; total_doubles].into_boxed_slice();

        Self {
            arena,
            n,
            stages,
            jac_width,
            lu_pitch,
            krylov_m,
            off_y,
            off_dy,
            off_stages,
            off_jac_band,
            off_lu_band,
            off_krylov,
            off_scratch,
        }
    }

    /// Total heap memory occupied by this pre-allocated workspace in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.arena.len() * std::mem::size_of::<f64>()
    }

    /// In-place mutable slice for the state vector $y$.
    #[inline(always)]
    pub fn y_mut(&mut self) -> &mut [f64] {
        &mut self.arena[self.off_y..self.off_y + self.n]
    }

    /// In-place slice for the state vector $y$.
    #[inline(always)]
    pub fn y(&self) -> &[f64] {
        &self.arena[self.off_y..self.off_y + self.n]
    }

    /// In-place mutable slice for the time derivative vector $dy = f(t, y)$.
    #[inline(always)]
    pub fn dy_mut(&mut self) -> &mut [f64] {
        &mut self.arena[self.off_dy..self.off_dy + self.n]
    }

    /// In-place mutable slice for stage $k \in [0, \text{stages})$.
    #[inline(always)]
    pub fn stage_mut(&mut self, k: usize) -> &mut [f64] {
        debug_assert!(k < self.stages);
        let start = self.off_stages + k * self.n;
        &mut self.arena[start..start + self.n]
    }

    /// In-place mutable slice for the row-major or col-major Jacobian band.
    #[inline(always)]
    pub fn jac_band_mut(&mut self) -> &mut [f64] {
        &mut self.arena[self.off_jac_band..self.off_jac_band + self.n * self.jac_width]
    }

    /// In-place mutable slice for LU factorization storage.
    #[inline(always)]
    pub fn lu_band_mut(&mut self) -> &mut [f64] {
        &mut self.arena[self.off_lu_band..self.off_lu_band + self.n * self.lu_pitch]
    }

    /// In-place mutable slice for Krylov basis vector $j \in [0, m]$.
    #[inline(always)]
    pub fn krylov_basis_mut(&mut self, j: usize) -> &mut [f64] {
        debug_assert!(j <= self.krylov_m);
        let start = self.off_krylov + j * self.n;
        &mut self.arena[start..start + self.n]
    }

    /// In-place mutable scratch vector $i \in [0, 4)$.
    #[inline(always)]
    pub fn scratch_mut(&mut self, idx: usize) -> &mut [f64] {
        debug_assert!(idx < 4);
        let start = self.off_scratch + idx * self.n;
        &mut self.arena[start..start + self.n]
    }

    /// Split workspace into non-overlapping disjoint references $(y, dy, stage_k)$ without allocation.
    #[inline(always)]
    pub fn split_step_views(&mut self, k: usize) -> (&mut [f64], &mut [f64], &mut [f64]) {
        debug_assert!(k < self.stages);
        let (left, rest) = self.arena.split_at_mut(self.off_dy);
        let y = &mut left[self.off_y..self.off_y + self.n];
        let (dy_part, rest2) = rest.split_at_mut(self.off_stages - self.off_dy);
        let dy = &mut dy_part[..self.n];
        let stage = &mut rest2[k * self.n..(k + 1) * self.n];
        (y, dy, stage)
    }

    /// Split workspace into non-overlapping disjoint references
    /// `SpatialStepViews` without allocation.
    #[inline(always)]
    pub fn split_all_views(&mut self, k: usize, j: usize) -> SpatialStepViews<'_> {
        debug_assert!(k < self.stages);
        debug_assert!(j <= self.krylov_m);
        let n = self.n;
        let jac_len = n * self.jac_width;
        let lu_len = n * self.lu_pitch;

        let (left, rest) = self.arena.split_at_mut(self.off_dy);
        let y = &mut left[self.off_y..self.off_y + n];

        let (dy_part, rest2) = rest.split_at_mut(self.off_stages - self.off_dy);
        let dy = &mut dy_part[..n];

        let (stages_part, rest3) = rest2.split_at_mut(self.off_jac_band - self.off_stages);
        let stage = &mut stages_part[k * n..(k + 1) * n];

        let (jac_part, rest4) = rest3.split_at_mut(self.off_lu_band - self.off_jac_band);
        let jac_band = &mut jac_part[..jac_len];

        let (lu_part, rest5) = rest4.split_at_mut(self.off_krylov - self.off_lu_band);
        let lu_band = &mut lu_part[..lu_len];

        let krylov = &mut rest5[j * n..(j + 1) * n];

        SpatialStepViews {
            y,
            dy,
            stage,
            jac_band,
            lu_band,
            krylov,
        }
    }
}

/// Moving Mesh PDE (MMPDE) r-refinement controller.
///
/// Implements continuous coordinate relocation while keeping state dimension $N$ strictly invariant,
/// guaranteeing 100% zero-allocation during time integration.
#[derive(Clone, Debug)]
pub struct MovingMesh1D {
    /// Number of interior nodes.
    pub nx: usize,
    /// Monitor function smoothing relaxation time $\tau$.
    pub tau: f64,
    /// Arc-length intensity weight $\alpha$.
    pub alpha: f64,
}

impl MovingMesh1D {
    /// Create a new 1D Moving Mesh controller.
    pub fn new(nx: usize, tau: f64, alpha: f64) -> Self {
        Self { nx, tau, alpha }
    }

    /// Compute arc-length monitor values $M_i = \sqrt{1 + \alpha |\partial_x u_i|^2}$.
    #[inline]
    pub fn compute_monitors(&self, u: &[f64], x: &[f64], out_m: &mut [f64]) {
        let n = self.nx;
        debug_assert!(u.len() >= n && x.len() >= n && out_m.len() >= n);
        for i in 0..n {
            let du = if i == 0 {
                (u[1] - u[0]) / (x[1] - x[0]).max(1e-12)
            } else if i == n - 1 {
                (u[n - 1] - u[n - 2]) / (x[n - 1] - x[n - 2]).max(1e-12)
            } else {
                (u[i + 1] - u[i - 1]) / (x[i + 1] - x[i - 1]).max(1e-12)
            };
            out_m[i] = (1.0 + self.alpha * du * du).sqrt();
        }
    }

    /// Evaluate MMPDE grid velocity $\dot{x}_i$ according to equidistribution principles.
    #[inline]
    pub fn mesh_velocities(&self, x: &[f64], m: &[f64], out_xdot: &mut [f64]) {
        let n = self.nx;
        out_xdot[0] = 0.0; // Left boundary pinned
        out_xdot[n - 1] = 0.0; // Right boundary pinned
        let inv_tau = 1.0 / self.tau;

        for i in 1..n - 1 {
            let m_right = 0.5 * (m[i] + m[i + 1]);
            let m_left = 0.5 * (m[i - 1] + m[i]);
            let flux_right = m_right * (x[i + 1] - x[i]);
            let flux_left = m_left * (x[i] - x[i - 1]);
            out_xdot[i] = inv_tau * (flux_right - flux_left);
        }
    }
}
