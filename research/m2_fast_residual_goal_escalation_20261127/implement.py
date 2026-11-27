from pathlib import Path
def patch(p,a,b):
 s=p.read_text();assert s.count(a)==1,(p,a,s.count(a));p.write_text(s.replace(a,b))
p=Path('crates/vigilode/src/spatial/shifted_adjoint.rs')
patch(p,'    solve_gmres_into, solve_gmres_into_right_monitored,','    solve_gmres_into, solve_gmres_into_right_monitored,solve_gmres_into_right_flexible_triggered_monitored,')
patch(p,'    FourierTranspose,\n','    FourierTranspose,\n    TriggeredIdentity,\n    TriggeredLongitude,\n')
patch(p,'    pub seed_accepts: u64,\n}\nfn zero_count','    pub seed_accepts: u64,\n    #[serde(skip_serializing_if="Option::is_none")]\n    pub residual_profile:Option<ResidualProfile>,\n}\nfn zero_count')
patch(p,'matches!(self, Self::EarlyIdentity | Self::EarlyLongitude)','matches!(self, Self::EarlyIdentity | Self::EarlyLongitude|Self::TriggeredIdentity|Self::TriggeredLongitude)')
patch(p,'    fn direct(self) -> bool {','    fn triggered(self)->bool{matches!(self,Self::TriggeredIdentity|Self::TriggeredLongitude)}\n    fn direct(self) -> bool {')
patch(p,'    fourier: Option<Box<super::separable::TransposeKernel>>,','    fourier: Option<Box<super::separable::TransposeKernel>>,\n    residual:Option<Box<ResidualWorkspace>>,')
patch(p,'        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;\n        let n = grid.dimension();','''        Self::new_impl(grid,route,None)
    }
    /// Opt-in original-source residual algorithm for direct Fourier stages.
    pub fn new_with_residual(grid:&DiffusionGrid,route:ShiftedAdjointRoute,plan:ResidualPlan)->CoreResult<Self>{
        if !route.direct(){return Err(err("residual policy requires direct Fourier"));}
        Self::new_impl(grid,route,Some(plan))
    }
    fn new_impl(grid:&DiffusionGrid,route:ShiftedAdjointRoute,residual:Option<ResidualPlan>)->CoreResult<Self>{
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = grid.dimension();''')
patch(p,'                | ShiftedAdjointRoute::EarlyLongitude\n','                | ShiftedAdjointRoute::EarlyLongitude\n                | ShiftedAdjointRoute::TriggeredLongitude\n')
patch(p,'            res: vec![Z; n],','            res: if residual.is_none(){vec![Z;n]}else{Vec::new()},')
patch(p,'            fourier,\n        })','            fourier,\n            residual:residual.map(|p|ResidualWorkspace::new(grid,p).map(Box::new)).transpose()?,\n        })')
patch(p,'        self.z = Some(z);\n        Ok(())','        if let Some(r)=self.residual.as_mut(){r.prepare(z)?;}\n        self.z = Some(z);\n        Ok(())')
patch(p,'''            self.x.copy_from_slice(b);
            k.solve(&mut self.x);
            c.direct_solve_calls += 1;
            let (rho, upper, rhs) = directed_residual(g, z, b, &self.x, &mut self.res)?;''','''            self.x.copy_from_slice(b);
            let start=self.residual.as_ref().map(|_|std::time::Instant::now());
            k.solve(&mut self.x);
            c.direct_solve_calls += 1;
            if let Some(r)=self.residual.as_mut(){
                let solve_ns=elapsed_ns(start.unwrap());
                let start=std::time::Instant::now();
                let a=r.evaluate(b,&self.x)?;
                let certificate_ns=elapsed_ns(start);
                if !a.admitted{return Err(err("transpose Fourier residual exceeds stage budget"));}
                c.linear_solves+=1;
                return Ok(ShiftedAdjointReport{residual_l1:a.residual_l1,error_l1:a.error_l1,rhs_l1:a.rhs_l1,directed_checks:a.interval_checks,residual_profile:Some(ResidualProfile{solve_ns,certificate_ns,fast_checks:a.fast_checks,interval_checks:a.interval_checks,fallbacks:a.fallbacks,fmas:a.fmas,max_rounding:a.rounding_upper}),..Default::default()});
            }
            let (rho, upper, rhs) = directed_residual(g, z, b, &self.x, &mut self.res)?;''')
patch(p,'''            let r = solve_gmres_into_right_monitored(
                &op,
                &Right(pc),
                b,
                &self.seed,
                &self.config,
                &mut self.x,
                &mut self.gmres,
                c,
                &mut monitor,
            )?;''','''            let r=if self.route.triggered(){
                let projected=allowed/(2.*(n as f64).sqrt());
                solve_gmres_into_right_flexible_triggered_monitored(&op,&Right(pc),b,&self.seed,&self.config,projected,self.capacity,&mut self.x,&mut self.gmres,c,&mut monitor)?
            }else{
                solve_gmres_into_right_monitored(&op,&Right(pc),b,&self.seed,&self.config,&mut self.x,&mut self.gmres,c,&mut monitor)?
            };''')
patch(p,'                seed_accepts: 0,\n            });','                seed_accepts: 0,\n                ..Default::default()\n            });')
patch(p,'''    pub seed_accepts: u64,
}
struct Client''','''    pub seed_accepts: u64,
    #[serde(skip_serializing_if="Option::is_none")]
    pub residual_profile:Option<ResidualProfile>,
}
/// Opt-in profiling, successful returned stage solves only.
#[derive(Clone,Copy,Debug,Default,Serialize)]
pub struct ResidualProfile{
 pub solve_ns:u64,pub certificate_ns:u64,pub fast_checks:u64,pub interval_checks:u64,pub fallbacks:u64,pub fmas:u64,pub max_rounding:f64
}
fn elapsed_ns(t:std::time::Instant)->u64{t.elapsed().as_nanos().min(u64::MAX as u128) as u64}
impl ResidualProfile{
 fn add(&mut self,r:Self){self.solve_ns=self.solve_ns.saturating_add(r.solve_ns);self.certificate_ns=self.certificate_ns.saturating_add(r.certificate_ns);self.fast_checks+=r.fast_checks;self.interval_checks+=r.interval_checks;self.fallbacks+=r.fallbacks;self.fmas+=r.fmas;self.max_rounding=self.max_rounding.max(r.max_rounding);}
}
struct Client''')
patch(p,'        w.seed_accepts += r.seed_accepts;\n        Ok(())','        w.seed_accepts += r.seed_accepts;\n        if let Some(p)=r.residual_profile{w.residual_profile.get_or_insert_with(Default::default).add(p);}\n        Ok(())')
patch(p,'''    let g = grid.clone();
    let j = grid.clone();
    let p = OdeProblem::new(''','''    solve_spatial_adjoint_impl(grid,span,initial,adaptive,route,None)
}
pub fn solve_spatial_adjoint_shifted_with_residual(grid:&DiffusionGrid,span:(f64,f64),initial:&[f64],adaptive:&AdaptiveStepConfig,plan:ResidualPlan)->CoreResult<(Rodas5pFastResult,ShiftedNativeWork)>{
    solve_spatial_adjoint_impl(grid,span,initial,adaptive,ShiftedAdjointRoute::FourierTranspose,Some(plan))
}
fn solve_spatial_adjoint_impl(grid:&DiffusionGrid,span:(f64,f64),initial:&[f64],adaptive:&AdaptiveStepConfig,route:ShiftedAdjointRoute,residual:Option<ResidualPlan>)->CoreResult<(Rodas5pFastResult,ShiftedNativeWork)>{
    let g = grid.clone();
    let j = grid.clone();
    let p = OdeProblem::new(''')
patch(p,'        w: ShiftedAdjointWorkspace::new(grid, route)?,','        w: ShiftedAdjointWorkspace::new_impl(grid,route,residual)?,')
# use fixed inline buffer in the existing triggered algorithm for <=32 columns;
# larger callers retain previous heap behavior. No change in arithmetic.
p=Path('crates/rodas5p-krylov/src/gmres_into_triggered.rs')
patch(p,'''struct ProjectedGivens {
    cosine: Vec<f64>,
    sine: Vec<f64>,
    rotated_rhs: Vec<f64>,
    column: Vec<f64>,
}''','''enum ProjectionBuffer {
    Inline([f64;33],usize),
    Heap(Vec<f64>),
}
impl ProjectionBuffer{
    fn new(n:usize)->Self{if n<=33{Self::Inline([0.;33],n)}else{Self::Heap(vec![0.;n])}}
}
impl std::ops::Deref for ProjectionBuffer{
    type Target=[f64];
    fn deref(&self)->&[f64]{match self{Self::Inline(a,n)=>&a[..*n],Self::Heap(v)=>v}}
}
impl std::ops::DerefMut for ProjectionBuffer{
    fn deref_mut(&mut self)->&mut[f64]{match self{Self::Inline(a,n)=>&mut a[..*n],Self::Heap(v)=>v}}
}
struct ProjectedGivens {
    cosine: ProjectionBuffer,
    sine: ProjectionBuffer,
    rotated_rhs: ProjectionBuffer,
    column: ProjectionBuffer,
}''')
patch(p,'let mut rotated_rhs = vec![0.0; columns + 1];','let mut rotated_rhs = ProjectionBuffer::new(columns+1);')
patch(p,'''            cosine: vec![0.0; columns],
            sine: vec![0.0; columns],
            rotated_rhs,
            column: vec![0.0; columns + 1],''','''            cosine:ProjectionBuffer::new(columns),
            sine:ProjectionBuffer::new(columns),
            rotated_rhs,
            column:ProjectionBuffer::new(columns+1),''')
p=Path('crates/vigilode/src/dae_evolution/retained/slabs/goal_budget.rs')
patch(p,'    let mut scratch: Option<Box<SecantGoalWorkspace>> = None;','''    if budget.plan==GoalBudgetPlan::TieredCost{
        mult=if budget.absolute>=nominal{64.}else if budget.absolute>=nominal/8.{8.}else{1.};
    }
    let mut scratch: Option<Box<SecantGoalWorkspace>> = None;''')
patch(p,'if budget.plan == GoalBudgetPlan::CostAware\n                                    && mult > 1.','if matches!(budget.plan,GoalBudgetPlan::CostAware|GoalBudgetPlan::TieredCost)\n                                    && mult > 1.')
patch(p,'|| (budget.plan == GoalBudgetPlan::CostAware\n                                                &&','|| (matches!(budget.plan,GoalBudgetPlan::CostAware|GoalBudgetPlan::TieredCost)\n                                                &&')
patch(p,'} else if budget.plan == GoalBudgetPlan::CostAware {','} else if matches!(budget.plan,GoalBudgetPlan::CostAware|GoalBudgetPlan::TieredCost) {')
