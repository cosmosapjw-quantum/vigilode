extern crate vigilode;
use vigilode::causal_budget::{assess_causal_budget, causal_residual_allowances, CausalBudgetInput, CausalTargetKey};
use vigilode::selection_evidence::{compare_qualified_costs, CostComparison, CostEvidence, CostEvidenceKind, CostInterval, CostUnit, SelectionIdentity};
use vigilode::SessionEpochs;

fn word(x: f64) -> String { format!("{:016x}",x.to_bits()) }
fn words(v: &[f64]) -> String { v.iter().map(|x|word(*x)).collect::<Vec<_>>().join(",") }
fn pow(e: i32) -> f64 { 2f64.powi(e) }
fn main() {
    let key=CausalTargetKey::new(SessionEpochs::default(),0,0,0,0.,0.125).unwrap();
    for k in 0..96 {
        let mut b=[0.;16];let mut d=[0.;4];let mut w=[0.;4];
        for i in 0..4 {
            d[i]=(1.+((k+3*i)%7) as f64/8.)*pow(((k*29+i*17)%401) as i32-200);
            w[i]=(1.+((k+5*i)%7) as f64/8.)*pow(((k*7+i*31)%201) as i32-100);
            for j in 0..i {
                b[i*4+j]=if (k+i+j)%5==0 {0.} else {(1.+((k+i*3+j)%7) as f64/8.)*pow(((k*11+i*17+j*23)%201) as i32-100)};
            }
        }
        let mut e=[0.;4];
        let input=CausalBudgetInput {expected:key,observed:key,lambda:1.,propagation:&b,residual_seeds:&d,output_weights:&w,reconstruction_error:0.,output_budget:f64::MAX};
        let r=assess_causal_budget(&input,&mut e).unwrap();
        let mut influence=[0.;4];let mut allowance=[0.;4];
        causal_residual_allowances(&b,&w,1e-8,&mut influence,&mut allowance).unwrap();
        println!("B|{k}|{}|{}|{}|{}|{}|{}|{}|{}",words(&b),words(&d),words(&w),words(&e),word(r.output_error_upper),words(&influence),words(&allowance),word(1e-8));
    }
    let ident=SelectionIdentity::default();
    for k in 0..256 {
        let base=pow((k%140) as i32-70)*(1.+(k%7) as f64/8.);
        let gain=match k%4 {0=>0.,1=>0.2,2=>0.5,_=>1f64.next_down()};
        let t=base*if k%3==0 {0.} else {0.125};
        let n=match k%3 {0=>1,1=>17,_=>usize::MAX};
        let pivot=(1.-gain)*base-t/(n as f64);
        let cand=if k%7==0 {pivot.max(0.)*0.5} else {match k%4 {0=>pivot.next_down().max(0.),1=>pivot.max(0.),2=>pivot.next_up().max(0.),_=>base*0.25}};
        let a=CostEvidence{identity:ident,cost:CostInterval{lower:base,upper:base*1.125},unit:CostUnit::Work,kind:CostEvidenceKind::DeclaredBound,admissible:true};
        let c=CostEvidence{identity:ident,cost:CostInterval{lower:cand*0.875,upper:cand},unit:CostUnit::Work,kind:CostEvidenceKind::DeclaredBound,admissible:true};
        let chosen=compare_qualified_costs(&a,&c,t,n,gain)==CostComparison::PreferCandidate;
        println!("C|{k}|{}|{}|{}|{n}|{}|{chosen}",word(base),word(cand),word(t),word(gain));
    }
}
