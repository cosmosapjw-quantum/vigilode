use rodas5p_core::CoreResult;
fn iv_point(v:f64)->CoreResult<Iv>{Iv::point(v)}
fn z()->Iv{Iv{lo:0.,hi:0.}}
fn legacy_evaluate(polys: &[DeclaredPolynomial], t: Iv, y: &[Iv]) -> CoreResult<(Vec<Iv>, Vec<Vec<Iv>>)> {
    let mut v = Vec::with_capacity(polys.len());
    let mut j = Vec::with_capacity(polys.len());
    for eq in polys {
        let mut sum = z();
        let mut row = vec![z(); 1 + y.len()];
        for term in eq.terms() {
            let vals = term
                .factors
                .iter()
                .map(|f| {
                    f.state
                        .iter()
                        .try_fold(iv_point(f.constant)?.add(t.scale(f.time)?)?, |s, &(i, a)| {
                            s.add(y[i].scale(a)?)
                        })
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let c = iv_point(term.coefficient)?;
            sum = sum.add(vals.iter().try_fold(c, |s, &v| s.mul(v))?)?;
            for (i, f) in term.factors.iter().enumerate() {
                let other = vals
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i)
                    .try_fold(c, |s, (_, &v)| s.mul(v))?;
                row[0] = row[0].add(other.scale(f.time)?)?;
                for &(k, a) in &f.state {
                    row[k + 1] = row[k + 1].add(other.scale(a)?)?;
                }
            }
        }
        v.push(sum);
        j.push(row);
    }
    Ok((v,j))
}
fn source_equations(src:&DaeSource)->Vec<P>{
 let s=serde_json::to_value(src).unwrap();
 let mut out=vec![];
 for group in ["f","g"]{
  for eq in s[group].as_array().unwrap(){
   let terms=eq["terms"].as_array().unwrap().iter().map(|t|{
    let factors=t["factors"].as_array().unwrap().iter().map(|f|F{
     constant:f["constant"].as_f64().unwrap(),time:f["time"].as_f64().unwrap(),
     state:f["state"].as_array().unwrap().iter().map(|v|(v[0].as_u64().unwrap()as usize,v[1].as_f64().unwrap())).collect()
    }).collect();
    T{coefficient:t["coefficient"].as_f64().unwrap(),factors}
   }).collect();
   let v=P::new(terms).unwrap();assert_eq!(serde_json::to_value(&v).unwrap(),*eq);
   out.push(v);
  }
 }
 out
}
fn same_bits(a:&[Iv],b:&[Iv]){
 assert_eq!(a.len(),b.len());
 for (a,b) in a.iter().zip(b){assert_eq!(a.lo.to_bits(),b.lo.to_bits());assert_eq!(a.hi.to_bits(),b.hi.to_bits());}
}
fn compare_interval(src:&DaeSource,t:Iv,y:&[Iv],detail:serde_json::Value)->serde_json::Value{
 use serde_json::json;
 let eq=source_equations(src);let n=y.len();
 let (a,legacy)=measure(||legacy_evaluate(&eq,t,y));let(a,j)=a.unwrap();
 let (b,owned)=measure(||src.interval_values(t,y));let(b,k)=b.unwrap();
 same_bits(&a,&b);for (a,b) in j.iter().zip(&k){same_bits(a,b);}
 let mut w=IntervalEvalWorkspace::new();let mut v=vec![z();n];let mut d=vec![z();n*(n+1)];
 let (r,into)=measure(||src.interval_into(t,y,&mut v,&mut d,&mut w));r.unwrap();
 assert_eq!(into.requests(),0);same_bits(&a,&v);
 for (i,row) in j.iter().enumerate(){same_bits(row,&d[i*(n+1)..(i+1)*(n+1)]);}
 assert!(owned.requests()<legacy.requests());
 json!({"detail":detail,"source":src,"t":t.as_pair(),"y":y.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),
        "values":a.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),
        "jacobian":j.iter().map(|r|r.iter().map(|v|v.as_pair()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "legacy":legacy,"owned":owned,"into":into,"bit_checks":2*2*(n+n*(n+1))})
}
#[test]
fn generic_interval_shape_and_error_atomicity(){
 use serde_json::json;
 for n in [2,3,8,16,32]{
  let eq=(0..n).map(|i|p((0..16).map(|j|{
   let f=F{constant:(j as f64-7.)/16.,time:(i as f64+1.)/32.,state:(0..16).map(|k|((i+k/2)%n,((k%5)as f64-2.)/32.)).collect()};
   tr((j as f64-8.)/64.,vec![f.clone(),f,F::component((i+1)%n,0.125),F::clock(-0.25)])
  }).collect())).collect::<Vec<_>>();
  let src=DaeSource::new(eq[..n-1].to_vec(),eq[n-1..].to_vec()).unwrap();
  for k in 0..3{
   let y=(0..n).map(|i|{
    let c=(((k+1)*(i+3)%17)as f64-8.)/16.;Iv::new(c-1./64.,c+1./128.).unwrap()
   }).collect::<Vec<_>>();
   let t=Iv::new((k as f64-2.)/8.,(k as f64-2.)/8.+1./64.).unwrap();
   println!("GENERIC {}",compare_interval(&src,t,&y,json!({"dimension":n,"case":k})));
  }
 }
 let src=RobertsonModel::new(3e7).unwrap();
 let mut w=IntervalEvalWorkspace::new();let sentinel=Iv{lo:-0.,hi:0.};
 let mut v=[sentinel;3];let mut d=[sentinel;12];
 for (t,y) in [
  (Iv{lo:1.,hi:0.},vec![z();3]),
  (z(),vec![Iv{lo:0.,hi:f64::INFINITY},z(),z()]),
  (z(),vec![Iv{lo:1.,hi:0.},z(),z()]),
  (z(),vec![z();2])
 ]{
  assert!(src.source().interval_values(t,&y).is_err());
  assert!(src.source().interval_into(t,&y,&mut v,&mut d,&mut w).is_err());
  same_bits(&v,&[sentinel;3]);same_bits(&d,&[sentinel;12]);
 }
 assert!(src.source().interval_into(z(),&[z();3],&mut v[..2],&mut d,&mut w).is_err());same_bits(&v,&[sentinel;3]);same_bits(&d,&[sentinel;12]);
 let over=DaeSource::new(vec![p(vec![tr(1.,vec![])])],vec![p(vec![tr(f64::MAX,vec![F{constant:0.,time:0.,state:vec![(0,2.)]}])])]).unwrap();
 assert!(legacy_evaluate(&source_equations(&over),z(),&[z();2]).is_err());
 assert!(over.interval_into(z(),&[z();2],&mut v[..2],&mut d[..6],&mut w).is_err());
 same_bits(&v,&[sentinel;3]);same_bits(&d,&[sentinel;12]);
 let (r,a)=measure(||src.source().interval_into(z(),&[z();3],&mut v,&mut d,&mut w));r.unwrap();assert_eq!(a.requests(),0);
 let (a,b)=src.source().interval_values(z(),&[z();3]).unwrap();same_bits(&a,&v);same_bits(&b.into_iter().flatten().collect::<Vec<_>>(),&d);
 // Signed zero and subnormal endpoints.
 let q=f64::from_bits(1);
 let y=[Iv{lo:-0.,hi:0.},Iv{lo:-q,hi:q},Iv{lo:0.,hi:q}];
 println!("TINY {}",compare_interval(src.source(),z(),&y,json!({"case":"subnormal"})));
}
#[test]
#[ignore="fixed actual native interval slab portfolio"]
fn interval_portfolio(){
 use serde_json::json;
 for solver in [EvolutionRoute::Mass,EvolutionRoute::Prepared5,EvolutionRoute::Scalar5]{
  for cert in [ConservationRoute::Max,ConservationRoute::Integral4]{
   let m=graph("alternating",solver,cert);
   let r=evolve_robertson_phase_atlas(&m,0,initial(2f64.powi(-26)),[0.,0.002],&opts(),policy(),pm(MultiGuardRoute::Correlated),atlas_budget(),PhaseReuseRoute::Reuse).unwrap();
   assert_eq!(r.stop(),PhaseAtlasStop::Horizon);
   for phase in r.phases().iter().take(2){
    let model=RobertsonModel::new(m[phase.mode].rate).unwrap();
    let slabs=phase.original_slab_diagnostics();
    for (tag,k) in [("first",0),("middle",slabs.len()/2),("last",slabs.len()-1)]{
     let s=&slabs[k];let c=s.certificate.as_ref().unwrap();assert!(c.global_qualified);
     let y=c.x_range.iter().chain(&c.branch_root_box).map(|v|Iv::new(v[0],v[1]).unwrap()).collect::<Vec<_>>();
     let t=Iv::new(s.times[0],s.times[1]).unwrap();
     println!("INTERVAL {}",compare_interval(model.source(),t,&y,json!({"solver":solver,"cert":cert,"mode":phase.mode,"position":tag,"slab":k,"rate":m[phase.mode].rate,"native_times":s.times})));
    }
   }
  }
 }
}
