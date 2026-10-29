use std::hint::black_box;
use std::time::Instant;
use vigilode_sg01::{Frozen, SparseFrozen, certify};
fn main() {
    println!("n,block,mode,ns_per_call,nnz");
    for n in [16_usize, 64, 128] {
        let mut j = vec![0.0; n * n];
        for i in 0..n {
            j[i * n + i] = -(0.2 + 9.0 * i as f64 / n as f64);
        }
        let b = vec![1.0; n];
        let w = vec![1.0; n];
        let x = vec![0.5; n];
        let p = Frozen {
            n,
            j: &j,
            sigma: 0.25,
            rhs: &b,
            w: &w,
            epoch: 7,
        };
        let c = SparseFrozen::new(&p).unwrap();
        for _ in 0..100 {
            black_box(certify(&p, &x, 1e6, 1e6).unwrap());
            black_box(c.certify(&x, 1e6, 1e6).unwrap());
            black_box(SparseFrozen::new(&p).unwrap());
        }
        for block in 0..7 {
            let order = if block % 2 == 0 {
                ["dense", "sparse", "sparse", "dense"]
            } else {
                ["sparse", "dense", "dense", "sparse"]
            };
            for mode in order {
                let iterations = if mode == "construct" { 100 } else { 200 };
                let begin = Instant::now();
                for _ in 0..iterations {
                    match mode {
                        "dense" => {
                            black_box(certify(&p, &x, 1e6, 1e6).unwrap());
                        }
                        "sparse" => {
                            black_box(c.certify(&x, 1e6, 1e6).unwrap());
                        }
                        _ => (),
                    }
                }
                println!(
                    "{n},{block},{mode},{:.3},{}",
                    begin.elapsed().as_nanos() as f64 / iterations as f64,
                    c.nnz()
                );
            }
            let begin = Instant::now();
            for _ in 0..200 {
                black_box(SparseFrozen::new(&p).unwrap());
            }
            println!(
                "{n},{block},construct,{:.3},{}",
                begin.elapsed().as_nanos() as f64 / 200.0,
                c.nnz()
            );
        }
    }
}
