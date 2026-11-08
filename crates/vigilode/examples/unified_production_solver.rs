//! Demonstration of the VigilODE Unified Production Solver (`VigilODE`).
//!
//! Run with: `cargo run -p vigilode --example unified_production_solver`
//!
//! Highlights:
//! 1. Single concise frontend API (`VigilODE::solve`).
//! 2. Autonomous multi-tier polyalgorithm routing across nonstiff, stiff, and matrix-free problems.
//! 3. Physical invariant drift monitoring.
//! 4. Fail-closed transactional protected fallback.
//! 5. Zero heap allocation in stepping hot loops (`VG-1`).

use vigilode::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("    VigilODE — Unified Production Solver Demonstration      ");
    println!("============================================================\n");

    // -------------------------------------------------------------------------
    // Problem 1: Stiff Chemical Kinetics (Robertson 3-Component Reaction)
    // -------------------------------------------------------------------------
    println!("--- 1. Robertson Stiff Chemical Kinetics ---");
    let robertson = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            0.0,
            6.0e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
    .invariant("total_mass", |y| y[0] + y[1] + y[2])
    .autonomous();

    let opts = VigilodeOptions::default()
        .rtol(1e-6)
        .atol(1e-10)
        .strategy(SolverStrategy::Auto);

    let sol = VigilODE::solve(&robertson, (0.0, 40.0), &[1.0, 0.0, 0.0], opts)?;
    println!("Status: {:?}", sol.status);
    println!("Final time: {}", sol.final_time);
    println!("Final state: {:?}", sol.final_state().unwrap());
    println!("Route taken: {:?}", sol.route_history);
    println!("Invariant drift: {:?}", sol.invariant_drift());
    println!(
        "Work counters: RHS={}, LU={}, Solves={}, Steps={}\n",
        sol.stats().counters.rhs_calls,
        sol.stats().counters.direct_factorizations,
        sol.stats().counters.direct_solve_calls,
        sol.stats().accepted_steps
    );

    // -------------------------------------------------------------------------
    // Problem 2: Nonstiff Conservative Harmonic Oscillator
    // -------------------------------------------------------------------------
    println!("--- 2. Nonstiff Harmonic Oscillator ---");
    let oscillator = Problem::new(2, |_t, y, dy| {
        dy[0] = y[1];
        dy[1] = -y[0];
        Ok(())
    })
    .invariant("energy", |y| y[0] * y[0] + y[1] * y[1])
    .autonomous();

    let osc_opts = VigilodeOptions::default().strategy(SolverStrategy::Auto);

    let osc_sol = VigilODE::solve(
        &oscillator,
        (0.0, std::f64::consts::TAU),
        &[1.0, 0.0],
        osc_opts,
    )?;
    println!("Status: {:?}", osc_sol.status);
    println!("Final state: {:?}", osc_sol.final_state().unwrap());
    println!("Route taken: {:?}", osc_sol.route_history);
    println!("Invariant drift: {:?}\n", osc_sol.invariant_drift());

    // -------------------------------------------------------------------------
    // Problem 3: High-Dimensional Spatial Diffusion with Matrix-Free JVP
    // -------------------------------------------------------------------------
    println!("--- 3. 1D Heat Diffusion (Matrix-Free Krylov) ---");
    let dim = 30;
    let dx = 1.0 / (dim as f64 + 1.0);
    let inv_dx2 = 1.0 / (dx * dx);

    let heat = Problem::new(dim, move |_t, y, dy| {
        for i in 0..dim {
            let left = if i == 0 { 0.0 } else { y[i - 1] };
            let right = if i + 1 == dim { 0.0 } else { y[i + 1] };
            dy[i] = (left - 2.0 * y[i] + right) * inv_dx2;
        }
        Ok(())
    })
    .jvp(move |_t, _y, v, out| {
        for i in 0..dim {
            let left = if i == 0 { 0.0 } else { v[i - 1] };
            let right = if i + 1 == dim { 0.0 } else { v[i + 1] };
            out[i] = (left - 2.0 * v[i] + right) * inv_dx2;
        }
        Ok(())
    })
    .autonomous();

    let y0 = vec![1.0; dim];
    let heat_opts = VigilodeOptions::default()
        .strategy(SolverStrategy::HighPerformance)
        .max_krylov_dim(15);

    let heat_sol = VigilODE::solve(&heat, (0.0, 0.05), &y0, heat_opts)?;
    println!("Status: {:?}", heat_sol.status);
    println!("Route taken: {:?}", heat_sol.route_history);
    println!(
        "Work counters: JVP vectors={}, Linear solves={}\n",
        heat_sol.stats().counters.jvp_vectors,
        heat_sol.stats().counters.linear_solves
    );

    // -------------------------------------------------------------------------
    // Problem 4: Fail-Closed Protected Fallback Demonstration
    // -------------------------------------------------------------------------
    println!("--- 4. Fail-Closed Fallback Demonstration ---");
    // System with stiff kinetics but without explicit Jacobian:
    // Starts on explicit DOPRI5, encounters stiffness rejection, and smoothly
    // rolls back to Protected RODAS5P!
    let fallback_prob = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });

    let fb_opts = VigilodeOptions::default()
        .strategy(SolverStrategy::Auto)
        .max_steps(1000);

    let fb_sol = VigilODE::solve(&fallback_prob, (0.0, 10.0), &[1.0, 0.0, 0.0], fb_opts)?;
    println!("Status: {:?}", fb_sol.status);
    println!("Success: {}", fb_sol.success());
    println!("Route transitions: {:?}", fb_sol.route_history);
    println!(
        "Fallback steps recorded: {}",
        fb_sol.stats().counters.fallback_steps
    );
    println!("\nAll unified production solver demonstrations completed successfully!");

    Ok(())
}
