//! Raw bindings to the native bridge built by `build.rs`
//! (`native/bridge/bench_bridge.f90`, `native/bridge/cvode_bridge.c`) and
//! the shared-model callbacks handed to it as function pointers.
//!
//! Safety contract: the bridge calls the callbacks only with the `ctx`
//! pointer it was given (a live `&Model`) and with buffers of the lengths
//! documented on each callback. The bridge is serial (COMMON state in DVODE
//! and the Hairer codes); every entry point here holds one process-wide
//! mutex for the duration of the call, so calls cannot overlap.

use std::ffi::c_void;
use std::sync::Mutex;

use crate::model::Model;

/// C right-hand-side callback: `f = f(t, y)`, both of length `n`.
pub type RhsCb = unsafe extern "C" fn(ctx: *const c_void, t: f64, y: *const f64, f: *mut f64);
/// C Jacobian callback. `ml < 0`: dense column-major, leading dimension
/// `ld`, `ld * n` entries. `ml >= 0`: band, element (i, j) at
/// `(mu + i - j) + ld * j`, `ld * (n - 1) + ml + mu + 1` entries.
pub type JacCb = unsafe extern "C" fn(
    ctx: *const c_void,
    t: f64,
    y: *const f64,
    ml: i32,
    mu: i32,
    ld: i32,
    pd: *mut f64,
);

unsafe extern "C" {
    fn bench_sundials_version() -> *const std::ffi::c_char;
    fn bench_dvode(
        ctx: *const c_void,
        rhs: RhsCb,
        jac: JacCb,
        n: i32,
        nt: i32,
        times: *const f64,
        y0: *const f64,
        rtol: f64,
        atol: f64,
        ml: i32,
        mu: i32,
        values: *mut f64,
        completed: *mut i32,
        code: *mut i32,
        stats: *mut i32,
    );
    fn bench_hairer(
        which: i32,
        ctx: *const c_void,
        rhs: RhsCb,
        jac: JacCb,
        n: i32,
        nt: i32,
        times: *const f64,
        y0: *const f64,
        rtol: f64,
        atol: f64,
        ml: i32,
        mu: i32,
        values: *mut f64,
        completed: *mut i32,
        code: *mut i32,
        stats: *mut i32,
    );
    fn bench_cvode(
        ctx: *const c_void,
        rhs: RhsCb,
        jac: JacCb,
        n: i32,
        nt: i32,
        times: *const f64,
        y0: *const f64,
        rtol: f64,
        atol: f64,
        ml: i32,
        mu: i32,
        max_steps: i64,
        values: *mut f64,
        completed: *mut i32,
        stats: *mut i64,
    ) -> i32;
}

unsafe extern "C" fn rhs_cb(ctx: *const c_void, t: f64, y: *const f64, f: *mut f64) {
    // SAFETY: ctx is the `&Model` passed by `NativeRun`; y and f have n entries.
    let m = unsafe { &*(ctx as *const Model) };
    let y = unsafe { std::slice::from_raw_parts(y, m.n) };
    let f = unsafe { std::slice::from_raw_parts_mut(f, m.n) };
    m.rhs(t, y, f);
}

unsafe extern "C" fn jac_cb(
    ctx: *const c_void,
    t: f64,
    y: *const f64,
    ml: i32,
    mu: i32,
    ld: i32,
    pd: *mut f64,
) {
    // SAFETY: see the module contract and `JacCb`.
    let m = unsafe { &*(ctx as *const Model) };
    let n = m.n;
    let ld = ld as usize;
    let y = unsafe { std::slice::from_raw_parts(y, n) };
    if ml < 0 {
        let pd = unsafe { std::slice::from_raw_parts_mut(pd, ld * n) };
        m.jac_dense_col_major(t, y, ld, pd);
    } else {
        let (ml, mu) = (ml as usize, mu as usize);
        let extent = ld * (n - 1) + ml + mu + 1;
        let pd = unsafe { std::slice::from_raw_parts_mut(pd, extent) };
        m.jac_band_col_major(t, y, ml, mu, ld, pd);
    }
}

/// The SUNDIALS version string of the headers the bridge was compiled with.
pub fn sundials_version() -> String {
    // SAFETY: returns a pointer to a static NUL-terminated string literal.
    unsafe { std::ffi::CStr::from_ptr(bench_sundials_version()) }
        .to_string_lossy()
        .into_owned()
}

static NATIVE: Mutex<()> = Mutex::new(());

/// Raw outcome of one native run.
#[derive(Clone, Debug)]
pub struct NativeOutcome {
    /// Grid states, `n * nt`, column `k` = state at `times[k]` (NaN if not reached).
    pub values: Vec<f64>,
    /// Number of leading grid points written.
    pub completed: usize,
    /// Solver return code (ISTATE, IDID or CVODE flag).
    pub code: i64,
    /// Raw solver statistics, solver-specific order.
    pub stats: Vec<i64>,
}

/// The Hairer code to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hairer {
    /// RADAU5.
    Radau5,
    /// RODAS.
    Rodas,
}

fn checked(m: &Model, times: &[f64], y0: &[f64]) -> (i32, i32) {
    assert_eq!(y0.len(), m.n);
    assert!(times.len() >= 2);
    (
        i32::try_from(m.n).expect("n fits i32"),
        i32::try_from(times.len()).expect("nt fits i32"),
    )
}

/// Netlib DVODE with vendor defaults; `band = None` is MF 21, `Some` MF 24.
pub fn dvode(
    m: &Model,
    times: &[f64],
    y0: &[f64],
    rtol: f64,
    atol: f64,
    band: Option<(usize, usize)>,
) -> NativeOutcome {
    let _g = NATIVE.lock().unwrap_or_else(|e| e.into_inner());
    let (n, nt) = checked(m, times, y0);
    let (ml, mu) = band.map_or((-1, -1), |(l, u)| (l as i32, u as i32));
    let mut values = vec![f64::NAN; m.n * times.len()];
    let (mut completed, mut code) = (0i32, 0i32);
    let mut stats = [0i32; 8];
    // SAFETY: buffers have the lengths the bridge documents; ctx outlives the call.
    unsafe {
        bench_dvode(
            m as *const Model as *const c_void,
            rhs_cb,
            jac_cb,
            n,
            nt,
            times.as_ptr(),
            y0.as_ptr(),
            rtol,
            atol,
            ml,
            mu,
            values.as_mut_ptr(),
            &mut completed,
            &mut code,
            stats.as_mut_ptr(),
        );
    }
    NativeOutcome {
        values,
        completed: completed.max(0) as usize,
        code: code as i64,
        stats: stats.iter().map(|&s| s as i64).collect(),
    }
}

/// Hairer RADAU5 or RODAS with vendor defaults.
pub fn hairer(
    which: Hairer,
    m: &Model,
    times: &[f64],
    y0: &[f64],
    rtol: f64,
    atol: f64,
    band: Option<(usize, usize)>,
) -> NativeOutcome {
    let _g = NATIVE.lock().unwrap_or_else(|e| e.into_inner());
    let (n, nt) = checked(m, times, y0);
    let (ml, mu) = band.map_or((-1, -1), |(l, u)| (l as i32, u as i32));
    let mut values = vec![f64::NAN; m.n * times.len()];
    let (mut completed, mut code) = (0i32, 0i32);
    let mut stats = [0i32; 7];
    // SAFETY: as in `dvode`.
    unsafe {
        bench_hairer(
            match which {
                Hairer::Radau5 => 0,
                Hairer::Rodas => 1,
            },
            m as *const Model as *const c_void,
            rhs_cb,
            jac_cb,
            n,
            nt,
            times.as_ptr(),
            y0.as_ptr(),
            rtol,
            atol,
            ml,
            mu,
            values.as_mut_ptr(),
            &mut completed,
            &mut code,
            stats.as_mut_ptr(),
        );
    }
    NativeOutcome {
        values,
        completed: completed.max(0) as usize,
        code: code as i64,
        stats: stats.iter().map(|&s| s as i64).collect(),
    }
}

/// SUNDIALS CVODE BDF; `band = None` dense linear solver, `Some` band.
pub fn cvode(
    m: &Model,
    times: &[f64],
    y0: &[f64],
    rtol: f64,
    atol: f64,
    band: Option<(usize, usize)>,
    max_steps: i64,
) -> NativeOutcome {
    let _g = NATIVE.lock().unwrap_or_else(|e| e.into_inner());
    let (n, nt) = checked(m, times, y0);
    let (ml, mu) = band.map_or((-1, -1), |(l, u)| (l as i32, u as i32));
    let mut values = vec![f64::NAN; m.n * times.len()];
    let mut completed = 0i32;
    let mut stats = [0i64; 9];
    // SAFETY: as in `dvode`.
    let code = unsafe {
        bench_cvode(
            m as *const Model as *const c_void,
            rhs_cb,
            jac_cb,
            n,
            nt,
            times.as_ptr(),
            y0.as_ptr(),
            rtol,
            atol,
            ml,
            mu,
            max_steps,
            values.as_mut_ptr(),
            &mut completed,
            stats.as_mut_ptr(),
        )
    };
    NativeOutcome {
        values,
        completed: completed.max(0) as usize,
        code: code as i64,
        stats: stats.to_vec(),
    }
}
