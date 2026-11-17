//! Band kernels of loop L1 node V19, stage a (arms A1, B, C(P) and the exploratory B2, B3 of
//! `../../PREDECLARED.json`). Safe Rust, one thread.
//!
//! Storage. The Jacobian band has the layout of `rodas5p_integrators::BandedJacobianFn`: row `i`,
//! column `j` at `i * (l + u + 1) + (j + l - i)`. `W = inv I - J`. The factors of a block of order
//! `m` are stored as in the protected kernel: row `i` holds columns `i - l ..= i + u + l` in
//! `2l + u + 1` slots, `U` in columns `>= i`, the multiplier of column `k` in column `k`.

/// Largest number of blocks of a partition.
pub const MAXP: usize = 16;
/// Largest separator width `max(l, u)` of a partition.
pub const MAXW: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    /// The matrix is too small for the partition or the band is too wide for it.
    Layout,
    /// Zero or non-finite pivot in the band LU of a block.
    BlockPivot { block: usize, column: usize },
    /// Zero or non-finite pivot in the dense LU of the reduced system.
    ReducedPivot,
    /// The check of the original system after a solve failed (verified arms).
    Residual { eta: f64 },
    /// An error of the protected client (arm A0).
    Protected(String),
}

/// A Jacobian band of order `n`.
pub struct Band {
    pub n: usize,
    pub l: usize,
    pub u: usize,
    pub jac: Vec<f64>,
}

impl Band {
    pub fn jw(&self) -> usize {
        self.l + self.u + 1
    }

    /// The largest absolute row sum of `W = inv I - J`.
    pub fn norm_inf(&self, inv: f64) -> f64 {
        let (n, l, u, jw) = (self.n, self.l, self.u, self.jw());
        let mut best = 0.0f64;
        for i in 0..n {
            let d_lo = l.saturating_sub(i);
            let d_hi = (n - 1 + l - i).min(l + u);
            let row = &self.jac[i * jw..(i + 1) * jw];
            let mut s = 0.0;
            for (d, a) in row.iter().enumerate().take(d_hi + 1).skip(d_lo) {
                let w = if d == l { inv - a } else { -a };
                s += w.abs();
            }
            if !(s <= best) {
                best = s;
            }
        }
        best
    }

    /// `out = W x`.
    pub fn matvec(&self, inv: f64, x: &[f64], out: &mut [f64]) {
        let (n, l, u, jw) = (self.n, self.l, self.u, self.jw());
        for i in 0..n {
            let d_lo = l.saturating_sub(i);
            let d_hi = (n - 1 + l - i).min(l + u);
            let row = &self.jac[i * jw + d_lo..=i * jw + d_hi];
            let xs = &x[i + d_lo - l..=i + d_hi - l];
            let mut jx = 0.0;
            for (a, y) in row.iter().zip(xs) {
                jx += a * y;
            }
            out[i] = inv * x[i] - jx;
        }
    }

    /// `(max |b - W x|, max |x|, max |b|)`. A NaN anywhere makes the first value NaN.
    pub fn residual_norms(&self, inv: f64, x: &[f64], b: &[f64]) -> (f64, f64, f64) {
        let (n, l, u, jw) = (self.n, self.l, self.u, self.jw());
        let (mut r, mut xn, mut bn) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let d_lo = l.saturating_sub(i);
            let d_hi = (n - 1 + l - i).min(l + u);
            let row = &self.jac[i * jw + d_lo..=i * jw + d_hi];
            let xs = &x[i + d_lo - l..=i + d_hi - l];
            let mut jx = 0.0;
            for (a, y) in row.iter().zip(xs) {
                jx += a * y;
            }
            let res = (b[i] - inv * x[i] + jx).abs();
            if !(res <= r) {
                r = res;
            }
            xn = xn.max(x[i].abs());
            bn = bn.max(b[i].abs());
        }
        (r, xn, bn)
    }
}

/// Pivoted band LU of one block (rows and columns `row0 .. row0 + m` of `W`), with the operation
/// order of the protected kernel.
pub struct BlockLu {
    pub m: usize,
    l: usize,
    ul: usize,
    width: usize,
    fac: Vec<f64>,
    piv: Vec<usize>,
    /// Reciprocals of the pivots (arms B, C, B2, B3).
    rdiag: Vec<f64>,
    /// Row `i`: `U[i, i + 1 + t] * rdiag[i]`, `t < u + l` (arms B2, B3).
    us: Vec<f64>,
    recip: bool,
    scaled: bool,
}

impl BlockLu {
    pub fn new(m: usize, l: usize, u: usize, recip: bool, scaled: bool) -> Self {
        let width = 2 * l + u + 1;
        Self {
            m,
            l,
            ul: u + l,
            width,
            fac: vec![0.0; m * width],
            piv: vec![0; m],
            rdiag: if recip { vec![0.0; m] } else { Vec::new() },
            us: if scaled { vec![0.0; m * (u + l)] } else { Vec::new() },
            recip,
            scaled,
        }
    }

    /// `W` on the block: rows `row0 .. row0 + m`, columns outside the block left out.
    pub fn assemble(&mut self, jac: &[f64], jw: usize, inv: f64, row0: usize) {
        let (m, l, width) = (self.m, self.l, self.width);
        self.fac.fill(0.0);
        for i in 0..m {
            let g = row0 + i;
            let src = &jac[g * jw..(g + 1) * jw];
            let dst = &mut self.fac[i * width..i * width + jw];
            // The local column of offset d is i + d - l; keep 0 <= i + d - l < m.
            let d_lo = l.saturating_sub(i);
            let d_hi = (m - 1 + l - i).min(jw - 1);
            for d in d_lo..=d_hi {
                dst[d] = -src[d];
            }
            dst[l] += inv;
        }
    }

    /// Elimination of column `k`: pivot search over rows `k ..= k + l`, interchange, row updates.
    /// `false` on a zero or non-finite pivot.
    #[inline(always)]
    pub fn factor_step(&mut self, k: usize) -> bool {
        let (m, l, ul, width) = (self.m, self.l, self.ul, self.width);
        let last_row = (k + l).min(m - 1);
        let mut p = k;
        let mut max = self.fac[k * width + l].abs();
        for i in k + 1..=last_row {
            let v = self.fac[i * width + (k + l - i)].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return false;
        }
        self.piv[k] = p;
        let last_col = (k + ul).min(m - 1);
        if p != k {
            for j in k..=last_col {
                self.fac
                    .swap(k * width + (j + l - k), p * width + (j + l - p));
            }
        }
        let span = last_col - k + 1;
        let (top, bottom) = self.fac.split_at_mut((k + 1) * width);
        let pivot_row = &top[k * width + l..k * width + l + span];
        let pivot = pivot_row[0];
        for i in k + 1..=last_row {
            let start = (i - k - 1) * width + (k + l - i);
            let row = &mut bottom[start..start + span];
            if row[0] == 0.0 {
                continue;
            }
            let mlt = row[0] / pivot;
            row[0] = mlt;
            for (x, r) in row[1..].iter_mut().zip(&pivot_row[1..]) {
                *x -= mlt * r;
            }
        }
        if self.recip {
            let r = 1.0 / pivot;
            self.rdiag[k] = r;
            if self.scaled {
                let dst = &mut self.us[k * ul..(k + 1) * ul];
                dst.fill(0.0);
                for (d, s) in dst.iter_mut().zip(&pivot_row[1..]) {
                    *d = s * r;
                }
            }
        }
        true
    }

    /// The whole factorisation in column order.
    pub fn factor(&mut self) -> Result<(), usize> {
        for k in 0..self.m {
            if !self.factor_step(k) {
                return Err(k);
            }
        }
        Ok(())
    }

    /// Forward substitution step `k` (interchange, then the multipliers of column `k`), in the
    /// operation order of the protected kernel.
    #[inline(always)]
    pub fn fwd_step(&self, b: &mut [f64], k: usize) {
        let (m, l, width) = (self.m, self.l, self.width);
        let p = self.piv[k];
        if p != k {
            b.swap(k, p);
        }
        let bk = b[k];
        let last_row = (k + l).min(m - 1);
        for i in k + 1..=last_row {
            b[i] -= self.fac[i * width + (k + l - i)] * bk;
        }
    }

    /// Back substitution row `i` in the operation order of the protected kernel: the unknowns
    /// `i + 1, i + 2, ...` are subtracted in that order, then one division.
    #[inline(always)]
    pub fn back_step_ref(&self, b: &mut [f64], i: usize) {
        let (m, l, ul, width) = (self.m, self.l, self.ul, self.width);
        let last = (i + ul).min(m - 1);
        let row = &self.fac[i * width + l..i * width + l + (last - i) + 1];
        let mut sum = b[i];
        for (f, x) in row[1..].iter().zip(&b[i + 1..=last]) {
            sum -= f * x;
        }
        b[i] = sum / row[0];
    }

    /// Back substitution row `i` of arm B: the older unknowns `i + 2, ...` first, the newest
    /// unknown `prev = x[i + 1]` last, then a multiplication by the reciprocal pivot. Returns
    /// `x[i]`.
    #[inline(always)]
    pub fn back_step_fast(&self, b: &mut [f64], i: usize, prev: f64) -> f64 {
        let (m, l, ul, width) = (self.m, self.l, self.ul, self.width);
        let last = (i + ul).min(m - 1);
        let row = &self.fac[i * width + l..i * width + l + (last - i) + 1];
        let mut sum = b[i];
        if last >= i + 2 {
            for (f, x) in row[2..].iter().zip(&b[i + 2..=last]) {
                sum -= f * x;
            }
        }
        if last > i {
            sum -= row[1] * prev;
        }
        let x = sum * self.rdiag[i];
        b[i] = x;
        x
    }

    /// Back substitution row `i` of the post hoc exploratory arm BD: the order of arm B (older
    /// unknowns first, the newest last) with the division of the protected kernel kept.
    #[inline(always)]
    pub fn back_step_fast_div(&self, b: &mut [f64], i: usize, prev: f64) -> f64 {
        let (m, l, ul, width) = (self.m, self.l, self.ul, self.width);
        let last = (i + ul).min(m - 1);
        let row = &self.fac[i * width + l..i * width + l + (last - i) + 1];
        let mut sum = b[i];
        if last >= i + 2 {
            for (f, x) in row[2..].iter().zip(&b[i + 2..=last]) {
                sum -= f * x;
            }
        }
        if last > i {
            sum -= row[1] * prev;
        }
        let x = sum / row[0];
        b[i] = x;
        x
    }

    /// Back substitution row `i` of the exploratory arm B2 on the scaled rows of `U`:
    /// `x[i] = b[i] r[i] - sum (U[i, j] r[i]) x[j]`, the newest unknown last.
    #[inline(always)]
    pub fn back_step_scaled(&self, b: &mut [f64], i: usize, prev: f64) -> f64 {
        let (m, ul) = (self.m, self.ul);
        let cnt = ul.min(m - 1 - i);
        let urow = &self.us[i * ul..i * ul + cnt];
        let mut s = b[i] * self.rdiag[i];
        if cnt >= 2 {
            for (f, x) in urow[1..].iter().zip(&b[i + 2..i + 1 + cnt]) {
                s -= f * x;
            }
        }
        if cnt >= 1 {
            s -= urow[0] * prev;
        }
        b[i] = s;
        s
    }

    /// Forward substitution step `k` on `nv` right-hand sides stored row-major in `v`.
    #[inline(always)]
    pub fn fwd_step_multi(&self, v: &mut [f64], nv: usize, k: usize) {
        let (m, l, width) = (self.m, self.l, self.width);
        let p = self.piv[k];
        if p != k {
            for c in 0..nv {
                v.swap(k * nv + c, p * nv + c);
            }
        }
        let last_row = (k + l).min(m - 1);
        let (head, tail) = v.split_at_mut((k + 1) * nv);
        let vk = &head[k * nv..(k + 1) * nv];
        for i in k + 1..=last_row {
            let f = self.fac[i * width + (k + l - i)];
            let vi = &mut tail[(i - k - 1) * nv..(i - k) * nv];
            for (x, y) in vi.iter_mut().zip(vk) {
                *x -= f * y;
            }
        }
    }

    /// Back substitution row `i` on `nv` right-hand sides (older unknowns first, reciprocal).
    #[inline(always)]
    pub fn back_step_multi(&self, v: &mut [f64], nv: usize, i: usize) {
        let (m, l, ul, width) = (self.m, self.l, self.ul, self.width);
        let last = (i + ul).min(m - 1);
        let row = &self.fac[i * width + l..i * width + l + (last - i) + 1];
        let r = self.rdiag[i];
        let (head, tail) = v.split_at_mut((i + 1) * nv);
        let vi = &mut head[i * nv..(i + 1) * nv];
        for j in i + 2..=last {
            let f = row[j - i];
            let vj = &tail[(j - i - 1) * nv..(j - i) * nv];
            for (x, y) in vi.iter_mut().zip(vj) {
                *x -= f * y;
            }
        }
        if last > i {
            let f = row[1];
            for (x, y) in vi.iter_mut().zip(&tail[..nv]) {
                *x -= f * y;
            }
        }
        for x in vi.iter_mut() {
            *x *= r;
        }
    }

    /// Arm A1: both substitutions in the operation order of the protected kernel.
    pub fn solve_ref(&self, b: &mut [f64]) {
        for k in 0..self.m {
            self.fwd_step(b, k);
        }
        for i in (0..self.m).rev() {
            self.back_step_ref(b, i);
        }
    }

    /// Arm B.
    pub fn solve_fast(&self, b: &mut [f64]) {
        for k in 0..self.m {
            self.fwd_step(b, k);
        }
        let mut prev = 0.0;
        for i in (0..self.m).rev() {
            prev = self.back_step_fast(b, i, prev);
        }
    }

    /// Post hoc exploratory arm BD.
    pub fn solve_fast_div(&self, b: &mut [f64]) {
        for k in 0..self.m {
            self.fwd_step(b, k);
        }
        let mut prev = 0.0;
        for i in (0..self.m).rev() {
            prev = self.back_step_fast_div(b, i, prev);
        }
    }

    /// Exploratory arm B2.
    pub fn solve_scaled(&self, b: &mut [f64]) {
        for k in 0..self.m {
            self.fwd_step(b, k);
        }
        let mut prev = 0.0;
        for i in (0..self.m).rev() {
            prev = self.back_step_scaled(b, i, prev);
        }
    }

    /// Exploratory arm B3 (`l == 2`, `m >= 3`): the forward substitution of the protected
    /// kernel, same operations in the same order, on three carried values; then B2's back
    /// substitution.
    pub fn solve_scaled_carry2(&self, b: &mut [f64]) {
        let (m, width) = (self.m, self.width);
        assert!(self.l == 2 && m >= 3);
        let (mut w0, mut w1, mut w2) = (b[0], b[1], b[2]);
        for k in 0..m {
            match self.piv[k] - k {
                0 => {}
                1 => std::mem::swap(&mut w0, &mut w1),
                _ => std::mem::swap(&mut w0, &mut w2),
            }
            if k + 1 < m {
                w1 -= self.fac[(k + 1) * width + 1] * w0;
            }
            if k + 2 < m {
                w2 -= self.fac[(k + 2) * width] * w0;
            }
            b[k] = w0;
            w0 = w1;
            w1 = w2;
            w2 = if k + 3 < m { b[k + 3] } else { 0.0 };
        }
        let mut prev = 0.0;
        for i in (0..m).rev() {
            prev = self.back_step_scaled(b, i, prev);
        }
    }
}

/// Dense LU with partial pivoting, row-major, order `n`. `false` on a zero or non-finite pivot.
fn dense_factor(a: &mut [f64], piv: &mut [usize], n: usize) -> bool {
    for k in 0..n {
        let mut p = k;
        let mut max = a[k * n + k].abs();
        for i in k + 1..n {
            let v = a[i * n + k].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return false;
        }
        piv[k] = p;
        if p != k {
            for j in 0..n {
                a.swap(k * n + j, p * n + j);
            }
        }
        let pivot = a[k * n + k];
        for i in k + 1..n {
            let mlt = a[i * n + k] / pivot;
            a[i * n + k] = mlt;
            if mlt != 0.0 {
                for j in k + 1..n {
                    a[i * n + j] -= mlt * a[k * n + j];
                }
            }
        }
    }
    true
}

fn dense_solve(a: &[f64], piv: &[usize], n: usize, b: &mut [f64]) {
    for k in 0..n {
        b.swap(k, piv[k]);
    }
    for i in 1..n {
        let mut s = b[i];
        for j in 0..i {
            s -= a[i * n + j] * b[j];
        }
        b[i] = s;
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in i + 1..n {
            s -= a[i * n + j] * b[j];
        }
        b[i] = s / a[i * n + i];
    }
}

/// The layout of a partition: `(m, rem, block starts, block lengths, separator starts)`.
pub fn layout(
    n: usize,
    l: usize,
    u: usize,
    np: usize,
) -> Result<(usize, usize, [usize; MAXP], [usize; MAXP], [usize; MAXP]), KernelError> {
    let w = l.max(u);
    if !(2..=MAXP).contains(&np) || w == 0 || w > MAXW {
        return Err(KernelError::Layout);
    }
    let seps = (np - 1) * w;
    if n < seps + np * w {
        return Err(KernelError::Layout);
    }
    let m = (n - seps) / np;
    let rem = (n - seps) % np;
    let (mut off, mut len, mut sep) = ([0usize; MAXP], [0usize; MAXP], [0usize; MAXP]);
    for p in 0..np {
        off[p] = p * (m + w);
        len[p] = if p + 1 == np { m + rem } else { m };
        if p + 1 < np {
            sep[p] = off[p] + m;
        }
    }
    Ok((m, rem, off, len, sep))
}

/// Arm C(P): `np` blocks, explicit spikes, dense reduced system, lockstep recurrences.
pub struct Partition {
    n: usize,
    l: usize,
    u: usize,
    w: usize,
    np: usize,
    m: usize,
    rem: usize,
    nv: usize,
    ns: usize,
    off: [usize; MAXP],
    len: [usize; MAXP],
    sep: [usize; MAXP],
    blocks: Vec<BlockLu>,
    /// Per block: `len * nv`, row-major; columns `0 .. w` belong to the separator before the
    /// block, columns `w .. 2w` to the separator after it.
    spikes: Vec<Vec<f64>>,
    /// The rows of `W` on the separators, `ns * (l + u + 1)`.
    sep_a: Vec<f64>,
    red: Vec<f64>,
    red_piv: Vec<usize>,
    gs: Vec<f64>,
}

fn split<'a>(b: &'a mut [f64], off: &[usize], len: &[usize]) -> [&'a mut [f64]; MAXP] {
    let mut parts: [&'a mut [f64]; MAXP] = Default::default();
    let mut rest = b;
    let mut pos = 0usize;
    for p in 0..off.len() {
        let (_, tail) = std::mem::take(&mut rest).split_at_mut(off[p] - pos);
        let (blk, tail2) = tail.split_at_mut(len[p]);
        parts[p] = blk;
        rest = tail2;
        pos = off[p] + len[p];
    }
    parts
}

impl Partition {
    pub fn new(n: usize, l: usize, u: usize, np: usize) -> Result<Self, KernelError> {
        let (m, rem, off, len, sep) = layout(n, l, u, np)?;
        let w = l.max(u);
        let nv = 2 * w;
        let ns = (np - 1) * w;
        Ok(Self {
            n,
            l,
            u,
            w,
            np,
            m,
            rem,
            nv,
            ns,
            off,
            len,
            sep,
            blocks: (0..np)
                .map(|p| BlockLu::new(len[p], l, u, true, false))
                .collect(),
            spikes: (0..np).map(|p| vec![0.0; len[p] * nv]).collect(),
            sep_a: vec![0.0; ns * (l + u + 1)],
            red: vec![0.0; ns * ns],
            red_piv: vec![0; ns],
            gs: vec![0.0; ns],
        })
    }

    pub fn factor(&mut self, band: &Band, inv: f64) -> Result<(), KernelError> {
        let jw = band.jw();
        let (n, l, u, w, np, m, rem, nv, ns) = (
            self.n, self.l, self.u, self.w, self.np, self.m, self.rem, self.nv, self.ns,
        );
        // 1. The blocks of W and the coupling columns (right-hand sides of the spikes).
        for p in 0..np {
            let (row0, len) = (self.off[p], self.len[p]);
            self.blocks[p].assemble(&band.jac, jw, inv, row0);
            let v = &mut self.spikes[p];
            v.fill(0.0);
            for i in 0..len {
                let g = row0 + i;
                let src = &band.jac[g * jw..(g + 1) * jw];
                if p > 0 && i < l {
                    // Local column i + d - l < 0: column i + d - l + w of the separator before.
                    for d in 0..(l - i) {
                        v[i * nv + (i + d + w - l)] = -src[d];
                    }
                }
                if p + 1 < np && i + u >= len {
                    // Local column i + d - l >= len: column i + d - l - len of the separator after.
                    for d in (len + l - i)..=(l + u) {
                        v[i * nv + w + (i + d - l - len)] = -src[d];
                    }
                }
            }
        }
        // 2. Band LU of the blocks, in lockstep.
        for k in 0..m {
            for p in 0..np {
                if !self.blocks[p].factor_step(k) {
                    return Err(KernelError::BlockPivot {
                        block: p,
                        column: k,
                    });
                }
            }
        }
        for k in m..m + rem {
            if !self.blocks[np - 1].factor_step(k) {
                return Err(KernelError::BlockPivot {
                    block: np - 1,
                    column: k,
                });
            }
        }
        // 3. Spikes: block inverse applied to the coupling columns, in lockstep.
        for k in 0..m {
            for p in 0..np {
                self.blocks[p].fwd_step_multi(&mut self.spikes[p], nv, k);
            }
        }
        for k in m..m + rem {
            self.blocks[np - 1].fwd_step_multi(&mut self.spikes[np - 1], nv, k);
        }
        for i in (m..m + rem).rev() {
            self.blocks[np - 1].back_step_multi(&mut self.spikes[np - 1], nv, i);
        }
        for i in (0..m).rev() {
            for p in 0..np {
                self.blocks[p].back_step_multi(&mut self.spikes[p], nv, i);
            }
        }
        // 4. Reduced system on the separators.
        self.red.fill(0.0);
        for q in 0..np - 1 {
            let e = self.sep[q];
            for j in 0..w {
                let g = e + j;
                let r = q * w + j;
                let src = &band.jac[g * jw..(g + 1) * jw];
                for d in 0..jw {
                    if g + d < l {
                        continue;
                    }
                    let c = g + d - l;
                    if c >= n {
                        continue;
                    }
                    let a = if d == l { inv - src[d] } else { -src[d] };
                    self.sep_a[r * jw + d] = a;
                    if c < e {
                        let jc = c - self.off[q];
                        let v = &self.spikes[q][jc * nv..(jc + 1) * nv];
                        if q > 0 {
                            for cc in 0..w {
                                self.red[r * ns + (q - 1) * w + cc] -= a * v[cc];
                            }
                        }
                        for cc in 0..w {
                            self.red[r * ns + q * w + cc] -= a * v[w + cc];
                        }
                    } else if c < e + w {
                        self.red[r * ns + q * w + (c - e)] += a;
                    } else {
                        let jc = c - self.off[q + 1];
                        let v = &self.spikes[q + 1][jc * nv..(jc + 1) * nv];
                        for cc in 0..w {
                            self.red[r * ns + q * w + cc] -= a * v[cc];
                        }
                        if q + 2 < np {
                            for cc in 0..w {
                                self.red[r * ns + (q + 1) * w + cc] -= a * v[w + cc];
                            }
                        }
                    }
                }
            }
        }
        if !dense_factor(&mut self.red, &mut self.red_piv, ns) {
            return Err(KernelError::ReducedPivot);
        }
        Ok(())
    }

    pub fn solve(&mut self, b: &mut [f64]) {
        let (l, u, w, np, m, rem, nv, ns) = (
            self.l, self.u, self.w, self.np, self.m, self.rem, self.nv, self.ns,
        );
        let jw = l + u + 1;
        {
            let parts = split(b, &self.off[..np], &self.len[..np]);
            for k in 0..m {
                for p in 0..np {
                    self.blocks[p].fwd_step(&mut parts[p][..], k);
                }
            }
            for k in m..m + rem {
                self.blocks[np - 1].fwd_step(&mut parts[np - 1][..], k);
            }
            let mut prev = [0.0f64; MAXP];
            for i in (m..m + rem).rev() {
                prev[np - 1] =
                    self.blocks[np - 1].back_step_fast(&mut parts[np - 1][..], i, prev[np - 1]);
            }
            for i in (0..m).rev() {
                for p in 0..np {
                    prev[p] = self.blocks[p].back_step_fast(&mut parts[p][..], i, prev[p]);
                }
            }
        }
        // Reduced right-hand side: the separator rows applied to the block solutions.
        for q in 0..np - 1 {
            let e = self.sep[q];
            for j in 0..w {
                let g = e + j;
                let r = q * w + j;
                let a = &self.sep_a[r * jw..(r + 1) * jw];
                let mut s = b[g];
                for (d, ad) in a.iter().enumerate() {
                    let c = g + d - l;
                    if c >= e && c < e + w {
                        continue;
                    }
                    s -= ad * b[c];
                }
                self.gs[r] = s;
            }
        }
        dense_solve(&self.red, &self.red_piv, ns, &mut self.gs);
        // Correction of the blocks, then the separators.
        for p in 0..np {
            let mut coef = [0.0f64; 2 * MAXW];
            if p > 0 {
                coef[..w].copy_from_slice(&self.gs[(p - 1) * w..p * w]);
            }
            if p + 1 < np {
                coef[w..2 * w].copy_from_slice(&self.gs[p * w..(p + 1) * w]);
            }
            let part = &mut b[self.off[p]..self.off[p] + self.len[p]];
            let v = &self.spikes[p];
            if nv == 4 {
                let (c0, c1, c2, c3) = (coef[0], coef[1], coef[2], coef[3]);
                for (x, s) in part.iter_mut().zip(v.chunks_exact(4)) {
                    *x -= s[0] * c0 + s[1] * c1 + s[2] * c2 + s[3] * c3;
                }
            } else {
                for (x, s) in part.iter_mut().zip(v.chunks_exact(nv)) {
                    let mut acc = 0.0;
                    for (a, c) in s.iter().zip(&coef[..nv]) {
                        acc += a * c;
                    }
                    *x -= acc;
                }
            }
        }
        for q in 0..np - 1 {
            b[self.sep[q]..self.sep[q] + w].copy_from_slice(&self.gs[q * w..(q + 1) * w]);
        }
    }
}

/// The sequential arms on one block that is the whole matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqMode {
    /// A1.
    Ref,
    /// B.
    Fast,
    /// BD (exploratory, added after the validity runs): B's order with the division kept.
    FastDiv,
    /// B2 (exploratory).
    Scaled,
    /// B3 (exploratory, `l == 2`).
    ScaledCarry2,
}

pub struct Seq {
    pub blk: BlockLu,
    pub mode: SeqMode,
}

impl Seq {
    pub fn new(n: usize, l: usize, u: usize, mode: SeqMode) -> Self {
        let recip = !matches!(mode, SeqMode::Ref | SeqMode::FastDiv);
        let scaled = matches!(mode, SeqMode::Scaled | SeqMode::ScaledCarry2);
        Self {
            blk: BlockLu::new(n, l, u, recip, scaled),
            mode,
        }
    }

    pub fn factor(&mut self, band: &Band, inv: f64) -> Result<(), KernelError> {
        self.blk.assemble(&band.jac, band.jw(), inv, 0);
        self.blk
            .factor()
            .map_err(|column| KernelError::BlockPivot { block: 0, column })
    }

    pub fn solve(&self, b: &mut [f64]) {
        match self.mode {
            SeqMode::Ref => self.blk.solve_ref(b),
            SeqMode::Fast => self.blk.solve_fast(b),
            SeqMode::FastDiv => self.blk.solve_fast_div(b),
            SeqMode::Scaled => self.blk.solve_scaled(b),
            SeqMode::ScaledCarry2 => self.blk.solve_scaled_carry2(b),
        }
    }
}
