//! Frozen manufactured point/stencil corpus. No ODE trajectory is integrated.
#![forbid(unsafe_code)]
extern crate d3_fixture;
#[path = "gateway.rs"] mod gateway;
use gateway::{BoxDomain, CallbackFault, Gate, RuntimeWitness};
use std::cell::Cell;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};

fn inside(t: f64, z: &[f64; 2]) -> bool {
    t.is_finite() && (0.0..=1.0).contains(&t)
        && z.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x))
}
fn rhs(t: f64, z: &[f64; 2]) -> [f64; 2] {
    let transfer = (2.0 + 0.5 * t * t) * z[1] * z[1];
    [-3.0 * z[0] + transfer, 3.0 * z[0] - z[1] - transfer]
}
fn jac(t: f64, z: &[f64; 2]) -> [[f64; 2]; 2] {
    let coupling = 2.0 * (2.0 + 0.5 * t * t) * z[1];
    [[-3.0, coupling], [3.0, -1.0 - coupling]]
}
fn gate() -> Gate<2> {
    Gate::new(BoxDomain::new(0.0, 1.0, [0.0; 2], [1.0; 2]).unwrap(), 7)
}

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("output JSONL path required");
    let mut file = BufWriter::new(OpenOptions::new().write(true).create_new(true).open(path)?);
    assert!(d3_fixture::admission::environment().is_ok());
    let witnesses = Cell::new(0_u64);
    let rhs_entries = Cell::new(0_u64);
    let jac_entries = Cell::new(0_u64);
    let forbidden = Cell::new(0_u64);
    let mut probe = || {
        witnesses.set(witnesses.get() + 1);
        RuntimeWitness { epoch: 7, environment_ok: d3_fixture::admission::environment().is_ok() }
    };
    let mut f = |t: f64, z: &[f64; 2], out: &mut [f64; 2]| {
        rhs_entries.set(rhs_entries.get() + 1);
        if !inside(t, z) { forbidden.set(forbidden.get() + 1); return Err(CallbackFault::Fatal); }
        *out = rhs(t, z); Ok(())
    };
    let mut j = |t: f64, z: &[f64; 2], out: &mut [[f64; 2]; 2]| {
        jac_entries.set(jac_entries.get() + 1);
        if !inside(t, z) { forbidden.set(forbidden.get() + 1); return Err(CallbackFault::Fatal); }
        *out = jac(t, z); Ok(())
    };

    let mut g = gate();
    let mut rows = 0_u64;
    for it in 0..=8 { for ix in 0..=8 { for iy in 0..=8 {
        let t = it as f64 / 8.0; let z = [ix as f64 / 8.0, iy as f64 / 8.0];
        let p = g.prepare_pair(t, &z, &mut probe, &mut f, &mut j).unwrap();
        let jv = [p.jacobian[0][0] - p.jacobian[0][1], p.jacobian[1][0] - p.jacobian[1][1]];
        let c = g.counters();
        writeln!(file,"{{\"kind\":\"point\",\"i\":[{it},{ix},{iy}],\"t\":{t:?},\"y\":{z:?},\"rhs\":{:?},\"jac\":{:?},\"jv\":{jv:?},\"mass_residual\":{:?},\"rhs_count\":{},\"jac_count\":{},\"pairs\":{}}}",
            p.rhs,p.jacobian,p.rhs[0]+p.rhs[1]+z[1],c.rhs,c.jac,c.pairs)?;
        rows += 1;
    }}}

    let mut fd = gate();
    for it in 1..=7 { for power in 3..=9 {
        let t = it as f64 / 8.0; let z = [0.25, 0.75]; let eps = 2.0_f64.powi(-power);
        let out = fd.estimate_partial_t_centered(t, &z, eps, &mut probe, &mut f).unwrap();
        let c = fd.counters();
        writeln!(file,"{{\"kind\":\"fd\",\"it\":{it},\"power\":{power},\"t\":{t:?},\"y\":{z:?},\"eps\":{eps:?},\"minus\":{:?},\"plus\":{:?},\"denominator\":{:?},\"estimate\":{out:?},\"rhs_count\":{}}}",t-eps,t+eps,(t+eps)-(t-eps),c.rhs)?;
        rows += 1;
    }}

    // A truthful interface can admit a finite, fully written, wrong derivative.
    let mut wrong = gate();
    let mut wrong_j = |t: f64, z: &[f64; 2], out: &mut [[f64; 2]; 2]| {
        j(t, z, out)?; out[0][0] += 1.0; out[1][1] += 1.0; Ok(())
    };
    let p = wrong.prepare_pair(0.5, &[0.25, 0.75], &mut probe, &mut f, &mut wrong_j).unwrap();
    writeln!(file,"{{\"kind\":\"finite_wrong_j\",\"t\":{:?},\"y\":{:?},\"rhs\":{:?},\"jac\":{:?},\"admitted\":true}}",p.t,p.y,p.rhs,p.jacobian)?;
    rows += 1;

    let invalid = [(-0.125,[0.25,0.75]),(1.125,[0.25,0.75]),(0.5,[-0.125,0.75]),
        (0.5,[0.25,1.125]),(f64::NAN,[0.25,0.75]),(f64::INFINITY,[0.25,0.75]),(0.5,[f64::NAN,0.75])];
    for (id,(t,z)) in invalid.into_iter().enumerate() {
        let mut g = gate(); let err = g.prepare_pair(t, &z, &mut probe, &mut f, &mut j).unwrap_err();
        let c=g.counters();
        writeln!(file,"{{\"kind\":\"input_refusal\",\"id\":{id},\"t_bits\":{},\"y_bits\":[{},{}],\"fault\":\"{err:?}\",\"rhs_count\":{},\"jac_count\":{},\"pairs\":{}}}",t.to_bits(),z[0].to_bits(),z[1].to_bits(),c.rhs,c.jac,c.pairs)?;
        rows += 1;
    }
    let refused = [("left",0.0,0.125),("right",1.0,0.125),("collapse",0.5,f64::MIN_POSITIVE),
        ("zero",0.5,0.0),("negative",0.5,-0.125),("infinite",0.5,f64::INFINITY),("nan",0.5,f64::NAN)];
    for (name,t,eps) in refused {
        let mut g = gate(); let before=witnesses.get();
        let err=g.estimate_partial_t_centered(t,&[0.25,0.75],eps,&mut probe,&mut f).unwrap_err();
        writeln!(file,"{{\"kind\":\"fd_refusal\",\"name\":\"{name}\",\"t_bits\":{},\"eps_bits\":{},\"fault\":\"{err:?}\",\"rhs_count\":{},\"probe_count\":{}}}",t.to_bits(),eps.to_bits(),g.counters().rhs,witnesses.get()-before)?;
        rows += 1;
    }
    writeln!(file,"{{\"kind\":\"summary\",\"rows_before_summary\":{rows},\"points\":729,\"stencils\":49,\"wrong_finite_pairs\":1,\"input_refusals\":7,\"fd_refusals\":7,\"rhs_entries\":{},\"jac_entries\":{},\"forbidden_callback_entries\":{},\"witness_calls\":{},\"new_ode_trajectories\":0}}",rhs_entries.get(),jac_entries.get(),forbidden.get(),witnesses.get())?;
    file.flush()?;
    println!("points=729 stencils=49 wrong_finite_pairs=1 refusals=14 RHS={} Jacobian={} forbidden={} witness={} rows={}",rhs_entries.get(),jac_entries.get(),forbidden.get(),witnesses.get(),rows+1);
    Ok(())
}
