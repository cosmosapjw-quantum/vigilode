//! Conditional original-source Schur enclosures for a caller-proved box.
//! The facade binds these boxes to the actual qualified primal trajectory.
use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SchurAdjointWork {
    pub source_rows: usize,
    pub column_solves: usize,
    pub root_passes: usize,
    pub refinement_sweeps: usize,
}
pub struct SchurAdjointWorkspace {
    guard: GuardColumnWorkspace,
    fjac: [[Iv; 33]; 32],
    zx: [Iv; 1024],
    state: [Iv; 32],
}
impl SchurAdjointWorkspace {
    pub fn new() -> Self {
        Self {
            guard: GuardColumnWorkspace::new(),
            fjac: [[zero(); 33]; 32],
            zx: [zero(); 1024],
            state: [zero(); 32],
        }
    }
    /// On the supplied time/state box enclose A=f_x+f_z Z_x and
    /// c=w_x+Z_x^T w_z. Input box must contain the true consistent state.
    /// Root narrowing preserves any consistent roots already in the box;
    /// it is not a new existence proof. All output committed on success.
    #[allow(clippy::too_many_arguments)]
    pub fn enclose(
        &mut self,
        src: &DaeSource,
        time: Iv,
        state: &[Iv],
        preconditioner: &[f64],
        weights: &[f64],
        passes: usize,
        refinement: GuardColumnRefinement,
        a: &mut [Iv],
        goal: &mut [Iv],
        work: &mut SchurAdjointWork,
    ) -> CoreResult<()> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx = src.nx();
        let nz = src.nz();
        let n = nx + nz;
        if state.len() != n
            || weights.len() != n
            || a.len() != nx * nx
            || goal.len() != nx
            || passes > 2
            || weights.iter().any(|x| !x.is_finite())
        {
            return Err(bad("adjoint Schur shape/policy"));
        }
        for v in state.iter().chain(std::iter::once(&time)) {
            Iv::new(v.lo, v.hi)?;
        }
        let c = GuardMatrix::new(nz, preconditioner)?;
        self.state[..n].copy_from_slice(state);
        for _ in 0..passes {
            let mut center = self.state;
            let mut zc = [0.; 32];
            for i in 0..nz {
                zc[i] = di::mid(self.state[nx + i]);
                center[nx + i] = p(zc[i])?;
            }
            guard_source_rows(
                &src.g,
                time,
                &center[..n],
                &mut self.guard.values[..nz],
                &mut self.guard.jacobian[..nz],
            )?;
            work.source_rows += nz;
            let mut cg = [zero(); 32];
            for (i, o) in cg[..nz].iter_mut().enumerate() {
                *o = c
                    .row(i)
                    .iter()
                    .zip(&self.guard.values[..nz])
                    .try_fold(zero(), |s, (&v, &u)| s.add(u.scale(v)?))?;
            }
            guard_source_rows(
                &src.g,
                time,
                &self.state[..n],
                &mut self.guard.values[..nz],
                &mut self.guard.jacobian[..nz],
            )?;
            work.source_rows += nz;
            guard_defect_into(
                c,
                &self.guard.jacobian[..nz],
                nx,
                &mut self.guard.defect[..nz * nz],
            )?;
            let mut image = [zero(); 32];
            slab_fixed_image(
                &cg[..nz],
                &zc[..nz],
                &self.state[nx..n],
                &self.guard.defect[..nz * nz],
                &mut image[..nz],
            )?;
            for i in 0..nz {
                self.state[nx + i] = intersect(self.state[nx + i], image[i])?;
            }
            work.root_passes += 1;
        }
        guard_source_rows(
            &src.g,
            time,
            &self.state[..n],
            &mut self.guard.values[..nz],
            &mut self.guard.jacobian[..nz],
        )?;
        work.source_rows += nz;
        let q = guard_defect_into(
            c,
            &self.guard.jacobian[..nz],
            nx,
            &mut self.guard.defect[..nz * nz],
        )?;
        let gap = sub_down(1., q)?;
        if gap <= 0. {
            return Err(bad("adjoint uniform algebraic inverse defect"));
        }
        let mut rhs = [zero(); 32];
        let mut sol = [zero(); 32];
        let mut count = di::AttemptCount::default();
        for k in 0..nx {
            for i in 0..nz {
                rhs[i] = -self.guard.jacobian[i][k + 1];
            }
            guard_solve_into(
                c,
                &rhs[..nz],
                q,
                gap,
                &mut sol[..nz],
                &self.guard.defect[..nz * nz],
                refinement,
                &mut count,
            )?;
            work.column_solves += 1;
            for i in 0..nz {
                self.zx[i * nx + k] = sol[i];
            }
        }
        work.refinement_sweeps += count.started;
        guard_source_rows(
            &src.f,
            time,
            &self.state[..n],
            &mut self.guard.values[..nx],
            &mut self.fjac[..nx],
        )?;
        work.source_rows += nx;
        let mut aa = [zero(); 1024];
        let mut gg = [zero(); 32];
        for i in 0..nx {
            for k in 0..nx {
                aa[i * nx + k] = (0..nz).try_fold(self.fjac[i][k + 1], |s, j| {
                    s.add(self.fjac[i][nx + 1 + j].mul(self.zx[j * nx + k])?)
                })?;
            }
        }
        for k in 0..nx {
            gg[k] = (0..nz).try_fold(p(weights[k])?, |s, j| {
                s.add(self.zx[j * nx + k].scale(weights[nx + j])?)
            })?;
        }
        a.copy_from_slice(&aa[..nx * nx]);
        goal.copy_from_slice(&gg[..nx]);
        Ok(())
    }
}
impl Default for SchurAdjointWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
