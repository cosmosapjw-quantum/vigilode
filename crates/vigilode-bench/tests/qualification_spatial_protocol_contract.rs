//! Work-precision benchmark protocol v2 qualification contracts (V3-BENCH-PROTOCOL).
//!
//! Validates:
//! 1. `adversarial_spatial_grid_v2.json` syntax and semantic validity via `Corpus::validate()`.
//! 2. Inclusion of `vigilode-dense` and `vigilode-banded-dense` as primary endpoint-only arms.
//! 3. Preservation of `vigilode` and `vigilode-banded` as separate continuity controls.
//! 4. Endpoint-only arms require `output_clipped_steps == 0`.
//! 5. Clear separation between dense Jacobian representation and dense continuous output.

use vigilode_bench::arms::Arm;
use vigilode_bench::corpus::Corpus;

const PROTOCOL_V2_PATH: &str = "../../bench/protocols/adversarial_spatial_grid_v2.json";
const PROTOCOL_V3_PATH: &str = "../../bench/protocols/adversarial_spatial_battery_v3.json";

#[test]
fn test_spatial_battery_protocol_v3_syntax_and_semantic_validation() {
    let content = std::fs::read_to_string(PROTOCOL_V3_PATH)
        .or_else(|_| std::fs::read_to_string("bench/protocols/adversarial_spatial_battery_v3.json"))
        .expect("adversarial_spatial_battery_v3.json must exist");

    let corpus = Corpus::from_json(&content).expect("Corpus JSON parsing must succeed");
    corpus
        .validate()
        .expect("Corpus semantic validation must pass");

    assert_eq!(corpus.id, "adversarial-spatial-battery-v3");
    assert_eq!(corpus.problems.len(), 9);
}

#[test]
fn test_spatial_grid_protocol_v2_syntax_and_semantic_validation() {
    let content = std::fs::read_to_string(PROTOCOL_V2_PATH)
        .or_else(|_| std::fs::read_to_string("bench/protocols/adversarial_spatial_grid_v2.json"))
        .expect("adversarial_spatial_grid_v2.json must exist");

    let corpus = Corpus::from_json(&content).expect("Corpus JSON parsing must succeed");
    corpus
        .validate()
        .expect("Corpus semantic validation must pass");

    assert_eq!(corpus.id, "adversarial-spatial-grid-v2");
    assert_eq!(corpus.grid_points, 101);

    // Primary endpoint-only arms must be present
    assert!(corpus.arms.contains(&Arm::VigilodeDense));
    assert!(corpus.arms.contains(&Arm::VigilodeBandedDense));

    // Continuity control arms must be present
    assert!(corpus.arms.contains(&Arm::Vigilode));
    assert!(corpus.arms.contains(&Arm::VigilodeBanded));

    // External comparators must be present
    assert!(corpus.arms.contains(&Arm::DiffsolBdfDense));
    assert!(corpus.arms.contains(&Arm::CvodeBdfDense));
    assert!(corpus.arms.contains(&Arm::Radau5));
}

#[test]
fn test_endpoint_only_vs_grid_clipped_arm_classifications() {
    assert!(Arm::VigilodeDense.is_facade_dense());
    assert!(Arm::VigilodeBandedDense.is_facade_dense());
    assert!(!Arm::VigilodeDense.is_grid_clipped());
    assert!(!Arm::VigilodeBandedDense.is_grid_clipped());

    assert!(Arm::Vigilode.is_grid_clipped());
    assert!(Arm::VigilodeBanded.is_grid_clipped());
    assert!(!Arm::Vigilode.is_facade_dense());
    assert!(!Arm::VigilodeBanded.is_facade_dense());
}

#[test]
fn test_dense_jacobian_vs_dense_output_distinction() {
    // Dense Jacobian is an algebraic matrix representation of df/dy (all n*n entries).
    // Dense output is continuous-time polynomial interpolation over accepted step slabs.
    // An arm can have:
    // - Dense Jacobian + Grid-clipped steps: `Arm::Vigilode`
    // - Banded Jacobian + Grid-clipped steps: `Arm::VigilodeBanded`
    // - Dense Jacobian + Dense continuous output: `Arm::VigilodeDense`
    // - Banded Jacobian + Dense continuous output: `Arm::VigilodeBandedDense`
    assert_eq!(Arm::Vigilode.id(), "vigilode");
    assert_eq!(Arm::VigilodeBanded.id(), "vigilode-banded");
    assert_eq!(Arm::VigilodeDense.id(), "vigilode-dense");
    assert_eq!(Arm::VigilodeBandedDense.id(), "vigilode-banded-dense");

    assert_eq!(Arm::Vigilode.implementation_family(), "vigilode-rodas5p");
    assert_eq!(
        Arm::VigilodeDense.implementation_family(),
        "vigilode-rodas5p"
    );
}

#[test]
fn test_endpoint_only_arms_take_zero_clipped_steps() {
    use vigilode_bench::arms::{self, RunStatus};
    use vigilode_bench::model::{Kind, Model};

    let model = Model::new(Kind::LinearExact, 2).unwrap();
    let grid: Vec<f64> = (0..=10).map(|i| i as f64 * 0.1).collect();
    let rtol = 1e-6;
    let atol = 1e-8;

    let res_dense = arms::run(Arm::VigilodeDense, &model, &grid, rtol, atol);
    assert_eq!(res_dense.status, RunStatus::Success);
    assert_eq!(
        res_dense.native["output_clipped_steps"], 0,
        "vigilode-dense must take exactly 0 clipped steps"
    );

    let res_banded_dense = arms::run(Arm::VigilodeBandedDense, &model, &grid, rtol, atol);
    assert_eq!(res_banded_dense.status, RunStatus::Success);
    assert_eq!(
        res_banded_dense.native["output_clipped_steps"], 0,
        "vigilode-banded-dense must take exactly 0 clipped steps"
    );
}
