//! Source-original REI F05: native directed thermal SDIRK2 implicit roots.
//! Actual 46,080-cell full/half1/half2 predictor/stage/final source boxes.
//! The owner remains source/ledger authority; this is not whole-IVP certification.
#[path = "rei_f05_verified_support/interval_math.rs"]
mod rei_math;
use rayon::prelude::*;
use rodas5p_core::directed::{exp_interval, sqrt_down, sqrt_up, Interval as I};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, time::Instant};

type R<T> = Result<T, String>;
const N: usize = 46_080;
const ROOTS: usize = 9;
const COLS: usize = 32;
const HEADER: usize = 24;
const RECORD_HEADER: usize = 32;
const MAGIC: &[u8; 8] = b"REIF05T2";
const RESULT_MAGIC: &[u8; 8] = b"REIF05R2";
const SOURCE_SHA: &str = "73ed05ef73ebc2b098d70aa66970b9e6ceb2bcb567c13fa80b2efc4017ebce3a";
const KB: f64 = 1.380649e-16; // cgs erg/K; original source.
fn err<T>(q: rodas5p_core::CoreResult<T>) -> R<T> {
    q.map_err(|e| e.to_string())
}
fn new(lo: f64, hi: f64) -> R<I> {
    err(I::new(lo, hi))
}
fn p(x: f64) -> R<I> {
    err(I::point(x))
}
fn add(a: I, b: I) -> R<I> {
    err(a.add(b))
}
fn sub(a: I, b: I) -> R<I> {
    err(a.sub(b))
}
fn mul(a: I, b: I) -> R<I> {
    err(a.mul(b))
}
fn div(a: I, b: I) -> R<I> {
    err(a.div(b))
}
fn s(a: I, x: f64) -> R<I> {
    mul(a, p(x)?)
}
fn sum(a: &[I]) -> R<I> {
    a.iter().copied().try_fold(p(0.0)?, add)
}
fn prod(a: &[I]) -> R<I> {
    a.iter().copied().try_fold(p(1.0)?, mul)
}
fn exp(x: I) -> R<I> {
    let a = err(exp_interval(x.lo))?;
    let b = err(exp_interval(x.hi))?;
    new(a.lo, b.hi)
}
fn sq(x: I) -> R<I> {
    if x.lo < 0.0 {
        return Err("THERMAL_SQRT_DOMAIN".into());
    }
    new(err(sqrt_down(x.lo))?, err(sqrt_up(x.hi))?)
}
fn orig(x: I) -> R<rei_math::Interval> {
    rei_math::Interval::new(x.lo, x.hi).map_err(|_| "ORIGINAL_REI_TRANSCENDENTAL_DOMAIN".into())
}
fn ln(x: I) -> R<I> {
    let q = orig(x)?
        .ln()
        .map_err(|_| "ORIGINAL_REI_LN_FAIL".to_owned())?;
    new(q.lo, q.hi)
}
fn pow(x: I, exponent: f64) -> R<I> {
    let q = orig(x)?
        .powf(exponent)
        .map_err(|_| "ORIGINAL_REI_POW_FAIL".to_owned())?;
    new(q.lo, q.hi)
}
fn read_f64(bytes: &[u8], pos: usize) -> R<f64> {
    if pos + 8 > bytes.len() {
        return Err("TRUNCATED_SOURCE".into());
    }
    let x = f64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
    if !x.is_finite() {
        return Err("NONFINITE_SOURCE".into());
    }
    Ok(x)
}
fn sha(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}
#[derive(Clone, Copy)]
struct Context {
    photoheat: I,
    expansion: I,
    ecoef: I,
    u0: I,
    constant: I,
    factors: [I; 9],
    dt: f64,
    weight: f64,
    owner_ln_t: I,
    owner_rhs: I,
}
impl Context {
    fn new(data: &[u8], dt: f64, weight: f64) -> R<Self> {
        let mut v = [0.0; COLS];
        for k in 0..COLS {
            v[k] = read_f64(data, k * 8)?;
        }
        let mut intervals = [I { lo: 0.0, hi: 0.0 }; 16];
        for k in 0..16 {
            intervals[k] = new(v[2 * k], v[2 * k + 1])?;
        }
        // The original interval exporter widens exact zero to [-minsub,minsub];
        // do not silently accept a truly negative rate as physical.
        for i in [0_usize, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13].iter() {
            if intervals[*i].lo < -f64::from_bits(1) {
                return Err("NEGATIVE_ORIGINAL_PHYSICAL_FACTOR".into());
            }
        }
        if intervals[2].lo <= 0.0 || intervals[3].lo <= 0.0 {
            return Err("NONPOSITIVE_SOURCE_ENERGY_COEFFICIENT".into());
        }
        if !(dt.is_finite() && dt > 0.0 && weight.is_finite() && weight > 0.0 && weight <= 1.0) {
            return Err("BAD_ORIGINAL_SDIRK_STAGE_CLOCK".into());
        }
        if !(intervals[14].lo > 0.0 && intervals[14].lo <= intervals[14].hi) {
            return Err("NONPHYSICAL_OWNER_LOGTEMPERATURE".into());
        }
        let mut factors = [intervals[5]; 9];
        factors.copy_from_slice(&intervals[5..14]);
        Ok(Self {
            photoheat: intervals[0],
            expansion: intervals[1],
            ecoef: intervals[2],
            u0: intervals[3],
            constant: intervals[4],
            factors,
            dt,
            weight,
            owner_ln_t: intervals[14],
            owner_rhs: intervals[15],
        })
    }
}
#[derive(Clone, Copy)]
struct Eval {
    f: I,
    df: I,
    rhs: I,
}
fn eval(c: &Context, x: I) -> R<Eval> {
    let one = p(1.0)?;
    let t = exp(x)?;
    if t.lo <= 0.0 || t.hi > 1.0e5 * (1.0 + 64.0 * f64::EPSILON) {
        return Err("ORIGINAL_SOURCE_T_TABLE_RANGE".into());
    }
    let lh = div(p(315614.0)?, t)?;
    let le = div(p(570670.0)?, t)?;
    let l3 = div(p(1263030.0)?, t)?;
    let r = pow(div(lh, p(2.250)?)?, 0.376)?;
    let r3 = pow(div(l3, p(2.250)?)?, 0.376)?;
    let sqrt = sq(div(t, p(1.0e5)?)?)?;
    let denom = add(one, sqrt)?;
    let rates = [
        div(
            prod(&[p(3.435e-30)?, t, pow(lh, 1.970)?])?,
            pow(add(one, r)?, 3.720)?,
        )?,
        prod(&[p(KB)?, t, p(1.26e-14)?, pow(le, 0.750)?])?,
        div(
            prod(&[p(8.0 * 3.435e-30)?, t, pow(l3, 1.970)?])?,
            pow(add(one, r3)?, 3.720)?,
        )?,
        div(
            s(p(-118348.0).and_then(|z| div(z, t)).and_then(exp)?, 7.5e-19)?,
            denom,
        )?,
        div(
            prod(&[p(5.54e-17)?, pow(t, -0.397)?, exp(div(p(-473638.0)?, t)?)?])?,
            denom,
        )?,
        prod(&[p(5.835e-11)?, sq(t)?, exp(div(p(-157804.0)?, t)?)?])?,
        prod(&[p(2.71e-11)?, sq(t)?, exp(div(p(-285331.0)?, t)?)?])?,
        prod(&[p(5.707e-12)?, sq(t)?, exp(div(p(-631495.0)?, t)?)?])?,
        {
            // Original source q=5.5 - log(T)/math.log(10), not an
            // unproved library-log simplification of an interval.
            let q = sub(p(5.5)?, div(ln(t)?, p(std::f64::consts::LN_10)?)?)?;
            let gauss = exp(s(mul(q, q)?, -1.0 / 3.0)?)?;
            prod(&[p(1.42e-27)?, sq(t)?, add(p(1.1)?, s(gauss, 0.34)?)?])?
        },
    ];
    let q = sub(p(5.5)?, div(ln(t)?, p(std::f64::consts::LN_10)?)?)?;
    let gauss = exp(s(mul(q, q)?, -1.0 / 3.0)?)?;
    let gaunt = add(p(1.1)?, s(gauss, 0.34)?)?;
    let root_correction = div(s(sqrt, 0.5)?, denom)?;
    let slopes = [
        add(p(-0.970)?, s(div(r, add(one, r)?)?, 3.720 * 0.376)?)?,
        p(0.250)?,
        add(p(-0.970)?, s(div(r3, add(one, r3)?)?, 3.720 * 0.376)?)?,
        sub(div(p(118348.0)?, t)?, root_correction)?,
        sub(add(p(-0.397)?, div(p(473638.0)?, t)?)?, root_correction)?,
        add(p(0.5)?, div(p(157804.0)?, t)?)?,
        add(p(0.5)?, div(p(285331.0)?, t)?)?,
        add(p(0.5)?, div(p(631495.0)?, t)?)?,
        add(
            p(0.5)?,
            mul(
                div(s(gauss, 0.34)?, gaunt)?,
                div(s(q, 2.0)?, p(3.0 * std::f64::consts::LN_10)?)?,
            )?,
        )?,
    ];
    let mut cooling = p(0.0)?;
    let mut dcooling = p(0.0)?;
    for k in 0..9 {
        let term = mul(c.factors[k], rates[k])?;
        cooling = add(cooling, term)?;
        dcooling = add(dcooling, mul(term, slopes[k])?)?;
    }
    let expansion = mul(c.expansion, t)?;
    let rhs = sub(sub(c.photoheat, cooling)?, expansion)?;
    let f = sub(
        sub(mul(c.ecoef, t)?, c.u0)?,
        s(add(c.constant, s(rhs, c.weight)?)?, c.dt)?,
    )?;
    let df = add(
        mul(c.ecoef, t)?,
        s(add(dcooling, expansion)?, c.dt * c.weight)?,
    )?;
    Ok(Eval { f, df, rhs })
}
#[derive(Clone, Copy)]
struct Row {
    status: u8,
    lo: f64,
    hi: f64,
    dlo: f64,
    rhs_lo: f64,
    rhs_hi: f64,
}
impl Row {
    fn blocked() -> Self {
        Self {
            status: 0,
            lo: 0.0,
            hi: 0.0,
            dlo: 0.0,
            rhs_lo: 0.0,
            rhs_hi: 0.0,
        }
    }
}
fn root(c: &Context) -> R<Row> {
    let mut lo = (c.owner_ln_t.lo - 0.008).max(0.0_f64.next_up());
    let mut hi = (c.owner_ln_t.hi + 0.008).min((1.0e5_f64).ln() - 1e-9);
    if lo >= hi {
        return Err("NO_VALID_SOURCE_T_DOMAIN_BRACKET".into());
    }
    let mut bracketed = false;
    for _ in 0..12 {
        let left = eval(c, p(lo)?)?.f;
        let right = eval(c, p(hi)?)?.f;
        if left.hi <= 0.0 && right.lo >= 0.0 {
            bracketed = true;
            break;
        }
        let margin = 0.008_f64.max((hi - lo) * 0.6);
        if left.hi > 0.0 {
            lo = (lo - margin).max(0.0_f64.next_up());
        }
        if right.lo < 0.0 {
            hi = (hi + margin).min((1.0e5_f64).ln() - 1e-9);
        }
    }
    if !bracketed {
        return Err("SOURCE_THERMAL_UNIFORM_SIGN_BRACKET_UNPROVED".into());
    }
    let mut x = new(lo, hi)?;
    let mut derivative = eval(c, x)?.df;
    if derivative.lo <= 0.0 {
        return Err("SOURCE_THERMAL_UNIQUE_ROOT_UNPROVED".into());
    }
    for _ in 0..5 {
        let mid = x.lo * 0.5 + x.hi * 0.5;
        let fm = eval(c, p(mid)?)?.f;
        derivative = eval(c, x)?.df;
        if derivative.lo <= 0.0 {
            return Err("SOURCE_THERMAL_NEWTON_DERIVATIVE_NOT_POSITIVE".into());
        }
        let nm = sub(p(mid)?, div(fm, derivative)?)?;
        let nlo = x.lo.max(nm.lo);
        let nhi = x.hi.min(nm.hi);
        if nlo > nhi {
            return Err("SOURCE_THERMAL_NEWTON_TUBE_EMPTY".into());
        }
        x = new(nlo, nhi)?;
    }
    let last = eval(c, x)?;
    if last.df.lo <= 0.0 {
        return Err("SOURCE_THERMAL_FINAL_DERIVATIVE_UNPROVED".into());
    }
    // If both enclosures refer to the same parameter family, disjoint roots
    // cannot be accepted as a source-consistent physical result.
    if x.hi < c.owner_ln_t.lo || c.owner_ln_t.hi < x.lo {
        return Err("ORIGINAL_THERMAL_OWNER_ROOT_DISJOINT".into());
    }
    // For source-faithful comparable temperature boxes, the thermal energy
    // derivative and source RHS are also independently evaluated.
    Ok(Row {
        status: 1,
        lo: x.lo,
        hi: x.hi,
        dlo: last.df.lo,
        rhs_lo: last.rhs.lo,
        rhs_hi: last.rhs.hi,
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("USAGE: rei_original46k_native_thermal_roots ORIGINAL.raw ORIGINAL.manifest.json RUST.output.raw".into());
    }
    let bytes = fs::read(&args[1])?;
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if sha(&bytes) != SOURCE_SHA
        || receipt["source_binary_sha256"] != SOURCE_SHA
        || receipt["original_source_head"] != "f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa"
        || receipt["original_kernel_sha256"]
            != "579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
        || receipt["owner_14_final_set_ledgers"] != "14/14_PASS_OWNER_ONLY"
        || receipt["official_tick161"] != "NOT_ACCEPTED"
    {
        return Err("ORIGINAL_THERMAL_SOURCE_AUTHORITY_DRIFT".into());
    }
    if bytes.len() != HEADER + ROOTS * (RECORD_HEADER + N * COLS * 8)
        || &bytes[..8] != MAGIC
        || u64::from_le_bytes(bytes[8..16].try_into()?) != N as u64
        || u64::from_le_bytes(bytes[16..24].try_into()?) != ROOTS as u64
    {
        return Err("ORIGINAL_THERMAL_CONTEXT_BINARY_SCHEMA_DRIFT".into());
    }
    let now = Instant::now();
    let mut result = Vec::with_capacity(24 + ROOTS * N * 41);
    result.extend_from_slice(RESULT_MAGIC);
    result.extend_from_slice(&(N as u64).to_le_bytes());
    result.extend_from_slice(&(ROOTS as u64).to_le_bytes());
    let mut reports = Vec::new();
    for m in 0..ROOTS {
        let head = HEADER + m * (RECORD_HEADER + N * COLS * 8);
        let map = m / 3;
        let stage = m % 3;
        if &bytes[head..head + 8] != [b"FULL0000", b"HALF1000", b"HALF2000"][map]
            || &bytes[head + 8..head + 16] != [b"PREDICT0", b"STAGE000", b"FINAL000"][stage]
        {
            return Err("ORIGINAL_THERMAL_SOURCE_STAGE_ORDER_DRIFT".into());
        }
        let dt = read_f64(&bytes, head + 16)?;
        let weight = read_f64(&bytes, head + 24)?;
        let rows = (0..N)
            .into_par_iter()
            .map(|i| {
                let start = head + RECORD_HEADER + i * COLS * 8;
                let c = Context::new(&bytes[start..start + COLS * 8], dt, weight);
                match c.and_then(|ctx| root(&ctx)) {
                    Ok(x) => (x, None),
                    Err(e) => (Row::blocked(), Some((i, e))),
                }
            })
            .collect::<Vec<_>>();
        let mut passed = 0usize;
        let mut min_d = f64::INFINITY;
        let mut max_width = 0.0_f64;
        let mut fails = Vec::new();
        for (row, why) in rows {
            result.push(row.status);
            for x in [row.lo, row.hi, row.dlo, row.rhs_lo, row.rhs_hi] {
                result.extend_from_slice(&x.to_le_bytes());
            }
            if row.status == 1 {
                passed += 1;
                min_d = min_d.min(row.dlo);
                max_width = max_width.max(row.hi - row.lo);
            } else if let Some((i, e)) = why {
                if fails.len() < 12 {
                    fails.push(json!({"cell":i,"reason":e}));
                }
            }
        }
        let map_name = ["full", "half1", "half2"][map];
        let root_name = ["predictor", "stage", "final"][stage];
        reports.push(json!({
          "map":map_name,
          "root":root_name,
          "true_source_cells":N,"positive_verified_root_cells":passed,
          "blocked_cells":N-passed,"min_verified_dF_dlogT":if passed>0{Some(min_d)}else{None},
          "max_verified_root_logT_width":max_width,
          "first_failures":fails,"duration_seconds":dt,"weight":weight,
          "original_owner_reference":"ORIGINAL_26EVENT_SDIRK2_PARAMETRIC_ROOTS"}));
    }
    fs::write(&args[3], &result)?;
    let verified_total: u64 = reports
        .iter()
        .map(|r| r["positive_verified_root_cells"].as_u64().unwrap_or(0))
        .sum();
    let out = json!({"schema":"REI_F05_NATIVE_SOURCE_THERMAL_SDIRK2_ROOTS_V1",
       "source_sha256":SOURCE_SHA,"source_26event_kernel_sha256":
         "579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644",
       "nodes":N,"roots":ROOTS,"source_root_reports":reports,
       "verified_root_cells":verified_total,
       "total_root_cells":(N*ROOTS),"native_result_sha256":sha(&result),"native_result_bytes":result.len(),
       "math_backend":"VigilODE_core_directed_plus_byte_exact_original_REI_transcendental",
       "original_14_set_ledgers":"OWNER_PASS_NOT_NATIVE",
       "source_OTS_and_26_event_interval_IVP":"NOT_ESTABLISHED",
       "original_v1_tick161":"NOT_ACCEPTED","canonical_delta":0,"production":"HOLD",
       "elapsed_seconds_native_root_only":now.elapsed().as_secs_f64()});
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verified_exp_contains_elementary_value() {
        let x = p(1.0).unwrap();
        let y = exp(x).unwrap();
        assert!(y.contains(std::f64::consts::E));
    }
    #[test]
    fn cooling_free_thermal_root() {
        let t = 10_000.0_f64;
        let c = Context {
            photoheat: p(0.0).unwrap(),
            expansion: p(0.0).unwrap(),
            ecoef: p(1.0).unwrap(),
            u0: p(t).unwrap(),
            constant: p(0.0).unwrap(),
            factors: [p(0.0).unwrap(); 9],
            dt: 1.0,
            weight: 1.0,
            owner_ln_t: new(t.ln() - 1e-8, t.ln() + 1e-8).unwrap(),
            owner_rhs: p(0.0).unwrap(),
        };
        let z = root(&c).unwrap();
        assert!(z.lo <= t.ln() && t.ln() <= z.hi && z.dlo > 0.0);
    }
    #[test]
    fn forged_source_negative_fails() {
        let mut b = [0.0_f64; COLS];
        for i in 0..16 {
            b[2 * i + 1] = 1.0;
        }
        b[2 * 5] = -1.0;
        let mut raw = Vec::new();
        for x in b {
            raw.extend_from_slice(&x.to_le_bytes());
        }
        assert!(Context::new(&raw, 1.0, 1.0).is_err());
    }
}
