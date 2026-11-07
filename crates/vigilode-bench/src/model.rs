//! The one physical model per problem, shared by every arm.
//!
//! Every arm evaluates the right-hand side through [`Model::rhs`] and the
//! Jacobian through [`Model::jac_entries`] (directly, or via one of the
//! layout writers below). The callbacks increment process-wide counters so
//! that the evaluation work of every arm is measured by the same authority,
//! independently of what the solver itself reports.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[allow(missing_docs)]
#[path = "../../rodas5p-integrators/examples/sphere_photon_fv_support/mod.rs"]
pub mod sphere_photon_fv_support;

use sphere_photon_fv_support::{
    Absorber, Boundary, Coordinate, Geometry3, Layout3, Model3, uniform_edges, uniform_mu_edges,
    uniform_phi_edges,
};

static SPHERE_2D_MODEL: OnceLock<Model3> = OnceLock::new();
static SPHERE_3D_MODEL: OnceLock<Model3> = OnceLock::new();

/// Cached 2D spherical photon transport model on (q, mu) with n = 86, bandwidth = 7.
pub fn sphere_2d() -> &'static Model3 {
    SPHERE_2D_MODEL.get_or_init(|| {
        let q = uniform_edges(-3.0, 2.0, 12).expect("valid q edges");
        let mu = uniform_mu_edges(6).expect("valid mu edges");
        let phi = uniform_phi_edges(1, 0.0).expect("valid phi edges");
        let layout = Layout3::new(q, mu, phi, Coordinate::Physical, Boundary::Outflow)
            .expect("valid layout");
        let geometry = Geometry3::Constant {
            h1: 0.2,
            h2: 0.2,
            h3: 0.5,
        };
        let absorber = Some(Absorber {
            k0: 0.5,
            q_chi: -0.4,
        });
        Model3::new(layout, geometry, 0.5, absorber, false).expect("valid model3")
    })
}

/// Cached 3D spherical photon transport model on (q, mu, phi) with n = 302, bandwidth = 25.
pub fn sphere_3d() -> &'static Model3 {
    SPHERE_3D_MODEL.get_or_init(|| {
        let q = uniform_edges(-3.0, 2.0, 12).expect("valid q edges");
        let mu = uniform_mu_edges(6).expect("valid mu edges");
        let phi = uniform_phi_edges(4, 0.3).expect("valid phi edges");
        let layout = Layout3::new(q, mu, phi, Coordinate::Physical, Boundary::Outflow)
            .expect("valid layout");
        let geometry = Geometry3::Constant {
            h1: 0.2,
            h2: 0.2,
            h3: 0.5,
        };
        let absorber = Some(Absorber {
            k0: 0.5,
            q_chi: -0.4,
        });
        Model3::new(layout, geometry, 0.5, absorber, false).expect("valid model3")
    })
}

/// The problem kinds of corpus v1 plus the linear test problem with an exact
/// solution used by the adapter-equivalence tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Robertson chemical kinetics, n = 3.
    Robertson,
    /// HIRES (Schäfer 1975), n = 8.
    Hires,
    /// Van der Pol with mu = 1000 in the unscaled form y1' = y2,
    /// y2' = mu (1 - y1^2) y2 - y1.
    VanDerPolMu1000,
    /// Brusselator 1D reaction-diffusion, interleaved (u_k, v_k), Dirichlet
    /// u = 1, v = 3 at both ends, alpha = 1/50; n = 2m.
    Brusselator1d,
    /// Oregonator (Field & Noyes 1974, Hairer & Wanner Vol II p. 144), n = 3.
    Oregonator,
    /// y1' = -y1, y2' = 1000 (y1 - y2), y(0) = (1, 0): exact solution
    /// y1 = e^-t, y2 = 1000/999 (e^-t - e^-1000t).
    LinearExact,
    /// Full-sphere photon transport in 2D (q, mu), n = 86, bandwidth = 7.
    SpherePhoton2d,
    /// Full-sphere photon transport in 3D (q, mu, phi), n = 302, bandwidth = 25.
    SpherePhoton3d,
    /// Semilinear advection-diffusion reaction 2D grid, n = 96 (8x12), bandwidth = 8.
    SemilinearAdvectionDiffusion2d,
    /// Semilinear advection-diffusion reaction 3D grid, n = 216 (6x6x6), bandwidth = 36.
    SemilinearAdvectionDiffusion3d,
}

/// A problem instance: kind and dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    /// Problem kind.
    pub kind: Kind,
    /// Dimension.
    pub n: usize,
}

static RHS_CALLS: AtomicU64 = AtomicU64::new(0);
static JAC_CALLS: AtomicU64 = AtomicU64::new(0);
static JVP_CALLS: AtomicU64 = AtomicU64::new(0);

/// Callback counts measured by the shared model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackCounts {
    /// Right-hand-side evaluations.
    pub rhs: u64,
    /// Full Jacobian evaluations (any layout).
    pub jac: u64,
    /// Jacobian-vector products (Diffsol assembles its Jacobian from these).
    pub jvp: u64,
}

/// Reset the callback counters (call before a run).
pub fn reset_counts() {
    RHS_CALLS.store(0, Ordering::SeqCst);
    JAC_CALLS.store(0, Ordering::SeqCst);
    JVP_CALLS.store(0, Ordering::SeqCst);
}

/// Read the callback counters.
pub fn counts() -> CallbackCounts {
    CallbackCounts {
        rhs: RHS_CALLS.load(Ordering::SeqCst),
        jac: JAC_CALLS.load(Ordering::SeqCst),
        jvp: JVP_CALLS.load(Ordering::SeqCst),
    }
}

impl Model {
    /// Build and check a model.
    pub fn new(kind: Kind, n: usize) -> Result<Self, String> {
        let ok = match kind {
            Kind::Robertson | Kind::Oregonator => n == 3,
            Kind::Hires => n == 8,
            Kind::VanDerPolMu1000 | Kind::LinearExact => n == 2,
            Kind::Brusselator1d => n >= 4 && n.is_multiple_of(2),
            Kind::SpherePhoton2d => n == 86,
            Kind::SpherePhoton3d => n == 302,
            Kind::SemilinearAdvectionDiffusion2d => n == 96,
            Kind::SemilinearAdvectionDiffusion3d => n == 216,
        };
        if ok {
            Ok(Self { kind, n })
        } else {
            Err(format!("dimension {n} is not valid for {kind:?}"))
        }
    }

    /// The initial state.
    pub fn y0(&self) -> Vec<f64> {
        match self.kind {
            Kind::Robertson => vec![1.0, 0.0, 0.0],
            Kind::Oregonator => vec![1.0, 2.0, 3.0],
            Kind::Hires => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            Kind::VanDerPolMu1000 => vec![2.0, 0.0],
            Kind::Brusselator1d => {
                let m = self.n / 2;
                (0..m)
                    .flat_map(|i| {
                        let x = (i + 1) as f64 / (m + 1) as f64;
                        [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
                    })
                    .collect()
            }
            Kind::LinearExact => vec![1.0, 0.0],
            Kind::SpherePhoton2d => {
                let m = sphere_2d();
                let l = &m.layout;
                let mut y0 = vec![0.0; l.n()];
                for i in 0..l.nq {
                    let q = 0.5 * (l.q_edges[i] + l.q_edges[i + 1]);
                    for j in 0..l.nm {
                        let mu = 0.5 * (l.mu_edges[j] + l.mu_edges[j + 1]);
                        let val =
                            (-((q - (-0.5)).powi(2) / 0.5 + (mu - 0.2).powi(2) / 0.4)).exp();
                        y0[l.cell(i, j, 0)] = val * l.dq[i] * l.dm[j] * l.dphi[0];
                    }
                }
                let sum: f64 = y0.iter().sum();
                if sum > 0.0 {
                    for v in &mut y0 {
                        *v /= sum;
                    }
                }
                y0
            }
            Kind::SpherePhoton3d => {
                let m = sphere_3d();
                let l = &m.layout;
                let mut y0 = vec![0.0; l.n()];
                for i in 0..l.nq {
                    let q = 0.5 * (l.q_edges[i] + l.q_edges[i + 1]);
                    for j in 0..l.nm {
                        let mu = 0.5 * (l.mu_edges[j] + l.mu_edges[j + 1]);
                        for k in 0..l.nphi {
                            let phi = 0.5 * (l.phi_edges[k] + l.phi_edges[k + 1]);
                            let val = (-((q - (-0.5)).powi(2) / 0.5
                                + (mu - 0.2).powi(2) / 0.4
                                + 0.1 * phi.cos()))
                            .exp();
                            y0[l.cell(i, j, k)] = val * l.dq[i] * l.dm[j] * l.dphi[k];
                        }
                    }
                }
                let sum: f64 = y0.iter().sum();
                if sum > 0.0 {
                    for v in &mut y0 {
                        *v /= sum;
                    }
                }
                y0
            }
            Kind::SemilinearAdvectionDiffusion2d => {
                let nx = 8;
                let ny = 12;
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let mut y0 = vec![0.0; nx * ny];
                for j in 0..ny {
                    let y = (j + 1) as f64 * hy;
                    let sy = (std::f64::consts::PI * y).sin();
                    for i in 0..nx {
                        let x = (i + 1) as f64 * hx;
                        y0[i + nx * j] = (std::f64::consts::PI * x).sin() * sy;
                    }
                }
                y0
            }
            Kind::SemilinearAdvectionDiffusion3d => {
                let (nx, ny, nz) = (6, 6, 6);
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let hz = 1.0 / (nz + 1) as f64;
                let stride_y = nx;
                let stride_z = nx * ny;
                let mut y0 = vec![0.0; nx * ny * nz];
                for l in 0..nz {
                    let z = (l + 1) as f64 * hz;
                    let sz = (std::f64::consts::PI * z).sin();
                    for j in 0..ny {
                        let y = (j + 1) as f64 * hy;
                        let sy = (std::f64::consts::PI * y).sin();
                        for i in 0..nx {
                            let x = (i + 1) as f64 * hx;
                            let sx = (std::f64::consts::PI * x).sin();
                            y0[i + stride_y * j + stride_z * l] = sx * sy * sz;
                        }
                    }
                }
                y0
            }
        }
    }

    /// Lower and upper half-bandwidths of the Jacobian.
    pub fn band(&self) -> (usize, usize) {
        match self.kind {
            Kind::SpherePhoton2d => (7, 7),
            Kind::SpherePhoton3d => (25, 25),
            Kind::SemilinearAdvectionDiffusion2d => (8, 8),
            Kind::SemilinearAdvectionDiffusion3d => (36, 36),
            Kind::Brusselator1d => (2, 2),
            Kind::LinearExact => (1, 0),
            _ => (self.n - 1, self.n - 1),
        }
    }

    /// The exact solution, where known.
    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        match self.kind {
            Kind::LinearExact => {
                let a = (-t).exp();
                Some(vec![a, 1000.0 / 999.0 * (a - (-1000.0 * t).exp())])
            }
            _ => None,
        }
    }

    /// The right-hand side f(t, y) into `dy`.
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        RHS_CALLS.fetch_add(1, Ordering::Relaxed);
        match self.kind {
            Kind::Robertson => {
                dy[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
                dy[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
                dy[2] = 3e7 * y[1] * y[1];
            }
            Kind::Oregonator => {
                let q = 8.375e-6;
                dy[0] = 77.27 * (y[1] - y[0] * y[1] + y[0] - q * y[0] * y[0]);
                dy[1] = (1.0 / 77.27) * (-y[1] - y[0] * y[1] + y[2]);
                dy[2] = 0.161 * (y[0] - y[2]);
            }
            Kind::Hires => {
                dy[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                dy[1] = 1.71 * y[0] - 8.75 * y[1];
                dy[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                dy[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                dy[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                dy[5] =
                    -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
                dy[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
                dy[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
            }
            Kind::VanDerPolMu1000 => {
                dy[0] = y[1];
                dy[1] = 1000.0 * (1.0 - y[0] * y[0]) * y[1] - y[0];
            }
            Kind::Brusselator1d => {
                let m = self.n / 2;
                let d = ((m + 1) as f64).powi(2) / 50.0;
                for k in 0..m {
                    let (a, b) = (2 * k, 2 * k + 1);
                    let (u, v) = (y[a], y[b]);
                    let (ul, vl) = if k == 0 {
                        (1.0, 3.0)
                    } else {
                        (y[a - 2], y[b - 2])
                    };
                    let (ur, vr) = if k + 1 == m {
                        (1.0, 3.0)
                    } else {
                        (y[a + 2], y[b + 2])
                    };
                    dy[a] = 1.0 + u * u * v - 4.0 * u + d * (ul - 2.0 * u + ur);
                    dy[b] = 3.0 * u - u * u * v + d * (vl - 2.0 * v + vr);
                }
            }
            Kind::LinearExact => {
                dy[0] = -y[0];
                dy[1] = 1000.0 * y[0] - 1000.0 * y[1];
            }
            Kind::SpherePhoton2d => {
                sphere_2d().rhs(_t, y, dy).expect("sphere_2d rhs failed");
            }
            Kind::SpherePhoton3d => {
                sphere_3d().rhs(_t, y, dy).expect("sphere_3d rhs failed");
            }
            Kind::SemilinearAdvectionDiffusion2d => {
                let nx = 8;
                let ny = 12;
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let d = 0.002;
                let vx = 0.5;
                let vy = 0.5;
                for j in 0..ny {
                    for i in 0..nx {
                        let k = i + nx * j;
                        let u = y[k];
                        let left = if i == 0 { 0.0 } else { y[k - 1] };
                        let right = if i + 1 == nx { 0.0 } else { y[k + 1] };
                        let down = if j == 0 { 0.0 } else { y[k - nx] };
                        let up = if j + 1 == ny { 0.0 } else { y[k + nx] };
                        let lap =
                            (left - 2.0 * u + right) / (hx * hx) + (down - 2.0 * u + up) / (hy * hy);
                        let adv = vx * (u - left) / hx + vy * (u - down) / hy;
                        let reaction = 2.0 * u * u - u;
                        dy[k] = d * lap - adv + reaction;
                    }
                }
            }
            Kind::SemilinearAdvectionDiffusion3d => {
                let (nx, ny, nz) = (6, 6, 6);
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let hz = 1.0 / (nz + 1) as f64;
                let d = 0.01;
                let (vx, vy, vz) = (0.2, 0.2, 0.2);
                let stride_y = nx;
                let stride_z = nx * ny;
                for l in 0..nz {
                    for j in 0..ny {
                        for i in 0..nx {
                            let k = i + stride_y * j + stride_z * l;
                            let u = y[k];
                            let left = if i == 0 { 0.0 } else { y[k - 1] };
                            let right = if i + 1 == nx { 0.0 } else { y[k + 1] };
                            let down = if j == 0 { 0.0 } else { y[k - stride_y] };
                            let up = if j + 1 == ny { 0.0 } else { y[k + stride_y] };
                            let back = if l == 0 { 0.0 } else { y[k - stride_z] };
                            let front = if l + 1 == nz { 0.0 } else { y[k + stride_z] };
                            let lap = (left - 2.0 * u + right) / (hx * hx)
                                + (down - 2.0 * u + up) / (hy * hy)
                                + (back - 2.0 * u + front) / (hz * hz);
                            let adv =
                                vx * (u - left) / hx + vy * (u - down) / hy + vz * (u - back) / hz;
                            let reaction = u * (1.0 - u * u);
                            dy[k] = d * lap - adv + reaction;
                        }
                    }
                }
            }
        }
    }

    /// Emit every structurally nonzero Jacobian entry (i, j, df_i/dy_j).
    /// Counts as one Jacobian evaluation; the layout writers below call it.
    fn entries(&self, y: &[f64], mut emit: impl FnMut(usize, usize, f64)) {
        match self.kind {
            Kind::Robertson => {
                emit(0, 0, -0.04);
                emit(0, 1, 1e4 * y[2]);
                emit(0, 2, 1e4 * y[1]);
                emit(1, 0, 0.04);
                emit(1, 1, -1e4 * y[2] - 6e7 * y[1]);
                emit(1, 2, -1e4 * y[1]);
                emit(2, 1, 6e7 * y[1]);
            }
            Kind::Oregonator => {
                let q = 8.375e-6;
                emit(0, 0, 77.27 * (-y[1] + 1.0 - 2.0 * q * y[0]));
                emit(0, 1, 77.27 * (1.0 - y[0]));
                emit(1, 0, (1.0 / 77.27) * (-y[1]));
                emit(1, 1, (1.0 / 77.27) * (-1.0 - y[0]));
                emit(1, 2, 1.0 / 77.27);
                emit(2, 0, 0.161);
                emit(2, 2, -0.161);
            }
            Kind::Hires => {
                for (i, j, z) in [
                    (0, 0, -1.71),
                    (0, 1, 0.43),
                    (0, 2, 8.32),
                    (1, 0, 1.71),
                    (1, 1, -8.75),
                    (2, 2, -10.03),
                    (2, 3, 0.43),
                    (2, 4, 0.035),
                    (3, 1, 8.32),
                    (3, 2, 1.71),
                    (3, 3, -1.12),
                    (4, 4, -1.745),
                    (4, 5, 0.43),
                    (4, 6, 0.43),
                    (5, 3, 0.69),
                    (5, 4, 1.71),
                    (5, 6, 0.69),
                    (6, 6, -1.81),
                    (7, 6, 1.81),
                ] {
                    emit(i, j, z);
                }
                emit(5, 5, -280.0 * y[7] - 0.43);
                emit(5, 7, -280.0 * y[5]);
                emit(6, 5, 280.0 * y[7]);
                emit(6, 7, 280.0 * y[5]);
                emit(7, 5, -280.0 * y[7]);
                emit(7, 7, -280.0 * y[5]);
            }
            Kind::VanDerPolMu1000 => {
                emit(0, 1, 1.0);
                emit(1, 0, -2000.0 * y[0] * y[1] - 1.0);
                emit(1, 1, 1000.0 * (1.0 - y[0] * y[0]));
            }
            Kind::Brusselator1d => {
                let m = self.n / 2;
                let d = ((m + 1) as f64).powi(2) / 50.0;
                for k in 0..m {
                    let (a, b) = (2 * k, 2 * k + 1);
                    let (u, v) = (y[a], y[b]);
                    emit(a, a, 2.0 * u * v - 4.0 - 2.0 * d);
                    emit(a, b, u * u);
                    emit(b, a, 3.0 - 2.0 * u * v);
                    emit(b, b, -u * u - 2.0 * d);
                    if k > 0 {
                        emit(a, a - 2, d);
                        emit(b, b - 2, d);
                    }
                    if k + 1 < m {
                        emit(a, a + 2, d);
                        emit(b, b + 2, d);
                    }
                }
            }
            Kind::LinearExact => {
                emit(0, 0, -1.0);
                emit(1, 0, 1000.0);
                emit(1, 1, -1000.0);
            }
            Kind::SpherePhoton2d => {
                sphere_2d()
                    .jacobian_entries(0.0, &mut emit)
                    .expect("sphere_2d jacobian failed");
            }
            Kind::SpherePhoton3d => {
                sphere_3d()
                    .jacobian_entries(0.0, &mut emit)
                    .expect("sphere_3d jacobian failed");
            }
            Kind::SemilinearAdvectionDiffusion2d => {
                let nx = 8;
                let ny = 12;
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let d = 0.002;
                let vx = 0.5;
                let vy = 0.5;
                let cx = d / (hx * hx);
                let cy = d / (hy * hy);
                let ax = vx / hx;
                let ay = vy / hy;
                for j in 0..ny {
                    for i in 0..nx {
                        let k = i + nx * j;
                        let u = y[k];
                        let diag = -2.0 * cx - 2.0 * cy - ax - ay + 4.0 * u - 1.0;
                        emit(k, k, diag);
                        if i > 0 {
                            emit(k, k - 1, cx + ax);
                        }
                        if i + 1 < nx {
                            emit(k, k + 1, cx);
                        }
                        if j > 0 {
                            emit(k, k - nx, cy + ay);
                        }
                        if j + 1 < ny {
                            emit(k, k + nx, cy);
                        }
                    }
                }
            }
            Kind::SemilinearAdvectionDiffusion3d => {
                let (nx, ny, nz) = (6, 6, 6);
                let hx = 1.0 / (nx + 1) as f64;
                let hy = 1.0 / (ny + 1) as f64;
                let hz = 1.0 / (nz + 1) as f64;
                let d = 0.01;
                let (vx, vy, vz) = (0.2, 0.2, 0.2);
                let cx = d / (hx * hx);
                let cy = d / (hy * hy);
                let cz = d / (hz * hz);
                let ax = vx / hx;
                let ay = vy / hy;
                let az = vz / hz;
                let stride_y = nx;
                let stride_z = nx * ny;
                for l in 0..nz {
                    for j in 0..ny {
                        for i in 0..nx {
                            let k = i + stride_y * j + stride_z * l;
                            let u = y[k];
                            let diag = -2.0 * (cx + cy + cz) - (ax + ay + az) + 1.0 - 3.0 * u * u;
                            emit(k, k, diag);
                            if i > 0 {
                                emit(k, k - 1, cx + ax);
                            }
                            if i + 1 < nx {
                                emit(k, k + 1, cx);
                            }
                            if j > 0 {
                                emit(k, k - stride_y, cy + ay);
                            }
                            if j + 1 < ny {
                                emit(k, k + stride_y, cy);
                            }
                            if l > 0 {
                                emit(k, k - stride_z, cz + az);
                            }
                            if l + 1 < nz {
                                emit(k, k + stride_z, cz);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Dense row-major Jacobian (`out[i * n + j]`), zero-filled first.
    pub fn jac_dense_row_major(&self, _t: f64, y: &[f64], out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        out[..n * n].fill(0.0);
        self.entries(y, |i, j, v| out[i * n + j] += v);
    }

    /// Dense column-major Jacobian with leading dimension `ld`
    /// (`out[i + ld * j]`), zero-filled first.
    pub fn jac_dense_col_major(&self, _t: f64, y: &[f64], ld: usize, out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        for j in 0..n {
            out[j * ld..j * ld + n].fill(0.0);
        }
        self.entries(y, |i, j, v| out[i + ld * j] += v);
    }

    /// LINPACK/Hairer band storage: `out[(mu + i - j) + ld * j]` for the
    /// entries inside the band, rows `0..=ml+mu` of each column zero-filled.
    /// `out` must start at the row of the first super-diagonal.
    pub fn jac_band_col_major(
        &self,
        _t: f64,
        y: &[f64],
        ml: usize,
        mu: usize,
        ld: usize,
        out: &mut [f64],
    ) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        let rows = ml + mu + 1;
        for j in 0..n {
            out[j * ld..j * ld + rows].fill(0.0);
        }
        self.entries(y, |i, j, v| {
            let k = mu as isize + i as isize - j as isize;
            if (0..rows as isize).contains(&k) {
                out[k as usize + ld * j] += v;
            }
        });
    }

    /// Row-major band storage of `rodas5p-integrators` (`out[i * w + (j + ml - i)]`
    /// with `w = ml + mu + 1`), zero-filled first.
    pub fn jac_band_row_major(&self, _t: f64, y: &[f64], ml: usize, mu: usize, out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let w = ml + mu + 1;
        out[..self.n * w].fill(0.0);
        self.entries(y, |i, j, v| {
            let k = j as isize + ml as isize - i as isize;
            if (0..w as isize).contains(&k) {
                out[i * w + k as usize] += v;
            }
        });
    }

    /// Jacobian-vector product J(y) v from the same entries.
    pub fn jvp(&self, _t: f64, y: &[f64], v: &[f64], out: &mut [f64]) {
        JVP_CALLS.fetch_add(1, Ordering::Relaxed);
        out[..self.n].fill(0.0);
        self.entries(y, |i, j, a| out[i] += a * v[j]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fd_check(m: Model, y: &[f64]) {
        let n = m.n;
        let mut j = vec![0.0; n * n];
        m.jac_dense_row_major(0.0, y, &mut j);
        let mut f0 = vec![0.0; n];
        let mut f1 = vec![0.0; n];
        m.rhs(0.0, y, &mut f0);
        for c in 0..n {
            let h = 1e-7 * y[c].abs().max(1.0);
            let mut yp = y.to_vec();
            yp[c] += h;
            m.rhs(0.0, &yp, &mut f1);
            for r in 0..n {
                let fd = (f1[r] - f0[r]) / h;
                let scale = j[r * n + c].abs().max(1.0);
                assert!(
                    (fd - j[r * n + c]).abs() <= 1e-4 * scale,
                    "{:?} J[{r},{c}] = {} vs fd {fd}",
                    m.kind,
                    j[r * n + c]
                );
            }
        }
    }

    #[test]
    fn jacobians_match_finite_differences() {
        for (k, n) in [
            (Kind::Robertson, 3),
            (Kind::Oregonator, 3),
            (Kind::Hires, 8),
            (Kind::VanDerPolMu1000, 2),
            (Kind::Brusselator1d, 20),
            (Kind::LinearExact, 2),
            (Kind::SpherePhoton2d, 86),
            (Kind::SemilinearAdvectionDiffusion2d, 96),
            (Kind::SemilinearAdvectionDiffusion3d, 216),
            (Kind::SpherePhoton3d, 302),
        ] {
            let m = Model::new(k, n).unwrap();
            let y: Vec<f64> = m
                .y0()
                .iter()
                .enumerate()
                .map(|(i, v)| v + 0.01 * (i as f64 + 1.0))
                .collect();
            fd_check(m, &y);
        }
    }

    #[test]
    fn layouts_agree() {
        for (k, n) in [
            (Kind::Brusselator1d, 12),
            (Kind::SpherePhoton2d, 86),
            (Kind::SemilinearAdvectionDiffusion2d, 96),
            (Kind::SemilinearAdvectionDiffusion3d, 216),
            (Kind::SpherePhoton3d, 302),
        ] {
            let m = Model::new(k, n).unwrap();
            let y = m.y0();
            let (ml, mu) = m.band();
            let mut d = vec![0.0; n * n];
            m.jac_dense_row_major(0.0, &y, &mut d);
            let mut c = vec![0.0; n * n];
            m.jac_dense_col_major(0.0, &y, n, &mut c);
            let ld = ml + mu + 1;
            let mut b = vec![0.0; ld * n];
            m.jac_band_col_major(0.0, &y, ml, mu, ld, &mut b);
            let mut r = vec![0.0; ld * n];
            m.jac_band_row_major(0.0, &y, ml, mu, &mut r);
            let v: Vec<f64> = (0..n).map(|i| ((i + 1) as f64).sin()).collect();
            let mut jv = vec![0.0; n];
            m.jvp(0.0, &y, &v, &mut jv);
            for i in 0..n {
                let mut s = 0.0;
                for j in 0..n {
                    assert_eq!(
                        d[i * n + j],
                        c[i + n * j],
                        "{k:?} dense row vs col mismatch at ({i}, {j})"
                    );
                    let in_band = (j as isize - i as isize >= -(ml as isize))
                        && (j as isize - i as isize <= mu as isize);
                    if in_band {
                        assert_eq!(
                            d[i * n + j],
                            b[(mu + i - j) + ld * j],
                            "{k:?} band col mismatch at ({i}, {j})"
                        );
                        assert_eq!(
                            d[i * n + j],
                            r[i * ld + (j + ml - i)],
                            "{k:?} band row mismatch at ({i}, {j})"
                        );
                    } else {
                        assert_eq!(
                            d[i * n + j],
                            0.0,
                            "{k:?} nonzero outside declared band at ({i}, {j})"
                        );
                    }
                    s += d[i * n + j] * v[j];
                }
                assert!(
                    (s - jv[i]).abs() <= 1e-11 * s.abs().max(1.0),
                    "{k:?} JVP mismatch at row {i}"
                );
            }
        }
    }
}
