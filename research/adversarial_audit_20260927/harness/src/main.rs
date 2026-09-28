//! Audit harness entry: prints identity; experiments live in src/bin/*.rs
fn main() {
    println!("vigilode-audit-harness ok; profile_dir={}", option_env!("VIGILODE_CARGO_PROFILE_DIR").unwrap_or("n/a"));
}
