//! Manufactured bordered band solve; reports diagnostics, not ODE accuracy.
use rodas5p_core::Preconditioner;
use rodas5p_core::bordered::BorderedBandedPreconditioner;
use std::time::Instant;

fn probe(n: usize) -> Result<(), Box<dyn std::error::Error>> {
    if n < 2 {
        return Err("n must be >= 2".into());
    }
    let m = 2;
    let mut a = vec![0.0; 3 * n];
    let mut b = vec![0.0; 2 * n];
    let mut c = vec![0.0; 2 * n];
    let d = [2.0, 0.125, 0.125, 2.0];
    for i in 0..n {
        a[3 * i] = if i > 0 { -0.5 } else { 0.0 };
        a[3 * i + 1] = 3.0;
        a[3 * i + 2] = if i + 1 < n { -0.5 } else { 0.0 };
        b[2 * i] = 0.125;
        b[2 * i + 1] = if i % 2 == 0 { 0.0625 } else { -0.0625 };
        c[i] = b[2 * i] / n as f64;
        c[n + i] = b[2 * i + 1] / n as f64;
    }
    let apply = |x: &[f64], y: &mut [f64]| {
        for i in 0..n {
            y[i] = 3.0 * x[i] + b[2 * i] * x[n] + b[2 * i + 1] * x[n + 1];
            if i > 0 {
                y[i] -= 0.5 * x[i - 1];
            }
            if i + 1 < n {
                y[i] -= 0.5 * x[i + 1];
            }
        }
        for j in 0..2 {
            y[n + j] = d[2 * j] * x[n] + d[2 * j + 1] * x[n + 1];
            for i in 0..n {
                y[n + j] += c[j * n + i] * x[i];
            }
        }
    };
    let exact: Vec<f64> = (0..n + m)
        .map(|i| ((i % 17) as f64 - 8.0) * 0.125)
        .collect();
    let mut rhs = vec![0.0; n + m];
    let mut out = vec![0.0; n + m];
    let mut residual = vec![0.0; n + m];
    apply(&exact, &mut rhs);
    let mut factors = BorderedBandedPreconditioner::new(n, m, 1, 1)?;
    let started = Instant::now();
    factors.refactor(n, m, &a, &b, &c, &d)?;
    let factor_ns = started.elapsed().as_nanos();
    factors.apply(&rhs, &mut out)?;
    let started = Instant::now();
    for _ in 0..100 {
        factors.apply(std::hint::black_box(&rhs), std::hint::black_box(&mut out))?;
    }
    let apply_total_ns = started.elapsed().as_nanos();
    apply(&out, &mut residual);
    let max_error = out
        .iter()
        .zip(&exact)
        .fold(0.0_f64, |e, (x, z)| e.max((x - z).abs()));
    let residual_inf = residual
        .iter()
        .zip(&rhs)
        .fold(0.0_f64, |e, (x, z)| e.max((x - z).abs()));
    println!(
        "{{\"kind\":\"manufactured_bordered_linear_solve\",\"n\":{n},\"m\":{m},\"lower\":1,\"upper\":1,\"allocated_f64_slots\":{},\"factor_ns_diagnostic\":{factor_ns},\"apply_repetitions\":100,\"apply_total_ns_diagnostic\":{apply_total_ns},\"max_solution_error\":{max_error:e},\"residual_inf\":{residual_inf:e}}}",
        factors.allocated_f64_slots()
    );
    if max_error > 1e-11 || residual_inf > 1e-11 {
        return Err("manufactured residual/error check failed".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() {
        for n in [16, 64, 256] {
            probe(n)?;
        }
    } else if args.len() == 2 && args[0] == "--n" {
        probe(args[1].parse()?)?;
    } else {
        return Err("usage: bordered_probe [--n N]".into());
    }
    Ok(())
}
