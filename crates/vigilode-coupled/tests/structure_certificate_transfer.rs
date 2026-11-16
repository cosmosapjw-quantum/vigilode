//! U08: typed source-owned structure-certificate boundary (test-level prototype) over the native
//! multiplicity-preserving quotient trace, with an independent original-equation oracle, plus a
//! research prototype of oriented incidence topology (not claimed as a native port).

#![allow(clippy::needless_range_loop)]

use serde_json::json;
use vigilode_coupled::trace_quotient::{
    QuarticInvariant, QuarticParameters, TRACE_TARGET_ID, TraceSemantics, TraceWork,
    quartic_invariant,
};

const ALL: TraceSemantics = TraceSemantics::AllComplexRootsAlgebraicMultiplicity;

fn qual(tag: &str, v: serde_json::Value) {
    println!("QUAL {}", json!({"test": tag, "data": v}));
}

// ------------------------------------------------------------------------------------------
// Independent oracle: roots of the original system q1^2 + q2 = a, q2^2 + q1 = b.
// Eliminating q2 = a - q1^2 gives (a - z^2)^2 + z - b = z^4 - 2a z^2 + z + (a^2 - b), z = q1.
// Every oracle root is checked in BOTH original equations, so the elimination is verified.
// ------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct C {
    re: f64,
    im: f64,
}
impl C {
    fn add(self, o: C) -> C {
        C {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
    fn sub(self, o: C) -> C {
        C {
            re: self.re - o.re,
            im: self.im - o.im,
        }
    }
    fn mul(self, o: C) -> C {
        C {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
    fn div(self, o: C) -> C {
        let d = o.re * o.re + o.im * o.im;
        C {
            re: (self.re * o.re + self.im * o.im) / d,
            im: (self.im * o.re - self.re * o.im) / d,
        }
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn r(x: f64) -> C {
        C { re: x, im: 0.0 }
    }
}

/// Durand-Kerner (simultaneous Weierstrass updates) on the monic quartic; roots and iterations.
fn oracle_roots(a: f64, b: f64) -> ([C; 4], usize) {
    let coef = [a * a - b, 1.0, -2.0 * a, 0.0]; // c0..c3 of z^4 + c3 z^3 + c2 z^2 + c1 z + c0
    let p = |z: C| {
        let mut acc = C::r(1.0);
        for k in (0..4).rev() {
            acc = acc.mul(z).add(C::r(coef[k]));
        }
        acc
    };
    let seed = C { re: 0.4, im: 0.9 };
    let mut z = [C::r(1.0), seed, seed.mul(seed), seed.mul(seed).mul(seed)];
    let mut iters = 0;
    let (mut best, mut stall) = (f64::INFINITY, 0usize);
    for it in 0..20_000 {
        iters = it + 1;
        // simultaneous (Weierstrass) update: all corrections from the same iterate
        let mut steps = [C::r(0.0); 4];
        for i in 0..4 {
            let mut den = C::r(1.0);
            for j in 0..4 {
                if i != j {
                    den = den.mul(z[i].sub(z[j]));
                }
            }
            steps[i] = p(z[i]).div(den);
        }
        let mut delta = 0.0_f64;
        for i in 0..4 {
            z[i] = z[i].sub(steps[i]);
            delta = delta.max(steps[i].abs());
        }
        // converged, or stagnated at the roundoff floor of a clustered (multiple) root
        if delta < 1e-15 * (1.0 + z.iter().map(|w| w.abs()).fold(0.0, f64::max)) {
            break;
        }
        if it > 200 && delta > 0.5 * best {
            stall += 1;
            if stall > 50 {
                break;
            }
        } else {
            stall = 0;
        }
        best = best.min(delta);
    }
    (z, iters)
}

fn original_residual(a: f64, b: f64, q1: C) -> f64 {
    let q2 = C::r(a).sub(q1.mul(q1)); // reconstruction map of the transverse coordinate
    let r1 = q1.mul(q1).add(q2).sub(C::r(a));
    let r2 = q2.mul(q2).add(q1).sub(C::r(b));
    r1.abs().max(r2.abs())
}

fn observable(inv: QuarticInvariant, q1: C, a: f64) -> C {
    let q2 = C::r(a).sub(q1.mul(q1));
    match inv {
        QuarticInvariant::One => C::r(1.0),
        QuarticInvariant::Q1 => q1,
        QuarticInvariant::Q2 => q2,
        QuarticInvariant::Q1Squared => q1.mul(q1),
        QuarticInvariant::Q2Squared => q2.mul(q2),
        QuarticInvariant::Q1Q2 => q1.mul(q2),
        QuarticInvariant::Q1SquaredQ2Squared => q1.mul(q1).mul(q2).mul(q2),
        QuarticInvariant::Q1SquaredPlusQ2 => q1.mul(q1).add(q2),
    }
}

const INVARIANTS: [QuarticInvariant; 8] = [
    QuarticInvariant::One,
    QuarticInvariant::Q1,
    QuarticInvariant::Q2,
    QuarticInvariant::Q1Squared,
    QuarticInvariant::Q2Squared,
    QuarticInvariant::Q1Q2,
    QuarticInvariant::Q1SquaredQ2Squared,
    QuarticInvariant::Q1SquaredPlusQ2,
];

// ------------------------------------------------------------------------------------------
// Test-level certificate boundary: source id, parameter epoch, validity domain, invariant,
// original residual of the oracle and reconstruction map.
// ------------------------------------------------------------------------------------------

#[derive(Debug)]
struct StructureCertificate {
    source_id: &'static str,
    epoch: (u64, u64),
    invariant: QuarticInvariant,
    enclosure: (f64, f64),
    oracle_residual: f64,
    reconstruction: &'static str,
}

fn certify(a: f64, b: f64, inv: QuarticInvariant, work: &mut TraceWork) -> StructureCertificate {
    let enc = quartic_invariant(ALL, QuarticParameters::point(a, b).unwrap(), inv, work).unwrap();
    let (roots, _) = oracle_roots(a, b);
    StructureCertificate {
        source_id: TRACE_TARGET_ID,
        epoch: (a.to_bits(), b.to_bits()),
        invariant: inv,
        enclosure: (enc.lo, enc.hi),
        oracle_residual: roots
            .iter()
            .map(|&q| original_residual(a, b, q))
            .fold(0.0, f64::max),
        reconstruction: "q2 = a - q1^2",
    }
}

fn use_certificate(c: &StructureCertificate, a: f64, b: f64) -> Result<(f64, f64), &'static str> {
    if c.source_id != TRACE_TARGET_ID {
        return Err("foreign source");
    }
    if c.epoch != (a.to_bits(), b.to_bits()) {
        return Err("stale parameter epoch");
    }
    Ok(c.enclosure)
}

fn native(a: f64, b: f64, inv: QuarticInvariant) -> (f64, f64) {
    let p = QuarticParameters::point(a, b).unwrap();
    let e = quartic_invariant(ALL, p, inv, &mut TraceWork::default()).unwrap();
    (e.lo, e.hi)
}

#[test]
fn s1_native_trace_matches_independent_root_oracle_with_multiplicity() {
    let mut rows = Vec::new();
    // the exact triple root (0.75, 0.75) is covered by s2/s3 (root count, original residual);
    // aggregate comparison uses nearby simple-root points so the oracle is well conditioned
    for (a, b) in [
        (0.75, 0.7501),
        (0.7501, 0.75),
        (0.0, 1.0),
        (1.3, -0.2),
        (-0.4, 0.9),
        (2.0, 3.5),
    ] {
        let (roots, iters) = oracle_roots(a, b);
        let res = roots
            .iter()
            .map(|&q| original_residual(a, b, q))
            .fold(0.0, f64::max);
        assert!(res < 1e-9, "oracle root not a solution: {res:e}");
        for inv in INVARIANTS {
            let mut work = TraceWork::default();
            let p = QuarticParameters::point(a, b).unwrap();
            let enc = quartic_invariant(ALL, p, inv, &mut work).unwrap();
            let sum = roots
                .iter()
                .fold(C::r(0.0), |s, &q| s.add(observable(inv, q, a)));
            let tol = 1e-7 * (1.0 + sum.abs());
            assert!(
                sum.im.abs() < tol,
                "{inv:?} aggregate must be real: {sum:?}"
            );
            assert!(
                sum.re >= enc.lo - tol && sum.re <= enc.hi + tol,
                "a={a} b={b} {inv:?}: oracle {} not in [{}, {}]",
                sum.re,
                enc.lo,
                enc.hi
            );
            let forbidden = [
                work.root_enumerations,
                work.jacobian_evaluations,
                work.jvp_evaluations,
                work.ad_evaluations,
                work.newton_updates,
            ];
            assert_eq!(forbidden, [0; 5]);
            rows.push(json!({"a": a, "b": b, "invariant": format!("{inv:?}"), "oracle": sum.re,
                             "enclosure": [enc.lo, enc.hi], "oracle_iterations": iters,
                             "oracle_max_original_residual": res,
                             "native_scalar_ops": work.scalar_additions + work.scalar_multiplications,
                             "native_interval_ops": work.interval_additions + work.interval_multiplications}));
        }
    }
    qual("s1_trace_vs_oracle", json!(rows));
}

#[test]
fn s2_counterexamples_losing_multiplicity_or_transverse_state_differ() {
    // triple root at a = b = 3/4: q1 = 1/2 (x3) and q1 = -3/2
    let (a, b) = (0.75, 0.75);
    let (roots, _) = oracle_roots(a, b);
    let near_half = roots
        .iter()
        .filter(|q| (q.re - 0.5).abs() < 1e-4 && q.im.abs() < 1e-4)
        .count();
    assert_eq!(near_half, 3, "triple root expected: {roots:?}");
    let all = native(a, b, QuarticInvariant::Q1);
    // multiplicity sum is 0; the distinct-root sum 1/2 - 3/2 = -1 is a different target
    assert!(all.0 <= 0.0 && all.1 >= 0.0);
    let distinct = 0.5 - 1.5;
    assert!(distinct < all.0 - 0.5);
    // Q1 sums agree at two parameter points (both 0) while Q2^2 sums differ
    // (q2^2 = b - q1 per root, so the sum is 4b): the low-dimensional invariant alone
    // loses the transverse coordinate. Note sum Q2 = 4a - sum q1^2 = 0 for every (a, b).
    let (q1a, q1b) = (
        native(0.2, 0.1, QuarticInvariant::Q1),
        native(0.9, -0.3, QuarticInvariant::Q1),
    );
    let (q2a, q2b) = (
        native(0.2, 0.1, QuarticInvariant::Q2Squared),
        native(0.9, -0.3, QuarticInvariant::Q2Squared),
    );
    assert!((q1a.0 - q1b.0).abs() < 1e-12);
    assert!((q2a.0 - q2b.0).abs() > 1.0);
    // a real-only aggregate differs where complex roots exist (a = 0, b = 1)
    let (roots01, _) = oracle_roots(0.0, 1.0);
    let real_sq: f64 = roots01
        .iter()
        .filter(|q| q.im.abs() < 1e-9)
        .map(|q| q.re * q.re)
        .sum();
    let all_sq = native(0.0, 1.0, QuarticInvariant::Q1Squared);
    assert!((real_sq - all_sq.0).abs() > 1e-3);
    // selected-real and real-only semantics are refused before evaluation and charged
    let p = QuarticParameters::point(a, b).unwrap();
    for wrong in [TraceSemantics::SelectedReal, TraceSemantics::RealOnly] {
        let mut w = TraceWork::default();
        assert!(quartic_invariant(wrong, p, QuarticInvariant::Q1, &mut w).is_err());
        assert_eq!(
            (
                w.requests_attempted,
                w.semantics_rejections,
                w.requests_completed
            ),
            (1, 1, 0)
        );
        assert_eq!(w.scalar_additions + w.interval_additions, 0);
    }
    qual(
        "s2_counterexamples",
        json!({"triple_root_count": near_half, "distinct_root_q1_sum": distinct,
               "multiplicity_q1_enclosure": [all.0, all.1], "real_only_q1sq_a0_b1": real_sq,
               "all_complex_q1sq_a0_b1": [all_sq.0, all_sq.1]}),
    );
}

#[test]
fn s3_certificate_boundary_refuses_stale_epoch_and_records_identity() {
    let mut work = TraceWork::default();
    let c = certify(0.75, 0.75, QuarticInvariant::Q1SquaredPlusQ2, &mut work);
    assert_eq!(c.source_id, TRACE_TARGET_ID);
    assert!(c.oracle_residual < 1e-9);
    // Q1^2 + Q2 = a for every root (original equation 1), so the aggregate is exactly 4a = 3
    let enc = use_certificate(&c, 0.75, 0.75).unwrap();
    assert!(enc.0 <= 3.0 && enc.1 >= 3.0);
    assert_eq!(
        use_certificate(&c, 0.75, 0.75 + f64::EPSILON),
        Err("stale parameter epoch")
    );
    assert_eq!(
        use_certificate(&c, 0.750_000_000_000_000_1, 0.75),
        Err("stale parameter epoch")
    );
    qual(
        "s3_certificate",
        json!({"source_id": c.source_id, "epoch_bits": [c.epoch.0, c.epoch.1],
               "invariant": format!("{:?}", c.invariant), "enclosure": [c.enclosure.0, c.enclosure.1],
               "oracle_residual": c.oracle_residual, "reconstruction": c.reconstruction,
               "provider_work": work}),
    );
}

// ------------------------------------------------------------------------------------------
// Research prototype (not a native port): oriented incidence on closed triangulations.
// ------------------------------------------------------------------------------------------

struct Complex {
    v: usize,
    edges: Vec<(usize, usize)>,
    faces: Vec<[usize; 3]>,
}

fn complex(faces: Vec<[usize; 3]>, v: usize) -> Complex {
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for f in &faces {
        for k in 0..3 {
            let (x, y) = (f[k], f[(k + 1) % 3]);
            let e = (x.min(y), x.max(y));
            if !edges.contains(&e) {
                edges.push(e);
            }
        }
    }
    Complex { v, edges, faces }
}

fn d0(c: &Complex) -> Vec<Vec<i64>> {
    c.edges
        .iter()
        .map(|&(i, j)| {
            let mut r = vec![0; c.v];
            r[i] = -1;
            r[j] = 1;
            r
        })
        .collect()
}

fn d1(c: &Complex) -> Vec<Vec<i64>> {
    c.faces
        .iter()
        .map(|f| {
            let mut r = vec![0; c.edges.len()];
            for k in 0..3 {
                let (x, y) = (f[k], f[(k + 1) % 3]);
                let idx = c
                    .edges
                    .iter()
                    .position(|&e| e == (x.min(y), x.max(y)))
                    .unwrap();
                r[idx] = if x < y { 1 } else { -1 };
            }
            r
        })
        .collect()
}

fn rank(m: &[Vec<i64>]) -> usize {
    let mut a: Vec<Vec<f64>> = m
        .iter()
        .map(|r| r.iter().map(|&x| x as f64).collect())
        .collect();
    if a.is_empty() {
        return 0;
    }
    let (rows, cols) = (a.len(), a[0].len());
    let mut r = 0;
    for c in 0..cols {
        if r >= rows {
            break;
        }
        let p = (r..rows)
            .max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))
            .unwrap();
        if a[p][c].abs() < 1e-9 {
            continue;
        }
        a.swap(r, p);
        for i in 0..rows {
            if i != r {
                let f = a[i][c] / a[r][c];
                for j in 0..cols {
                    a[i][j] -= f * a[r][j];
                }
            }
        }
        r += 1;
    }
    r
}

fn matmul(a: &[Vec<i64>], b: &[Vec<i64>]) -> Vec<Vec<i64>> {
    a.iter()
        .map(|row| {
            (0..b[0].len())
                .map(|j| (0..b.len()).map(|k| row[k] * b[k][j]).sum())
                .collect()
        })
        .collect()
}

fn octahedron() -> Complex {
    // vertices: 0 top, 5 bottom, 1..4 equator
    let f = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 1],
        [5, 2, 1],
        [5, 3, 2],
        [5, 4, 3],
        [5, 1, 4],
    ];
    complex(f, 6)
}

fn torus(m: usize) -> Complex {
    let id = |i: usize, j: usize| (i % m) * m + (j % m);
    let mut f = Vec::new();
    for i in 0..m {
        for j in 0..m {
            f.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
            f.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    complex(f, m * m)
}

#[test]
fn t1_prototype_boundary_of_boundary_betti_and_laplacian_nullspace() {
    let mut rows = Vec::new();
    for (name, c, want) in [
        ("octahedron_sphere", octahedron(), (1, 0, 1)),
        ("torus_3x3", torus(3), (1, 2, 1)),
    ] {
        let (a0, a1) = (d0(&c), d1(&c));
        let comp = matmul(&a1, &a0);
        assert!(comp.iter().flatten().all(|&x| x == 0), "{name}: D1 D0 != 0");
        let (r0, r1) = (rank(&a0), rank(&a1));
        let (v, e, f) = (c.v, c.edges.len(), c.faces.len());
        let betti = (v - r0, e - r0 - r1, f - r1);
        assert_eq!(betti, want, "{name}");
        // L0 = D0^T D0 annihilates constants (closed-domain mass balance of graph diffusion)
        let l0: Vec<Vec<i64>> = (0..v)
            .map(|i| {
                (0..v)
                    .map(|j| (0..e).map(|k| a0[k][i] * a0[k][j]).sum())
                    .collect()
            })
            .collect();
        assert!(l0.iter().all(|row| row.iter().sum::<i64>() == 0));
        assert_eq!(rank(&l0), v - 1);
        // Hodge 1-Laplacian nullity equals b1
        let l1: Vec<Vec<i64>> = (0..e)
            .map(|i| {
                (0..e)
                    .map(|j| {
                        (0..v).map(|k| a0[i][k] * a0[j][k]).sum::<i64>()
                            + (0..f).map(|k| a1[k][i] * a1[k][j]).sum::<i64>()
                    })
                    .collect()
            })
            .collect();
        let harmonic = e - rank(&l1);
        assert_eq!(harmonic, betti.1, "{name}: harmonic dimension");
        rows.push(
            json!({"complex": name, "V": v, "E": e, "F": f, "rank_d0": r0, "rank_d1": r1,
                         "betti": [betti.0, betti.1, betti.2],
                         "euler": v as i64 - e as i64 + f as i64, "harmonic_dim": harmonic}),
        );
    }
    qual("t1_topology_prototype", json!(rows));
}
