fn main() {
    let cargo_profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    let profile_directory = std::env::var_os("OUT_DIR")
        .as_deref()
        .and_then(|path| std::path::Path::new(path).ancestors().nth(3))
        .and_then(std::path::Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    println!("cargo:rustc-env=VIGILODE_CARGO_PROFILE={cargo_profile}");
    println!("cargo:rustc-env=VIGILODE_CARGO_PROFILE_DIR={profile_directory}");
    println!("cargo:rerun-if-changed=build.rs");
}
