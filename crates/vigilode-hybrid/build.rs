use std::{env, path::PathBuf, process::Command};
fn main() {
    if env::var_os("CARGO_FEATURE_EXTERNAL").is_none() {
        return;
    }
    let src = PathBuf::from(
        env::var("HYBRID_VODE_SOURCE").expect("HYBRID_VODE_SOURCE must name pinned Fortran inputs"),
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut objects = Vec::new();
    for name in [
        "vode.f",
        "dgefa.f",
        "dgesl.f",
        "dgbfa.f",
        "dgbsl.f",
        "vode_bridge.f90",
    ] {
        let p = src.join(name);
        println!("cargo:rerun-if-changed={}", p.display());
        let obj = out.join(format!("{name}.o"));
        assert!(
            Command::new("gfortran")
                .args([
                    "-c",
                    "-O3",
                    "-fPIC",
                    "-ffp-contract=off",
                    "-fallow-argument-mismatch"
                ])
                .arg(&p)
                .arg("-o")
                .arg(&obj)
                .status()
                .unwrap()
                .success()
        );
        objects.push(obj);
    }
    let archive = out.join("libhybridvode.a");
    assert!(
        Command::new("ar")
            .arg("crs")
            .arg(&archive)
            .args(objects)
            .status()
            .unwrap()
            .success()
    );
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=hybridvode");
    println!("cargo:rustc-link-lib=gfortran");
    println!("cargo:rustc-link-lib=blas");
    println!("cargo:rerun-if-env-changed=HYBRID_VODE_SOURCE");
}
