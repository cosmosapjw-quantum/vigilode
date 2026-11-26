
#[cfg(test)]
mod bounded_polynomial_scratch_tests {
    use super::*;
    use crate::retro_event_poly::{AffineFactor as F, EventTerm as T};
    fn same(a:&[Iv],b:&[Iv]){
        assert_eq!(a.len(),b.len());
        for (x,y) in a.iter().zip(b){assert_eq!(x.lo.to_bits(),y.lo.to_bits());assert_eq!(x.hi.to_bits(),y.hi.to_bits());}
    }
    fn old_restrict(a:&[Iv],l:f64,w:f64)->CoreResult<Poly>{
        let mut out=vec![*a.last().unwrap()];
        let v=[p(l)?,p(w)?];
        for &c in a[..a.len()-1].iter().rev(){out=add(&mul(&out,&v)?,&[c])?;}
        Ok(out)
    }
    fn old_bernstein(a:&[Iv])->CoreResult<Poly>{
        let n=a.len()-1;
        (0..=n).map(|i|(0..=i).try_fold(zero(),|s,j|{
            s.add(a[j].mul(p(choose(i,j)as f64)?.div(p(choose(n,j)as f64)?)?)?)
        })).collect()
    }
    #[test]
    fn exact_operation_order_and_capacity(){
        let a=F{constant:0.125,time:-0.5,state:vec![(0,0.75),(1,-0.25),(0,0.125)]};
        let q=DeclaredPolynomial::new(vec![
          T{coefficient:1./3.,factors:vec![a.clone(),a.clone(),a.clone(),a]},
          T{coefficient:-0.25,factors:vec![]}
        ]).unwrap();q.validate(2).unwrap();
        let t=[p(0.125).unwrap(),p(0.25).unwrap()];
        let mut checked=0;
        // Includes non-degenerate interval coefficients, all native
        // polynomial lengths, and the old longer-row fallback.
        for n in 1..=6{
         for k in 0..8{
          let y=(0..2).map(|j|(0..n).map(|i|{
              let x=((k+1)*(i+2+j))as f64/17.-0.5;
              Iv::new(x.next_down(),x.next_up()).unwrap()
          }).collect::<Vec<_>>()).collect::<Vec<_>>();
          let b=compose_owned(&q,&t,&y).unwrap();let c=compose(&q,&t,&y).unwrap();
          same(&b,&c);checked+=b.len()*2;
          let old=old_bernstein(&b).unwrap();
          let r=range(&c).unwrap();
          same(&[r],&[Iv::new(old.iter().map(|v|v.lo).fold(f64::INFINITY,f64::min),old.iter().map(|v|v.hi).fold(f64::NEG_INFINITY,f64::max)).unwrap()]);
          let max=old.iter().map(Iv::mag).fold(0_f64,f64::max);
          let mean=div_up(old.iter().try_fold(0_f64,|s,v|add_up(s,v.mag())).unwrap(),old.len()as f64).unwrap();
          assert_eq!(bernstein_magnitude(&c,false).unwrap().to_bits(),max.to_bits());
          assert_eq!(bernstein_magnitude(&c,true).unwrap().to_bits(),mean.to_bits());
          if c.len()<=POLY_SCRATCH{
           for m in [1,4,16]{
            let w=1./m as f64;
            for i in 0..m{
             let l=i as f64*w;
             same(conservation_restrict(&c,l,w).unwrap().slice(),&old_restrict(&b,l,w).unwrap());checked+=2*b.len();
            }
           }
          }
          let u=Iv::new(0.25,0.75).unwrap();
          let center=p(di::mid(u)).unwrap();
          let d=c.iter().enumerate().skip(1).map(|(k,v)|v.scale(k as f64)).collect::<CoreResult<Vec<_>>>().unwrap();
          let tight=eval(&c,center).unwrap().add(u.sub(center).unwrap().mul(eval(&d,u).unwrap()).unwrap()).unwrap();
          same(&[reset_centered_eval(&c,u).unwrap()],&[intersect(eval(&c,u).unwrap(),tight).unwrap()]);
         }
        }
        assert!(ScratchPoly::new(&[zero();18]).is_err());
        assert!(conservation_restrict(&[],0.,1.).is_err());
        assert!(range(&[]).is_err());
        println!("POLYNOMIAL_BIT_CHECKS {checked}");
    }
}
