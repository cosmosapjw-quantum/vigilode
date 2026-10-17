//! Pure setup and certification instruction profiler for PY05 (callgrind 2-minus-1 Ir).
//!
//! Takes: `<target_component> <repetitions>`

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    leja_action::{
        CertifiedLejaCoeffCache, CertifiedLejaWorkspace, leja_divided_differences,
        leja_divided_differences_certified, leja_phi_action_certified_with_cache_and_workspace,
        leja_points,
    },
    polynomial_action::{
        CoefficientCache, JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator,
        chebyshev_coefficient_enclosures, joint_phi_action,
    },
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

    let mat = DenseMatrix::from_rows(&[&[-20.0]]).unwrap();
    let op = SymmetricNonpositiveOperator::gershgorin(mat).unwrap();
    let w = [vec![1.0], vec![1.0], vec![1.0], vec![1.0], vec![1.0]];
    let input = JointPhiInput::Distinct(&w);
    let h = 1.0;
    let lambda = 20.0;
    let rho = 20.0;

    let mut leja_cache = CertifiedLejaCoeffCache::new();
    leja_cache.precompute_certified(c, gamma, m).unwrap();

    let mut cheb_cache = CoefficientCache::default();
    let mut cheb_work = WorkCounters::default();
    let _ = joint_phi_action(
        &op,
        h,
        input,
        PolynomialBasis::Chebyshev,
        1e-10,
        Some(&mut cheb_cache),
        &mut cheb_work,
    );

    let mut leja_work = WorkCounters::default();
    let mut workspace = CertifiedLejaWorkspace::new(1, 5);

    for _ in 0..repetitions {
        match component.as_str() {
            "leja-cert-setup-cold" => {
                let res = leja_divided_differences_certified(c, gamma, xi);
                std::hint::black_box(res).unwrap();
            }
            "leja-cert-setup-warm" => {
                let res = leja_cache.get_certified(c, gamma, m);
                std::hint::black_box(res).unwrap();
            }
            "leja-cert-certification" => {
                let rep = leja_phi_action_certified_with_cache_and_workspace(
                    &op,
                    h,
                    input,
                    1e-10,
                    m,
                    Some(&mut leja_cache),
                    &mut leja_work,
                    &mut workspace,
                );
                std::hint::black_box(rep).unwrap();
            }
            "cheb-cert-setup-cold" => {
                let res = chebyshev_coefficient_enclosures(h, lambda, rho, m);
                std::hint::black_box(res).unwrap();
            }
            "cheb-cert-setup-warm" => {
                let mut w = WorkCounters::default();
                let rep = joint_phi_action(
                    &op,
                    h,
                    input,
                    PolynomialBasis::Chebyshev,
                    1e-10,
                    Some(&mut cheb_cache),
                    &mut w,
                );
                std::hint::black_box(rep).unwrap();
            }
            "cheb-cert-certification" => {
                let mut w = WorkCounters::default();
                let rep = joint_phi_action(
                    &op,
                    h,
                    input,
                    PolynomialBasis::Chebyshev,
                    1e-10,
                    Some(&mut cheb_cache),
                    &mut w,
                );
                std::hint::black_box(rep).unwrap();
            }
            "leja-est-setup" => {
                let res = leja_divided_differences(c, gamma, xi);
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
