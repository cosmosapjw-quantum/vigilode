use rodas5p_integrators::affine_transport::{
    AffineErrorMap, AffinePrefixWorkspace, parallel_error_prefix,
};

#[test]
fn ordered_composition_and_empty_prefix() {
    let first = AffineErrorMap::new(2.0, 3.0).unwrap();
    let second = AffineErrorMap::new(5.0, 7.0).unwrap();
    assert_eq!(second.after(first).unwrap().apply(1.0).unwrap(), 32.0);
    assert_eq!(parallel_error_prefix(&[], 0.25).unwrap().radii(), &[0.25]);
}

#[test]
fn exact_fraction_oracle_bounds_and_thread_determinism() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/cloud_completion_20261018/evidence/affine_oracles.json"
    ))
    .unwrap();
    for case in fixtures["cases"].as_array().unwrap() {
        let maps: Vec<_> = case["maps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                AffineErrorMap::new(pair[0].as_f64().unwrap(), pair[1].as_f64().unwrap()).unwrap()
            })
            .collect();
        let e0 = case["initial"].as_f64().unwrap();
        let mut previous = None;
        for threads in [1, 4] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let output = pool.install(|| parallel_error_prefix(&maps, e0)).unwrap();
            for (got, expected) in output
                .radii()
                .iter()
                .zip(case["ceil_exact"].as_array().unwrap())
            {
                assert!(
                    *got >= expected.as_f64().unwrap(),
                    "{}: {got}",
                    case["name"]
                );
                assert!(got.is_finite());
            }
            if let Some(old) = previous {
                assert_eq!(output.radii(), old);
            }
            previous = Some(output.radii().to_vec());
            let n = maps.len();
            let levels = if n <= 1 {
                0
            } else {
                usize::BITS - (n - 1).leading_zeros()
            };
            assert_eq!(output.compositions(), if n == 0 { 0 } else { 3 * n - 2 });
            assert_eq!(output.sweep_levels(), 2 * levels);
            let mut workspace = AffinePrefixWorkspace::new(n).unwrap();
            assert_eq!(workspace.evaluate(&maps, e0).unwrap(), output.radii());
            assert_eq!(workspace.evaluate(&maps, e0).unwrap(), output.radii());
        }
    }
}

#[test]
fn subnormal_and_fail_closed_contracts() {
    let tiny = f64::from_bits(1);
    let map = AffineErrorMap::new(0.5, 0.0).unwrap();
    assert!(parallel_error_prefix(&[map], tiny).unwrap().radii()[1] >= tiny);
    for x in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(AffineErrorMap::new(x, 0.0).is_err());
        assert!(AffineErrorMap::new(1.0, x).is_err());
        assert!(parallel_error_prefix(&[map], x).is_err());
    }
    let grow = AffineErrorMap::new(f64::MAX, 0.0).unwrap();
    assert!(parallel_error_prefix(&[grow, grow], 1.0).is_err());
}
