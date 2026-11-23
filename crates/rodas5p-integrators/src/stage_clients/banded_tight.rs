//! Opt-in banded stage client with the bits of the protected banded kernel and fewer
//! instructions (loop L1 node V21, stage b; evidence in
//! `research/production_loop_20261116/banded_solve/`).
//!
//! [`BandedTightStageLu`] performs the arithmetic of the protected banded kernel
//! ([`crate::BandedStageLu`], kernel `Indexed`) in the same order: the same assembly of
//! `W = I/(h gamma) - J` on the band, the same pivot search over rows `k ..= k + lower`, the
//! same interchanges, row updates and substitutions. Only the way the operands are reached
//! differs:
//!
//! - `lower = upper = 2` (two interleaved species on a line) with at least six unknowns: fixed
//!   row offsets and carried values, no index arithmetic in the loops;
//! - every other band: loops over row slices.
//!
//! Under [`crate::integrate_rodas5p_fast_client_observed_with_atol`] a run therefore equals a
//! run of [`crate::integrate_rodas5p_fast_banded_observed_with_atol`] bit for bit: states,
//! accepted and rejected steps and every work counter (`direct_factorizations` once per
//! factorisation, `direct_solve_calls` once per solve, as [`crate::BandedStageLu`] charges
//! them). The counted band operations of [`crate::BandedWork`] are not kept by this client.
//!
//! Nothing here allocates after [`BandedTightStageLu::new`].

use std::cell::Cell;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};

use crate::{BandedJacobian, BandedJacobianFn, OdeProblem, StageLinearSystem};

/// The banded pipeline's Jacobian band, LU and solve as a client, with the operation order of
/// the protected kernel (see the module documentation).
pub struct BandedTightStageLu {
    n: usize,
    lower: usize,
    upper: usize,
    fill: BandedJacobianFn,
    /// Jacobian band, `n (lower + upper + 1)`; row `i`, column `j` at
    /// `i (lower + upper + 1) + (j + lower - i)`.
    jacobian: Vec<f64>,
    /// `W` and then its factors, row `i` holding columns `i - lower ..= i + upper + lower`
    /// (`n (2 lower + upper + 1)`): `U` in columns `>= i`, the multiplier of column `k` in
    /// column `k`.
    factors: Vec<f64>,
    pivots: Vec<usize>,
}

impl BandedTightStageLu {
    /// Band storage of order `n` for `band`. Requires `lower, upper < n`.
    pub fn new(n: usize, band: &BandedJacobian) -> CoreResult<Self> {
        if band.lower >= n.max(1) || band.upper >= n.max(1) {
            return Err(CoreError::InvalidInput(
                "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
            ));
        }
        let (lower, upper) = (band.lower, band.upper);
        Ok(Self {
            n,
            lower,
            upper,
            fill: band.fill.clone(),
            jacobian: vec![0.0; n * (lower + upper + 1)],
            factors: vec![0.0; n * (2 * lower + upper + 1)],
            pivots: vec![0; n],
        })
    }

    /// Whether the fixed-offset loops apply (`lower = upper = 2` and at least six unknowns).
    pub fn uses_fixed_offsets(&self) -> bool {
        self.lower == 2 && self.upper == 2 && self.n >= 6
    }

    fn singular(column: usize) -> CoreError {
        CoreError::LinearSolve(format!(
            "RODAS5P fast banded LU: singular or non-finite pivot at column {column}"
        ))
    }

    /// Elimination of column `k` over row slices: pivot search over rows `k ..= k + lower`,
    /// interchange, row updates for ascending columns.
    fn factor_step(&mut self, k: usize) -> bool {
        let (n, l, ul) = (self.n, self.lower, self.upper + self.lower);
        let width = 2 * l + self.upper + 1;
        let last_row = (k + l).min(n - 1);
        let mut p = k;
        let mut max = self.factors[k * width + l].abs();
        for i in k + 1..=last_row {
            let v = self.factors[i * width + (k + l - i)].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return false;
        }
        self.pivots[k] = p;
        let last_col = (k + ul).min(n - 1);
        if p != k {
            for j in k..=last_col {
                self.factors
                    .swap(k * width + (j + l - k), p * width + (j + l - p));
            }
        }
        let span = last_col - k + 1;
        let (top, bottom) = self.factors.split_at_mut((k + 1) * width);
        let pivot_row = &top[k * width + l..k * width + l + span];
        let pivot = pivot_row[0];
        for i in k + 1..=last_row {
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
        true
    }

    /// `W` on the band and its LU, any band.
    fn factor_slices(&mut self, inv: f64) -> CoreResult<()> {
        let (n, l) = (self.n, self.lower);
        let jw = l + self.upper + 1;
        let width = 2 * l + self.upper + 1;
        self.factors.fill(0.0);
        for (dst, src) in self
            .factors
            .chunks_exact_mut(width)
            .zip(self.jacobian.chunks_exact(jw))
        {
            for (d, s) in dst[..jw].iter_mut().zip(src) {
                *d = -s;
            }
            dst[l] += inv;
        }
        for k in 0..n {
            if !self.factor_step(k) {
                return Err(Self::singular(k));
            }
        }
        Ok(())
    }

    /// `W` on the band and its LU for `lower = upper = 2`: the operations of
    /// [`Self::factor_slices`] in the same order on fixed row offsets.
    fn factor_fixed2(&mut self, inv: f64) -> CoreResult<()> {
        let n = self.n;
        {
            let (rows, _) = self.factors.as_chunks_mut::<7>();
            let (src, _) = self.jacobian.as_chunks::<5>();
            for (d, s) in rows.iter_mut().zip(src) {
                d[0] = -s[0];
                d[1] = -s[1];
                d[2] = -s[2] + inv;
                d[3] = -s[3];
                d[4] = -s[4];
                d[5] = 0.0;
                d[6] = 0.0;
            }
            for k in 0..n - 4 {
                let [r0, r1, r2, ..] = &mut rows[k..] else {
                    unreachable!()
                };
                // Column k sits at offset 2 of row k, 1 of row k + 1 and 0 of row k + 2.
                let mut p = 0usize;
                let mut max = r0[2].abs();
                let a1 = r1[1].abs();
                if a1 > max {
                    max = a1;
                    p = 1;
                }
                let a2 = r2[0].abs();
                if a2 > max {
                    max = a2;
                    p = 2;
                }
                if !(max > 0.0 && max.is_finite()) {
                    return Err(Self::singular(k));
                }
                self.pivots[k] = k + p;
                if p == 1 {
                    for t in 0..5 {
                        std::mem::swap(&mut r0[2 + t], &mut r1[1 + t]);
                    }
                } else if p == 2 {
                    for t in 0..5 {
                        std::mem::swap(&mut r0[2 + t], &mut r2[t]);
                    }
                }
                let pivot = r0[2];
                if r1[1] != 0.0 {
                    let m = r1[1] / pivot;
                    r1[1] = m;
                    r1[2] -= m * r0[3];
                    r1[3] -= m * r0[4];
                    r1[4] -= m * r0[5];
                    r1[5] -= m * r0[6];
                }
                if r2[0] != 0.0 {
                    let m = r2[0] / pivot;
                    r2[0] = m;
                    r2[1] -= m * r0[3];
                    r2[2] -= m * r0[4];
                    r2[3] -= m * r0[5];
                    r2[4] -= m * r0[6];
                }
            }
        }
        for k in n - 4..n {
            if !self.factor_step(k) {
                return Err(Self::singular(k));
            }
        }
        Ok(())
    }

    /// Back substitution row `i` over a row slice: the unknowns `i + 1, i + 2, ...` in that
    /// order, then one division.
    fn back_step(&self, b: &mut [f64], i: usize) {
        let (n, l, ul) = (self.n, self.lower, self.upper + self.lower);
        let width = 2 * l + self.upper + 1;
        let last = (i + ul).min(n - 1);
        let row = &self.factors[i * width + l..i * width + l + (last - i) + 1];
        let mut sum = b[i];
        for (f, x) in row[1..].iter().zip(&b[i + 1..=last]) {
            sum -= f * x;
        }
        b[i] = sum / row[0];
    }

    /// Both substitutions, any band.
    #[allow(clippy::needless_range_loop)] // the band offsets index both arrays
    fn solve_slices(&self, b: &mut [f64]) {
        let (n, l) = (self.n, self.lower);
        let width = 2 * l + self.upper + 1;
        for k in 0..n {
            let p = self.pivots[k];
            if p != k {
                b.swap(k, p);
            }
            let bk = b[k];
            let last_row = (k + l).min(n - 1);
            for i in k + 1..=last_row {
                b[i] -= self.factors[i * width + (k + l - i)] * bk;
            }
        }
        for i in (0..n).rev() {
            self.back_step(b, i);
        }
    }

    /// Both substitutions for `lower = upper = 2`: the operations of [`Self::solve_slices`] in
    /// the same order, the forward sweep on three carried values and the back sweep on four.
    fn solve_fixed2(&self, b: &mut [f64]) {
        let n = self.n;
        let (rows, _) = self.factors.as_chunks::<7>();
        {
            let cells = Cell::from_mut(&mut b[..n]).as_slice_of_cells();
            let (mut w0, mut w1, mut w2) = (cells[0].get(), cells[1].get(), cells[2].get());
            let steps = cells[..n - 3]
                .iter()
                .zip(&cells[3..])
                .zip(rows[1..].windows(2))
                .zip(&self.pivots[..n - 3])
                .enumerate();
            for (k, (((out, next), below), &p)) in steps {
                match p - k {
                    0 => {}
                    1 => std::mem::swap(&mut w0, &mut w1),
                    _ => std::mem::swap(&mut w0, &mut w2),
                }
                w1 -= below[0][1] * w0;
                w2 -= below[1][0] * w0;
                out.set(w0);
                w0 = w1;
                w1 = w2;
                w2 = next.get();
            }
            // Column n - 3.
            match self.pivots[n - 3] - (n - 3) {
                0 => {}
                1 => std::mem::swap(&mut w0, &mut w1),
                _ => std::mem::swap(&mut w0, &mut w2),
            }
            w1 -= rows[n - 2][1] * w0;
            w2 -= rows[n - 1][0] * w0;
            cells[n - 3].set(w0);
            // Column n - 2.
            if self.pivots[n - 2] != n - 2 {
                std::mem::swap(&mut w1, &mut w2);
            }
            w2 -= rows[n - 1][1] * w1;
            cells[n - 2].set(w1);
            // Column n - 1.
            cells[n - 1].set(w2);
        }
        for i in (n - 4..n).rev() {
            self.back_step(b, i);
        }
        let (mut x1, mut x2, mut x3, mut x4) = (b[n - 4], b[n - 3], b[n - 2], b[n - 1]);
        for (r, bi) in rows[..n - 4].iter().zip(b[..n - 4].iter_mut()).rev() {
            let mut s = *bi;
            s -= r[3] * x1;
            s -= r[4] * x2;
            s -= r[5] * x3;
            s -= r[6] * x4;
            let x = s / r[2];
            *bi = x;
            x4 = x3;
            x3 = x2;
            x2 = x1;
            x1 = x;
        }
    }
}

impl StageLinearSystem for BandedTightStageLu {
    fn dimension(&self) -> usize {
        self.n
    }

    /// The band of `J` at `(t, y)` from the band callback; the driver has charged
    /// `jacobian_builds`.
    fn linearize(
        &mut self,
        _problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.jacobian.fill(0.0);
        (self.fill)(t, y, &mut self.jacobian)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_factorizations += 1;
        if self.uses_fixed_offsets() {
            self.factor_fixed2(inv_h_gamma)
        } else {
            self.factor_slices(inv_h_gamma)
        }
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "banded stage solve shape mismatch".into(),
            ));
        }
        counters.direct_solve_calls += 1;
        if self.uses_fixed_offsets() {
            self.solve_fixed2(b);
        } else {
            self.solve_slices(b);
        }
        Ok(())
    }
}
