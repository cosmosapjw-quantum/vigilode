use std::{env,path::PathBuf,process::Command};
fn execute(mut c:Command){let shown=format!("{c:?}");assert!(c.status().expect(&shown).success(),"failed {shown}");}
fn main(){
    let node=PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out=PathBuf::from(env::var("OUT_DIR").unwrap());
    let source=PathBuf::from(env::var("STRICT_VODE_SOURCE").expect("set STRICT_VODE_SOURCE to frozen Netlib files"));
    let mut objects=Vec::new();
    for name in ["vode","dgefa","dgesl","dgbfa","dgbsl"]{
        let src=source.join(format!("{name}.f"));let obj=out.join(format!("{name}.o"));
        let mut cmd=Command::new("gfortran");cmd.args(["-O3","-fPIC","-std=legacy","-fallow-argument-mismatch","-c"]).arg(&src).arg("-o").arg(&obj);execute(cmd);objects.push(obj);
        println!("cargo:rerun-if-changed={}",src.display());
    }
    let wrapper=node.join("src/vode_bridge.f90");let obj=out.join("vode_bridge.o");
    let mut cmd=Command::new("gfortran");cmd.current_dir(&out).args(["-O3","-fPIC","-ffree-line-length-none","-c"]).arg(&wrapper).arg("-o").arg(&obj);execute(cmd);objects.push(obj);
    let mut ar=Command::new("ar");ar.arg("crs").arg(out.join("libstrictdvode.a")).args(objects);execute(ar);
    println!("cargo:rustc-link-search=native={}",out.display());
    for lib in ["static=strictdvode","blas","gfortran","quadmath"]{println!("cargo:rustc-link-lib={lib}");}
    let sha=Command::new("git").args(["rev-parse","HEAD"]).current_dir(&node).output().unwrap();assert!(sha.status.success());
    println!("cargo:rustc-env=STRICT_SOURCE_COMMIT={}",String::from_utf8(sha.stdout).unwrap().trim());
    println!("cargo:rerun-if-changed=src/vode_bridge.f90");println!("cargo:rerun-if-env-changed=STRICT_VODE_SOURCE");
}
