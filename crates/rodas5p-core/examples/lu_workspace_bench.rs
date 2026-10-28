//! Same factorization, RHS and kernels; compare allocating and reusable-buffer
//! solves. Input construction and JSON serialization are outside the timer.
use rodas5p_core::{DenseMatrix, LuFactorization, LuSolveWorkspace};
use serde_json::json;
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 3 {
        return Err("usage: lu_workspace_bench legacy|workspace N REPS".into());
    }
    let n: usize = a[1].parse()?;
    let reps: usize = a[2].parse()?;
    if ![16, 32, 65, 96, 192, 384].contains(&n) || reps == 0 || reps > 10_000_000 {
        return Err("outside bounded experiment".into());
    }
    faer::set_global_parallelism(faer::Par::Seq);
    let mut mat = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            mat[(i, j)] = if i == j {
                4.0 + i as f64 / n as f64
            } else {
                0.02 * ((i * 7 + j * 11) % 17) as f64 / n as f64
            };
        }
    }
    let expected: Vec<_> = (0..n).map(|i| (i as f64 + 0.3).sin()).collect();
    let mut b = vec![0.0; n];
    mat.matvec_into(&expected, &mut b)?;
    let lu = LuFactorization::new(&mat)?;
    let mut ws = LuSolveWorkspace::new(n);
    let reference = lu.solve(&b)?;
    let mut output = b.clone();
    lu.solve_in_place_with_workspace(&mut output, &mut ws)?;
    let bits_equal = reference
        .iter()
        .zip(&output)
        .all(|(x, y)| x.to_bits() == y.to_bits());
    let error = output
        .iter()
        .zip(&expected)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    if error > 2e-13 {
        return Err("solve accuracy failure".into());
    }
    let mut checksum = 0.0;
    let start = Instant::now();
    match a[0].as_str() {
        "legacy" => {
            for _ in 0..reps {
                let x = lu.solve(black_box(&b))?;
                checksum += black_box(x[0]);
            }
        }
        "workspace" => {
            for _ in 0..reps {
                output.copy_from_slice(black_box(&b));
                lu.solve_in_place_with_workspace(&mut output, &mut ws)?;
                checksum += black_box(output[0]);
            }
        }
        _ => return Err("unknown arm".into()),
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{}",
        json!({"arm":a[0],"n":n,"reps":reps,"seconds":elapsed,
        "ns_per_solve":elapsed*1e9/reps as f64,"max_error":error,
        "legacy_bitwise_equal":bits_equal,"checksum":checksum,
        "scope":"single-rhs solve; factorization/setup excluded"})
    );
    Ok(())
}
