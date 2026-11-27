from pathlib import Path
p=Path('crates/rodas5p-integrators/src/retro_dae_evolution.rs');s=p.read_text();a='mod schur_adjoint;';assert s.count(a)==1;s=s.replace(a,a+'\nmod scalar_adjoint_jet;\npub use scalar_adjoint_jet::{ScalarAdjointJet,ScalarJetWork,scalar_adjoint_jet,scalar_autonomous_source};');p.write_text(s)
p=Path('crates/vigilode/src/dae_evolution/retained/slabs/nonlinear_adjoint.rs');s=p.read_text()
a='use super::*;';assert s.count(a)==1;s=s.replace(a,a+'\nmod scalar;')
a=s.index('fn bound_cell(');b=s.index('    let nx = src.nx();',a);c=s.index('    let refinement =',b)
sig=s[a:b];body=s[b:c]
s=s[:a]+'''fn physical_cell_box(src:&DaeSource,s:&CompactSlabView<'_>,theta:Iv,local:bool)->CoreResult<(Iv,Iv,[Iv;32])>{
'''+body+'''    Ok((h,time,y))
}
'''+sig+'''    let n=src.nx()+src.nz();
    let c=s.certificate.as_ref().ok_or_else(||bad("adjoint certificate"))?;
    let (h,time,y)=physical_cell_box(src,s,theta,local)?;
'''+s[c:]
a=s.index('    /// RED compatibility delegate, intentionally not optimized.');b=s.index('    /// Numerical proposal using the original projection/Jacobian',a);s=s[:a]+s[b:]
a='    FlowRatio,\n}';assert s.count(a)==1;s=s.replace(a,'    FlowRatio,\n    QuadraticSeed16,\n}')
a='    pub cap_leaves: usize,';assert s.count(a)==1;s=s.replace(a,a+'\n    pub quadratic_seed_uses:usize,')
p.write_text(s)
p=Path('crates/rodas5p-integrators/src/dopri5.rs');s=p.read_text()
a='pub struct DopriStageWork {'
# no extra serialization fields for old logical counts
a='#[derive(Debug)]\npub struct Dopri5Workspace'
assert s.count(a)==1
s=s.replace(a,'''// Bounded scratch with contiguous derivative lanes and one destination write
// per component. Logical destination writes do not count local scratch stores.
#[inline]
fn block_accumulate<const B:usize>(base:Option<&[f64]>,out:&mut[f64],stages:&[f64],coeff:&[f64],h:f64,tiny:bool){
    let n=out.len();let mut weights=[0.;7];
    for (i,&c) in coeff.iter().enumerate(){weights[i]=h*c;}
    let mut offset=0;
    while offset<n{
        let len=B.min(n-offset);let mut v=[0.;B];
        if let Some(y)=base{v[..len].copy_from_slice(&y[offset..offset+len]);}
        for (j,&c)in coeff.iter().enumerate(){
            if c==0.{continue;}
            let k=&stages[j*n+offset..j*n+offset+len];
            if tiny{for (v,&k)in v[..len].iter_mut().zip(k){*v+=c*(h*k);}}
            else{let weight=weights[j];for (v,&k)in v[..len].iter_mut().zip(k){*v+=weight*k;}}
        }
        out[offset..offset+len].copy_from_slice(&v[..len]);offset+=len;
    }
}
impl DopriStageKernel{
    fn blocked(self)->bool{matches!(self,Self::Block4|Self::Block8|Self::Block16|Self::Block32)}
    fn accumulate(self,base:Option<&[f64]>,out:&mut[f64],stages:&[f64],coeff:&[f64],h:f64,tiny:bool){
        match self{
            Self::Block4=>block_accumulate::<4>(base,out,stages,coeff,h,tiny),
            Self::Block8=>block_accumulate::<8>(base,out,stages,coeff,h,tiny),
            Self::Block16=>block_accumulate::<16>(base,out,stages,coeff,h,tiny),
            Self::Block32=>block_accumulate::<32>(base,out,stages,coeff,h,tiny),
            _=>unreachable!(),
        }
    }
}
'''+a)
a='            if self.kernel == DopriStageKernel::Components {';assert s.count(a)==1
s=s.replace(a,'''            if self.kernel.blocked(){
                let c=&DOPRI5_A[stage][..stage];
                self.kernel.accumulate(Some(y),&mut self.stage_state,&self.stages,c,h,tiny_step);
                self.stage_work.state_writes+=self.n as u64;
                self.stage_work.linear_products+=self.n as u64*c.iter().filter(|&&v|v!=0.).count()as u64;
            }else if self.kernel == DopriStageKernel::Components {''')
a='        if self.kernel == DopriStageKernel::Components {';assert s.count(a)==1
s=s.replace(a,'''        if self.kernel.blocked(){
            self.kernel.accumulate(None,error,&self.stages,&ERROR_WEIGHTS,h,tiny_step);
            self.stage_work.error_writes+=self.n as u64;
            self.stage_work.linear_products+=self.n as u64*ERROR_WEIGHTS.iter().filter(|&&v|v!=0.).count()as u64;
        }else if self.kernel == DopriStageKernel::Components {''')
p.write_text(s)
