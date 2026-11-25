// Standalone timing harness: lines 60-284 and 1041-1081 of ppsw1.rs copied verbatim by sed.
#![allow(dead_code, clippy::all)]
#[derive(Debug)]
pub enum CoreError { InvalidInput(String), Coefficients(String) }
pub type CoreResult<T> = Result<T, CoreError>;
#[derive(Clone, Copy, Debug)]
struct Dd {
    hi: f64,
    lo: f64,
}
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
fn quick(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}
fn dd(x: f64) -> Dd {
    Dd { hi: x, lo: 0.0 }
}
fn add(x: Dd, y: Dd) -> Dd {
    let (s, e) = two_sum(x.hi, y.hi);
    let (t, f) = two_sum(x.lo, y.lo);
    let u = quick(s, e + t);
    quick(u.hi, u.lo + f)
}
fn neg(x: Dd) -> Dd {
    Dd {
        hi: -x.hi,
        lo: -x.lo,
    }
}
fn sub(x: Dd, y: Dd) -> Dd {
    add(x, neg(y))
}
fn mul(x: Dd, y: Dd) -> Dd {
    let p = x.hi * y.hi;
    let e = x.hi.mul_add(y.hi, -p) + (x.hi * y.lo + x.lo * y.hi);
    quick(p, e)
}
fn div(x: Dd, y: Dd) -> Dd {
    let q1 = x.hi / y.hi;
    let r = sub(x, mul(y, dd(q1)));
    let q2 = r.hi / y.hi;
    let r = sub(r, mul(y, dd(q2)));
    let q3 = r.hi / y.hi;
    add(quick(q1, q2), dd(q3))
}
fn from_i128(n: i128) -> Dd {
    let hi = n as f64;
    quick(hi, (n - hi as i128) as f64)
}

type D6 = [[Dd; 6]; 6];
fn z6() -> D6 {
    [[dd(0.0); 6]; 6]
}
fn i6() -> D6 {
    let mut m = z6();
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = dd(1.0);
    }
    m
}
fn mm(a: &D6, b: &D6) -> D6 {
    let mut m = z6();
    for i in 0..6 {
        for j in 0..6 {
            let mut s = dd(0.0);
            for k in 0..6 {
                s = add(s, mul(a[i][k], b[k][j]));
            }
            m[i][j] = s;
        }
    }
    m
}
fn inv(a: &D6) -> D6 {
    let mut m = *a;
    let mut r = i6();
    for col in 0..6 {
        let mut piv = col;
        for row in col..6 {
            if m[row][col].hi.abs() > m[piv][col].hi.abs() {
                piv = row;
            }
        }
        m.swap(col, piv);
        r.swap(col, piv);
        let d = m[col][col];
        for j in 0..6 {
            m[col][j] = div(m[col][j], d);
            r[col][j] = div(r[col][j], d);
        }
        for row in 0..6 {
            if row != col {
                let f = m[row][col];
                for j in 0..6 {
                    m[row][j] = sub(m[row][j], mul(f, m[col][j]));
                    r[row][j] = sub(r[row][j], mul(f, r[col][j]));
                }
            }
        }
    }
    r
}

fn gamma_dd() -> Dd {
    // First 38 decimals of gamma_hat (60-digit value certified in the contract tests).
    let n38: i128 = 61_941_197_506_625_051_631_076_337_400_290_510_912;
    let ten38: i128 = 100_000_000_000_000_000_000_000_000_000_000_000_000;
    div(from_i128(n38), from_i128(ten38))
}

pub fn gamma() -> f64 {
    let g = gamma_dd();
    g.hi + g.lo
}
pub fn nodes() -> [f64; 6] {
    [-1.0, -0.5, 0.0, 0.4, 0.8, 1.0]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tableau {
    pub gamma: f64,
    pub nodes: [f64; 6],
    pub b_matrix: [[f64; 6]; 6],
    pub a_matrix: [[f64; 6]; 6],
    pub gamma_matrix: [[f64; 6]; 6],
    pub beta: [[f64; 6]; 6],
}

pub fn tableau(sigma: f64) -> CoreResult<Tableau> {
    if !(sigma.is_finite() && sigma > 0.0) {
        return Err(CoreError::InvalidInput(
            "sigma must be finite and positive".into(),
        ));
    }
    let c = [
        dd(-1.0),
        dd(-0.5),
        dd(0.0),
        div(dd(2.0), dd(5.0)),
        div(dd(4.0), dd(5.0)),
        dd(1.0),
    ];
    let sg = dd(sigma);
    let mut v = z6();
    let mut s = z6();
    let mut p = z6();
    let mut d = z6();
    let mut di = z6();
    let mut f0 = z6();
    for i in 0..6 {
        let mut pw = dd(1.0);
        for j in 0..6 {
            v[i][j] = pw;
            pw = mul(pw, c[i]);
            // binom(j,i)
            let mut b = 1.0;
            if j >= i {
                for t in 0..i {
                    b = b * (j - t) as f64 / (t + 1) as f64;
                }
            } else {
                b = 0.0;
            }
            p[i][j] = dd(b.round());
        }
        let mut sp = dd(1.0);
        for _ in 0..i {
            sp = mul(sp, sg);
        }
        s[i][i] = sp;
        d[i][i] = dd((i + 1) as f64);
        di[i][i] = div(dd(1.0), dd((i + 1) as f64));
        if i >= 1 {
            f0[i][i - 1] = dd(1.0);
        }
    }
    let vi = inv(&v);
    let theta = mm(&mm(&mm(&v, &s), &p), &vi);
    let mut ft = z6();
    for i in 0..6 {
        for j in 0..6 {
            ft[i][j] = f0[j][i];
        }
    }
    let mut e = mm(&mm(&mm(&v, &d), &ft), &vi);
    for row in e.iter_mut() {
        for x in row.iter_mut() {
            *x = neg(*x);
        }
    }
    let beta = mm(&mm(&mm(&mm(&v, &f0), &s), &di), &vi);
    let g = gamma_dd();
    let mut ie = i6();
    for i in 0..6 {
        for j in 0..6 {
            ie[i][j] = add(ie[i][j], mul(g, e[i][j]));
        }
    }
    let be = mm(&beta, &e);
    let t1 = mm(&ie, &theta);
    let mut out = Tableau {
        gamma: g.hi + g.lo,
        nodes: nodes(),
        b_matrix: [[0.0; 6]; 6],
        a_matrix: [[0.0; 6]; 6],
        gamma_matrix: [[0.0; 6]; 6],
        beta: [[0.0; 6]; 6],
    };
    let f = |x: Dd| x.hi + x.lo;
    for i in 0..6 {
        for j in 0..6 {
            let bb = add(t1[i][j], mul(sg, be[i][j]));
            let gam = neg(mul(g, theta[i][j]));
            out.b_matrix[i][j] = f(bb);
            out.gamma_matrix[i][j] = f(gam);
            out.a_matrix[i][j] = f(sub(beta[i][j], gam));
            out.beta[i][j] = f(beta[i][j]);
        }
    }
    Ok(out)
}
fn legendre_shifted(n: usize, x: f64) -> f64 {
    let t = 2.0 * x - 1.0;
    let (mut p0, mut p1) = (1.0, t);
    if n == 0 {
        return p0;
    }
    for k in 2..=n {
        let kf = k as f64;
        let p2 = ((2.0 * kf - 1.0) * t * p1 - (kf - 1.0) * p0) / kf;
        p0 = p1;
        p1 = p2;
    }
    p1
}

/// Nodes of the 5-stage Radau IIA method (roots of `P5(2x-1) - P4(2x-1)` on `(0, 1]`).
pub fn radau_nodes() -> CoreResult<[f64; 5]> {
    let q = |x: f64| legendre_shifted(5, x) - legendre_shifted(4, x);
    let grid = 4000usize;
    let mut roots: Vec<f64> = Vec::new();
    for k in 1..grid {
        let (mut a, mut b) = ((k - 1) as f64 / grid as f64, k as f64 / grid as f64);
        let (qa, qb) = (q(a), q(b));
        if qa == 0.0 && a > 0.0 {
            roots.push(a);
        } else if qa * qb < 0.0 {
            for _ in 0..200 {
                let m = 0.5 * (a + b);
                if q(a) * q(m) <= 0.0 {
                    b = m;
                } else {
                    a = m;
                }
            }
            roots.push(0.5 * (a + b));
        }
    }
    roots.push(1.0);
    <[f64; 5]>::try_from(roots)
        .map_err(|_| CoreError::Coefficients("Radau IIA node search failed".into()))
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args[1].as_str();
    let n: usize = args[2].parse().unwrap();
    let mut acc = 0.0f64;
    let t = std::time::Instant::now();
    match mode {
        "tableau1" => for _ in 0..n { let tb = tableau(std::hint::black_box(1.0)).unwrap(); acc += tb.b_matrix[5][5]; },
        "tableau11" => for _ in 0..n { let tb = tableau(std::hint::black_box(1.1)).unwrap(); acc += tb.b_matrix[5][5]; },
        "radau" => for _ in 0..n { let c = std::hint::black_box(radau_nodes().unwrap()); acc += c[0]; },
        "none" => {},
        _ => panic!(),
    }
    let dt = t.elapsed().as_secs_f64();
    println!("{{\"mode\":\"{mode}\",\"calls\":{n},\"wall_s\":{dt:e},\"ns_per_call\":{:.1},\"checksum\":{acc:e}}}", if n>0 {dt*1e9/n as f64} else {0.0});
}
