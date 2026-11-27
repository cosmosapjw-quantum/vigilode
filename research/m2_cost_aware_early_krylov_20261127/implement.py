from pathlib import Path
def patch(p,a,b):
 s=p.read_text();assert s.count(a)==1,(p,a,s.count(a));p.write_text(s.replace(a,b))
p=Path('crates/vigilode/src/spatial/separable.rs')
patch(p,'''/// Mode-wise Thomas factors of the sphere's stage matrices.''','''/// Reuse the primal modal factors to solve the actual transpose. This
/// is a source-bound numerical proposal; its directed residual is checked
/// by the shifted-adjoint client before it can be returned.
pub(super) struct TransposeKernel(Kernel);
impl TransposeKernel {
    pub(super) fn new(grid:&DiffusionGrid)->Self{
        Self(Kernel::new(grid,BoxTransform::PairedReal,None))
    }
    pub(super) fn prepare(&mut self,z:f64)->CoreResult<()>{
        self.0.prepare(z,0.)
    }
    pub(super) fn solve(&mut self,b:&mut[f64]){
        match &mut self.0{
            Kernel::Sphere(k)=>k.solve_orientation(b,true),
            Kernel::Box(k)=>k.solve(b),
        }
    }
}

/// Mode-wise Thomas factors of the sphere's stage matrices.''')
patch(p,'''    fn solve(&mut self, b: &mut [f64]) {
        let (nt, np, modes) = (self.nt, self.np, self.modes);''','''    fn solve(&mut self, b: &mut [f64]) {
        self.solve_orientation(b,false);
    }
    fn solve_orientation(&mut self,b:&mut[f64],transpose:bool){
        let (nt, np, modes) = (self.nt, self.np, self.modes);''')
patch(p,'''            for r in 1..nt {
                let prev = col[r - 1];
                col[r] -= prev * mult[r];
            }
            col[nt - 1] /= pivot[nt - 1];
            for r in (0..nt - 1).rev() {
                let next = col[r + 1];
                col[r] = (col[r] + next * self.south[r]) / pivot[r];
            }''','''            if transpose {
                // W=L_unit U, so W^T=U^T L_unit^T.
                // Original U has upper entries -south[r], and L_unit
                // has lower entries mult[r]. No area similarity.
                col[0]/=pivot[0];
                for r in 1..nt{
                    let prev=col[r-1];
                    col[r]=(col[r]+prev*self.south[r-1])/pivot[r];
                }
                for r in (0..nt-1).rev(){
                    let next=col[r+1];
                    col[r]-=next*mult[r+1];
                }
            }else{
                for r in 1..nt {
                    let prev = col[r - 1];
                    col[r] -= prev * mult[r];
                }
                col[nt - 1] /= pivot[nt - 1];
                for r in (0..nt - 1).rev() {
                    let next = col[r + 1];
                    col[r] = (col[r] + next * self.south[r]) / pivot[r];
                }
            }''')
p=Path('crates/vigilode/src/spatial/shifted_adjoint.rs')
patch(p,'''    ClosureOperator, CoreError, CoreResult, IdentityPreconditioner, Preconditioner, WorkCounters,''','''    ClosureOperator, CoreError, CoreResult, IdentityPreconditioner, Preconditioner, WorkCounters,
    PreconditionerSide,ApplyCategory,apply_counted,''')
patch(p,'''    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,''','''    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
    GmresMonitorDecision,solve_gmres_into_right_monitored,''')
patch(p,'''    pub pc_apps: u64,
}''','''    pub pc_apps: u64,
    #[serde(skip_serializing_if="zero_count")]
    pub screen_checks:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub directed_checks:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub prefix_solves:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub seed_accepts:u64,
}
fn zero_count(x:&u64)->bool{*x==0}
struct Right<'a>(&'a dyn Preconditioner);
impl Preconditioner for Right<'_>{
    fn dimension(&self)->usize{self.0.dimension()}
    fn apply(&self,x:&[f64],y:&mut[f64])->CoreResult<()>{self.0.apply(x,y)}
    fn is_identity(&self)->bool{self.0.is_identity()}
    fn side(&self)->PreconditionerSide{PreconditionerSide::Right}
}
impl ShiftedAdjointRoute{
    fn early(self)->bool{matches!(self,Self::EarlyIdentity|Self::EarlyLongitude)}
    fn direct(self)->bool{self==Self::FourierTranspose}
}''')
patch(p,'''    res: Vec<Iv>,
}''','''    res: Vec<Iv>,
    seed:Vec<f64>,
    probe:Vec<f64>,
    fourier:Option<Box<super::separable::TransposeKernel>>,
}''')
patch(p,'''        let columns = 32.min(n);
        let mut gmres = GmresWorkspace::with_ls_workspace();
        gmres.set_givens_least_squares(true);
        gmres.reserve(n, columns)?;''','''        let columns = (if route==ShiftedAdjointRoute::ShortIdentity{8}else{32}).min(n);
        let mut gmres=GmresWorkspace::default();
        if !route.direct(){
            gmres=GmresWorkspace::with_ls_workspace();
            gmres.set_givens_least_squares(true);
            gmres.reserve(n,columns)?;
        }
        let fourier=if route.direct(){Some(Box::new(super::separable::TransposeKernel::new(grid)))}else{None};''')
patch(p,'''            res: vec![Z; n],
        })''','''            res: vec![Z; n],
            seed:if route.early(){vec![0.;n]}else{Vec::new()},
            probe:if route.early(){vec![0.;n]}else{Vec::new()},
            fourier,
        })''')
patch(p,'''        self.z = Some(z);
        Ok(())''','''        if let Some(k)=self.fourier.as_mut(){k.prepare(z)?;}
        self.z = Some(z);
        Ok(())''')
patch(p,'''        let g = &self.grid;
        let op = ClosureOperator::new(n,''','''        let g = &self.grid;
        if let Some(k)=self.fourier.as_mut(){
            self.x.copy_from_slice(b);
            k.solve(&mut self.x);
            c.direct_solve_calls+=1;
            let (rho,upper,rhs)=directed_residual(g,z,b,&self.x,&mut self.res)?;
            if rho>add_up(mul_up(1e-10,rhs)?,1e-13)?{return Err(err("transpose Fourier residual exceeds stage budget"));}
            c.linear_solves+=1;
            return Ok(ShiftedAdjointReport{residual_l1:rho,error_l1:upper,rhs_l1:rhs,directed_checks:1,..Default::default()});
        }
        let op = ClosureOperator::new(n,''')
patch(p,'''        let r = solve_gmres_into(
''','''        if self.route.early(){
            let before=*c;
            let rhs=b.iter().try_fold(0.,|s,x|add_up(s,x.abs()))?;
            let allowed=add_up(mul_up(1e-10,rhs)?,1e-13)?;
            for (x,&v) in self.seed.iter_mut().zip(b){*x=v/z;}
            // An approximate inverse-shift seed is only a proposal. Its
            // exact original residual may already meet the target.
            let mut checks=1u64;let mut screens=0u64;
            let seed=directed_residual(g,z,b,&self.seed,&mut self.res)?;
            if seed.0<=allowed{
                self.x.copy_from_slice(&self.seed);c.linear_solves+=1;
                return Ok(ShiftedAdjointReport{residual_l1:seed.0,error_l1:seed.1,rhs_l1:rhs,directed_checks:checks,seed_accepts:1,..Default::default()});
            }
            let mut receipt=None;
            let mut monitor=|_column:u64,v:&[f64],c:&mut WorkCounters|{
                screens+=1;
                apply_counted(&op,v,&mut self.probe,c,ApplyCategory::Diagnostic)?;
                let proposal=b.iter().zip(&self.probe).map(|(&b,&a)|(b-a).abs()).sum::<f64>();
                // This screen can decline a good candidate, but cannot admit
                // a bad one. The directed original-source gate is authority.
                if proposal<=0.75*allowed{
                    checks+=1;
                    let r=directed_residual(g,z,b,v,&mut self.res)?;
                    if r.0<=allowed{receipt=Some(r);return Ok(GmresMonitorDecision::Accept);}
                }
                Ok(GmresMonitorDecision::Continue)
            };
            let r=solve_gmres_into_right_monitored(&op,&Right(pc),b,&self.seed,&self.config,&mut self.x,&mut self.gmres,c,&mut monitor)?;
            if !r.accepted{return Err(err("monitored original residual target exhausted"));}
            let (rho,upper,_)=receipt.ok_or_else(||err("missing monitored certificate"))?;
            return Ok(ShiftedAdjointReport{residual_l1:rho,error_l1:upper,rhs_l1:rhs,iterations:r.iterations,matvecs:c.linear_matvecs-before.linear_matvecs,pc_apps:c.preconditioner_apps-before.preconditioner_apps,screen_checks:screens,directed_checks:checks,prefix_solves:r.least_squares_solves,seed_accepts:0});
        }
        let r = solve_gmres_into(
''')
patch(p,'''            pc_apps: r.preconditioner_apps,
        })''','''            pc_apps: r.preconditioner_apps,
            ..Default::default()
        })''')
patch(p,'''    pub max_residual_l1: f64,
}''','''    pub max_residual_l1: f64,
    #[serde(skip_serializing_if="zero_count")]
    pub screen_checks:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub directed_checks:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub prefix_solves:u64,
    #[serde(skip_serializing_if="zero_count")]
    pub seed_accepts:u64,
}''')
patch(p,'''        if self.w.line.is_some() {
            c.direct_factorizations += 1;
        }''','''        if self.w.line.is_some()||self.w.route.direct() {
            c.direct_factorizations += 1;
        }''')
patch(p,'''        w.max_residual_l1 = w.max_residual_l1.max(r.residual_l1);
        Ok(())''','''        w.max_residual_l1 = w.max_residual_l1.max(r.residual_l1);
        w.screen_checks+=r.screen_checks;w.directed_checks+=r.directed_checks;
        w.prefix_solves+=r.prefix_solves;w.seed_accepts+=r.seed_accepts;
        Ok(())''')
p=Path('crates/vigilode/src/dae_evolution/retained/slabs/goal_budget.rs')
patch(p,'''    let mut mult = if budget.plan == GoalBudgetPlan::Fixed {
        1.
    } else {
        64.
    };''','''    // Dimensionful comparison in goal units; not a certified error model.
    let scale=x.iter().chain(z).map(|v|v.abs()).fold(0.,f64::max);
    let nominal=64.*(atol+opts.rtol*scale)*norm;
    let mut mult=if budget.plan==GoalBudgetPlan::Fixed||
        (budget.plan==GoalBudgetPlan::CostAware&&budget.absolute<nominal){1.}else{64.};''')
patch(p,'''                                let mut iv = Iv::point(0.)?;''','''                                if budget.plan==GoalBudgetPlan::CostAware&&mult>1.&&matches!(route,SecantGoalRoute::Sub4Horner|SecantGoalRoute::Scaled16){continue;}
                                let mut iv = Iv::point(0.)?;''')
patch(p,'''                                        if bound.unwrap().mag() <= budget.absolute {
                                            break;
                                        }''','''                                        let b=bound.unwrap();
                                        if b.mag() <= budget.absolute || (budget.plan==GoalBudgetPlan::CostAware&&
                                            (b.lo>budget.absolute||b.hi< -budget.absolute)){
                                            break;
                                        }''')
patch(p,'''        } else {
            (mult / 8.).max(1.)
        };''','''        } else if budget.plan==GoalBudgetPlan::CostAware{
            1.
        }else{
            (mult / 8.).max(1.)
        };''')
