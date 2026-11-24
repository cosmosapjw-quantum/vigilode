use std::{env, path::PathBuf, process::Command};

fn run(mut cmd: Command) {
    let status = cmd.status().expect("native SUNDIALS build tool");
    assert!(status.success(), "{cmd:?} exited with {status}");
}

fn main() {
    let bridge = "../../../../crates/vigilode-bench/native/bridge/cvode_bridge.c";
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let object = out.join("cvode.o");
    let mut cc = Command::new("cc");
    cc.args(["-O3", "-std=c11", "-c", bridge, "-o"])
        .arg(&object);
    run(cc);
    let mut ar = Command::new("ar");
    ar.arg("crs").arg(out.join("libu11cvode.a")).arg(object);
    run(ar);
    println!("cargo:rerun-if-changed={bridge}");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=u11cvode");
    for name in [
        "cvode",
        "nvecserial",
        "sunmatrixdense",
        "sunmatrixband",
        "sunlinsoldense",
        "sunlinsolband",
    ] {
        println!("cargo:rustc-link-lib=sundials_{name}");
    }
}
