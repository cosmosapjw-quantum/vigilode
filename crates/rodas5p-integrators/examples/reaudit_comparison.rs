//! Fixed-input source benchmark. All admission is endpoint/stage scoped.
#[path = "reaudit_comparison_support/mod.rs"]
mod support;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--list"] {
        println!("{}", support::manifest());
        return;
    }
    let parsed = support::parse_args(&args);
    let (suite, case, arm, threads) = match parsed {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let pool = match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("thread pool: {e}");
            std::process::exit(2);
        }
    };
    let row = pool.install(|| support::run(&suite, &case, &arm, threads));
    println!("{}", row);
}
