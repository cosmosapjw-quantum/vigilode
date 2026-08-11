use std::fs;
use std::path::{Path, PathBuf};

use rodas5p_core::sha256_hex;

const BDF_SHA256: &str = "68c681b58686e706ab6890d3be68bb1ca4a9398517e0e755a5c64b2cef746a04";
const RADAU_SHA256: &str = "edec0453a5338fc8357a7c026aff894869641b486a3aa3df858353a315bb441b";

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
fn bdf_and_radau_remain_byte_frozen_comparators() {
    assert_eq!(sha256_hex(include_bytes!("../src/bdf.rs")), BDF_SHA256);
    assert_eq!(sha256_hex(include_bytes!("../src/radau.rs")), RADAU_SHA256);
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
