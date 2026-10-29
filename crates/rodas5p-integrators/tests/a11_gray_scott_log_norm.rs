use rodas5p_core::directed::Interval;
use rodas5p_integrators::gray_scott::gray_scott_2d_with_log_norm;

fn box_interval(lo: f64, hi: f64) -> Interval {
    Interval::new(lo, hi).unwrap()
}

fn reaction_mu(u: f64, v: f64) -> f64 {
    let a = -v * v - 0.04;
    let d = 2.0 * u * v - (0.04 + 0.06);
    let b_plus_c = v * v - 2.0 * u * v;
    0.5 * (a + d) + (0.25 * (a - d).powi(2) + 0.25 * b_plus_c.powi(2)).sqrt()
}

#[test]
fn generic_max_diagonal_is_not_a_logarithmic_norm_bound() {
    // Symmetric part of J is [[-1, 50], [50, -1]], whose largest eigenvalue
    // is 49. The max diagonal shortcut would return -1 and is invalid here.
    let max_diagonal = -1.0_f64;
    let actual_mu2 = -1.0 + 50.0;
    assert_eq!(actual_mu2, 49.0);
    assert!(actual_mu2 > max_diagonal);
}

#[test]
fn gray_scott_reaction_block_bounds_enclose_tube_values() {
    let (_, _, certificate) = gray_scott_2d_with_log_norm(2).unwrap();
    let u_tube = [
        box_interval(0.1, 0.3),
        box_interval(0.2, 0.4),
        box_interval(0.4, 0.6),
        box_interval(0.6, 0.8),
    ];
    let v_tube = [
        box_interval(0.05, 0.15),
        box_interval(0.1, 0.2),
        box_interval(0.15, 0.25),
        box_interval(0.2, 0.3),
    ];
    let upper = certificate
        .verified_log_norm_upper(&u_tube, &v_tube)
        .unwrap();
    for (&u, &v) in u_tube.iter().zip(&v_tube) {
        for u_value in [u.lo, 0.5 * (u.lo + u.hi), u.hi] {
            for v_value in [v.lo, 0.5 * (v.lo + v.hi), v.hi] {
                assert!(upper >= reaction_mu(u_value, v_value));
            }
        }
    }
    assert!(upper.is_finite());
}

#[test]
fn gray_scott_log_norm_refuses_malformed_or_incomplete_tubes() {
    let (_, _, descriptor) = gray_scott_2d_with_log_norm(2).unwrap();
    let u = vec![box_interval(0.0, 1.0); 4];
    let v = vec![box_interval(0.0, 1.0); 4];
    assert!(descriptor.verified_log_norm_upper(&u[..3], &v).is_err());

    let mut malformed = u.clone();
    malformed[2] = Interval {
        lo: 1.0,
        hi: f64::NAN,
    };
    assert!(descriptor.verified_log_norm_upper(&malformed, &v).is_err());

    malformed[2] = Interval { lo: 1.0, hi: 0.0 };
    assert!(descriptor.verified_log_norm_upper(&malformed, &v).is_err());

    let mut overflowing = v.clone();
    overflowing[0] = box_interval(1.0e200, 1.1e200);
    assert!(
        descriptor
            .verified_log_norm_upper(&u, &overflowing)
            .is_err()
    );
}
