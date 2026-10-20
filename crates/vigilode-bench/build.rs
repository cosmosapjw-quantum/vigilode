//! Compiles the pinned Fortran comparators and the CVODE C bridge into one
//! static archive, after verifying every vendored source against
//! `native/SHA256SUMS`.
use std::{env, path::PathBuf, process::Command};

const FFLAGS_F77: &[&str] = &["-O3", "-fPIC", "-std=legacy", "-fallow-argument-mismatch"];
const FFLAGS_F90: &[&str] = &["-O3", "-fPIC"];
const CFLAGS: &[&str] = &["-O3", "-fPIC", "-std=c11", "-Wall"];

fn run(mut c: Command) -> Vec<u8> {
    let shown = format!("{c:?}");
    let out = c
        .output()
        .unwrap_or_else(|e| panic!("cannot run {shown}: {e}"));
    assert!(
        out.status.success(),
        "failed {shown}\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn defined_globals(obj: &PathBuf) -> Vec<String> {
    let mut c = Command::new("nm");
    c.args(["-g", "--defined-only", "-P"]).arg(obj);
    String::from_utf8(run(c))
        .unwrap()
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Text symbols only; COMMON blocks (/CONRA5/, /CONROS/) are distinct.
            matches!(kind, "T").then(|| name.to_string())
        })
        .collect()
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let native = root.join("native");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=native");

    let mut check = Command::new("sha256sum");
    check
        .current_dir(&native)
        .args(["--quiet", "-c", "SHA256SUMS"]);
    run(check);

    let mut objects = Vec::new();
    let compile_f = |src: PathBuf, flags: &[&str], name: &str| {
        let obj = out.join(format!("{name}.o"));
        let mut c = Command::new("gfortran");
        c.current_dir(&out)
            .args(flags)
            .arg("-c")
            .arg(&src)
            .arg("-o")
            .arg(&obj);
        run(c);
        obj
    };
    for name in ["vode", "dgefa", "dgesl", "dgbfa", "dgbsl"] {
        objects.push(compile_f(
            native.join(format!("netlib/{name}.f")),
            FFLAGS_F77,
            name,
        ));
    }
    let radau = compile_f(
        native.join("hairer/radau_decsol.f"),
        FFLAGS_F77,
        "radau_decsol",
    );
    let rodas = compile_f(
        native.join("hairer/rodas_decsol.f"),
        FFLAGS_F77,
        "rodas_decsol",
    );
    // Both Hairer files carry their own copy of DECSOL (DEC, SOL, DECB, ...)
    // and the shared linear-algebra drivers (DECOMR, SLVRAR, ...). Localize
    // the duplicates inside the RODAS object so each solver keeps its own
    // copy, exactly as in Assimulo's separate extension modules.
    let radau_syms = defined_globals(&radau);
    let mut objcopy = Command::new("objcopy");
    let mut localized = Vec::new();
    for s in defined_globals(&rodas) {
        if radau_syms.contains(&s) {
            objcopy.arg(format!("--localize-symbol={s}"));
            localized.push(s);
        }
    }
    objcopy.arg(&rodas);
    run(objcopy);
    objects.push(radau);
    objects.push(rodas);
    objects.push(compile_f(
        native.join("bridge/bench_bridge.f90"),
        FFLAGS_F90,
        "bench_bridge",
    ));

    let cobj = out.join("cvode_bridge.o");
    let mut cc = Command::new("gcc");
    cc.args(CFLAGS)
        .arg("-c")
        .arg(native.join("bridge/cvode_bridge.c"))
        .arg("-o")
        .arg(&cobj);
    run(cc);
    objects.push(cobj);

    let archive = out.join("libbenchnative.a");
    let _ = std::fs::remove_file(&archive);
    let mut ar = Command::new("ar");
    ar.arg("crs").arg(&archive).args(&objects);
    run(ar);

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=benchnative");
    for lib in [
        "sundials_cvode",
        "sundials_nvecserial",
        "sundials_sunmatrixdense",
        "sundials_sunmatrixband",
        "sundials_sunlinsoldense",
        "sundials_sunlinsolband",
        "blas",
        "gfortran",
        "m",
    ] {
        println!("cargo:rustc-link-lib={lib}");
    }
    // The bench binary interposes the C allocator so that heap allocations of
    // the Fortran and C comparators are counted; export the symbols so shared
    // libraries (SUNDIALS, libgfortran) bind to them too.
    for sym in [
        "malloc",
        "calloc",
        "realloc",
        "posix_memalign",
        "aligned_alloc",
        "memalign",
    ] {
        println!("cargo:rustc-link-arg-bins=-Wl,--export-dynamic-symbol={sym}");
    }
    let gfortran = String::from_utf8(run({
        let mut c = Command::new("gfortran");
        c.arg("--version");
        c
    }))
    .unwrap();
    println!(
        "cargo:rustc-env=BENCH_GFORTRAN_VERSION={}",
        gfortran.lines().next().unwrap_or("").trim()
    );
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let rv = String::from_utf8(run({
        let mut c = Command::new(rustc);
        c.arg("-V");
        c
    }))
    .unwrap();
    println!("cargo:rustc-env=BENCH_RUSTC_VERSION={}", rv.trim());
    println!("cargo:rustc-env=BENCH_FFLAGS_F77={}", FFLAGS_F77.join(" "));
    println!("cargo:rustc-env=BENCH_FFLAGS_F90={}", FFLAGS_F90.join(" "));
    println!("cargo:rustc-env=BENCH_CFLAGS={}", CFLAGS.join(" "));
    println!(
        "cargo:rustc-env=BENCH_RODAS_LOCALIZED={}",
        localized.join(",")
    );
}
