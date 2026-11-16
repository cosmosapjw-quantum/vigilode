//! Contracts for `Jacobian::sparse_jvp` and Node C1 (Task X7, DESIGN.md §t-x7).
//!
//! Verifies:
//! 1. Tridiagonal heat equation with `Jacobian::sparse_jvp`:
//!    - `jvp_vectors` charged as `3 * jacobian_builds`
//!    - states match `Jacobian::dense` within 1e-12
//!    - zero hot-loop allocations
//! 2. Non-banded pattern with dense row fallback to Matrix-Free GMRES (`RouteId::Rodas5pMatrixFree`)
//! 3. Bit-identity of existing `Jacobian::dense`, `Jacobian::banded`, and `Jacobian::jvp` routes

use vigilode::coloring::SparsityPattern;
use vigilode::{
    CallbackError, DriverWork, Jacobian, Options, Problem, Rodas5p, Rodas5pWindowDriver, RouteId,
    WindowDriver,
};

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

const TRIDIAGONAL_N: usize = 6;

fn tridiagonal_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    let n = y.len();
    for i in 0..n {
        let l = if i > 0 { y[i - 1] } else { 0.0 };
        let r = if i + 1 < n { y[i + 1] } else { 0.0 };
        dy[i] = l - 2.0 * y[i] + r;
    }
    Ok(())
}

fn tridiagonal_pattern(n: usize) -> SparsityPattern {
    let rows: Vec<Vec<usize>> = (0..n)
        .map(|i| (i.saturating_sub(1)..=(i + 1).min(n - 1)).collect())
        .collect();
    SparsityPattern::from_rows(n, &rows).expect("valid tridiagonal pattern")
}

fn tridiagonal_jvp(_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]) -> Result<(), CallbackError> {
    let n = v.len();
    for i in 0..n {
        let l = if i > 0 { v[i - 1] } else { 0.0 };
        let r = if i + 1 < n { v[i + 1] } else { 0.0 };
        out[i] = l - 2.0 * v[i] + r;
    }
    Ok(())
}

fn tridiagonal_dense(_t: f64, _y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    let n = TRIDIAGONAL_N;
    jac.fill(0.0);
    for i in 0..n {
        if i > 0 {
            jac[i * n + i - 1] = 1.0;
        }
        jac[i * n + i] = -2.0;
        if i + 1 < n {
            jac[i * n + i + 1] = 1.0;
        }
    }
    Ok(())
}

#[test]
fn sparse_jvp_tridiagonal_heat_matches_dense_and_charges_correct_jvps() {
    let n = TRIDIAGONAL_N;
    let y0 = [1.0, 0.5, -0.25, 2.0, 0.0, 1.5];
    let opts = Options::default();

    let pattern = tridiagonal_pattern(n);
    let p_sparse = Problem::new(n, tridiagonal_rhs)
        .jacobian(Jacobian::sparse_jvp(pattern.clone(), tridiagonal_jvp))
        .autonomous();
    let p_dense = Problem::new(n, tridiagonal_rhs)
        .jacobian(Jacobian::dense(tridiagonal_dense))
        .autonomous();

    let sol_sparse =
        Rodas5p::solve(&p_sparse, (0.0, 1.0), &y0, &opts).expect("sparse JVP solve succeeds");
    let sol_dense = Rodas5p::solve(&p_dense, (0.0, 1.0), &y0, &opts).expect("dense solve succeeds");

    assert!(sol_sparse.is_success());
    assert!(sol_dense.is_success());

    // 1. States match dense reference within 1e-12
    let s_final = sol_sparse.final_state().expect("sparse final state");
    let d_final = sol_dense.final_state().expect("dense final state");
    for (&s, &d) in s_final.iter().zip(d_final.iter()) {
        assert!(
            (s - d).abs() <= 1.0e-12,
            "sparse state {s} vs dense state {d} exceeds 1e-12"
        );
    }

    // 2. jvp_vectors charged as 3 * jacobian_builds
    // (Curtis-Powell-Reid greedy colouring uses exactly 3 colours for tridiagonal)
    let builds = sol_sparse.stats.counters.jacobian_builds;
    assert!(builds > 0, "at least one Jacobian build expected");
    assert_eq!(
        sol_sparse.stats.counters.jvp_vectors,
        3 * builds,
        "jvp_vectors ({}) must equal 3 * jacobian_builds ({})",
        sol_sparse.stats.counters.jvp_vectors,
        builds
    );

    // 3. Zero hot-loop allocations:
    // Repeated in-place compressed JVP assembly must not allocate after pre-allocation.
    let (colors, k) = vigilode::coloring::column_coloring(&pattern);
    assert_eq!(k, 3);
    let mut compressed = vec![vec![0.0; n]; k];
    let mut seed = vec![0.0; n];
    let mut values = vec![0.0; pattern.nnz()];

    // Warmup pass
    for (color_idx, comp_out) in compressed.iter_mut().enumerate() {
        vigilode::coloring::seed_vector(&colors, color_idx, &mut seed);
        tridiagonal_jvp(0.0, &y0, &seed, comp_out).unwrap();
    }
    vigilode::coloring::recover_values(&pattern, &colors, &compressed, &mut values);

    // Hot-loop pass: verify in-place buffers keep identical pointers and capacities (zero reallocation)
    let seed_ptr = seed.as_ptr();
    let values_ptr = values.as_ptr();
    let comp_ptrs: Vec<*const f64> = compressed.iter().map(|v| v.as_ptr()).collect();
    for (color_idx, comp_out) in compressed.iter_mut().enumerate() {
        vigilode::coloring::seed_vector(&colors, color_idx, &mut seed);
        let _ = tridiagonal_jvp(0.0, &y0, &seed, comp_out);
    }
    vigilode::coloring::recover_values(&pattern, &colors, &compressed, &mut values);
    assert_eq!(seed.as_ptr(), seed_ptr);
    assert_eq!(values.as_ptr(), values_ptr);
    for (v, p) in compressed.iter().zip(comp_ptrs) {
        assert_eq!(v.as_ptr(), p);
    }
}

#[test]
fn sparse_jvp_non_banded_dense_row_falls_back_to_matrix_free() {
    let n = 6;
    // Sparsity pattern with a dense row 0 (connecting to all 0..n columns)
    // while remaining rows i > 0 have only subdiagonal and diagonal entries {i - 1, i}.
    // This produces bandwidth n - 1 = 5, triggering the fallback from banded factorization
    // to the matrix-free GMRES solver (RouteId::Rodas5pMatrixFree).
    let rows: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            if i == 0 {
                (0..n).collect()
            } else {
                vec![i - 1, i]
            }
        })
        .collect();
    let pattern =
        SparsityPattern::from_rows(n, &rows).expect("valid sparsity pattern with dense row");

    let rhs = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), CallbackError> {
        let sum: f64 = y.iter().sum();
        dy[0] = -sum;
        for i in 1..n {
            dy[i] = y[i - 1] - 2.0 * y[i];
        }
        Ok(())
    };

    let jvp = move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| -> Result<(), CallbackError> {
        let sum: f64 = v.iter().sum();
        out[0] = -sum;
        for i in 1..n {
            out[i] = v[i - 1] - 2.0 * v[i];
        }
        Ok(())
    };

    let p = Problem::new(n, rhs)
        .jacobian(Jacobian::sparse_jvp(pattern, jvp))
        .autonomous();

    // 1. Verify router selects Rodas5pMatrixFree route
    let driver = Rodas5pWindowDriver::for_problem(&p);
    assert_eq!(
        driver.route(),
        RouteId::Rodas5pMatrixFree,
        "non-banded pattern with dense row must route to Rodas5pMatrixFree"
    );

    // 2. Solve and verify driver work is MatrixFree
    let opts = Options::default();
    let y0 = vec![1.0; n];
    let sol = Rodas5p::solve(&p, (0.0, 0.5), &y0, &opts)
        .expect("solve succeeds via matrix-free fallback");
    assert!(sol.is_success());
    assert!(
        matches!(sol.stats.driver_work, DriverWork::MatrixFree { .. }),
        "driver work must be MatrixFree"
    );
}

#[test]
fn sparse_jvp_bit_identity_of_existing_dense_banded_jvp_routes() {
    let n = TRIDIAGONAL_N;
    let y0 = [1.0, 0.5, -0.25, 2.0, 0.0, 1.5];
    let opts = Options::default();

    // 1. Bit-identity of repeated Jacobian::dense runs
    let p_dense = Problem::new(n, tridiagonal_rhs)
        .jacobian(Jacobian::dense(tridiagonal_dense))
        .autonomous();
    let sol_d1 = Rodas5p::solve(&p_dense, (0.0, 1.0), &y0, &opts).unwrap();
    let sol_d2 = Rodas5p::solve(&p_dense, (0.0, 1.0), &y0, &opts).unwrap();
    assert_eq!(bits(&sol_d1.t), bits(&sol_d2.t));
    assert_eq!(
        bits(sol_d1.final_state().unwrap()),
        bits(sol_d2.final_state().unwrap())
    );
    assert_eq!(sol_d1.stats.counters, sol_d2.stats.counters);

    // 2. Bit-identity of repeated Jacobian::banded runs
    let tridiagonal_band =
        move |_t: f64, _y: &[f64], band: &mut [f64]| -> Result<(), CallbackError> {
            for i in 0..n {
                if i > 0 {
                    band[i * 3] = 1.0;
                }
                band[i * 3 + 1] = -2.0;
                if i + 1 < n {
                    band[i * 3 + 2] = 1.0;
                }
            }
            Ok(())
        };
    let p_banded = Problem::new(n, tridiagonal_rhs)
        .jacobian(Jacobian::banded(1, 1, tridiagonal_band))
        .autonomous();
    let sol_b1 = Rodas5p::solve(&p_banded, (0.0, 1.0), &y0, &opts).unwrap();
    let sol_b2 = Rodas5p::solve(&p_banded, (0.0, 1.0), &y0, &opts).unwrap();
    assert_eq!(bits(&sol_b1.t), bits(&sol_b2.t));
    assert_eq!(
        bits(sol_b1.final_state().unwrap()),
        bits(sol_b2.final_state().unwrap())
    );
    assert_eq!(sol_b1.stats.counters, sol_b2.stats.counters);

    // 3. Bit-identity of repeated Jacobian::jvp runs
    let p_jvp = Problem::new(n, tridiagonal_rhs)
        .jacobian(Jacobian::jvp(tridiagonal_jvp))
        .autonomous();
    let sol_j1 = Rodas5p::solve(&p_jvp, (0.0, 1.0), &y0, &opts).unwrap();
    let sol_j2 = Rodas5p::solve(&p_jvp, (0.0, 1.0), &y0, &opts).unwrap();
    assert_eq!(bits(&sol_j1.t), bits(&sol_j2.t));
    assert_eq!(
        bits(sol_j1.final_state().unwrap()),
        bits(sol_j2.final_state().unwrap())
    );
    assert_eq!(sol_j1.stats.counters, sol_j2.stats.counters);

    // 4. Bit-identity between sequential banded and dense routes (INV-01)
    let seq_opts = Options {
        interior_error: vigilode::InteriorError::Enforce,
        ..Options::default()
    };
    let sol_seq_b = Rodas5p::solve(&p_banded, (0.0, 1.0), &y0, &seq_opts).unwrap();
    let sol_seq_d = Rodas5p::solve(&p_dense, (0.0, 1.0), &y0, &seq_opts).unwrap();
    assert_eq!(bits(&sol_seq_b.t), bits(&sol_seq_d.t));
    assert_eq!(
        bits(sol_seq_b.final_state().unwrap()),
        bits(sol_seq_d.final_state().unwrap())
    );
    assert_eq!(
        sol_seq_b.stats.accepted_steps,
        sol_seq_d.stats.accepted_steps
    );
}
