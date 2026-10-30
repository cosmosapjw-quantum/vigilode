use rvj_n4_source_kernel::{
    n3h_source_bounded_copy::n4_compare_original_monomial_kernel, source_moment,
};
#[test]
fn all_240_exact_dyadic_fraction_enclosures() {
    let csv = include_str!("../../evidence/EXACT_DYADIC_MOMENTS.csv");
    let mut passed = 0usize;
    let mut refused = Vec::new();
    let mut max_w = 0.0_f64;
    let mut outside_native = 0usize;
    let mut outside_high_precision = 0usize;
    let mut max_native_absolute_error_envelope = 0.0_f64;
    for (ix, line) in csv.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        assert_eq!(f.len(), 11);
        let x: Vec<f64> = f.iter().map(|a| a.parse().unwrap()).collect();
        let degree = x[4] as usize;
        let actual = source_moment(x[0], x[1], x[2], x[3], degree).unwrap();
        let within = actual.re.lo <= x[5]
            && actual.re.hi >= x[6]
            && actual.im.lo <= x[7]
            && actual.im.hi >= x[8];
        let (raw_re, raw_im) = n4_compare_original_monomial_kernel(x[0], x[1], x[2], x[3], degree);
        assert!(raw_re.is_finite() && raw_im.is_finite());
        if raw_re < x[5] || raw_re > x[6] || raw_im < x[7] || raw_im > x[8] {
            outside_high_precision += 1;
        }
        if raw_re < actual.re.lo
            || raw_re > actual.re.hi
            || raw_im < actual.im.lo
            || raw_im > actual.im.hi
        {
            outside_native += 1;
        }
        let native_enclosure = (raw_re - actual.re.lo)
            .abs()
            .max((raw_re - actual.re.hi).abs())
            .max((raw_im - actual.im.lo).abs())
            .max((raw_im - actual.im.hi).abs());
        max_native_absolute_error_envelope =
            max_native_absolute_error_envelope.max(native_enclosure);
        max_w = max_w
            .max(actual.re.hi - actual.re.lo)
            .max(actual.im.hi - actual.im.lo);
        if !within {
            refused.push((ix, actual, [x[5], x[6], x[7], x[8]]));
        } else {
            passed += 1;
        }
    }
    eprintln!(
        "N4_INTERVAL_FRACTION_CASES={passed}/240 disagreement={} max_width={max_w:e}",
        refused.len()
    );
    for r in refused.iter().take(5) {
        eprintln!("first bad {r:?}");
    }
    eprintln!(
        "N4_N3H_NATIVE_OUTSIDE_HIGH_PRECISION_FRACTION_INTERVAL={outside_high_precision}/240"
    );
    eprintln!(
        "N4_ORIGINAL_N3H_NATIVE_MOMENT_OUTSIDE_CERT_INTERVAL={outside_native}/240; max_absolute_component_error_envelope={max_native_absolute_error_envelope:e}"
    );
    assert!(
        refused.is_empty(),
        "not all intervals contain independent exact-dyadic enclosures"
    );
    assert_eq!(passed, 240);
}
