//! Interleaved serial/parallel timing for the public matrix-free diffusion callback.
//! Run with `cargo run -p vigilode --release --example spatial_rows -- 4`.
use rayon::ThreadPoolBuilder;
use std::{error::Error, hint::black_box, time::Instant};
use vigilode::spatial::{DiffusionGrid, SpatialRows};

fn main() -> Result<(), Box<dyn Error>> {
    let workers: usize = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "4".into())
        .parse()?;
    if workers == 0 {
        return Err("provide a positive thread count".into());
    }
    let pool = ThreadPoolBuilder::new().num_threads(workers).build()?;
    let cases = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([16, 16], [1.0, 1.0], [0.1; 2])?,
        ),
        (
            "box2_large",
            DiffusionGrid::periodic_box([512, 512], [1.0, 1.0], [0.1; 2])?,
        ),
        (
            "box3_large",
            DiffusionGrid::periodic_box([64; 3], [1.0; 3], [0.1; 3])?,
        ),
        ("sphere_small", DiffusionGrid::sphere(12, 24, 1.3, 0.15)?),
        ("sphere_large", DiffusionGrid::sphere(256, 512, 1.3, 0.15)?),
    ];
    for (name, grid) in cases {
        pool.install(|| -> Result<(), vigilode::CallbackError> {
            let u: Vec<_> = (0..grid.dimension()).map(|i| (0.73*i as f64).sin()).collect();
            let mut out = vec![0.0; grid.dimension()];
            let mut par = out.clone();
            grid.apply(&u,&mut out,SpatialRows::Serial)?;
            grid.apply(&u,&mut par,SpatialRows::Parallel)?;
            assert!(out.iter().zip(&par).all(|(a,b)| a.to_bits()==b.to_bits()));
            // Identical workloads per arm, balanced warm/cold order across paired samples.
            let reps = if grid.dimension() < 1024 { 128 } else { 12 };
            let mut serial_ns = Vec::new();
            let mut parallel_ns = Vec::new();
            for sample in 0..10 {
                for arm in [sample%2,(sample+1)%2] {
                    let mode = [SpatialRows::Serial, SpatialRows::Parallel][arm];
                    let start = Instant::now();
                    for _ in 0..reps { grid.apply(black_box(&u), black_box(&mut out), mode)?; }
                    let ns = start.elapsed().as_nanos() as f64 / reps as f64;
                    if arm == 0 {serial_ns.push(ns)} else {parallel_ns.push(ns)}
                }
            }
            println!("{}", serde_json::json!({"geometry":name,"cells":grid.dimension(),"workers":workers,
                          "repetitions_per_sample":reps,"serial_ns":serial_ns,"parallel_ns":parallel_ns,
                          "quantity":"single_stencil_application"}));
            Ok(())
        })?;
    }
    Ok(())
}
