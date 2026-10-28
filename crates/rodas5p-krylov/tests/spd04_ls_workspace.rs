//! Contract of `LeastSquaresWorkspace::solve_into` (research node
//! `research/spd04_ls_workspace_20261007`): bit for bit
//! `small::least_squares` (exported as `rodas5p_krylov::least_squares`), the
//! same error text, on seeded `(m + 1) x m` upper-Hessenberg systems and on
//! reuse sequences in one workspace.

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_krylov::{LeastSquaresWorkspace, least_squares};

struct Rng(u64);

impl Rng {
    /// SplitMix64, uniform in [-1, 1).
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
    }
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Generic,
    /// A column equal to an earlier one (rank deficient).
    RepeatedColumn,
    /// A zero column (rank deficient).
    ZeroColumn,
    /// The last subdiagonal entry tiny (near happy breakdown).
    NearBreakdown,
    /// The last subdiagonal entry exactly zero (happy breakdown).
    ExactBreakdown,
    /// Arnoldi-like: orthogonal-ish columns with a decaying subdiagonal.
    Decaying,
    /// Entries of magnitude ~1e-150.
    Tiny,
    /// Entries of magnitude ~1e150.
    Huge,
}

const KINDS: [Kind; 8] = [
    Kind::Generic,
    Kind::RepeatedColumn,
    Kind::ZeroColumn,
    Kind::NearBreakdown,
    Kind::ExactBreakdown,
    Kind::Decaying,
    Kind::Tiny,
    Kind::Huge,
];

/// A seeded `(m + 1) x m` upper-Hessenberg matrix and a right-hand side
/// (`[beta, 0, ...]` as GMRES uses it when `gmres_rhs`, else dense).
fn system(m: usize, kind: Kind, seed: u64, gmres_rhs: bool) -> (DenseMatrix, Vec<f64>) {
    let mut rng = Rng(seed);
    let mut a = DenseMatrix::zeros(m + 1, m);
    for j in 0..m {
        for i in 0..=(j + 1) {
            a[(i, j)] = rng.next();
        }
        // Arnoldi subdiagonals are nonnegative norms.
        a[(j + 1, j)] = a[(j + 1, j)].abs() + 0.1;
    }
    match kind {
        Kind::Generic => {}
        Kind::RepeatedColumn if m >= 2 => {
            let (from, to) = (m / 3, m - 1);
            for i in 0..=m {
                a[(i, to)] = if i <= from + 1 { a[(i, from)] } else { 0.0 };
            }
        }
        Kind::ZeroColumn => {
            let column = (seed as usize) % m;
            for i in 0..=m {
                a[(i, column)] = 0.0;
            }
        }
        Kind::NearBreakdown => a[(m, m - 1)] = 1.0e-15 * (1.0 + rng.next().abs()),
        Kind::ExactBreakdown => a[(m, m - 1)] = 0.0,
        Kind::Decaying => {
            for j in 0..m {
                a[(j + 1, j)] *= 0.5_f64.powi(j as i32);
            }
        }
        Kind::Tiny | Kind::Huge => {
            let scale = if matches!(kind, Kind::Tiny) {
                1.0e-150
            } else {
                1.0e150
            };
            for j in 0..m {
                for i in 0..=m {
                    a[(i, j)] *= scale;
                }
            }
        }
        Kind::RepeatedColumn => {}
    }
    let b = if gmres_rhs {
        let mut b = vec![0.0; m + 1];
        b[0] = 1.0 + rng.next().abs();
        b
    } else {
        (0..=m).map(|_| rng.next()).collect()
    };
    (a, b)
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// The reference outcome as solution bits or error text.
fn outcome(result: CoreResult<Vec<f64>>) -> Result<Vec<u64>, String> {
    result.map(|x| bits(&x)).map_err(|e| e.to_string())
}

fn workspace_outcome(
    ws: &mut LeastSquaresWorkspace,
    a: &DenseMatrix,
    b: &[f64],
    out: &mut Vec<f64>,
) -> Result<Vec<u64>, String> {
    ws.solve_into(a, b, out)
        .map(|()| bits(out))
        .map_err(|e| e.to_string())
}

#[test]
fn seeded_hessenberg_systems_match_bit_for_bit() {
    let mut shared = LeastSquaresWorkspace::default();
    let mut shared_out = Vec::new();
    let mut checked = 0usize;
    let mut errors = 0usize;
    // Visit m in an interleaved order so the shared workspace sees both
    // larger and smaller systems than the one before.
    let order: Vec<usize> = (1..=64).map(|k| (k * 37) % 64 + 1).collect();
    let mut seen = order.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 64);
    for &m in &order {
        for (k, kind) in KINDS.iter().enumerate() {
            for gmres_rhs in [true, false] {
                let seed = (m as u64) * 1_000 + (k as u64) * 10 + u64::from(gmres_rhs);
                let (a, b) = system(m, *kind, seed, gmres_rhs);
                let expected = outcome(least_squares(&a, &b));
                errors += usize::from(expected.is_err());
                let mut fresh = LeastSquaresWorkspace::default();
                let mut out = vec![f64::NAN; 3];
                let got_fresh = workspace_outcome(&mut fresh, &a, &b, &mut out);
                let got_shared = workspace_outcome(&mut shared, &a, &b, &mut shared_out);
                assert_eq!(expected, got_fresh, "fresh m={m} kind={kind:?} seed={seed}");
                assert_eq!(
                    expected, got_shared,
                    "shared m={m} kind={kind:?} seed={seed}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 1_000, "{checked} systems");
    // Rank-deficient systems are solved, not refused, by the reference;
    // the count is printed for the record.
    println!("{checked} systems, {errors} reference errors");
}

#[test]
fn reuse_sequences_grow_only_above_the_previous_maximum() {
    // m + 1 in {8, 16, 24, 32, 40}.
    let sequences: [&[usize]; 5] = [
        &[7, 15, 23, 31, 39],
        &[39, 31, 23, 15, 7],
        &[15, 7, 39, 23, 7, 31, 39, 15],
        &[23, 23, 7, 7, 31, 15, 39, 39],
        &[7, 39, 7, 39, 15, 31, 23, 7],
    ];
    for (s, sequence) in sequences.iter().enumerate() {
        let mut ws = LeastSquaresWorkspace::default();
        let mut out = Vec::new();
        let mut maximum = 0usize;
        for (step, &m) in sequence.iter().enumerate() {
            for (k, kind) in KINDS.iter().enumerate() {
                let seed = 7_000_000 + (s as u64) * 10_000 + (step as u64) * 100 + k as u64;
                let (a, b) = system(m, *kind, seed, k % 2 == 0);
                let expected = outcome(least_squares(&a, &b));
                let got = workspace_outcome(&mut ws, &a, &b, &mut out);
                assert_eq!(expected, got, "sequence {s} step {step} m={m} {kind:?}");
                assert_eq!(
                    ws.last_solve_grew(),
                    m > maximum,
                    "growth flag, sequence {s} step {step} m={m}"
                );
                maximum = maximum.max(m);
            }
        }
        assert!(ws.growth_events() <= sequence.len() as u64);
    }
}

#[test]
fn non_finite_input_fails_with_the_same_error() {
    let mut ws = LeastSquaresWorkspace::default();
    let mut out = Vec::new();
    for m in [1usize, 4, 15, 39] {
        for (k, poison) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
            .into_iter()
            .enumerate()
        {
            let (mut a, b) = system(m, Kind::Generic, 11 + k as u64, true);
            a[(m / 2, m / 2)] = poison;
            let expected = outcome(least_squares(&a, &b));
            assert!(expected.is_err(), "m={m} poison={poison}");
            assert_eq!(expected, workspace_outcome(&mut ws, &a, &b, &mut out));
            // A poisoned right-hand side.
            let (a, mut b) = system(m, Kind::Generic, 21 + k as u64, false);
            b[m] = poison;
            let expected = outcome(least_squares(&a, &b));
            assert!(expected.is_err(), "rhs m={m} poison={poison}");
            assert_eq!(expected, workspace_outcome(&mut ws, &a, &b, &mut out));
            // The workspace is still exact after a failure.
            let (a, b) = system(m, Kind::Decaying, 31 + k as u64, true);
            assert_eq!(
                outcome(least_squares(&a, &b)),
                workspace_outcome(&mut ws, &a, &b, &mut out)
            );
        }
    }
}

#[test]
fn shape_errors_and_empty_systems_match() {
    let mut ws = LeastSquaresWorkspace::default();
    let mut out = vec![1.0];
    let a = DenseMatrix::zeros(4, 3);
    assert_eq!(
        outcome(least_squares(&a, &[1.0; 3])),
        workspace_outcome(&mut ws, &a, &[1.0; 3], &mut out)
    );
    let a = DenseMatrix::zeros(1, 0);
    assert_eq!(
        outcome(least_squares(&a, &[2.0])),
        workspace_outcome(&mut ws, &a, &[2.0], &mut out)
    );
    assert!(out.is_empty());
    assert_eq!(ws.solves(), 0);
}

/// GMRES-into with and without the workspace gives the same bits, and the
/// workspace solves once per cycle.
#[test]
fn gmres_into_with_the_workspace_matches_the_default() {
    use rodas5p_core::{DenseOperator, IdentityPreconditioner, WorkCounters};
    use rodas5p_krylov::{GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into};
    let n = 30;
    let mut a = DenseMatrix::zeros(n, n);
    let mut rng = Rng(5);
    for i in 0..n {
        for j in 0..n {
            a[(i, j)] = 0.1 * rng.next();
        }
        a[(i, i)] += 2.0 + 0.1 * i as f64;
    }
    let op = DenseOperator::new(a).unwrap();
    let pc = IdentityPreconditioner::new(n);
    let b: Vec<f64> = (0..n).map(|_| rng.next()).collect();
    let config = GmresConfig {
        restart: 7,
        ..GmresConfig::default()
    };
    let mut results = Vec::new();
    for mut ws in [
        GmresWorkspace::default(),
        GmresWorkspace::with_ls_workspace(),
    ] {
        let mut out = vec![0.0; n];
        let mut counters = WorkCounters::default();
        let report = solve_gmres_into(
            &op,
            &pc,
            &b,
            None,
            &config,
            None,
            &mut out,
            &mut ws,
            GmresCapacity::unbounded(),
            &mut counters,
        )
        .unwrap();
        assert!(report.cycles > 1);
        results.push((bits(&out), report.residual_norm.to_bits(), counters));
        if let Some(ls) = ws.ls_workspace() {
            assert_eq!(ls.solves(), report.least_squares_solves);
            assert_eq!(ls.growth_events(), 1);
        }
    }
    assert_eq!(results[0], results[1]);
}
