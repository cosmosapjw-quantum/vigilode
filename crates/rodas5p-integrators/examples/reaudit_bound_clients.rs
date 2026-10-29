#[path = "reaudit_bound_clients_support/mod.rs"]
mod support;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--list"] {
        println!("{}", support::manifest());
        return;
    }
    let (case, arm, threads) = match support::parse(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2)
        }
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    println!("{}", pool.install(|| support::run(&case, &arm, threads)));
}
