// Fixed-width scalar Reuse projection. Each call recomputes all source
// evaluations and certificates; no cached state, factor, or authority.
// The one-row faer 0.24.4 triangular base case is b * recip(a).
struct ScalarEval {
    values: [Iv; 32],
    jac: [[Iv; 33]; 32],
}
impl ScalarEval {
    fn new() -> Self {
        Self { values: [z();32], jac: [[z();33];32] }
    }
    fn eval(&mut self, src:&DaeSource,t:f64,x:&[f64],a:Iv)->CoreResult<()> {
        let nx=src.nx();
        let mut y=[z();32];
        for (v,&x) in y[..nx].iter_mut().zip(x) {*v=p(x)?;}
        y[nx]=a;
        let time=p(t)?;
        for (i,eq) in src.f.iter().chain(&src.g).enumerate(){
            self.values[i]=evaluate_row_bounded(eq,time,&y[..nx+1],&mut self.jac[i][..nx+2])?;
        }
        Ok(())
    }
}
/// Fixed-size, fully checked scalar Reuse projection result. It is not a
/// substitute for a slab certificate. Materialize the owning report only
/// when a caller actually needs persistent evidence.
pub struct ScalarReuseProjection {
    nx:usize, mode:InitMode,t:f64,x:[f64;31],seed:f64,center:f64,branch:[f64;2],root:Iv,
    correction:f64,original_residual:f64,residual:f64,state_error:f64,
    branch_defect:f64,local_defect:f64,inverse_norm:f64,condition:f64,
    xdot:[f64;31],sol:[f64;33],reduced:[f64;31*31],derivative_error:f64,tangent_residual:f64,
    work:InitWork,
}
impl ScalarReuseProjection {
    pub fn z(&self)->f64{self.center}
    pub fn zt(&self)->f64{self.sol[0]}
    pub fn xdot(&self)->&[f64]{&self.xdot[..self.nx]}
    pub fn reduced_jacobian(&self)->&[f64]{&self.reduced[..self.nx*self.nx]}
    pub fn work(&self)->&InitWork{&self.work}
    pub fn to_owned_report(&self)->InitReport{
        InitReport{
            route:InitRoute::Reuse, mode:self.mode,t:self.t,x:self.x[..self.nx].to_vec(),
            original_z:vec![self.seed],z:vec![self.center],correction_upper:self.correction,
            branch:vec![self.branch],root_box:vec![self.root.as_pair()],
            branch_authority:"whole-branch contraction + local self-map".to_string(),affine_root:None,
            original_residual_upper:self.original_residual,residual_upper:self.residual,
            state_error_upper:self.state_error,exact_center_consistency:self.residual==0.,
            branch_defect_upper:Some(self.branch_defect),local_defect_upper:self.local_defect,
            inverse_norm_upper:self.inverse_norm,condition_upper:self.condition,rank:1,
            xdot:self.xdot().to_vec(),zdot:vec![self.sol[self.nx+1]],zt:vec![self.sol[0]],
            zx_columns:self.sol[1..=self.nx].iter().map(|&x|vec![x]).collect(),
            reduced_jacobian:self.reduced_jacobian().chunks_exact(self.nx).map(|r|r.to_vec()).collect(),
            derivative_error_upper:self.derivative_error,tangent_residual_upper:self.tangent_residual,
            work:self.work.clone(),qualified:true,
        }
    }
}
fn scalar_batch(a:f64,rhs:&mut [f64],attempt:&mut InitAttemptWork)->CoreResult<()>{
    // Actual scalar linear batch, not an assertion that a general LU owner
    // was constructed. Preserve original batch and RHS accounting.
    attempt.legacy.linear_batches+=1;
    attempt.legacy.rhs_count+=rhs.len();
    attempt.rhs_requested+=rhs.len();
    let result=(|| {
        if !a.is_finite(){return Err(CoreError::NonFinite("matrix contains NaN/Inf".into()));}
        let inv=a.recip();
        for b in rhs.iter_mut(){
            if !b.is_finite(){return Err(CoreError::NonFinite("LU RHS contains NaN/Inf".into()));}
            *b *= inv;
            if !b.is_finite(){return Err(CoreError::LinearSolve("LU produced NaN/Inf".into()));}
        }
        Ok(())
    })();
    attempt.linear.record(result.is_ok());
    if result.is_ok(){attempt.rhs_returned+=rhs.len();}
    result
}
fn scalar_defect(c:f64,j:Iv)->CoreResult<Iv>{p(1.)?.sub(j.scale(c)?)}
impl DaeSource{
    /// Same scalar Reuse admission and interval proof as initialize(),
    /// with fixed-size results. Successful path makes no heap requests.
    /// Other initialization routes and vector blocks retain their own API.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_scalar_reuse_audited(&self,t:f64,x:&[f64],seed:&[f64],branch:&[[f64;2]],policy:InitPolicy)->(CoreResult<ScalarReuseProjection>,InitAttemptWork){
        let mut w=InitAttemptWork::default();
        let r=self.scalar_reuse_recorded(t,x,seed,branch,policy,&mut w);
        (r,w)
    }
    fn scalar_reuse_recorded(&self,t:f64,x:&[f64],seed:&[f64],branch:&[[f64;2]],policy:InitPolicy,w:&mut InitAttemptWork)->CoreResult<ScalarReuseProjection>{
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let nx=self.nx();
        if self.nz()!=1{return Err(bad("scalar Reuse requires nz=1"));}
        if !t.is_finite()||x.len()!=nx||seed.len()!=1||branch.len()!=1||x.iter().chain(seed).any(|v|!v.is_finite())
            ||[policy.state_tol,policy.residual_tol,policy.derivative_tol,policy.condition_cap].iter().any(|v|!v.is_finite()||*v<=0.)
            ||!(1..=64).contains(&policy.max_newton){return Err(bad("policy/input shape"));}
        let b=Iv::new(branch[0][0],branch[0][1])?;
        if b.lo>=b.hi||!b.contains(seed[0]){return Err(bad("invalid branch or initial guess outside"));}
        w.admitted=true;
        let zi=nx+1;
        let mut work=ScalarEval::new();
        let mut point=ScalarEval::new();
        macro_rules! eval {
            ($a:expr,$v:expr)=>{{
                let r=(||$a.eval(self,t,x,$v))();
                w.evaluations.record(r.is_ok());r?;w.legacy.evaluations+=1;
            }};
        }
        eval!(work,p(seed[0])?);
        let original_residual=work.values[nx].mag();
        if policy.mode==InitMode::Strict&&original_residual>policy.residual_tol{return Err(bad("strict initial residual exceeds tolerance"));}
        let mut center=seed[0];
        if policy.mode==InitMode::ProjectAlgebraic{
            for _ in 0..policy.max_newton{
                eval!(work,p(center)?);
                let r=work.values[nx].mag();
                if r==0.{break;}
                let a=mid(work.jac[nx][zi]);
                let mut d=[-mid(work.values[nx])];
                scalar_batch(a,&mut d,w)?;
                let mut accepted=false;
                for j in 0..16{
                    let trial=2f64.powi(-j).mul_add(d[0],center);
                    let q=p(trial)?;
                    if !b.contains(trial){continue;}
                    eval!(work,q);
                    if work.values[nx].mag()<r{
                        center=trial;accepted=true;w.legacy.newton_steps+=1;break;
                    }
                }
                if !accepted{break;}
            }
        }
        eval!(point,p(center)?);
        let residual=point.values[nx].mag();
        if residual>policy.residual_tol{return Err(bad("original constraint residual exceeds tolerance"));}
        let a=mid(point.jac[nx][zi]);
        let mut c=[1.];scalar_batch(a,&mut c,w)?;let c=c[0];
        eval!(work,b);
        let defect=scalar_defect(c,work.jac[nx][zi])?;
        let branch_defect=add_up(0.,defect.mag())?;
        let gap=sub_down(1.,branch_defect)?;
        if gap<=0.{return Err(bad("whole branch contraction not proved"));}
        let cg=z().add(point.values[nx].scale(c)?)?;
        let rad=mul_up(2.,div_up(cg.mag(),gap)?)?;
        let root=p(center)?.add(Iv::new(-rad,rad)?)?;
        if root.lo<b.lo||root.hi>b.hi{return Err(bad("existence box leaves declared branch"));}
        let delta=root.sub(p(center)?)?;
        let ed=z().add(defect.mul(delta)?)?;
        let mapped=p(center)?.sub(cg)?.add(ed)?;
        if mapped.lo<root.lo||mapped.hi>root.hi{return Err(bad("local self-map inclusion not proved"));}
        let state_error=distance(root,center)?;
        if state_error>policy.state_tol{return Err(bad("certified root distance exceeds tolerance"));}
        eval!(work,root);
        let j=work.jac[nx][zi];
        let local_defect=add_up(0.,scalar_defect(c,j)?.mag())?;
        let gap=sub_down(1.,local_defect)?;
        if gap<=0.{return Err(bad("rank/regularity not proved at root"));}
        let inverse_norm=div_up(add_up(0.,p(c)?.mag())?,gap)?;
        let condition=mul_up(inverse_norm,add_up(0.,j.mag())?)?;
        if condition>policy.condition_cap{return Err(bad("certified condition exceeds cap"));}
        let gc=&point.jac[nx];let gr=&work.jac[nx];
        let mut rhs=[z();33];
        for k in 0..=nx{rhs[k]=-gr[k];}
        rhs[nx+1]=gr[1..=nx].iter().zip(&work.values[..nx]).try_fold(-gr[0],|s,(&g,&f)|s.sub(g.mul(f)?))?;
        let mut xdot=[0.;31];for (v,&f) in xdot[..nx].iter_mut().zip(&point.values[..nx]){*v=mid(f);}
        let mut sol=[0.;33];
        for k in 0..=nx{sol[k]=-mid(gc[k]);}
        sol[nx+1]=mid(gc[1..=nx].iter().zip(&xdot[..nx]).try_fold(-gc[0],|s,(&g,&f)|s.sub(g.scale(f)?))?);
        scalar_batch(a,&mut sol[..nx+2],w)?;
        let mut enc=[z();33];
        let mut derivative_error=0_f64;
        for k in 0..nx+2{
            let av=z().add(j.mul(p(sol[k])?)?)?;
            let residual=rhs[k].sub(av)?;
            let pc=z().add(residual.scale(c)?)?;
            let e=div_up(pc.mag(),gap)?;
            derivative_error=derivative_error.max(e);
            enc[k]=p(sol[k])?.add(Iv::new(-e,e)?)?;
        }
        for (&v,&f) in xdot[..nx].iter().zip(&work.values[..nx]){derivative_error=derivative_error.max(distance(f,v)?);}
        let mut reduced=[0.;31*31];
        for i in 0..nx{
            for k in 0..nx{
                let v=point.jac[i][k+1].add(point.jac[i][zi].scale(sol[k+1])?)?;
                let exact=work.jac[i][k+1].add(work.jac[i][zi].mul(enc[k+1])?)?;
                let m=mid(v);reduced[i*nx+k]=m;derivative_error=derivative_error.max(distance(exact,m)?);
            }
        }
        let tangent_residual=gc[1..=nx].iter().zip(&xdot[..nx]).chain(gc[zi..=zi].iter().zip(&sol[nx+1..nx+2])).try_fold(gc[0],|s,(&a,&v)|s.add(a.scale(v)?))?.mag();
        if derivative_error>policy.derivative_tol||tangent_residual>policy.derivative_tol{return Err(bad("derivative/Schur certificate exceeds tolerance"));}
        let correction=p(center)?.sub(p(seed[0])?)?.mag();
        let mut xx=[0.;31];xx[..nx].copy_from_slice(x);
        Ok(ScalarReuseProjection{nx,mode:policy.mode,t,x:xx,seed:seed[0],center,branch:branch[0],root,correction,original_residual,residual,state_error,branch_defect,local_defect,inverse_norm,condition,xdot,sol,reduced,derivative_error,tangent_residual,work:w.legacy.clone()})
    }
}
