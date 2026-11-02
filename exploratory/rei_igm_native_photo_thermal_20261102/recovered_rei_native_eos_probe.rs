//! Optional RUST-TO-RUST independent EOS check using ORIGINAL historical REI
//! Cargo crate from REI Draft PR #360. This source is built with rustc --extern
//! rei_microphysics=... (see REPRODUCE.md) and NEVER vendors or modifies REI.
//! It must be linked to exact pinned historical repo HEAD and Cargo artifact.
use rei_microphysics::igm_state::IgmGasState;
use std::fs;
use std::env;
use std::convert::TryInto;

fn get(x:&[u8],i:usize)->f64 {
    f64::from_le_bytes(x[i..i+8].try_into().unwrap())
}
fn check(parent:&[u8],derived:&[u8])->Result<(usize,f64,f64,f64),String>{
    if parent.len()<24 || &parent[..8]!=b"REIF05P2" ||
       derived.len()<16 || &derived[..8]!=b"REIF05D1" {
        return Err("ORIGINAL_F05_CROSS_REPO_SOURCE_SCHEMA_INVALID".into());
    }
    let n=usize::try_from(u64::from_le_bytes(parent[8..16].try_into().unwrap()))
        .map_err(|_|"SOURCE_COUNT_OVERFLOW")?;
    let nd=usize::try_from(u64::from_le_bytes(derived[8..16].try_into().unwrap()))
        .map_err(|_|"DERIVED_COUNT_OVERFLOW")?;
    if n!=46080||nd!=n||parent.len()!=24+8*12*n||derived.len()!=16+8*10*n {
        return Err("ORIGINAL_F05_46080_DIMENSION_MISMATCH".into());
    }
    let mut max_u_rel=0.0_f64;
    let mut max_ne_rel=0.0_f64;
    let mut total_u=0.0_f64;
    for i in 0..n {
        let mut p=[0.0;5];
        for s in 0..5 {
            let offset=24+8*(5*i+s);
            let lo=get(parent,offset);
            let hi=get(parent,24+8*(5*n+5*i+s));
            p[s]=(lo+hi)*0.5;
        }
        let lnt_lo=get(parent,24+8*(10*n+i));
        let lnt_hi=get(parent,24+8*(11*n+i));
        let t=((lnt_lo+lnt_hi)*0.5).exp();
        let h=p[0]+p[1];
        let he=p[2]+p[3]+p[4];
        let xh=p[1]/h;
        let y=p[3]/he;
        let z=p[4]/he;
        if !h.is_finite() || !he.is_finite()|| h<=0.0 || he<=0.0 {
            return Err(format!("PARENT_NUCLEI_COUNT_INVALID:{i}"));
        }
        let gas=IgmGasState::from_temperature([xh,y,z],1.0,he/h,t)
            .map_err(|e|format!("HISTORIC_REI_RUST_IGM_STATE_FAILED_AT:{i}:{e}"))?;
        let eos=gas.eos(1.0,he/h)
            .map_err(|e|format!("HISTORIC_REI_RUST_EOS_FAILED_AT:{i}:{e}"))?;
        let native_u=eos.u_erg_cm3*h;
        let stage_u=get(derived,16+8*(i*10+8));
        let ne=eos.electron_density_cm3*h;
        let original_ne=p[1]+p[3]+2.0*p[4];
        let u_rel=(native_u-stage_u).abs()/stage_u.abs().max(f64::MIN_POSITIVE);
        let ne_rel=(ne-original_ne).abs()/original_ne.abs().max(f64::MIN_POSITIVE);
        if !u_rel.is_finite() || !ne_rel.is_finite() {
            return Err(format!("NONFINITE_EOS_RUST_CROSS_REPO:{i}"));
        }
        max_u_rel=max_u_rel.max(u_rel);
        max_ne_rel=max_ne_rel.max(ne_rel);
        total_u+=native_u;
    }
    if max_u_rel>64.0*f64::EPSILON || max_ne_rel>64.0*f64::EPSILON {
        return Err(format!("REI_RUST_EOS_NOT_EQUIVALENT_SOURCE_POINTS:u={max_u_rel},ne={max_ne_rel}"));
    }
    Ok((n,max_u_rel,max_ne_rel,total_u))
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<String>=env::args().collect();
    if args.len()!=3{return Err("USAGE: historic_rei_eos_probe ACCEPTED_PARENT_RAW RUST_DERIVED_STAGE_RAW".into());}
    let (n,u,ne,total)=check(&fs::read(&args[1])?,&fs::read(&args[2])?)?;
    println!("{{\"schema\":\"REI_HISTORICAL_RUST_EOS_VIGILODE_STAGE_CROSSCHECK_V1\",\"cells\":{n},\"native_REI_rust_eos_max_u_relative_diff\":{u:.17e},\"native_REI_rust_eos_max_ne_relative_diff\":{ne:.17e},\"native_REI_rust_eos_uncompensated_thermal_erg\":{total:.17e},\"source_owner_26event_integrator\":\"NOT_RUN\",\"original_tick161\":\"NOT_ACCEPTED\",\"canonical_delta\":0,\"production\":\"HOLD\"}}");
    Ok(())
}
