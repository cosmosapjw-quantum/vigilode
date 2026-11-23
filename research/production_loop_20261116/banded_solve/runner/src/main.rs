//! Measurement program of loop L1 node V19, stage a (see `../../PREDECLARED.json`).
//!
//! ```text
//! runner check <S1|S2|S3|S4|S5>                 one CHECK line per case and arm
//! runner bench <arm> <n> <patterns> <solves> [lu]  one BENCH line (lu given: S2-type matrix)
//! ```

mod kernel;
// The problem definitions of node V04, used unchanged by the `drive` mode.
#[allow(dead_code)]
#[path = "../../../cost_anatomy/runner/src/problems.rs"]
mod problems;

use std::f64::consts::PI;
use std::hint::black_box;
use std::sync::Arc;

use kernel::{Band, KernelError, Partition, Seq, SeqMode, layout};
use rodas5p_core::WorkCounters;
use rodas5p_integrators::stage_clients::BandedTightStageLu;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, BandedStageLu, OdeProblem, OutputSchedule,
    StageLinearSystem, integrate_rodas5p_fast_banded_observed_with_kernel,
    integrate_rodas5p_fast_client_observed_with_atol, rodas5p_fast_banded_solve,
};

/// SplitMix64, as declared in PREDECLARED.json.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [-1, 1).
    fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0) * 2.0 - 1.0
    }

    fn vector(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.uniform()).collect()
    }
}

/// The Jacobian band of the V04 Brusselator (cost_anatomy/runner/src/problems.rs), species
/// interleaved, lower = upper = 2.
fn bruss_band(cells: usize, state: u8) -> Band {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let mut jac = vec![0.0; n * 5];
    for i in 0..cells {
        let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
        let (u, v) = if state == 1 {
            (1.0 + (2.0 * PI * x).sin(), 3.0)
        } else {
            (2.0 + (6.0 * PI * x).cos(), 1.0 + 0.5 * (10.0 * PI * x).sin())
        };
        let (a, b) = (2 * i, 2 * i + 1);
        let (da, db) = (a * 5 + 2, b * 5 + 2);
        jac[da] = 2.0 * u * v - 4.0 - 2.0 * c;
        jac[da + 1] = u * u;
        jac[db - 1] = 3.0 - 2.0 * u * v;
        jac[db] = -u * u - 2.0 * c;
        if i > 0 {
            jac[da - 2] = c;
            jac[db - 2] = c;
        }
        if i + 1 < cells {
            jac[da + 2] = c;
            jac[db + 2] = c;
        }
    }
    Band {
        n,
        l: 2,
        u: 2,
        jac,
    }
}

/// A random band `W` (returned as `J = -W`, to be used with `inv = 0`).
fn random_band(n: usize, lu: usize, rng: &mut SplitMix, dominant: bool) -> Band {
    let jw = 2 * lu + 1;
    let mut w = vec![0.0; n * jw];
    for i in 0..n {
        let mut off = 0.0;
        for d in 0..jw {
            if d == lu || i + d < lu || i + d - lu >= n {
                continue;
            }
            let v = rng.uniform();
            w[i * jw + d] = v;
            off += v.abs();
        }
        w[i * jw + lu] = if dominant {
            let sign = if rng.uniform() < 0.0 { -1.0 } else { 1.0 };
            sign * (off + 1.0 + rng.uniform().abs())
        } else {
            rng.uniform()
        };
    }
    Band {
        n,
        l: lu,
        u: lu,
        jac: w.iter().map(|x| -x).collect(),
    }
}

/// Suites S4 and S5: the last row of every block of the `np`-partition gets zeros on the columns
/// of its block (`diag` on the diagonal) and 2 on the first column of the separator after it.
fn break_blocks(band: &mut Band, np: usize, diag: f64) {
    let (n, l, u) = (band.n, band.l, band.u);
    let jw = band.jw();
    let (_, _, off, _, sep) = layout(n, l, u, np).expect("layout of a suite matrix");
    for q in 0..np - 1 {
        let r = sep[q] - 1;
        for d in 0..jw {
            if r + d < l {
                continue;
            }
            let c = r + d - l;
            if c >= off[q] && c < sep[q] {
                band.jac[r * jw + d] = 0.0;
            }
        }
        band.jac[r * jw + l] = -diag;
        band.jac[r * jw + l + 1] = -2.0;
    }
}

struct Case {
    id: String,
    band: Band,
    inv: f64,
    b: Vec<f64>,
    x_true: Option<Vec<f64>>,
}

fn with_true_solution(id: String, band: Band, x_true: Vec<f64>) -> Case {
    let mut b = vec![0.0; band.n];
    band.matvec(0.0, &x_true, &mut b);
    Case {
        id,
        band,
        inv: 0.0,
        b,
        x_true: Some(x_true),
    }
}

fn suite(name: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    match name {
        "S1" => {
            let mut idx = 0u64;
            for n in [1000usize, 10_000, 100_000] {
                for state in [1u8, 2] {
                    for inv in [1e6, 1e4, 1e2, 1.0, 1e-2, 1e-4, 1e-6] {
                        let band = bruss_band(n / 2, state);
                        let b = SplitMix(1001 + idx).vector(n);
                        cases.push(Case {
                            id: format!("n{n}_s{state}_inv{inv:e}"),
                            band,
                            inv,
                            b,
                            x_true: None,
                        });
                        idx += 1;
                    }
                }
            }
        }
        "S2" => {
            for (li, lu) in [1usize, 2, 4, 8].into_iter().enumerate() {
                for (ni, n) in [1000usize, 10_007].into_iter().enumerate() {
                    for k in 0..5u64 {
                        let seed = 2000 + 100 * li as u64 + 10 * ni as u64 + k;
                        let mut rng = SplitMix(seed);
                        let band = random_band(n, lu, &mut rng, true);
                        let x = rng.vector(n);
                        cases.push(with_true_solution(format!("lu{lu}_n{n}_k{k}"), band, x));
                    }
                }
            }
        }
        "S3" => {
            for (li, lu) in [1usize, 2, 4].into_iter().enumerate() {
                for k in 0..20u64 {
                    let mut rng = SplitMix(3000 + 100 * li as u64 + k);
                    let band = random_band(1000, lu, &mut rng, false);
                    let x = rng.vector(1000);
                    cases.push(with_true_solution(format!("lu{lu}_k{k}"), band, x));
                }
            }
        }
        "S4" | "S5" => {
            let diag = if name == "S4" { 0.0 } else { 1e-10 };
            for (pi, np) in [2usize, 4, 8].into_iter().enumerate() {
                for (li, lu) in [1usize, 2].into_iter().enumerate() {
                    let mut rng = SplitMix(4000 + 10 * pi as u64 + li as u64);
                    let mut band = random_band(1000, lu, &mut rng, true);
                    let x = rng.vector(1000);
                    break_blocks(&mut band, np, diag);
                    cases.push(with_true_solution(format!("P{np}_lu{lu}"), band, x));
                }
            }
        }
        other => panic!("unknown suite {other}"),
    }
    cases
}

/// Arm A0: the protected client through the trait.
struct ProtectedArm {
    client: BandedStageLu,
    counters: WorkCounters,
}

impl ProtectedArm {
    fn new(band: &Band) -> Result<Self, KernelError> {
        let protected = |e: rodas5p_core::CoreError| KernelError::Protected(e.to_string());
        let n = band.n;
        let jac = Arc::new(band.jac.clone());
        let fill = Arc::new(move |_t: f64, _y: &[f64], out: &mut [f64]| {
            out.copy_from_slice(&jac);
            Ok(())
        });
        let mut client = BandedStageLu::new(
            n,
            &BandedJacobian {
                lower: band.l,
                upper: band.u,
                fill,
            },
        )
        .map_err(protected)?;
        let problem = OdeProblem::new(
            "v19",
            n,
            Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| Ok(())),
            None,
            None,
            // The constructor asks for a Jacobian or a JVP; the banded client never calls it.
            Some(Arc::new(
                |_t: f64, _y: &[f64], _w: &[f64], out: &mut [f64]| {
                    out.fill(0.0);
                    Ok(())
                },
            )),
            None,
            true,
            None,
            None,
        )
        .map_err(protected)?;
        let mut counters = WorkCounters::default();
        let y = vec![0.0; n];
        client
            .linearize(&problem, 0.0, &y, &mut counters)
            .map_err(protected)?;
        Ok(Self { client, counters })
    }
}

enum Imp {
    Protected(Box<ProtectedArm>),
    Seq(Seq),
    Part(Partition),
}

struct Arm {
    imp: Imp,
    verify: bool,
    b0: Vec<f64>,
    wnorm: f64,
}

impl Arm {
    fn new(name: &str, band: &Band) -> Result<Self, KernelError> {
        let (n, l, u) = (band.n, band.l, band.u);
        let (imp, verify) = match name {
            "A0" => (Imp::Protected(Box::new(ProtectedArm::new(band)?)), false),
            "A1" => (Imp::Seq(Seq::new(n, l, u, SeqMode::Ref)), false),
            "B" => (Imp::Seq(Seq::new(n, l, u, SeqMode::Fast)), false),
            "BV" => (Imp::Seq(Seq::new(n, l, u, SeqMode::Fast)), true),
            "BD" => (Imp::Seq(Seq::new(n, l, u, SeqMode::FastDiv)), false),
            "TA" | "TD" | "TS" => {
                if l != 2 || u != 2 || n < 6 {
                    return Err(KernelError::Layout);
                }
                let mode = match name {
                    "TA" => SeqMode::TightRef,
                    "TD" => SeqMode::TightDiv,
                    _ => SeqMode::TightScaled,
                };
                (Imp::Seq(Seq::new(n, l, u, mode)), false)
            }
            "B2" => (Imp::Seq(Seq::new(n, l, u, SeqMode::Scaled)), false),
            "B3" => {
                if l != 2 || n < 3 {
                    return Err(KernelError::Layout);
                }
                (Imp::Seq(Seq::new(n, l, u, SeqMode::ScaledCarry2)), false)
            }
            other => {
                let (verify, digits) = match other.strip_prefix("CV") {
                    Some(d) => (true, d),
                    None => (false, other.strip_prefix('C').expect("unknown arm")),
                };
                let np: usize = digits.parse().expect("number of blocks");
                (Imp::Part(Partition::new(n, l, u, np)?), verify)
            }
        };
        Ok(Self {
            imp,
            verify,
            b0: if verify { vec![0.0; n] } else { Vec::new() },
            wnorm: 0.0,
        })
    }

    fn factor(&mut self, band: &Band, inv: f64) -> Result<(), KernelError> {
        match &mut self.imp {
            Imp::Protected(p) => p
                .client
                .factor(inv, &mut p.counters)
                .map_err(|e| KernelError::Protected(e.to_string()))?,
            Imp::Seq(s) => s.factor(band, inv)?,
            Imp::Part(p) => p.factor(band, inv)?,
        }
        if self.verify {
            self.wnorm = band.norm_inf(inv);
        }
        Ok(())
    }

    fn solve(&mut self, band: &Band, inv: f64, b: &mut [f64]) -> Result<(), KernelError> {
        if self.verify {
            self.b0.copy_from_slice(b);
        }
        match &mut self.imp {
            Imp::Protected(p) => p
                .client
                .solve_in_place(b, &mut p.counters)
                .map_err(|e| KernelError::Protected(e.to_string()))?,
            Imp::Seq(s) => s.solve(b),
            Imp::Part(p) => p.solve(b),
        }
        if self.verify {
            let (r, xn, bn) = band.residual_norms(inv, b, &self.b0);
            let den = self.wnorm * xn + bn;
            if !(r <= 1e-13 * den) {
                return Err(KernelError::Residual { eta: r / den });
            }
        }
        Ok(())
    }
}

fn num(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else if v.is_infinite() {
        "inf".into()
    } else {
        format!("{v:.3e}")
    }
}

fn max_rel_diff(x: &[f64], y: &[f64]) -> f64 {
    let scale = y.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut d = 0.0f64;
    for (a, b) in x.iter().zip(y) {
        let e = (a - b).abs();
        if !(e <= d) {
            d = e;
        }
    }
    d / scale
}

const CHECK_ARMS: [&str; 18] = [
    "A0", "A1", "B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8", "B2", "B3", "C16", "CV16", "BD",
    "TA", "TD", "TS",
];

fn check(name: &str) {
    for case in suite(name) {
        let (band, inv) = (&case.band, case.inv);
        let (n, lu) = (band.n, band.l);
        let reference = rodas5p_fast_banded_solve(n, band.l, band.u, &band.jac, inv, &case.b)
            .ok()
            .map(|(x, _)| x);
        let wnorm = band.norm_inf(inv);
        for arm_name in CHECK_ARMS {
            let mut x = case.b.clone();
            let outcome = Arm::new(arm_name, band).and_then(|mut arm| {
                arm.factor(band, inv)?;
                arm.solve(band, inv, &mut x)
            });
            let head = format!(
                "CHECK suite={name} case={} n={n} lu={lu} arm={arm_name}",
                case.id
            );
            match outcome {
                Err(e) => {
                    let kind = match e {
                        KernelError::Layout => "layout".to_string(),
                        KernelError::BlockPivot { .. } => "block_pivot".to_string(),
                        KernelError::ReducedPivot => "reduced_pivot".to_string(),
                        KernelError::Residual { eta } => format!("residual:{}", num(eta)),
                        KernelError::Protected(msg) => {
                            format!("protected:{}", msg.replace(' ', "_"))
                        }
                    };
                    println!(
                        "{head} status=err kind={kind} eta=NA ferr=NA diff_a0=NA bits_a0=NA ref_ok={}",
                        u8::from(reference.is_some())
                    );
                }
                Ok(()) => {
                    let finite = x.iter().all(|v| v.is_finite());
                    let eta = if finite {
                        let (r, xn, bn) = band.residual_norms(inv, &x, &case.b);
                        r / (wnorm * xn + bn)
                    } else {
                        f64::INFINITY
                    };
                    let ferr = case
                        .x_true
                        .as_ref()
                        .map_or("NA".to_string(), |t| num(max_rel_diff(&x, t)));
                    let (diff, bits) = match &reference {
                        Some(r) => (
                            num(max_rel_diff(&x, r)),
                            u8::from(x.iter().zip(r).all(|(a, b)| a.to_bits() == b.to_bits()))
                                .to_string(),
                        ),
                        None => ("NA".to_string(), "NA".to_string()),
                    };
                    println!(
                        "{head} status=ok kind=NA eta={} ferr={ferr} diff_a0={diff} bits_a0={bits} ref_ok={}",
                        num(eta),
                        u8::from(reference.is_some())
                    );
                }
            }
        }
    }
}

fn bench(arm_name: &str, n: usize, patterns: usize, solves: usize, lu: Option<usize>) {
    let (band, inv) = match lu {
        None => (bruss_band(n / 2, 1), 100.0),
        Some(lu) => (
            random_band(n, lu, &mut SplitMix(5000 + lu as u64), true),
            0.0,
        ),
    };
    let mut rng = SplitMix(77);
    let rhs: Vec<Vec<f64>> = (0..8).map(|_| rng.vector(n)).collect();
    let mut work = vec![0.0; n];
    let mut arm = Arm::new(arm_name, &band).expect("arm");
    let (mut checksum, mut errors) = (0u64, 0u64);
    for _ in 0..patterns {
        let arm = black_box(&mut arm);
        if arm.factor(&band, black_box(inv)).is_err() {
            errors += 1;
            continue;
        }
        for s in 0..solves {
            work.copy_from_slice(&rhs[s % 8]);
            if arm.solve(&band, inv, &mut work).is_err() {
                errors += 1;
            }
            checksum ^= work[0].to_bits().rotate_left(31)
                ^ work[n / 2].to_bits()
                ^ work[n - 1].to_bits().rotate_left(17);
        }
    }
    black_box(&arm);
    println!(
        "BENCH arm={arm_name} n={n} lu={} patterns={patterns} solves={solves} checksum={checksum:016x} errors={errors}",
        band.l
    );
}

/// Exploratory: one whole run of the protected banded driver on a V04 problem with one of the
/// protected kernels (all of them documented as bit-identical), stopped after `budget` attempts.
fn drive(family: &str, size: usize, kernel_name: &str, budget: usize) {
    let def = match family {
        "bruss1d" => problems::bruss1d(size),
        "ac2d" => problems::allen_cahn(2, size),
        "ac3d" => problems::allen_cahn(3, size),
        "bruss2d" => problems::bruss2d(size),
        other => panic!("unknown family {other}"),
    };
    let kernel = match kernel_name {
        "indexed" => Some(BandedKernel::Indexed),
        "slices" => Some(BandedKernel::Slices),
        "fixed2" => Some(BandedKernel::FixedWidth2),
        // Node V21 stage b: the client driver with the opt-in client BandedTightStageLu.
        "tight" => None,
        other => panic!("unknown kernel {other}"),
    };
    let band = def.band.as_ref().expect("banded problem");
    let banded = BandedJacobian {
        lower: band.lower,
        upper: band.upper,
        fill: band.fill.clone(),
    };
    let problem = OdeProblem::new(
        "v19-drive",
        def.n,
        def.rhs.clone(),
        None,
        None,
        def.jvp.clone(),
        None,
        true,
        None,
        None,
    )
    .expect("problem");
    let adaptive = AdaptiveStepConfig {
        atol: def.atol,
        rtol: def.rtol,
        initial_step: def.initial_step,
        min_step: 1.0e-14,
        max_step: def.t_span.1 - def.t_span.0,
        max_attempts: budget,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![def.t_span.0, def.t_span.1]).expect("schedule");
    let (fast, partial, factor_ops, solve_ops) = match kernel {
        Some(kernel) => {
            let out = integrate_rodas5p_fast_banded_observed_with_kernel(
                &problem, &banded, def.t_span, &def.y0, &adaptive, &output, kernel,
            )
            .expect("driver");
            (
                out.fast,
                out.partial,
                out.work.factor_operations,
                out.work.solve_operations,
            )
        }
        None => {
            let client = Box::new(BandedTightStageLu::new(def.n, &banded).expect("client"));
            let atol = vec![def.atol; def.n];
            let (fast, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
                &problem, client, def.t_span, &def.y0, &adaptive, &output, &atol,
            )
            .expect("client driver");
            // The client does not keep the counted band operations.
            (fast, partial, 0, 0)
        }
    };
    // FNV-1a over the bits of every state the run returned (outputs and the stop state).
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut eat = |v: f64| {
        for byte in v.to_bits().to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for state in &fast.observed.y {
        state.iter().copied().for_each(&mut eat);
    }
    let stop_t = match &partial {
        Some(p) => {
            p.y.iter().copied().for_each(&mut eat);
            p.t
        }
        None => def.t_span.1,
    };
    println!(
        "DRIVE family={family} size={size} n={} lower={} upper={} kernel={kernel_name} budget={budget} attempts={} accepted={} rejected={} stop_t={stop_t:e} factor_ops={} solve_ops={} state_hash={hash:016x}",
        def.n,
        band.lower,
        band.upper,
        fast.attempts,
        fast.accepted_steps,
        fast.rejected_steps,
        factor_ops,
        solve_ops,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("drive") => drive(
            &args[2],
            args[3].parse().expect("size"),
            &args[4],
            args[5].parse().expect("budget"),
        ),
        Some("check") => check(&args[2]),
        Some("bench") => bench(
            &args[2],
            args[3].parse().expect("n"),
            args[4].parse().expect("patterns"),
            args[5].parse().expect("solves"),
            args.get(6).map(|s| s.parse().expect("lu")),
        ),
        _ => {
            eprintln!(
                "usage: runner check <suite> | runner bench <arm> <n> <patterns> <solves> [lu] | runner drive <family> <size> <kernel> <budget>"
            );
            std::process::exit(2);
        }
    }
}
