//! Straight-segment full-source averaged derivative enclosure.
//! Caller supplies a true-state box; reference polynomial need not satisfy g=0.
use super::*;
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SecantCellWork {
    pub source_rows: usize,
    pub compositions: usize,
    pub guard_solves: usize,
    pub direct_divisions: usize,
    pub refinement_sweeps: usize,
    pub horner_ranges: usize,
    pub bernstein_ranges: usize,
    pub max_residual_upper: f64,
    pub max_inverse_defect: f64,
}
pub struct SecantCellWorkspace {
    guard: GuardColumnWorkspace,
    fjac: [[Iv; 33]; 32],
    power: [[Iv; 5]; 32],
    residual: [ScratchPoly; 32],
    zx: [Iv; 1024],
}
impl SecantCellWorkspace {
    pub fn new() -> Self {
        Self {
            guard: GuardColumnWorkspace::new(),
            fjac: [[zero(); 33]; 32],
            power: [[zero(); 5]; 32],
            residual: [ScratchPoly {
                a: [zero(); 17],
                len: 0,
            }; 32],
            zx: [zero(); 1024],
        }
    }
    /// Choose a fixed represented polynomial, not a different uncertain
    /// coefficient in each cell. Optional offset is an explicit reference
    /// perturbation; the caller must hull this reference with the true box.
    pub fn prepare<'a>(
        &'a mut self,
        src: &'a DaeSource,
        times: [f64; 2],
        rows: &[[Iv; 5]],
        offset: &[f64],
        work: &mut SecantCellWork,
    ) -> CoreResult<SecantSlab<'a>> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = src.nx();
        let nz = src.nz();
        let n = nx + nz;
        if rows.len() != n || offset.len() != n {
            return Err(bad("secant polynomial shape"));
        }
        let h = p(times[1])?.sub(p(times[0])?)?;
        if h.lo <= 0. {
            return Err(bad("secant slab clock"));
        }
        for (i, row) in rows.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                Iv::new(v.lo, v.hi)?;
                self.power[i][j] = p(di::mid(v) + if j == 0 { offset[i] } else { 0. })?;
            }
        }
        let clock = [p(times[0])?, h];
        for (i, eq) in src.f.iter().enumerate() {
            let f = compose_bounded(eq, &clock, &self.power[..n])?.scale(h)?;
            let mut dx = [zero(); 4];
            for k in 1..5 {
                dx[k - 1] = self.power[i][k].scale(-(k as f64))?;
            }
            self.residual[i] = f.add(&dx)?;
        }
        for (i, eq) in src.g.iter().enumerate() {
            self.residual[nx + i] = compose_bounded(eq, &clock, &self.power[..n])?;
        }
        work.compositions += n;
        Ok(SecantSlab {
            w: self,
            src,
            times,
            h,
        })
    }
}
impl Default for SecantCellWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
pub struct SecantSlab<'a> {
    w: &'a mut SecantCellWorkspace,
    src: &'a DaeSource,
    times: [f64; 2],
    h: Iv,
}
fn local_range(a: &[Iv], theta: Iv, bernstein: bool, work: &mut SecantCellWork) -> CoreResult<Iv> {
    if bernstein && theta.lo != theta.hi {
        // All callers use exact dyadic endpoints. Require exact subtraction
        // before using the existing normalized restriction kernel.
        let d = p(theta.hi)?.sub(p(theta.lo)?)?;
        if d.lo != d.hi {
            return Err(bad("secant subcell width not exact"));
        }
        work.bernstein_ranges += 1;
        range(conservation_restrict(a, theta.lo, d.lo)?.slice())
    } else {
        work.horner_ranges += 1;
        eval(a, theta)
    }
}
impl SecantSlab<'_> {
    pub fn point_into(&self, u: f64, out: &mut [Iv]) -> CoreResult<()> {
        if !(0. ..=1.).contains(&u) || out.len() != self.src.nx() + self.src.nz() {
            return Err(bad("secant endpoint shape"));
        }
        for (o, r) in out.iter_mut().zip(&self.w.power) {
            *o = eval(r, p(u)?)?;
        }
        Ok(())
    }
    /// Enclose h*Abar and normalized inhomogeneity on the full convex hull
    /// of the true and reference state. No branch-root contraction here:
    /// off-manifold points are required by the mean-value identity.
    /// The returned terminal covector and correction use the same averaged
    /// blocks, and are valid at a point cell as well.
    #[allow(clippy::too_many_arguments)]
    pub fn enclose(
        &mut self,
        theta: Iv,
        true_box: &[Iv],
        preconditioner: &[f64],
        weights: &[f64],
        bernstein: bool,
        diagonal: bool,
        a: &mut [Iv],
        rhs: &mut [Iv],
        goal: &mut [Iv],
        offset: &mut Iv,
        work: &mut SecantCellWork,
    ) -> CoreResult<()> {
        let nx = self.src.nx();
        let nz = self.src.nz();
        let n = nx + nz;
        if true_box.len() != n
            || weights.len() != n
            || a.len() != nx * nx
            || rhs.len() != nx
            || goal.len() != nx
            || theta.lo < 0.
            || theta.hi > 1.
            || weights.iter().any(|x| !x.is_finite())
        {
            return Err(bad("secant cell shape"));
        }
        Iv::new(theta.lo, theta.hi)?;
        let time = intersect(
            p(self.times[0])?.add(self.h.mul(theta)?)?,
            Iv::new(self.times[0], self.times[1])?,
        )?;
        let mut y = [zero(); 32];
        let mut res = [zero(); 32];
        for i in 0..n {
            let b = Iv::new(true_box[i].lo, true_box[i].hi)?;
            let v = local_range(&self.w.power[i], theta, bernstein, work)?;
            y[i] = Iv::new(b.lo.min(v.lo), b.hi.max(v.hi))?;
            res[i] = local_range(self.w.residual[i].slice(), theta, bernstein, work)?;
        }
        guard_source_rows(
            &self.src.g,
            time,
            &y[..n],
            &mut self.w.guard.values[..nz],
            &mut self.w.guard.jacobian[..nz],
        )?;
        guard_source_rows(
            &self.src.f,
            time,
            &y[..n],
            &mut self.w.guard.values[..nx],
            &mut self.w.fjac[..nx],
        )?;
        work.source_rows += n;
        let c = GuardMatrix::new(nz, preconditioner)?;
        let mut q = 0.;
        let mut gap = 1.;
        if diagonal {
            for i in 0..nz {
                for j in 0..nz {
                    let v = self.w.guard.jacobian[i][nx + 1 + j];
                    if i == j {
                        if v.contains_zero() {
                            return Err(bad("secant diagonal singular"));
                        }
                    } else if v != zero() {
                        return Err(bad("secant Gz not diagonal"));
                    }
                }
            }
        } else {
            q = guard_defect_into(
                c,
                &self.w.guard.jacobian[..nz],
                nx,
                &mut self.w.guard.defect[..nz * nz],
            )?;
            gap = sub_down(1., q)?;
            if gap <= 0. {
                return Err(bad("secant averaged inverse defect"));
            }
            work.max_inverse_defect = work.max_inverse_defect.max(q);
        }
        let mut input = [zero(); 32];
        let mut sol = [zero(); 32];
        let mut defect_sol = [zero(); 32];
        let mut count = di::AttemptCount::default();
        for k in 0..=nx {
            for i in 0..nz {
                input[i] = if k == nx {
                    -res[nx + i]
                } else {
                    -self.w.guard.jacobian[i][k + 1]
                };
            }
            if diagonal {
                for i in 0..nz {
                    sol[i] = if input[i] == zero() {
                        zero()
                    } else {
                        work.direct_divisions += 1;
                        input[i].div(self.w.guard.jacobian[i][nx + 1 + i])?
                    };
                }
            } else {
                guard_solve_into(
                    c,
                    &input[..nz],
                    q,
                    gap,
                    &mut sol[..nz],
                    &self.w.guard.defect[..nz * nz],
                    GuardColumnRefinement::AdaptiveGaussSeidel2,
                    &mut count,
                )?;
                work.guard_solves += 1;
            }
            if k == nx {
                defect_sol[..nz].copy_from_slice(&sol[..nz]);
            } else {
                for i in 0..nz {
                    self.w.zx[i * nx + k] = sol[i];
                }
            }
        }
        work.refinement_sweeps += count.started;
        let mut aa = [zero(); 1024];
        let mut rr = [zero(); 32];
        let mut cc = [zero(); 32];
        for i in 0..nx {
            for k in 0..nx {
                let v = (0..nz).try_fold(self.w.fjac[i][k + 1], |s, j| {
                    s.add(self.w.fjac[i][nx + 1 + j].mul(self.w.zx[j * nx + k])?)
                })?;
                aa[i * nx + k] = self.h.mul(v)?;
            }
            let v = (0..nz).try_fold(zero(), |s, j| {
                s.add(self.w.fjac[i][nx + 1 + j].mul(defect_sol[j])?)
            })?;
            rr[i] = res[i].add(self.h.mul(v)?)?;
            work.max_residual_upper = work.max_residual_upper.max(rr[i].mag());
        }
        for k in 0..nx {
            cc[k] = (0..nz).try_fold(p(weights[k])?, |s, j| {
                s.add(self.w.zx[j * nx + k].scale(weights[nx + j])?)
            })?;
        }
        let d = (0..nz).try_fold(zero(), |s, j| s.add(defect_sol[j].scale(weights[nx + j])?))?;
        a.copy_from_slice(&aa[..nx * nx]);
        rhs.copy_from_slice(&rr[..nx]);
        goal.copy_from_slice(&cc[..nx]);
        *offset = d;
        Ok(())
    }
}
