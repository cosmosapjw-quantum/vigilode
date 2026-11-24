//! Opt-in banded stage client that eliminates from both ends of the band (research node
//! SL-C5 of `research/stage_lattice_referee_20261124`).
//!
//! The protected banded kernel factors `W = I/(h gamma) - J` from the first row to the last
//! and every stage solve is two sweeps of length `n`: the critical path of the eight stage
//! solves is `16 n` substitution rows, whatever the number of cores.
//! [`BandedTwoWayStageLu`] splits the unknowns at `m = n / 2` and runs two independent
//! fronts:
//!
//! - the top front eliminates columns `0 .. m - lower` in ascending order, pivoting among
//!   rows `k ..= k + lower` exactly as the protected kernel does;
//! - the bottom front eliminates columns `n - 1 .. m + upper` in descending order, pivoting
//!   among rows `j - upper ..= j`;
//! - the `lower + upper` unknowns in between are solved by a dense partial-pivoting LU of
//!   the rows the two fronts leave over.
//!
//! This is Gaussian elimination with partial pivoting of `Q W Q^T`, where `Q` orders the
//! unknowns as `[0 .. m - lower, n - 1 .. m + upper, m - lower .. m + upper]`: the pivot
//! candidates of a front never include a row of the other front
//! (`research/stage_lattice_referee_20261124/results/twoway_gepp.json`, 30 of 30 seeded cases
//! with equal pivot sequences). No pivot is restricted, so the stability is that of partial
//! pivoting on a reordered matrix; the factors and the rounding differ from the protected
//! kernel, and a run is therefore not bit-identical to the protected banded driver.
//!
//! With [`TwoWayExecution::Join`] the two fronts of the factorization and of each of the two
//! substitution phases run under `rayon::join`; the critical path of the eight stage solves
//! becomes `8 n` rows. [`TwoWayExecution::Serial`] runs the same arithmetic on one thread and
//! returns the same bits as `Join`. The right-hand sides, the stage combinations and the
//! controller stay with the driver and are not parallelised by this client.
//!
//! Systems with fewer than `2 (lower + upper + 1)` unknowns are delegated to
//! [`BandedTightStageLu`]. Nothing here allocates after [`BandedTwoWayStageLu::new`].

use rodas5p_core::{CoreError, CoreResult, WorkCounters};

use super::BandedTightStageLu;
use crate::{BandedJacobian, BandedJacobianFn, OdeProblem, StageLinearSystem};

/// Identifier recorded by callers that run [`BandedTwoWayStageLu`].
pub const BANDED_TWO_WAY_CLIENT_ID: &str = "rodas5p-fast-client-banded-two-way-v1";

/// How the two elimination fronts of [`BandedTwoWayStageLu`] are executed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwoWayExecution {
    /// One thread: the top front, then the bottom front.
    #[default]
    Serial,
    /// `rayon::join` of the two fronts. Call the driver from inside a rayon pool
    /// (`ThreadPool::install`) so that the join is a local push instead of a blocking
    /// injection. The results are the bits of [`TwoWayExecution::Serial`].
    Join,
}

/// One elimination front in its own orientation: `rows` rows of a band with half widths
/// `lower` and `upper`, of which the first `rows - lower` columns are eliminated. Row `i`
/// holds columns `i - lower ..= i + upper + lower` (`2 lower + upper + 1` entries): `U` in
/// columns `>= i`, the multiplier of column `k` in column `k`.
struct Front {
    rows: usize,
    lower: usize,
    upper: usize,
    width: usize,
    eliminated: usize,
    factors: Vec<f64>,
    pivots: Vec<usize>,
}

impl Front {
    fn new(rows: usize, lower: usize, upper: usize) -> Self {
        let width = 2 * lower + upper + 1;
        Self {
            rows,
            lower,
            upper,
            width,
            eliminated: rows - lower,
            factors: vec![0.0; rows * width],
            pivots: vec![0; rows - lower],
        }
    }

    /// Entry `(i, j)` of the stored band, `i - lower <= j <= i + upper + lower`.
    #[inline]
    fn at(&self, i: usize, j: usize) -> f64 {
        self.factors[i * self.width + (j + self.lower - i)]
    }

    /// Partial-pivoting elimination of the front's columns. Every pivot window
    /// `k ..= k + lower` and every updated column `k ..= k + upper + lower` lies inside the
    /// front, so the loops need no clipping. `Err(k)`: zero or non-finite pivot at column
    /// `k` of the front.
    fn eliminate(&mut self) -> Result<(), usize> {
        let (l, width) = (self.lower, self.width);
        let span = self.upper + l + 1;
        for k in 0..self.eliminated {
            let mut p = k;
            let mut max = self.factors[k * width + l].abs();
            for i in k + 1..=k + l {
                let v = self.factors[i * width + (k + l - i)].abs();
                if v > max {
                    max = v;
                    p = i;
                }
            }
            if !(max > 0.0 && max.is_finite()) {
                return Err(k);
            }
            self.pivots[k] = p;
            if p != k {
                for j in k..k + span {
                    self.factors
                        .swap(k * width + (j + l - k), p * width + (j + l - p));
                }
            }
            let (top, bottom) = self.factors.split_at_mut((k + 1) * width);
            let pivot_row = &top[k * width + l..k * width + l + span];
            let pivot = pivot_row[0];
            for i in k + 1..=k + l {
                let start = (i - k - 1) * width + (k + l - i);
                let row = &mut bottom[start..start + span];
                if row[0] == 0.0 {
                    continue;
                }
                let m = row[0] / pivot;
                row[0] = m;
                for (x, r) in row[1..].iter_mut().zip(&pivot_row[1..]) {
                    *x -= m * r;
                }
            }
        }
        Ok(())
    }

    /// Forward substitution with the interchanges applied as it goes. `b` holds the
    /// front's rows; with `REV` row `i` of the front is `b[len - 1 - i]`.
    fn forward<const REV: bool>(&self, b: &mut [f64]) {
        let (l, width, len) = (self.lower, self.width, b.len());
        let pos = |i: usize| if REV { len - 1 - i } else { i };
        for (k, &p) in self.pivots.iter().enumerate() {
            if p != k {
                b.swap(pos(k), pos(p));
            }
            let bk = b[pos(k)];
            for i in k + 1..=k + l {
                b[pos(i)] -= self.factors[i * width + (k + l - i)] * bk;
            }
        }
    }

    /// Back substitution of the eliminated columns. The unknowns of columns `>= rows` are
    /// the `upper` ghost values in `ghosts`; the unknowns of the leftover rows are already
    /// in `b`.
    fn backward<const REV: bool>(&self, b: &mut [f64], ghosts: &[f64]) {
        let (l, width, len, rows) = (self.lower, self.width, b.len(), self.rows);
        let pos = |i: usize| if REV { len - 1 - i } else { i };
        let span = self.upper + l;
        // Rows whose window `i + 1 ..= i + span` reaches a ghost column, nearest the middle.
        let bulk = rows.saturating_sub(span).min(self.eliminated);
        for i in (bulk..self.eliminated).rev() {
            let row = &self.factors[i * width + l..i * width + l + span + 1];
            let mut sum = b[pos(i)];
            for (d, f) in row[1..].iter().enumerate() {
                let j = i + 1 + d;
                let x = if j < rows {
                    b[pos(j)]
                } else {
                    ghosts[j - rows]
                };
                sum -= f * x;
            }
            b[pos(i)] = sum / row[0];
        }
        // Every other row reads `span` stored unknowns: contiguous slices, the terms
        // subtracted in the same order (ascending column) as above.
        for i in (0..bulk).rev() {
            let row = &self.factors[i * width + l..i * width + l + span + 1];
            if REV {
                let (known, rest) = b.split_at_mut(len - 1 - i);
                let mut sum = rest[0];
                for (f, x) in row[1..]
                    .iter()
                    .zip(known[len - 1 - i - span..].iter().rev())
                {
                    sum -= f * x;
                }
                rest[0] = sum / row[0];
            } else {
                let (head, known) = b.split_at_mut(i + 1);
                let mut sum = head[i];
                for (f, x) in row[1..].iter().zip(&known[..span]) {
                    sum -= f * x;
                }
                head[i] = sum / row[0];
            }
        }
    }
}

/// Dense partial-pivoting LU of the `lower + upper` unknowns between the fronts.
struct Middle {
    q: usize,
    lu: Vec<f64>,
    pivots: Vec<usize>,
    rhs: Vec<f64>,
}

impl Middle {
    fn new(q: usize) -> Self {
        Self {
            q,
            lu: vec![0.0; q * q],
            pivots: vec![0; q],
            rhs: vec![0.0; q],
        }
    }

    fn factor(&mut self) -> bool {
        let q = self.q;
        for k in 0..q {
            let mut p = k;
            let mut max = self.lu[k * q + k].abs();
            for i in k + 1..q {
                let v = self.lu[i * q + k].abs();
                if v > max {
                    max = v;
                    p = i;
                }
            }
            if !(max > 0.0 && max.is_finite()) {
                return false;
            }
            self.pivots[k] = p;
            if p != k {
                for j in 0..q {
                    self.lu.swap(k * q + j, p * q + j);
                }
            }
            let (head, below) = self.lu.split_at_mut((k + 1) * q);
            let pivot_row = &head[k * q + k..(k + 1) * q];
            let pivot = pivot_row[0];
            for row in below.chunks_exact_mut(q) {
                let row = &mut row[k..];
                let m = row[0] / pivot;
                row[0] = m;
                if m != 0.0 {
                    for (x, r) in row[1..].iter_mut().zip(&pivot_row[1..]) {
                        *x -= m * r;
                    }
                }
            }
        }
        true
    }

    /// Solve in place on `self.rhs`. [`Self::factor`] interchanges whole rows, multipliers
    /// included, so every interchange is applied before the forward substitution.
    fn solve(&mut self) {
        let q = self.q;
        for k in 0..q {
            self.rhs.swap(k, self.pivots[k]);
        }
        for k in 0..q {
            let bk = self.rhs[k];
            for i in k + 1..q {
                self.rhs[i] -= self.lu[i * q + k] * bk;
            }
        }
        for i in (0..q).rev() {
            let mut sum = self.rhs[i];
            for j in i + 1..q {
                sum -= self.lu[i * q + j] * self.rhs[j];
            }
            self.rhs[i] = sum / self.lu[i * q + i];
        }
    }
}

struct TwoWay {
    n: usize,
    lower: usize,
    upper: usize,
    /// First row of the bottom front.
    split: usize,
    fill: BandedJacobianFn,
    /// Jacobian band, `n (lower + upper + 1)`; row `i`, column `j` at
    /// `i (lower + upper + 1) + (j + lower - i)`.
    jacobian: Vec<f64>,
    /// Rows `0 .. split` in the natural orientation.
    top: Front,
    /// Rows `n - 1 .. split` in the reversed orientation: row `i'` of the front is row
    /// `n - 1 - i'` of `W`, column `j'` is column `n - 1 - j'`, and the half widths exchange.
    bottom: Front,
    middle: Middle,
    ghosts_top: Vec<f64>,
    ghosts_bottom: Vec<f64>,
    execution: TwoWayExecution,
}

impl TwoWay {
    fn singular(column: usize) -> CoreError {
        CoreError::LinearSolve(format!(
            "RODAS5P two-way banded LU: singular or non-finite pivot at column {column}"
        ))
    }

    fn factor(&mut self, inv: f64) -> CoreResult<()> {
        let (n, l, u, m) = (self.n, self.lower, self.upper, self.split);
        let jw = l + u + 1;
        // W = I inv - J on the two fronts. The bottom front stores each band row reversed.
        self.top.factors.fill(0.0);
        for (dst, src) in self
            .top
            .factors
            .chunks_exact_mut(self.top.width)
            .zip(self.jacobian.chunks_exact(jw))
        {
            for (d, s) in dst[..jw].iter_mut().zip(src) {
                *d = -s;
            }
            dst[l] += inv;
        }
        self.bottom.factors.fill(0.0);
        for (ip, dst) in self
            .bottom
            .factors
            .chunks_exact_mut(self.bottom.width)
            .enumerate()
        {
            let i = n - 1 - ip;
            let src = &self.jacobian[i * jw..(i + 1) * jw];
            for (d, s) in dst[..jw].iter_mut().zip(src.iter().rev()) {
                *d = -s;
            }
            dst[u] += inv;
        }
        let (top, bottom) = (&mut self.top, &mut self.bottom);
        let (rt, rb) = match self.execution {
            TwoWayExecution::Serial => (top.eliminate(), bottom.eliminate()),
            TwoWayExecution::Join => rayon::join(|| top.eliminate(), || bottom.eliminate()),
        };
        rt.map_err(Self::singular)?;
        rb.map_err(|k| Self::singular(n - 1 - k))?;
        // The rows the fronts leave over live on columns [m - l, m + u).
        let q = l + u;
        let first = m - l;
        let rows_bottom = n - m;
        let first_bottom = rows_bottom - u;
        for r in 0..l {
            for c in 0..q {
                self.middle.lu[r * q + c] = self.top.at(first + r, first + c);
            }
        }
        for r in 0..u {
            for c in 0..q {
                let column = n - 1 - (first + c);
                self.middle.lu[(l + r) * q + c] = self.bottom.at(first_bottom + r, column);
            }
        }
        if !self.middle.factor() {
            return Err(Self::singular(first));
        }
        Ok(())
    }

    fn solve(&mut self, b: &mut [f64]) {
        let (n, l, u, m) = (self.n, self.lower, self.upper, self.split);
        let first = m - l;
        let rows_bottom = n - m;
        let (tb, bb) = b.split_at_mut(m);
        let (top, bottom) = (&self.top, &self.bottom);
        match self.execution {
            TwoWayExecution::Serial => {
                top.forward::<false>(tb);
                bottom.forward::<true>(bb);
            }
            TwoWayExecution::Join => {
                rayon::join(|| top.forward::<false>(tb), || bottom.forward::<true>(bb));
            }
        }
        // Leftover rows: positions first .. m of the top, the last u rows of the bottom
        // front (natural positions u - 1 .. 0 of `bb`).
        self.middle.rhs[..l].copy_from_slice(&tb[first..]);
        for r in 0..u {
            self.middle.rhs[l + r] = bb[u - 1 - r];
        }
        self.middle.solve();
        // The middle unknowns are x[first .. m + u] in ascending order.
        tb[first..].copy_from_slice(&self.middle.rhs[..l]);
        bb[..u].copy_from_slice(&self.middle.rhs[l..]);
        self.ghosts_top.copy_from_slice(&self.middle.rhs[l..]);
        for g in 0..l {
            self.ghosts_bottom[g] = self.middle.rhs[l - 1 - g];
        }
        debug_assert_eq!(rows_bottom, bb.len());
        let (gt, gb) = (&self.ghosts_top[..], &self.ghosts_bottom[..]);
        match self.execution {
            TwoWayExecution::Serial => {
                top.backward::<false>(tb, gt);
                bottom.backward::<true>(bb, gb);
            }
            TwoWayExecution::Join => {
                rayon::join(
                    || top.backward::<false>(tb, gt),
                    || bottom.backward::<true>(bb, gb),
                );
            }
        }
    }
}

enum Inner {
    /// Too few unknowns for two fronts: the single-front kernel.
    Small(BandedTightStageLu),
    TwoWay(Box<TwoWay>),
}

/// The banded stage linear algebra with two elimination fronts (see the module
/// documentation). Opt-in; not bit-identical to the protected banded driver.
pub struct BandedTwoWayStageLu {
    n: usize,
    inner: Inner,
}

impl BandedTwoWayStageLu {
    /// Band storage of order `n` for `band`. Requires `lower, upper < n`.
    pub fn new(n: usize, band: &BandedJacobian, execution: TwoWayExecution) -> CoreResult<Self> {
        if band.lower >= n.max(1) || band.upper >= n.max(1) {
            return Err(CoreError::InvalidInput(
                "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
            ));
        }
        let (lower, upper) = (band.lower, band.upper);
        if n < Self::min_two_way_dimension(lower, upper) {
            return Ok(Self {
                n,
                inner: Inner::Small(BandedTightStageLu::new(n, band)?),
            });
        }
        let split = n / 2;
        Ok(Self {
            n,
            inner: Inner::TwoWay(Box::new(TwoWay {
                n,
                lower,
                upper,
                split,
                fill: band.fill.clone(),
                jacobian: vec![0.0; n * (lower + upper + 1)],
                top: Front::new(split, lower, upper),
                bottom: Front::new(n - split, upper, lower),
                middle: Middle::new(lower + upper),
                ghosts_top: vec![0.0; upper],
                ghosts_bottom: vec![0.0; lower],
                execution,
            })),
        })
    }

    /// Smallest order at which two fronts are used; smaller systems run the single-front
    /// kernel of [`BandedTightStageLu`].
    pub fn min_two_way_dimension(lower: usize, upper: usize) -> usize {
        2 * (lower + upper + 1)
    }

    /// Whether this instance runs two fronts.
    pub fn uses_two_fronts(&self) -> bool {
        matches!(self.inner, Inner::TwoWay(_))
    }
}

impl StageLinearSystem for BandedTwoWayStageLu {
    fn dimension(&self) -> usize {
        self.n
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        match &mut self.inner {
            Inner::Small(client) => client.linearize(problem, t, y, counters),
            Inner::TwoWay(two) => {
                two.jacobian.fill(0.0);
                (two.fill)(t, y, &mut two.jacobian)
            }
        }
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        match &mut self.inner {
            Inner::Small(client) => client.factor(inv_h_gamma, counters),
            Inner::TwoWay(two) => {
                counters.direct_factorizations += 1;
                two.factor(inv_h_gamma)
            }
        }
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "banded stage solve shape mismatch".into(),
            ));
        }
        match &mut self.inner {
            Inner::Small(client) => client.solve_in_place(b, counters),
            Inner::TwoWay(two) => {
                counters.direct_solve_calls += 1;
                two.solve(b);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// Deterministic generator (splitmix64 mapped to [-1, 1)).
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            (z >> 11) as f64 / (1u64 << 52) as f64 - 1.0
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Dominant,
        WeakDiagonal,
        Hostile,
    }

    /// A dense banded matrix; the client receives `J = -W` and factors with `inv = 0`.
    fn matrix(n: usize, l: usize, u: usize, kind: Kind, rng: &mut Rng) -> Vec<f64> {
        let mut w = vec![0.0; n * n];
        for i in 0..n {
            for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
                w[i * n + j] = rng.next();
            }
        }
        for i in 0..n {
            match kind {
                Kind::Dominant => {
                    let sum: f64 = (0..n).map(|j| w[i * n + j].abs()).sum();
                    w[i * n + i] = sum + 1.0;
                }
                Kind::WeakDiagonal => w[i * n + i] *= 1e-3,
                Kind::Hostile => w[i * n + i] = 1e-9 * rng.next(),
            }
        }
        w
    }

    fn client(
        n: usize,
        l: usize,
        u: usize,
        w: &[f64],
        execution: TwoWayExecution,
    ) -> BandedTwoWayStageLu {
        let dense = w.to_vec();
        let band = BandedJacobian {
            lower: l,
            upper: u,
            fill: Arc::new(move |_t, _y, band: &mut [f64]| {
                let jw = l + u + 1;
                for i in 0..n {
                    for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
                        band[i * jw + (j + l - i)] = -dense[i * n + j];
                    }
                }
                Ok(())
            }),
        };
        BandedTwoWayStageLu::new(n, &band, execution).unwrap()
    }

    fn problem(n: usize) -> OdeProblem {
        OdeProblem::new(
            "two-way-test".to_owned(),
            n,
            Arc::new(|_t, _y, out: &mut [f64]| {
                out.fill(0.0);
                Ok(())
            }),
            None,
            None,
            // The constructor requires a Jacobian form; the client never calls it.
            Some(Arc::new(|_t, _y, _v, out: &mut [f64]| {
                out.fill(0.0);
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap()
    }

    fn solve(c: &mut BandedTwoWayStageLu, n: usize, b: &[f64]) -> (Vec<f64>, WorkCounters) {
        let mut counters = WorkCounters::default();
        c.linearize(&problem(n), 0.0, &vec![0.0; n], &mut counters)
            .unwrap();
        c.factor(0.0, &mut counters).unwrap();
        let mut x = b.to_vec();
        c.solve_in_place(&mut x, &mut counters).unwrap();
        (x, counters)
    }

    fn backward_error(n: usize, w: &[f64], x: &[f64], b: &[f64]) -> f64 {
        let mut residual = 0.0f64;
        let mut norm_w = 0.0f64;
        for i in 0..n {
            let row = &w[i * n..(i + 1) * n];
            let ax: f64 = row.iter().zip(x).map(|(a, x)| a * x).sum();
            residual = residual.max((b[i] - ax).abs());
            norm_w = norm_w.max(row.iter().map(|a| a.abs()).sum());
        }
        let norm_x = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let norm_b = b.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        residual / (norm_w * norm_x + norm_b)
    }

    #[test]
    fn two_way_solves_are_backward_stable_and_join_equals_serial_bit_for_bit() {
        let mut rng = Rng(20261124);
        let mut checked = 0;
        for kind in [Kind::Dominant, Kind::WeakDiagonal, Kind::Hostile] {
            for &(n, l, u) in &[
                (6usize, 1usize, 1usize),
                (7, 1, 1),
                (10, 2, 2),
                (11, 2, 2),
                (61, 2, 2),
                (64, 3, 1),
                (65, 1, 4),
                (40, 0, 2),
                (41, 2, 0),
                (33, 0, 0),
                (120, 6, 6),
                (257, 2, 2),
            ] {
                // A triangular band with a tiny diagonal is numerically singular
                // (condition number about 1e9^n): only the dominant kind is meaningful there.
                if (l == 0 || u == 0) && !matches!(kind, Kind::Dominant) {
                    continue;
                }
                let w = matrix(n, l, u, kind, &mut rng);
                let b: Vec<f64> = (0..n).map(|_| rng.next()).collect();
                let mut serial = client(n, l, u, &w, TwoWayExecution::Serial);
                let mut join = client(n, l, u, &w, TwoWayExecution::Join);
                assert_eq!(
                    serial.uses_two_fronts(),
                    n >= BandedTwoWayStageLu::min_two_way_dimension(l, u)
                );
                let (xs, counters) = solve(&mut serial, n, &b);
                let (xj, _) = solve(&mut join, n, &b);
                assert_eq!(counters.direct_factorizations, 1);
                assert_eq!(counters.direct_solve_calls, 1);
                assert!(
                    xs.iter().zip(&xj).all(|(a, b)| a.to_bits() == b.to_bits()),
                    "join differs from serial: {kind:?} n={n} l={l} u={u}"
                );
                let eta = backward_error(n, &w, &xs, &b);
                assert!(eta <= 2e-15, "eta {eta:e}: {kind:?} n={n} l={l} u={u}");
                checked += 1;
            }
        }
        assert_eq!(checked, 30);
    }

    #[test]
    fn repeated_factorizations_and_solves_reuse_the_storage() {
        let mut rng = Rng(7);
        let (n, l, u) = (90, 2, 2);
        let w = matrix(n, l, u, Kind::Dominant, &mut rng);
        let mut c = client(n, l, u, &w, TwoWayExecution::Serial);
        let mut counters = WorkCounters::default();
        c.linearize(&problem(n), 0.0, &vec![0.0; n], &mut counters)
            .unwrap();
        for inv in [0.0, 3.5, 1e4] {
            c.factor(inv, &mut counters).unwrap();
            let mut shifted = w.clone();
            for i in 0..n {
                shifted[i * n + i] += inv;
            }
            for _ in 0..3 {
                let b: Vec<f64> = (0..n).map(|_| rng.next()).collect();
                let mut x = b.clone();
                c.solve_in_place(&mut x, &mut counters).unwrap();
                assert!(backward_error(n, &shifted, &x, &b) <= 2e-15);
            }
        }
        assert_eq!(counters.direct_factorizations, 3);
        assert_eq!(counters.direct_solve_calls, 9);
    }

    #[test]
    fn singular_fronts_and_a_singular_middle_block_are_typed_linear_solve_errors() {
        let (n, l, u) = (40usize, 1usize, 1usize);
        // A zero column in the top front, in the bottom front and in the middle block.
        for column in [3usize, 36, 20] {
            let mut w = vec![0.0; n * n];
            for i in 0..n {
                w[i * n + i] = 4.0;
                if i > 0 {
                    w[i * n + i - 1] = -1.0;
                }
                if i + 1 < n {
                    w[i * n + i + 1] = -1.0;
                }
            }
            for i in 0..n {
                w[i * n + column] = 0.0;
            }
            let mut c = client(n, l, u, &w, TwoWayExecution::Serial);
            let mut counters = WorkCounters::default();
            c.linearize(&problem(n), 0.0, &vec![0.0; n], &mut counters)
                .unwrap();
            let error = c.factor(0.0, &mut counters).unwrap_err();
            assert!(matches!(error, CoreError::LinearSolve(_)), "{error}");
        }
        let mut c = client(
            n,
            l,
            u,
            &matrix(n, l, u, Kind::Dominant, &mut Rng(1)),
            TwoWayExecution::Serial,
        );
        let mut counters = WorkCounters::default();
        let error = c
            .solve_in_place(&mut vec![0.0; n + 1], &mut counters)
            .unwrap_err();
        assert!(matches!(error, CoreError::Dimension(_)));
    }
}
