//! U09 geometry/MOL contracts. Reuses the existing periodic 2D Gray-Scott operator, adds a 3D
//! anisotropic periodic Cartesian Laplacian and a latitude-longitude finite-volume sphere
//! Laplacian, and checks a weighted self-adjoint/dissipative certificate. Spatial and temporal
//! errors are reported separately. Layout3 (log-energy, mu, phi) is NOT Cartesian 3D and is
//! not used here.

#![allow(clippy::needless_range_loop)]

use std::f64::consts::PI;

use serde_json::json;
use vigilode::{
    CapabilitySet, Jacobian, MethodAdmission, MethodRegistry, MethodRequest, Options, Problem,
    Rodas5p,
};

fn qual(tag: &str, v: serde_json::Value) {
    println!("QUAL {}", json!({"test": tag, "data": v}));
}

fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 11) as f64 / (1u64 << 53) as f64) - 0.5
}

/// Weighted self-adjointness and dissipativity over random vectors:
/// (max |u'HLv - v'HLu| / scale, max u'HLu / u'Hu).
fn certificate(apply: &dyn Fn(&[f64]) -> Vec<f64>, w: &[f64], trials: usize) -> (f64, f64) {
    let n = w.len();
    let mut seed = 7u64;
    let (mut sym, mut quad) = (0.0_f64, f64::NEG_INFINITY);
    let dot = |a: &[f64], b: &[f64]| -> f64 { (0..n).map(|i| w[i] * a[i] * b[i]).sum() };
    for _ in 0..trials {
        let u: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();
        let v: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();
        let (lu, lv) = (apply(&u), apply(&v));
        let scale =
            dot(&u, &u).sqrt() * dot(&lv, &lv).sqrt() + dot(&v, &v).sqrt() * dot(&lu, &lu).sqrt();
        sym = sym.max((dot(&u, &lv) - dot(&v, &lu)).abs() / scale);
        quad = quad.max(dot(&u, &lu) / dot(&u, &u));
    }
    (sym, quad)
}

fn orders(e: &[f64]) -> Vec<f64> {
    e.windows(2).map(|w| (w[0] / w[1]).log2()).collect()
}

// ---------------- 2D: reuse the existing periodic Gray-Scott Laplacian ----------------

const GS_DU: f64 = 2.0e-3;
const GS_F: f64 = 0.04;

fn gray_scott_diffusion(n_grid: usize) -> impl Fn(&[f64]) -> Vec<f64> {
    let (p, _) = rodas5p_integrators::gray_scott_2d(n_grid).unwrap();
    let n2 = n_grid * n_grid;
    move |u: &[f64]| {
        let mut y = u.to_vec();
        y.extend(std::iter::repeat_n(0.0, n2));
        let mut w = rodas5p_core::WorkCounters::default();
        let f = p.eval_rhs(0.0, &y, &mut w).unwrap();
        // with v = 0: u_t = D_u Lap_h u + F (1 - u)
        (0..n2).map(|i| f[i] - GS_F * (1.0 - u[i])).collect()
    }
}

#[test]
fn g1_reused_gray_scott_laplacian_fourier_constant_and_certificate() {
    let mut rows = Vec::new();
    let mut eig_err = Vec::new();
    for n in [16usize, 32, 64] {
        let lap = gray_scott_diffusion(n);
        let dx = 1.0 / n as f64;
        let (kx, ky) = (2.0_f64, 1.0_f64);
        let mode: Vec<f64> = (0..n * n)
            .map(|idx| {
                let (i, j) = ((idx % n) as f64, (idx / n) as f64);
                (2.0 * PI * (kx * (i + 0.5) + ky * (j + 0.5)) * dx).cos()
            })
            .collect();
        let lm = lap(&mode);
        let lam_h = -GS_DU * 4.0 / (dx * dx)
            * ((PI * kx * dx).sin().powi(2) + (PI * ky * dx).sin().powi(2));
        let lam = -GS_DU * 4.0 * PI * PI * (kx * kx + ky * ky);
        let defect = lm
            .iter()
            .zip(&mode)
            .map(|(a, b)| (a - lam_h * b).abs())
            .fold(0.0, f64::max)
            / lam_h.abs();
        assert!(defect < 1e-10, "Fourier eigenvector defect {defect:e}");
        let constant = lap(&vec![1.0; n * n])
            .iter()
            .fold(0.0_f64, |m, v| m.max(v.abs()));
        assert!(constant < 1e-12, "constant mode {constant:e}");
        let w = vec![dx * dx; n * n];
        let (sym, quad) = certificate(&lap, &w, 4);
        assert!(sym < 1e-12 && quad <= 1e-12, "sym {sym:e} quad {quad:e}");
        eig_err.push((lam_h - lam).abs() / lam.abs());
        rows.push(
            json!({"n": n, "lambda_h": lam_h, "lambda": lam, "eigvec_defect": defect,
                         "constant_mode": constant, "sym_defect": sym, "max_rayleigh": quad}),
        );
    }
    let o = orders(&eig_err);
    assert!(
        o.iter().all(|&x| (x - 2.0).abs() < 0.1),
        "spatial order {o:?}"
    );
    qual(
        "g1_gray_scott_2d",
        json!({"rows": rows, "eigen_rel_error": eig_err, "spatial_order": o}),
    );
}

// ---------------- 3D anisotropic periodic Cartesian diffusion (new) ----------------

#[derive(Clone, Copy)]
struct Box3 {
    n: [usize; 3],
    len: [f64; 3],
    d: f64,
}

impl Box3 {
    fn dx(&self, k: usize) -> f64 {
        self.len[k] / self.n[k] as f64
    }
    fn size(&self) -> usize {
        self.n[0] * self.n[1] * self.n[2]
    }
    fn apply(&self, u: &[f64]) -> Vec<f64> {
        let [nx, ny, nz] = self.n;
        let id = |i: usize, j: usize, k: usize| i + nx * (j + ny * k);
        let (cx, cy, cz) = (self.dx(0).powi(2), self.dx(1).powi(2), self.dx(2).powi(2));
        let mut out = vec![0.0; u.len()];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let c = u[id(i, j, k)];
                    let sx = u[id((i + 1) % nx, j, k)] + u[id((i + nx - 1) % nx, j, k)] - 2.0 * c;
                    let sy = u[id(i, (j + 1) % ny, k)] + u[id(i, (j + ny - 1) % ny, k)] - 2.0 * c;
                    let sz = u[id(i, j, (k + 1) % nz)] + u[id(i, j, (k + nz - 1) % nz)] - 2.0 * c;
                    out[id(i, j, k)] = self.d * (sx / cx + sy / cy + sz / cz);
                }
            }
        }
        out
    }
    fn mode(&self, kk: [usize; 3]) -> Vec<f64> {
        let [nx, ny, _] = self.n;
        (0..self.size())
            .map(|idx| {
                let c = [idx % nx, (idx / nx) % ny, idx / (nx * ny)];
                let ph: f64 = (0..3)
                    .map(|d| kk[d] as f64 * (c[d] as f64 + 0.5) / self.n[d] as f64)
                    .sum();
                (2.0 * PI * ph).cos()
            })
            .collect()
    }
    fn lambda_h(&self, kk: [usize; 3]) -> f64 {
        -self.d
            * (0..3)
                .map(|d| {
                    4.0 * (PI * kk[d] as f64 / self.n[d] as f64).sin().powi(2) / self.dx(d).powi(2)
                })
                .sum::<f64>()
    }
    fn lambda(&self, kk: [usize; 3]) -> f64 {
        -self.d
            * (0..3)
                .map(|d| (2.0 * PI * kk[d] as f64 / self.len[d]).powi(2))
                .sum::<f64>()
    }
}

#[test]
fn g2_cartesian_3d_anisotropic_spatial_and_temporal_errors_are_separate() {
    let kk = [1, 1, 2];
    let mut rows = Vec::new();
    let mut eig_err = Vec::new();
    for m in [8usize, 16, 32] {
        let b = Box3 {
            n: [m, m, 2 * m],
            len: [1.0, 2.0, 0.5],
            d: 0.1,
        };
        let mode = b.mode(kk);
        let lm = b.apply(&mode);
        let lh = b.lambda_h(kk);
        let defect = lm
            .iter()
            .zip(&mode)
            .map(|(a, x)| (a - lh * x).abs())
            .fold(0.0, f64::max)
            / lh.abs();
        assert!(defect < 1e-10, "3D eigvec defect {defect:e}");
        let constant = b
            .apply(&vec![1.0; b.size()])
            .iter()
            .fold(0.0_f64, |acc, v| acc.max(v.abs()));
        assert!(constant < 1e-9);
        let w = vec![b.dx(0) * b.dx(1) * b.dx(2); b.size()];
        let (sym, quad) = certificate(&|u| b.apply(u), &w, 3);
        assert!(sym < 1e-12 && quad <= 1e-12);
        eig_err.push((lh - b.lambda(kk)).abs() / b.lambda(kk).abs());
        rows.push(json!({"grid": b.n, "lambda_h": lh, "lambda": b.lambda(kk),
                         "eigvec_defect": defect, "sym_defect": sym, "max_rayleigh": quad}));
    }
    let o = orders(&eig_err);
    assert!(
        o.iter().all(|&x| (x - 2.0).abs() < 0.1),
        "3D spatial order {o:?}"
    );
    // temporal error of the protected RODAS5P facade (JVP route) on the semi-discrete mode,
    // against exp(lambda_h t); the spatial error exp(lambda_h t) - exp(lambda t) is separate
    let b = Box3 {
        n: [8, 8, 16],
        len: [1.0, 2.0, 0.5],
        d: 0.1,
    };
    let (lh, lam) = (b.lambda_h(kk), b.lambda(kk));
    let t_end = 0.05;
    let mode = b.mode(kk);
    let mut temporal = Vec::new();
    for rtol in [1e-4, 1e-6, 1e-8] {
        let (bb, bj) = (b, b);
        let p = Problem::new(b.size(), move |_t, u, du| {
            du.copy_from_slice(&bb.apply(u));
            Ok(())
        })
        .jacobian(Jacobian::jvp(move |_t, _u, v, o| {
            o.copy_from_slice(&bj.apply(v));
            Ok(())
        }))
        .autonomous();
        let opts = Options {
            rtol,
            ..Options::default()
        };
        let s = Rodas5p::solve(&p, (0.0, t_end), &mode, &opts).unwrap();
        let y = s.final_state().unwrap();
        let exact_h = (lh * t_end).exp();
        let e_t = y
            .iter()
            .zip(&mode)
            .map(|(a, m)| (a - exact_h * m).abs())
            .fold(0.0, f64::max);
        assert!(s.is_success());
        temporal.push(json!({"rtol": rtol, "temporal_error": e_t}));
    }
    let spatial = ((lh * t_end).exp() - (lam * t_end).exp()).abs();
    qual(
        "g2_cartesian_3d",
        json!({"rows": rows, "eigen_rel_error": eig_err, "spatial_order": o,
               "time_integration": {"grid": b.n, "t_end": t_end,
                                    "spatial_amplitude_error": spatial, "temporal": temporal}}),
    );
}

// ---------------- Sphere: lat-lon finite volumes, H = cell areas, zero polar flux ----------------

struct Sphere {
    nt: usize,
    np: usize,
    r: f64,
}

impl Sphere {
    fn dth(&self) -> f64 {
        PI / self.nt as f64
    }
    fn dph(&self) -> f64 {
        2.0 * PI / self.np as f64
    }
    fn area(&self, i: usize) -> f64 {
        let (a, b) = (i as f64 * self.dth(), (i + 1) as f64 * self.dth());
        self.r * self.r * self.dph() * (a.cos() - b.cos())
    }
    fn centre(&self, i: usize, j: usize) -> (f64, f64) {
        ((i as f64 + 0.5) * self.dth(), (j as f64 + 0.5) * self.dph())
    }
    fn apply(&self, u: &[f64]) -> Vec<f64> {
        let (nt, np) = (self.nt, self.np);
        let id = |i: usize, j: usize| i * np + j;
        let mut out = vec![0.0; u.len()];
        for i in 0..nt {
            let st = self.centre(i, 0).0.sin();
            for j in 0..np {
                let c = u[id(i, j)];
                let mut flux = 0.0;
                // theta faces: length R sin(theta_f) dphi, gradient / (R dtheta); poles: length 0
                if i > 0 {
                    let th = i as f64 * self.dth();
                    flux += th.sin() * self.dph() * (u[id(i - 1, j)] - c) / self.dth();
                }
                if i + 1 < nt {
                    let th = (i + 1) as f64 * self.dth();
                    flux += th.sin() * self.dph() * (u[id(i + 1, j)] - c) / self.dth();
                }
                // phi faces (periodic): length R dtheta, gradient / (R sin(theta_c) dphi)
                for jj in [(j + np - 1) % np, (j + 1) % np] {
                    flux += self.dth() * (u[id(i, jj)] - c) / (st * self.dph());
                }
                out[id(i, j)] = flux / self.area(i);
            }
        }
        out
    }
    fn weights(&self) -> Vec<f64> {
        (0..self.nt * self.np)
            .map(|k| self.area(k / self.np))
            .collect()
    }
    fn field(&self, f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
        (0..self.nt * self.np)
            .map(|k| {
                let (th, ph) = self.centre(k / self.np, k % self.np);
                f(th, ph)
            })
            .collect()
    }
}

type Harmonic = (&'static str, f64, fn(f64, f64) -> f64);

#[test]
fn g3_sphere_area_weights_constants_harmonics_and_poles() {
    let r = 2.0;
    let modes: [Harmonic; 3] = [
        ("Y10", 2.0, |t, _| t.cos()),
        ("Y20", 6.0, |t, _| 1.5 * t.cos().powi(2) - 0.5),
        ("Y11", 2.0, |t, p| t.sin() * p.cos()),
    ];
    let mut rows = Vec::new();
    let mut errs: Vec<Vec<f64>> = vec![Vec::new(); modes.len()];
    for nt in [16usize, 32, 64] {
        let s = Sphere { nt, np: 2 * nt, r };
        let w = s.weights();
        let total: f64 = w.iter().sum();
        assert!(
            (total - 4.0 * PI * r * r).abs() < 1e-10 * total,
            "area {total}"
        );
        let lc = s.apply(&vec![1.0; nt * 2 * nt]);
        assert!(lc.iter().all(|v| v.abs() < 1e-9), "constant mode");
        let (sym, quad) = certificate(&|u| s.apply(u), &w, 3);
        assert!(
            sym < 1e-12 && quad <= 1e-12,
            "sphere sym {sym:e} quad {quad:e}"
        );
        let mut seed = 3u64;
        let u: Vec<f64> = (0..w.len()).map(|_| lcg(&mut seed)).collect();
        let cons: f64 = s.apply(&u).iter().zip(&w).map(|(a, b)| a * b).sum();
        assert!(cons.abs() < 1e-10, "weighted mass balance {cons:e}");
        let mut mrow = Vec::new();
        for (m, (name, ell, f)) in modes.iter().enumerate() {
            let y = s.field(f);
            let ly = s.apply(&y);
            let lam = -ell / (r * r);
            let num: f64 = ly
                .iter()
                .zip(&y)
                .zip(&w)
                .map(|((a, b), c)| c * (a - lam * b).powi(2))
                .sum();
            let den: f64 = y.iter().zip(&w).map(|(b, c)| c * (lam * b).powi(2)).sum();
            let e = (num / den).sqrt();
            let ymax = y.iter().fold(0.0_f64, |acc, v| acc.max(v.abs()));
            let pole = (0..2 * nt)
                .map(|j| (ly[j] - lam * y[j]).abs())
                .fold(0.0, f64::max)
                / (lam.abs() * ymax);
            errs[m].push(e);
            mrow.push(json!({"mode": name, "weighted_rel_error": e, "pole_ring_max_rel": pole}));
        }
        rows.push(json!({"n_theta": nt, "area_sum": total, "sym_defect": sym,
                         "max_rayleigh": quad, "mass_balance": cons, "modes": mrow}));
    }
    let ords: Vec<Vec<f64>> = errs.iter().map(|e| orders(e)).collect();
    qual("g3_sphere_fv", json!({"rows": rows, "orders": ords}));
    for (m, o) in ords.iter().enumerate() {
        assert!(o.iter().all(|&x| x > 0.9), "mode {m} orders {o:?}");
    }
}

#[test]
fn g4_nonsymmetric_advection_is_refused_by_the_certificate() {
    let n = 64;
    let dx = 1.0 / n as f64;
    let central = move |u: &[f64]| -> Vec<f64> {
        (0..n)
            .map(|i| -(u[(i + 1) % n] - u[(i + n - 1) % n]) / (2.0 * dx))
            .collect()
    };
    let upwind = move |u: &[f64]| -> Vec<f64> {
        (0..n).map(|i| -(u[i] - u[(i + n - 1) % n]) / dx).collect()
    };
    let w = vec![dx; n];
    let (sc, _) = certificate(&central, &w, 4);
    let (su, qu) = certificate(&upwind, &w, 4);
    assert!(sc > 1e-3 && su > 1e-3, "advection must fail: {sc} {su}");
    qual(
        "g4_advection_refused",
        json!({"central_sym_defect": sc, "upwind_sym_defect": su, "upwind_max_rayleigh": qu,
               "verdict": "refused by the self-adjoint certificate; a weighted dissipativity/resolvent proof is required"}),
    );
}

#[test]
fn g5_mass_and_event_capabilities_come_from_the_registry() {
    let allowed = CapabilitySet {
        explicit_jacobian: true,
        dense_matrix: true,
        newton: true,
        ..CapabilitySet::NONE
    };
    let out = MethodRegistry::query(&MethodRequest {
        allowed,
        needs_mass: true,
        needs_events: true,
        needs_dense_output: true,
        admit_unqualified: false,
    });
    let admitted: Vec<&str> = out
        .iter()
        .filter_map(|a| match a {
            MethodAdmission::Admitted { method_id, .. } => Some(*method_id),
            _ => None,
        })
        .collect();
    assert_eq!(admitted, vec!["rodas5p"]);
    qual(
        "g5_capabilities",
        json!({"admitted_with_mass_events_dense": admitted}),
    );
}
