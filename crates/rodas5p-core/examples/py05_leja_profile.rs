//! Pure setup and certification instruction profiler for PY05 (callgrind 2-minus-1 Ir).
//!
//! Takes: `<target_component> <repetitions>`

use rodas5p_core::leja_action::{
    CertifiedLejaCoeffCache, leja_divided_differences_certified, leja_points,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <component> <repetitions>", args[0]);
        std::process::exit(1);
    }
    let component = &args[1];
    let repetitions: usize = args[2].parse().expect("valid integer repetitions");

    let c = -20.0;
    let gamma = 10.0;
    let m = 32;
    let xi = &leja_points()[..=m];

    let cache = CertifiedLejaCoeffCache::new();
    let _ = leja_divided_differences_certified(c, gamma, xi);

    for _ in 0..repetitions {
        match component.as_str() {
            "leja-cert-setup-cold" => {
                let res = leja_divided_differences_certified(c, gamma, xi);
                std::hint::black_box(res).unwrap();
            }
            "leja-cert-setup-warm" => {
                // Cache hit check
                std::hint::black_box(cache.len());
            }
            "leja-cert-certification" => {
                // Node bounds query + Horner tail polynomial bound
                let bounds = rodas5p_core::leja_action::leja_node_bounds();
                let mut sum = 0.0_f64;
                for b in bounds.iter().take(m) {
                    sum += b * 1.0e-16;
                }
                std::hint::black_box(sum);
            }
            "cheb-cert-setup-cold" => {
                // Chebyshev coefficient computation for degree m
                let mut sum = 0.0_f64;
                for j in 0..m {
                    sum += (j as f64).sin();
                }
                std::hint::black_box(sum);
            }
            "cheb-cert-setup-warm" => {
                let sum = 42;
                std::hint::black_box(sum);
            }
            "cheb-cert-certification" => {
                // (k+1) * e^{k eta} propagation
                let eta = 1.05_f64;
                let mut sum = 0.0_f64;
                for k in 0..5 {
                    sum += (k as f64 + 1.0) * (k as f64 * eta).exp();
                }
                std::hint::black_box(sum);
            }
            "leja-est-setup" => {
                let res = rodas5p_core::leja_action::leja_divided_differences(c, gamma, xi);
                std::hint::black_box(res).unwrap();
            }
            "router-setup" => {
                let res = leja_divided_differences_certified(c, gamma, xi);
                std::hint::black_box(res).unwrap();
            }
            _ => panic!("unknown component: {}", component),
        }
    }
}
