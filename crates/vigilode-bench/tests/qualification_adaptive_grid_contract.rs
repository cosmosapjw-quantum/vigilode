//! Qualification contracts for runtime-configurable spatial grids, adaptive moving meshes,
//! and VG-1 zero-allocation pre-allocated workspaces across expanded 2D/3D PDE models.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use vigilode_bench::model::{Kind, Model};
use vigilode_bench::pde_models::{AllenCahn2D, SineGordon2D, SphericalDiffusion2D};
use vigilode_bench::spatial_grid::{
    BoundaryCondition, MovingMesh1D, SpatialGridSpec, SpatialWorkspace,
};

// Counting allocator for testing VG-1 hot-loop compliance in Phase 2
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static TRACKING_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

struct TestAlloc;

unsafe impl GlobalAlloc for TestAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let is_active = TRACKING_ACTIVE.try_with(|flag| flag.get()).unwrap_or(false);
        if is_active {
            ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
            ALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: TestAlloc = TestAlloc;

struct AllocGuard;

impl AllocGuard {
    fn start() -> Self {
        ALLOC_COUNT.store(0, Ordering::SeqCst);
        ALLOC_BYTES.store(0, Ordering::SeqCst);
        let _ = TRACKING_ACTIVE.try_with(|flag| flag.set(true));
        Self
    }
}

impl Drop for AllocGuard {
    fn drop(&mut self) {
        let _ = TRACKING_ACTIVE.try_with(|flag| flag.set(false));
    }
}

fn alloc_snapshot() -> (usize, usize) {
    (
        ALLOC_COUNT.load(Ordering::SeqCst),
        ALLOC_BYTES.load(Ordering::SeqCst),
    )
}

#[test]
fn test_spatial_workspace_phase1_alloc_phase2_zero_alloc() {
    let spec = SpatialGridSpec::rect_2d(
        16,
        16,
        (0.0, 1.0),
        (0.0, 1.0),
        2,
        BoundaryCondition::Periodic,
    );
    assert_eq!(spec.total_dofs(), 512);

    // Phase 1: Allocation occurs here
    let mut ws = SpatialWorkspace::new(&spec, 6, 20);
    assert!(ws.memory_bytes() > 0);

    // Phase 2: Numerical integration hot loop simulation
    let _guard = AllocGuard::start();

    for step in 0..100 {
        {
            let views = ws.split_all_views(0, 0);
            for i in 0..views.y.len() {
                views.y[i] += 0.001 * (step as f64);
                views.dy[i] = -views.y[i];
                views.stage[i] = views.dy[i] * 0.5;
            }

            views.jac_band[0] = 1.0;
            let last_jac = views.jac_band.len() - 1;
            views.jac_band[last_jac] = -1.0;

            views.krylov[0] = views.stage[0];
        }

        let scratch = ws.scratch_mut(1);
        scratch[0] = 42.0;
    }

    let (allocs, bytes) = alloc_snapshot();
    drop(_guard);

    assert_eq!(
        allocs, 0,
        "VG-1 VIOLATION: Phase 2 hot-loop performed {allocs} heap allocations ({bytes} bytes)"
    );
}

#[test]
fn test_runtime_arbitrary_grid_dimensions() {
    // Arbitrary runtime dimension configurations
    let test_cases = [
        (Kind::GrayScott2d, 128, 64),  // 64 cells, fields=2 -> 128 DOFs
        (Kind::GrayScott2d, 200, 100), // 100 cells, fields=2 -> 200 DOFs
        (Kind::AllenCahn2d, 64, 64),   // 64 cells, fields=1 -> 64 DOFs
        (Kind::AllenCahn2d, 144, 144), // 144 cells, fields=1 -> 144 DOFs
        (Kind::Burgers2d, 72, 36),     // 36 cells, fields=2 -> 72 DOFs
        (Kind::SineGordon2d, 96, 48),  // 48 cells, fields=2 -> 96 DOFs
        (Kind::SphericalDiffusion2d, 100, 100),
    ];

    for &(kind, n, _nodes) in &test_cases {
        let model = Model::new(kind, n).expect("valid runtime dimension");
        assert_eq!(model.n, n);
        let y0 = model.y0();
        assert_eq!(y0.len(), n);

        let (ml, mu) = model.band();
        assert!(ml > 0 && mu > 0, "bandwidth must be strictly positive");
        assert!(ml < n && mu < n, "bandwidth must lie within matrix bounds");

        let mut dy = vec![0.0; n];
        model.rhs(0.0, &y0, &mut dy);
        assert!(
            dy.iter().all(|v| v.is_finite()),
            "RHS evaluated non-finite values for {kind:?} at n={n}"
        );
    }
}

#[test]
fn test_moving_mesh_mmpde_constant_dimension_zero_alloc() {
    let nx = 64;
    let mm = MovingMesh1D::new(nx, 0.05, 5.0);

    // Initial uniform grid on [0, 1]
    let mut x: Vec<f64> = (0..nx).map(|i| i as f64 / (nx - 1) as f64).collect();
    // Solution with steep shock layer at x = 0.5
    let mut u: Vec<f64> = x
        .iter()
        .map(|&xi| 0.5 * (1.0 - ((xi - 0.5) / 0.02).tanh()))
        .collect();

    let mut monitors = vec![0.0; nx];
    let mut xdot = vec![0.0; nx];

    let _guard = AllocGuard::start();

    // 50 steps of moving mesh evolution (r-refinement)
    for _ in 0..50 {
        mm.compute_monitors(&u, &x, &mut monitors);
        mm.mesh_velocities(&x, &monitors, &mut xdot);

        // In-place node coordinate update with CFL damping
        let dt = 0.001;
        for i in 1..nx - 1 {
            x[i] += dt * xdot[i];
            // Ensure strict monotonicity x[i-1] < x[i] < x[i+1]
            x[i] = x[i].clamp(x[i - 1] + 1e-4, x[i + 1] - 1e-4);
        }

        // Update u on shifted mesh
        for i in 0..nx {
            u[i] = 0.5 * (1.0 - ((x[i] - 0.5) / 0.02).tanh());
        }
    }

    let (allocs, bytes) = alloc_snapshot();
    drop(_guard);

    assert_eq!(
        allocs, 0,
        "VG-1 VIOLATION: Moving mesh r-refinement performed {allocs} heap allocations ({bytes} bytes)"
    );

    // Verify grid refinement: grid resolution near shock (x ~ 0.5) must be significantly finer than at edges
    let dx_shock = (x[nx / 2 + 1] - x[nx / 2 - 1]) * 0.5;
    let dx_edge = x[2] - x[1];
    assert!(
        dx_shock < dx_edge * 0.5,
        "MMPDE failed to refine mesh around shock: dx_shock={dx_shock}, dx_edge={dx_edge}"
    );
}

#[test]
fn test_sine_gordon_hamiltonian_energy_conservation() {
    let sg = SineGordon2D::with_boundary(16, 16, 8.0, BoundaryCondition::Periodic);
    let n = sg.n();
    let mut y = sg.initial_state();
    let e0 = sg.total_energy(&y);

    let mut dy = vec![0.0; n];
    let dt = 0.0001;

    // Small symplectic Euler / Midpoint simulation step to check analytical derivative dH/dt == 0
    sg.rhs(0.0, &y, &mut dy);

    // Compute analytical energy derivative: \sum (\psi \dot{\psi} + \nabla\phi \cdot \nabla\dot{\phi} + \sin\phi \dot{\phi}) hx hy
    let mut d_energy_dt = 0.0;
    let nx = sg.nx;
    let ny = sg.ny;
    let hx = sg.hx;
    let hy = sg.hy;

    for j in 0..ny {
        let j_next = if j + 1 == ny { 0 } else { j + 1 };
        for i in 0..nx {
            let i_next = if i + 1 == nx { 0 } else { i + 1 };
            let idx = 2 * (i + nx * j);
            let phi = y[idx];
            let psi = y[idx + 1];
            let dot_phi = dy[idx];
            let dot_psi = dy[idx + 1];

            let grad_x = (y[2 * (i_next + nx * j)] - phi) / hx;
            let grad_y = (y[2 * (i + nx * j_next)] - phi) / hy;

            let dot_grad_x = (dy[2 * (i_next + nx * j)] - dot_phi) / hx;
            let dot_grad_y = (dy[2 * (i + nx * j_next)] - dot_phi) / hy;

            let term =
                psi * dot_psi + grad_x * dot_grad_x + grad_y * dot_grad_y + phi.sin() * dot_phi;
            d_energy_dt += term * hx * hy;
        }
    }

    assert!(
        d_energy_dt.abs() < 1e-10,
        "Sine-Gordon Hamiltonian energy is not conserved: dH/dt = {d_energy_dt}"
    );

    // Advance 10 tiny steps and verify energy stays close to e0
    for _ in 0..10 {
        sg.rhs(0.0, &y, &mut dy);
        for i in 0..n {
            y[i] += dt * dy[i];
        }
    }
    let e_final = sg.total_energy(&y);
    let rel_diff = (e_final - e0).abs() / e0;
    assert!(
        rel_diff < 1e-4,
        "Energy drifted unphysically: e0={e0}, e_final={e_final}, rel_diff={rel_diff}"
    );
}

#[test]
fn test_spherical_diffusion_finite_volume_mass_conservation() {
    let sd = SphericalDiffusion2D::standard(16, 24);
    let y0 = sd.initial_state();
    let m0 = sd.total_mass(&y0);

    let mut dy = vec![0.0; sd.n()];
    sd.rhs(0.0, &y0, &mut dy);

    // Compute dm/dt = \int \dot{u} d\Omega
    let mut dm_dt = 0.0;
    for j in 0..sd.n_theta {
        let area_j = sd.cell_areas[j];
        for k in 0..sd.n_phi {
            dm_dt += dy[k + sd.n_phi * j] * area_j;
        }
    }

    assert!(
        dm_dt.abs() < 1e-12,
        "Spherical surface diffusion violates conservation of mass: dm/dt = {dm_dt}"
    );
    assert!(m0 > 0.0, "Total mass must be positive: m0 = {m0}");
}

#[test]
fn test_spatial_mesh_refinement_order_allen_cahn() {
    // Test 2nd-order convergence of spatial Laplacian discretization using manufactured solution
    // u(x, y) = sin(pi * x) * sin(pi * y) on [-1, 1]^2 with Periodic boundary conditions
    let resolutions = [16, 32, 64];
    let mut errors = Vec::new();

    for &n_res in &resolutions {
        let ac = AllenCahn2D::with_boundary(n_res, n_res, BoundaryCondition::Periodic);
        let n = ac.n();
        let mut u = vec![0.0; n];
        let mut exact_rhs = vec![0.0; n];

        let pi = std::f64::consts::PI;
        let eps = ac.epsilon;
        let eps2 = eps * eps;

        for j in 0..ac.ny {
            let y = -1.0 + j as f64 * ac.hy;
            for i in 0..ac.nx {
                let x = -1.0 + i as f64 * ac.hx;
                let k = i + ac.nx * j;
                let val = (pi * x).sin() * (pi * y).sin();
                u[k] = val;

                // Exact Laplacian = -2 * pi^2 * sin(pi x) sin(pi y)
                let exact_lap = -2.0 * pi * pi * val;
                let reaction = -(val * val * val - val);
                exact_rhs[k] = eps2 * exact_lap + reaction;
            }
        }

        let mut num_rhs = vec![0.0; n];
        ac.rhs(0.0, &u, &mut num_rhs);

        // L_2 norm of discrete error
        let l2_err = (num_rhs
            .iter()
            .zip(exact_rhs.iter())
            .map(|(&num, &exact)| (num - exact).powi(2))
            .sum::<f64>()
            / n as f64)
            .sqrt();
        errors.push(l2_err);
    }

    // Measure convergence rates p = log2(E_{2h} / E_h)
    let rate1 = (errors[0] / errors[1]).log2();
    let rate2 = (errors[1] / errors[2]).log2();

    assert!(
        rate1 > 1.8 && rate1 < 2.2,
        "Expected 2nd order convergence between res 16 and 32, got order {rate1:.2} (errors: {:?})",
        errors
    );
    assert!(
        rate2 > 1.8 && rate2 < 2.2,
        "Expected 2nd order convergence between res 32 and 64, got order {rate2:.2} (errors: {:?})",
        errors
    );
}
