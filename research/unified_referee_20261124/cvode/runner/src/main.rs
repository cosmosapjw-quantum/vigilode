//! Native CVODE-BDF and protected dense Rodas5p, identical autonomous models.
#[path = "../../../../production_unification_20261115/production_holdout/runner/src/models.rs"]
mod models;

use models::{Spec, problem, specs};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, c_int, c_long, c_void},
    slice,
    time::Instant,
};
use vigilode::{Atol, Options, Rodas5p};

type Rhs = unsafe extern "C" fn(*const c_void, f64, *const f64, *mut f64);
type Jac = unsafe extern "C" fn(*const c_void, f64, *const f64, c_int, c_int, c_int, *mut f64);
unsafe extern "C" {
    fn bench_sundials_version() -> *const std::ffi::c_char;
    fn bench_cvode(
        ctx: *const c_void,
        rhs: Rhs,
        jac: Jac,
        n: c_int,
        nt: c_int,
        times: *const f64,
        y0: *const f64,
        rtol: f64,
        atol: f64,
        ml: c_int,
        mu: c_int,
        max_steps: c_long,
        values: *mut f64,
        completed: *mut c_int,
        stats: *mut c_long,
    ) -> c_int;
}

// The serial SUNDIALS bridge borrows the model and a nonaliased scratch matrix
// for one synchronous call. The shared Rust model fills row-major Jacobians.
struct Call<'a> {
    spec: &'a Spec,
    row_major: Vec<f64>,
}
unsafe extern "C" fn rhs(ctx: *const c_void, _: f64, y: *const f64, out: *mut f64) {
    unsafe {
        let s = (&*ctx.cast::<Call<'_>>()).spec;
        (s.f)(
            slice::from_raw_parts(y, s.n),
            slice::from_raw_parts_mut(out, s.n),
        );
    }
}
unsafe extern "C" fn jac(
    ctx: *const c_void,
    _: f64,
    y: *const f64,
    _: c_int,
    _: c_int,
    ld: c_int,
    out: *mut f64,
) {
    unsafe {
        let call = &mut *ctx.cast_mut().cast::<Call<'_>>();
        let s = call.spec;
        (s.j)(slice::from_raw_parts(y, s.n), &mut call.row_major);
        let dst = slice::from_raw_parts_mut(out, ld as usize * s.n);
        for col in 0..s.n {
            for row in 0..s.n {
                dst[col * ld as usize + row] = call.row_major[row * s.n + col];
            }
        }
    }
}

fn run(s: &Spec, arm: usize, tol: f64) -> (Value, f64) {
    let clock = Instant::now();
    let options = Options {
        rtol: tol,
        atol: Atol::Scalar(tol * 0.01),
        ..Options::default()
    };
    if arm == 0 {
        let sol = match Rodas5p::solve(&problem(s), (0.0, s.t_end), &s.y0, &options) {
            Ok(solution) => solution,
            Err(error) => {
                return (
                    json!({"success":false, "error":error.to_string(), "accepted":0}),
                    clock.elapsed().as_secs_f64(),
                );
            }
        };
        let secs = clock.elapsed().as_secs_f64();
        let success =
            sol.is_success() && sol.final_state().is_some() && sol.t.last() == Some(&s.t_end);
        return (
            json!({"success":success,"stop":format!("{:?}",sol.status),"last_t":sol.t.last(),"y_end":sol.final_state().filter(|_| success), "accepted":sol.stats.accepted_steps,
                       "rejected":sol.stats.rejected_steps, "counters":sol.stats.counters}),
            secs,
        );
    }
    let mut call = Call {
        spec: s,
        row_major: vec![0.; s.n * s.n],
    };
    let mut values = vec![0.; 2 * s.n];
    let mut done = 0;
    let mut stats = [0; 9];
    let flag = unsafe {
        bench_cvode(
            (&raw mut call).cast(),
            rhs,
            jac,
            s.n as c_int,
            2,
            [0., s.t_end].as_ptr(),
            s.y0.as_ptr(),
            tol,
            tol * 0.01,
            -1,
            -1,
            options.max_steps as c_long,
            values.as_mut_ptr(),
            &mut done,
            stats.as_mut_ptr(),
        )
    };
    let secs = clock.elapsed().as_secs_f64();
    let success = (flag, done) == (0, 2) && values[s.n..].iter().all(|x| x.is_finite());
    (
        json!({"success":success,"flag":flag,"outputs_completed":done,"y_end":success.then_some(&values[s.n..]),"accepted":stats[0],"rhs":stats[1],
            "setups":stats[2],"error_test_failures":stats[3],"nonlinear_iterations":stats[4],
            "nonlinear_convergence_failures":stats[5],"jacobians":stats[6],"linear_rhs":stats[7]}),
        secs,
    )
}

fn main() {
    let version = unsafe { CStr::from_ptr(bench_sundials_version()) }
        .to_str()
        .unwrap();
    for s in specs() {
        for tol in [1e-4, 1e-5, 1e-6, 1e-7, 1e-8] {
            let mut times = [vec![], vec![]];
            let mut reports = [None, None];
            for rep in 0..9 {
                for k in 0..2 {
                    let arm = (rep + k) % 2;
                    let (res, secs) = run(&s, arm, tol);
                    if rep >= 2 {
                        times[arm].push(secs);
                    }
                    if let Some(first) = &reports[arm] {
                        assert_eq!(first, &res, "repeat-dependent result");
                    } else {
                        reports[arm] = Some(res);
                    }
                }
            }
            for (a, name) in ["rodas5p_dense", "cvode_bdf_dense"].iter().enumerate() {
                println!(
                    "{}",
                    json!({"arm":name,"problem":s.name,"rtol":tol,"sundials":version,
                    "result":reports[a],"wall_s":times[a]})
                );
            }
        }
    }
}
