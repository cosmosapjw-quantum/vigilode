
#[cfg(test)]
mod affine_lattice_tests {
 use super::*;
 const ROUTES:[RootRoute;5]=[RootRoute::Sturm,RootRoute::NativeBernstein,RootRoute::DictionaryBernstein,RootRoute::Companion,RootRoute::DictionaryCompanion];
 fn compare(p:&RepresentedPolynomial,tol:f64)->usize{
    for route in ROUTES{
        let old=p.isolate_impl(route,tol,false);
        let new=p.isolate(route,tol);
        match (old,new){
            (Ok(a),Ok(b))=>assert_eq!(serde_json::to_string(&a).unwrap(),serde_json::to_string(&b).unwrap(),"{:?} {:?} {tol}",p.exact_coefficients(),route),
            (Err(a),Err(b))=>assert_eq!(a.to_string(),b.to_string()),
            (a,b)=>panic!("root outcome mismatch {a:?} {b:?}"),
        }
    }
    ROUTES.len()
 }
 #[test]
 fn identical_exact_root_reports(){
    let mut n=0;
    for a in [
        vec![-1.,3.],vec![0.,1.],vec![-1.,1.],vec![-1.,2.],vec![1.,-2.],
        vec![1.,1.],vec![-2.,1.],vec![0.,0.],vec![3.],
        vec![-f64::from_bits(1),1.],vec![f64::from_bits(1),-1.],
        vec![-1e300,3e300],vec![-1.,f64::MAX],
        vec![0.25,-1.,1.],vec![0.125,-0.75,1.],vec![0.,-1.,1.],
        vec![-0.125,0.75,-1.5,1.],
    ]{
        let p=RepresentedPolynomial::from_power(&a).unwrap();
        for t in [2f64.powi(-50),2f64.powi(-48),1e-6,0.001]{n+=compare(&p,t);}
    }
    // Genuine exact frozen source, including non-dyadic normalized root.
    let g=DeclaredPolynomial::new(vec![
        EventTerm{coefficient:1.,factors:vec![AffineFactor::component(0,0.)]},
        EventTerm{coefficient:1.,factors:vec![AffineFactor::component(1,0.)]},
        EventTerm{coefficient:1.,factors:vec![AffineFactor::component(2,0.)]},
        EventTerm{coefficient:-1.,factors:vec![]},
    ]).unwrap();
    for k in 0..32{
        let x=[0.9998658289383162-(k as f64)*2f64.powi(-48),6.457305003072096e-5+(k as f64)*2f64.powi(-54)];
        let p=RepresentedPolynomial::frozen_scalar(&g,0.0005,&x,[-0.25,1.25]).unwrap();
        for t in [2f64.powi(-50),2f64.powi(-48)]{n+=compare(&p,t);}
    }
    // Nonautonomous affine source with repeated state indices,
    // and exact branch endpoint roots. Exact degree after freezing,
    // not a rounded degree or a source-name heuristic, is authority.
    let a=AffineFactor{constant:0.25,time:0.5,state:vec![(0,0.125),(1,1.),(1,0.5)]};
    let g=DeclaredPolynomial::new(vec![EventTerm{coefficient:1.,factors:vec![a]},EventTerm{coefficient:-0.5,factors:vec![]}]).unwrap();
    for t in [0.,0.125,0.25,0.5]{
       for x in [0.,0.125,0.5]{
        let p=RepresentedPolynomial::frozen_scalar(&g,t,&[x],[0.,1.]).unwrap();
        n+=compare(&p,2f64.powi(-48));
       }
    }
    println!("EXACT_ROOT_REPORT_COMPARISONS {n}");
 }
}
