use std::cell::Cell;
use vigilode_sg01::{Failure, Frozen, Options, Work, certify, solve_into};

fn cfg() -> Options {
    Options {
        max_columns: 12,
        restart: 5,
        projected_trigger: 1e-9,
        original_budget: 1e-9,
        stage_budget: 1e-9,
        every_prefix: false,
    }
}

#[test]
fn exact_negative_lognorm_inverse_bound() {
    let p = Frozen {
        n: 1,
        j: &[-2.0],
        sigma: 0.25,
        rhs: &[1.0],
        w: &[1.0],
        epoch: 1,
    };
    let c = certify(&p, &[2.0 / 3.0], 1e-12, 1e-12).unwrap().unwrap();
    assert!(c.mu_upper >= -2.0 && c.inverse_upper >= 2.0 / 3.0);
    assert!(c.rho_upper >= 0.0 && c.stage_error_upper < 1e-12);
}
#[test]
fn original_residual_rejects_projected_false_positive() {
    let p = Frozen {
        n: 1,
        j: &[0.0],
        sigma: 0.0,
        rhs: &[1.0],
        w: &[1.0],
        epoch: 0,
    };
    assert!(certify(&p, &[0.0], 1e-9, 1e-9).unwrap().is_none());
}
#[test]
fn source_coefficient_matrix_not_assumed_exact_rounded_a() {
    let p = Frozen {
        n: 1,
        j: &[-2.0],
        sigma: 0.25,
        rhs: &[1.0],
        w: &[1.0],
        epoch: 0,
    };
    assert!(certify(&p, &[2.0 / 3.0], 1e-12, 1e-12).unwrap().is_some());
}
#[test]
fn changing_preconditioner_safe_with_stored_z() {
    let p = Frozen {
        n: 2,
        j: &[0.0; 4],
        sigma: 0.0,
        rhs: &[1.0, 2.0],
        w: &[1.0, 1.0],
        epoch: 4,
    };
    let mut out = [99.0, 99.0];
    let mut work = Work::default();
    let mut calls = 0;
    let c = solve_into(
        &p,
        &[0.0, 0.0],
        &mut out,
        cfg(),
        |v, z| {
            calls += 1;
            let factor = if calls == 1 { 1.0 } else { 4.0 };
            for (o, x) in z.iter_mut().zip(v) {
                *o = x * factor;
            }
            Ok(())
        },
        || 4,
        &mut work,
    )
    .unwrap();
    assert!((out[0] - 1.0).abs() < 1e-12 && (out[1] - 2.0).abs() < 1e-12);
    assert!(c.rho_upper < 1e-12 && work.pc_apps == 1 && work.accepted == 1);
    // The old reconstruction applied a second P to a combined V*y,
    // which could silently produce 4*b instead of b here.
}
#[test]
fn bad_pc_return_does_not_commit() {
    let p = Frozen {
        n: 2,
        j: &[0.0; 4],
        sigma: 0.0,
        rhs: &[1.0, 2.0],
        w: &[1.0, 1.0],
        epoch: 4,
    };
    let mut out = [99.0, 99.0];
    let mut work = Work::default();
    let r = solve_into(
        &p,
        &[0.0, 0.0],
        &mut out,
        cfg(),
        |_, z| {
            z.fill(f64::NAN);
            Ok(())
        },
        || 4,
        &mut work,
    );
    assert!(matches!(r, Err(Failure::Nonfinite)) && out == [99.0, 99.0]);
    assert_eq!(work.outcome, Some(Failure::Nonfinite));
}
#[test]
fn stale_epoch_does_not_commit() {
    let p = Frozen {
        n: 2,
        j: &[0.0; 4],
        sigma: 0.0,
        rhs: &[1.0, 2.0],
        w: &[1.0, 1.0],
        epoch: 4,
    };
    let mut out = [99.0, 99.0];
    let mut work = Work::default();
    let epoch = Cell::new(4);
    let r = solve_into(
        &p,
        &[0.0, 0.0],
        &mut out,
        cfg(),
        |v, z| {
            z.copy_from_slice(v);
            epoch.set(5);
            Ok(())
        },
        || epoch.get(),
        &mut work,
    );
    assert!(matches!(r, Err(Failure::SourceChanged)) && out == [99.0, 99.0]);
}
#[test]
fn zero_pc_rejects_breakdown_and_preserves_output() {
    let p = Frozen {
        n: 2,
        j: &[0.0; 4],
        sigma: 0.0,
        rhs: &[1.0, 2.0],
        w: &[1.0, 1.0],
        epoch: 4,
    };
    let mut out = [99.0, 99.0];
    let mut work = Work::default();
    let r = solve_into(
        &p,
        &[0.0, 0.0],
        &mut out,
        cfg(),
        |_, z| {
            z.fill(0.0);
            Ok(())
        },
        || 4,
        &mut work,
    );
    assert!(matches!(r, Err(Failure::Breakdown)) && out == [99.0, 99.0]);
}
#[test]
fn unacceptable_budget_does_not_commit() {
    let p = Frozen {
        n: 2,
        j: &[-2.0, 0.0, 0.0, -3.0],
        sigma: 0.25,
        rhs: &[1.0, 2.0],
        w: &[1.0, 1.0],
        epoch: 4,
    };
    let mut out = [99.0, 99.0];
    let mut work = Work::default();
    let mut zcfg = cfg();
    zcfg.original_budget = 0.0;
    zcfg.stage_budget = 0.0;
    let r = solve_into(
        &p,
        &[0.0, 0.0],
        &mut out,
        zcfg,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 4,
        &mut work,
    );
    assert!(r.is_err() && out == [99.0, 99.0]);
}
#[test]
fn nonpositive_resolvent_margin_declines() {
    let p = Frozen {
        n: 1,
        j: &[5.0],
        sigma: 0.25,
        rhs: &[1.0],
        w: &[1.0],
        epoch: 0,
    };
    assert!(certify(&p, &[1.0], 1e10, 1e10).unwrap().is_none());
}
#[test]
fn bad_weights_refused() {
    let p = Frozen {
        n: 1,
        j: &[-1.0],
        sigma: 0.25,
        rhs: &[1.0],
        w: &[0.0],
        epoch: 0,
    };
    assert_eq!(certify(&p, &[1.0], 1.0, 1.0).unwrap_err(), Failure::Input);
}
#[test]
fn two_cycle_restart_stiff_diagonal() {
    let j = [-0.5, 0.0, 0.0, 0.0, -1.5, 0.0, 0.0, 0.0, -3.0];
    let p = Frozen {
        n: 3,
        j: &j,
        sigma: 0.5,
        rhs: &[1.0, 2.0, -3.0],
        w: &[1.0; 3],
        epoch: 8,
    };
    let mut out = [0.0; 3];
    let mut work = Work::default();
    let mut o = cfg();
    o.restart = 2;
    o.max_columns = 20;
    o.original_budget = 1e-8;
    o.stage_budget = 1e-8;
    let c = solve_into(
        &p,
        &[0.0; 3],
        &mut out,
        o,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 8,
        &mut work,
    )
    .unwrap();
    assert!(c.stage_error_upper < 1e-8);
    for i in 0..3 {
        assert!((out[i] - p.rhs[i] / (1.0 - 0.5 * j[4 * i])).abs() < 1e-8);
    }
    assert!(work.itinerary.ends_with('C'));
}
#[test]
fn first_benchmark_control_same_output() {
    let n = 16;
    let mut j = vec![0.0; n * n];
    for i in 0..n {
        j[i * n + i] = -(0.2 + i as f64 * 0.3);
    }
    let rhs = vec![1.0; n];
    let w = vec![1.0; n];
    let p = Frozen {
        n,
        j: &j,
        sigma: 0.2,
        rhs: &rhs,
        w: &w,
        epoch: 1,
    };
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut wa = Work::default();
    let mut wb = Work::default();
    let mut o = cfg();
    o.original_budget = 1e-8;
    o.stage_budget = 1e-8;
    o.max_columns = 16;
    o.restart = 16;
    solve_into(
        &p,
        &vec![0.0; n],
        &mut a,
        o,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 1,
        &mut wa,
    )
    .unwrap();
    o.every_prefix = true;
    solve_into(
        &p,
        &vec![0.0; n],
        &mut b,
        o,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 1,
        &mut wb,
    )
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(wa.columns, wb.columns);
    assert!(wa.candidate_builds < wb.candidate_builds);
}

#[test]
fn structural_zero_sparse_cache_same_represented_solution() {
    use vigilode_sg01::{SparseFrozen, solve_into_sparse};
    let n = 16;
    let mut j = vec![0.0; n * n];
    for i in 0..n {
        j[i * n + i] = -(0.2 + i as f64 * 0.3);
    }
    let rhs = vec![1.0; n];
    let w = vec![1.0; n];
    let p = Frozen {
        n,
        j: &j,
        sigma: 0.2,
        rhs: &rhs,
        w: &w,
        epoch: 1,
    };
    let sparse = SparseFrozen::new(&p).unwrap();
    assert_eq!(sparse.nnz(), n);
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut wa = Work::default();
    let mut wb = Work::default();
    let mut o = cfg();
    o.original_budget = 1e-8;
    o.stage_budget = 1e-8;
    o.max_columns = 16;
    o.restart = 16;
    let ca = solve_into(
        &p,
        &vec![0.0; n],
        &mut a,
        o,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 1,
        &mut wa,
    )
    .unwrap();
    let cb = solve_into_sparse(
        &sparse,
        &vec![0.0; n],
        &mut b,
        o,
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 1,
        &mut wb,
    )
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(wa.columns, wb.columns);
    assert!(ca.stage_error_upper >= 0.0 && cb.stage_error_upper >= 0.0);
    assert!(sparse.certify(&b, 1e-8, 1e-8).unwrap().is_some());
}

#[test]
fn sparse_cache_exact_nonzero_offdiagonal() {
    use vigilode_sg01::SparseFrozen;
    let j = [-2.0, 0.2, 0.0, -1.0];
    let b = [1.0, 2.0];
    let w = [1.0, 2.0];
    let p = Frozen {
        n: 2,
        j: &j,
        sigma: 0.1,
        rhs: &b,
        w: &w,
        epoch: 1,
    };
    let sparse = SparseFrozen::new(&p).unwrap();
    assert_eq!(sparse.nnz(), 3);
    let candidate = [0.9, 1.7];
    let dense = certify(&p, &candidate, 1e3, 1e3).unwrap().unwrap();
    let smart = sparse.certify(&candidate, 1e3, 1e3).unwrap().unwrap();
    assert!(dense.rho_upper >= 0.0 && smart.rho_upper >= 0.0);
    assert!(dense.mu_upper >= smart.mu_upper - 1e-15);
}

#[test]
fn symbolic_route_regular_language_good_and_bad_words() {
    use vigilode_sg01::valid_itinerary;
    for word in [
        "KQC",
        "KKQRKQC",
        "KQX",
        "KQ R KQC".replace(' ', "").as_str(),
        "X",
    ] {
        assert!(valid_itinerary(word), "{word}");
    }
    for word in ["", "QC", "KRC", "KQCC", "KQRQC", "KXR", "IKQC", "KQ"] {
        assert!(!valid_itinerary(word), "{word}");
    }
}
#[test]
fn solver_emits_legal_accepted_itinerary() {
    use vigilode_sg01::valid_itinerary;
    let p = Frozen {
        n: 1,
        j: &[-2.0],
        sigma: 0.25,
        rhs: &[1.0],
        w: &[1.0],
        epoch: 2,
    };
    let mut out = [7.0];
    let mut work = Work::default();
    solve_into(
        &p,
        &[0.0],
        &mut out,
        cfg(),
        |v, z| {
            z.copy_from_slice(v);
            Ok(())
        },
        || 2,
        &mut work,
    )
    .unwrap();
    assert!(valid_itinerary(&work.itinerary));
}
