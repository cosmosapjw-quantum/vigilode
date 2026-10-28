use std::fs;
use std::path::{Path, PathBuf};

use rodas5p_core::sha256_hex;

const LEGACY_BDF_SHA256: &str = "68c681b58686e706ab6890d3be68bb1ca4a9398517e0e755a5c64b2cef746a04";

fn rust_sources_under(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("scope source directory must be readable") {
        let entry = entry.expect("scope source entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            rust_sources_under(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn bdf_v2_semantic_baseline_explicitly_retires_the_legacy_source_byte_freeze() {
    assert_ne!(
        sha256_hex(include_bytes!("../src/bdf.rs")),
        LEGACY_BDF_SHA256,
        "the v2 predictor-corrector estimator and dense-output baseline must not silently revert to the legacy step-doubling source"
    );
}

#[test]
fn radau3_cell_g_baseline_has_embedded_semantics_instead_of_a_source_byte_claim() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, OutputSchedule, RadauConfig, RadauIiaStages,
        integrate_radau_adaptive_observed, scalar_linear_problem,
    };

    let (problem, y0) = scalar_linear_problem(-5.0, 1.0);
    let result = integrate_radau_adaptive_observed(
        &problem,
        (0.0, 0.1),
        &y0,
        &RadauConfig {
            stages: RadauIiaStages::Three,
            ..Default::default()
        },
        &AdaptiveStepConfig {
            atol: 1.0,
            rtol: 0.0,
            initial_step: 0.1,
            max_step: 0.1,
            ..Default::default()
        },
        &OutputSchedule::new(vec![0.0, 0.1]).unwrap(),
    )
    .unwrap();

    assert!(result.observed.success);
    assert_eq!(result.diagnostics.estimator_orders, vec![4]);
    assert_eq!(
        result.diagnostics.estimator_ids,
        vec!["radau-iia3-scipy-1.17.0-embedded-order3"]
    );
    assert_eq!(result.observed.counters.accepted_steps, 1);
    assert_eq!(result.observed.internal_steps, 1);
}

#[test]
fn generic_solver_core_does_not_embed_physical_client_bindings() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("integrator crate must be inside workspace");
    let crates = workspace.join("crates");
    let mut sources = Vec::new();
    for crate_entry in fs::read_dir(&crates).expect("workspace crates directory must exist") {
        let crate_path = crate_entry.expect("crate entry").path();
        let src = crate_path.join("src");
        if src.is_dir() {
            rust_sources_under(&src, &mut sources);
        }
    }

    let forbidden = [
        "rec_bianchi",
        "rei_bianchi",
        "RABBIT_v1.0.5_PRODUCTION",
        "AcceptedRadiationParent",
        "CoupledCollisionTransportProblem",
        "FLEX_JF_TX",
    ];
    for source in sources {
        let text = fs::read_to_string(&source).expect("Rust source must be UTF-8");
        for token in forbidden {
            assert!(
                !text.contains(token),
                "generic production source {} embeds client token {token}",
                source.display()
            );
        }
    }
}

#[test]
fn required_vectorized_jf_modules_exist_in_the_generic_base() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for name in [
        "stage_batch.rs",
        "common_w_gate.rs",
        "path_controller.rs",
        "homotopy.rs",
        "sequential.rs",
        "adaptive.rs",
    ] {
        assert!(src.join(name).is_file(), "missing generic module: {name}");
    }
}
