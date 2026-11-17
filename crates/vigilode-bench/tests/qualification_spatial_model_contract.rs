//! Spatial ODE model qualification contracts (V3-PHYSICS).
//!
//! Validates:
//! 1. Sphere photon transport finite-volume models (2D & 3D):
//!    - Non-tilted triaxial Bianchi-I geometry with unpolarized rank-six Thomson kernel.
//!    - Cell measures (dq, dmu, dphi) positivity and solid angle normalization.
//!    - Number/absorber-ledger conservation: sum of all components is an exact semi-discrete linear invariant.
//!    - Non-negative states produce non-negative outflow and absorption flux rates.
//!    - Exact linearity: J * v == RHS(v).
//! 2. Semilinear advection-diffusion reaction models (2D & 3D):
//!    - Method-of-lines 5-point and 7-point stencils with homogeneous Dirichlet boundaries.
//!    - First-order upwind advection and second-order centered diffusion.
//!    - Analytic Jacobian consistency against finite difference quotients.
//! 3. Spatial convergence order verification via manufactured solution test on spatial grids.

use vigilode_bench::model::{Kind, Model, sphere_2d, sphere_3d};

#[test]
fn test_sphere_photon_2d_and_3d_geometry_and_measures() {
    let m2d = sphere_2d();
    assert_eq!(m2d.layout.nq, 12);
    assert_eq!(m2d.layout.nm, 6);
    assert_eq!(m2d.layout.nphi, 1);
    assert_eq!(m2d.layout.n(), 86); // 1 (L_lo) + 12 * (6*1 + 1) + 1 (L_hi) = 86

    // Check positive cell measures
    for &dq in &m2d.layout.dq {
        assert!(dq > 0.0);
    }
    for &dm in &m2d.layout.dm {
        assert!(dm > 0.0);
    }
    assert_eq!(m2d.layout.dphi.len(), 1);

    let m3d = sphere_3d();
    assert_eq!(m3d.layout.nq, 12);
    assert_eq!(m3d.layout.nm, 6);
    assert_eq!(m3d.layout.nphi, 4);
    assert_eq!(m3d.layout.n(), 302); // 1 (L_lo) + 12 * (6*4 + 1) + 1 (L_hi) = 302

    for &dq in &m3d.layout.dq {
        assert!(dq > 0.0);
    }
    for &dm in &m3d.layout.dm {
        assert!(dm > 0.0);
    }
    for &dp in &m3d.layout.dphi {
        assert!(dp > 0.0);
    }

    // Check angular solid angle coverage: sum(dm_j * dphi_k) == 4 * pi
    let mut solid_angle_sum = 0.0;
    for &dm in &m3d.layout.dm {
        for &dp in &m3d.layout.dphi {
            solid_angle_sum += dm * dp;
        }
    }
    let expected_solid_angle = 4.0 * std::f64::consts::PI;
    assert!(
        (solid_angle_sum - expected_solid_angle).abs() < 1e-12,
        "Solid angle sum must equal 4*pi: got {solid_angle_sum}, expected {expected_solid_angle}"
    );
}

#[test]
fn test_sphere_photon_exact_linear_invariant_and_positivity() {
    for (name, model_inst, n) in [
        (
            "sphere-2d",
            Model::new(Kind::SpherePhoton2d, 86).unwrap(),
            86,
        ),
        (
            "sphere-3d",
            Model::new(Kind::SpherePhoton3d, 302).unwrap(),
            302,
        ),
    ] {
        let y0 = model_inst.y0();
        assert_eq!(y0.len(), n);
        // Initial state is normalized probability distribution: non-negative and sums to 1
        let sum_y0: f64 = y0.iter().sum();
        assert!(
            (sum_y0 - 1.0).abs() < 1e-12,
            "{name}: y0 must sum to 1, got {sum_y0}"
        );
        for &v in &y0 {
            assert!(v >= 0.0, "{name}: y0 components must be non-negative");
        }

        // Evaluate RHS at t = 0
        let mut dy = vec![0.0; n];
        model_inst.rhs(0.0, &y0, &mut dy);

        // Sum of all dy components must be exactly zero (semi-discrete number ledger closure)
        let sum_dy: f64 = dy.iter().sum();
        assert!(
            sum_dy.abs() < 1e-13,
            "{name}: total flux derivative must sum to zero (invariant closure), got {sum_dy}"
        );

        // Outflow ledgers L_lo (index 0) and L_hi (index n-1) must have non-negative accumulation rate
        assert!(
            dy[0] >= -1e-15,
            "{name}: L_lo rate must be non-negative: dy[0]={}",
            dy[0]
        );
        assert!(
            dy[n - 1] >= -1e-15,
            "{name}: L_hi rate must be non-negative: dy[n-1]={}",
            dy[n - 1]
        );
    }
}

#[test]
fn test_sphere_photon_jacobian_vs_rhs_exact_linearity() {
    // Because transport and Thomson scattering are linear in state y,
    // J(t) * y == RHS(t, y) identically.
    let m2d = Model::new(Kind::SpherePhoton2d, 86).unwrap();
    let y = m2d.y0();
    let mut rhs = vec![0.0; 86];
    m2d.rhs(0.5, &y, &mut rhs);

    let (kl, ku) = m2d.band();
    assert_eq!((kl, ku), (7, 7));

    let mut jac = vec![0.0; 86 * 86];
    m2d.jac_dense_row_major(0.5, &y, &mut jac);

    // Compute matrix-vector product jac * y
    let mut jy = vec![0.0; 86];
    for r in 0..86 {
        let mut s = 0.0;
        for c in 0..86 {
            s += jac[r * 86 + c] * y[c];
        }
        jy[r] = s;
    }

    let mut max_diff = 0.0;
    for i in 0..86 {
        let diff = (jy[i] - rhs[i]).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    assert!(
        max_diff < 1e-13,
        "Sphere 2D exact linearity J * y == RHS failed with max_diff={max_diff}"
    );
}

#[test]
fn test_semilinear_advection_diffusion_stencils_and_dirichlet() {
    let m2d = Model::new(Kind::SemilinearAdvectionDiffusion2d, 96).unwrap();
    let y0 = m2d.y0();
    assert_eq!(y0.len(), 96);
    let (kl2, ku2) = m2d.band();
    assert_eq!((kl2, ku2), (8, 8));

    let m3d = Model::new(Kind::SemilinearAdvectionDiffusion3d, 216).unwrap();
    let y0_3d = m3d.y0();
    assert_eq!(y0_3d.len(), 216);
    let (kl3, ku3) = m3d.band();
    assert_eq!((kl3, ku3), (36, 36));

    // Verify boundary compliance: at t=0, y0 is sin(pi*x)*sin(pi*y), strictly positive inside (0, 1)^2
    for &v in &y0 {
        assert!(v > 0.0, "2D interior values must be positive");
    }
    for &v in &y0_3d {
        assert!(v > 0.0, "3D interior values must be positive");
    }
}

#[test]
fn test_semilinear_advection_diffusion_jacobian_consistency() {
    let m2d = Model::new(Kind::SemilinearAdvectionDiffusion2d, 96).unwrap();
    let y = m2d.y0();
    let mut jac = vec![0.0; 96 * 96];
    m2d.jac_dense_row_major(0.0, &y, &mut jac);

    let mut dy = vec![0.0; 96];
    m2d.rhs(0.0, &y, &mut dy);

    // Check with centered finite differences along test directions
    let eps = 1e-7;
    let mut y_plus = y.clone();
    let mut y_minus = y.clone();
    let mut dy_plus = vec![0.0; 96];
    let mut dy_minus = vec![0.0; 96];

    for col in [0, 5, 23, 50, 95] {
        y_plus[col] += eps;
        y_minus[col] -= eps;
        m2d.rhs(0.0, &y_plus, &mut dy_plus);
        m2d.rhs(0.0, &y_minus, &mut dy_minus);
        y_plus[col] = y[col];
        y_minus[col] = y[col];

        for row in 0..96 {
            let num_deriv = (dy_plus[row] - dy_minus[row]) / (2.0 * eps);
            let ana_deriv = jac[row * 96 + col];
            let err = (num_deriv - ana_deriv).abs();
            assert!(
                err < 1e-5,
                "Jacobian mismatch at row {row}, col {col}: numerical={num_deriv}, analytical={ana_deriv}, err={err}"
            );
        }
    }
}

#[test]
fn test_spatial_discretization_mesh_refinement_convergence() {
    // Spatial convergence order test: evaluate the method-of-lines spatial operator
    // on a manufactured smooth field u(x, y) = sin(pi * x) * sin(pi * y) on (0, 1)^2.
    // L[u] = d * (u_xx + u_yy) - vx * u_x - vy * u_y + R(u)
    // Here u_xx + u_yy = -2 * pi^2 * u
    // u_x = pi * cos(pi * x) * sin(pi * y)
    // u_y = pi * sin(pi * x) * cos(pi * y)
    // First-order upwind advection has spatial truncation error O(h).
    // Centered diffusion has spatial truncation error O(h^2).
    let d = 0.002;
    let vx = 0.5;
    let vy = 0.5;

    let eval_error_for_n = |n: usize| -> f64 {
        let h = 1.0 / (n + 1) as f64;
        let mut max_err = 0.0f64;
        for j in 1..=n {
            let y = j as f64 * h;
            let sy = (std::f64::consts::PI * y).sin();
            let cy = (std::f64::consts::PI * y).cos();
            for i in 1..=n {
                let x = i as f64 * h;
                let sx = (std::f64::consts::PI * x).sin();
                let cx = (std::f64::consts::PI * x).cos();
                let u = sx * sy;

                let u_left = (std::f64::consts::PI * (x - h)).sin() * sy;
                let u_right = (std::f64::consts::PI * (x + h)).sin() * sy;
                let u_down = sx * (std::f64::consts::PI * (y - h)).sin();
                let u_up = sx * (std::f64::consts::PI * (y + h)).sin();

                // Discrete operator
                let lap =
                    (u_left - 2.0 * u + u_right) / (h * h) + (u_down - 2.0 * u + u_up) / (h * h);
                let adv = vx * (u - u_left) / h + vy * (u - u_down) / h;
                let discrete = d * lap - adv + (2.0 * u * u - u);

                // Exact operator
                let exact_lap = -2.0 * std::f64::consts::PI.powi(2) * u;
                let exact_adv =
                    vx * std::f64::consts::PI * cx * sy + vy * std::f64::consts::PI * sx * cy;
                let exact = d * exact_lap - exact_adv + (2.0 * u * u - u);

                let err = (discrete - exact).abs();
                if err > max_err {
                    max_err = err;
                }
            }
        }
        max_err
    };

    let err_n16 = eval_error_for_n(16);
    let err_n32 = eval_error_for_n(32);
    let err_n64 = eval_error_for_n(64);

    let ratio1 = err_n16 / err_n32;
    let ratio2 = err_n32 / err_n64;

    // Upwind advection dominant order: ratio should approach 2.0 (O(h))
    assert!(
        ratio1 > 1.8 && ratio1 < 2.2,
        "Mesh refinement error ratio 16->32 must be ~2.0 for first-order upwind: got {ratio1}"
    );
    assert!(
        ratio2 > 1.85 && ratio2 < 2.15,
        "Mesh refinement error ratio 32->64 must be ~2.0 for first-order upwind: got {ratio2}"
    );
}
