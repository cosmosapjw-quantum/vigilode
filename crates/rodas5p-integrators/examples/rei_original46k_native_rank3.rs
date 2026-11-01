//! Original REI IGM F05 46,080-cell PHOTON-PARTIAL, native Rust rank-three JVP.
//!
//! This opt-in source-scoped experiment reuses VigilODE's EXISTING native
//! low-rank bordered factorization, not a Python ODE solver. The fixture is
//! exported from immutable original 26-event REI owner and contains its actual
//! neutral populations, photon source and original strict interval source boxes.
//! Full H/He/OTS/thermal Jacobian, native 26-event IVP and tick161 are NOT solved.
//! The positive HeII trace underflows in binary64; native log10 diagnostic
//! preserves its SOURCE magnitude rather than calling it a physical zero.
use rodas5p_core::low_rank_bordered::{LowRankApplyWorkspace, LowRankBorderedPreconditioner};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::io::{self, Read};
use std::time::Instant;

const SOURCE_RAW_SHA: &str = "a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469";
const ORIGINAL_N: usize = 46_080;
const MAGIC: &[u8; 8] = b"REIF05R1";

#[derive(Clone, Copy, Debug, Default)]
struct DD {
    hi: f64,
    lo: f64,
}
impl DD {
    fn new(x: f64) -> Self {
        Self { hi: x, lo: 0.0 }
    }
    fn value(self) -> f64 {
        self.hi + self.lo
    }
    fn add(self, rhs: Self) -> Self {
        let s = self.hi + rhs.hi;
        let v = s - self.hi;
        let e = (self.hi - (s - v)) + (rhs.hi - v);
        let tail = e + self.lo + rhs.lo;
        let hi = s + tail;
        Self {
            hi,
            lo: tail - (hi - s),
        }
    }
    fn neg(self) -> Self {
        Self {
            hi: -self.hi,
            lo: -self.lo,
        }
    }
    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }
    fn mul(self, rhs: Self) -> Self {
        let p = self.hi * rhs.hi;
        let e =
            self.hi.mul_add(rhs.hi, -p) + self.hi * rhs.lo + self.lo * rhs.hi + self.lo * rhs.lo;
        let hi = p + e;
        Self {
            hi,
            lo: e - (hi - p),
        }
    }
    fn div(self, rhs: Self) -> Result<Self, String> {
        if !(rhs.hi.is_finite() && rhs.hi > 0.0) {
            return Err("NONPOSITIVE_NATIVE_PHOTON_DENOMINATOR".into());
        }
        let q1 = self.hi / rhs.hi;
        if !q1.is_finite() {
            return Err("NONFINITE_DD_QUOTIENT".into());
        }
        let rem = self.sub(rhs.mul(Self::new(q1)));
        let q2 = (rem.hi + rem.lo) / rhs.hi;
        let q = Self::new(q1).add(Self::new(q2));
        if !q.value().is_finite() {
            return Err("NONFINITE_DD_RESULT".into());
        }
        Ok(q)
    }
}

#[derive(Clone)]
struct Snapshot {
    n: usize,
    j: [f64; 4],
    e: [f64; 4],
    alpha: [[f64; 3]; 4],
    excess: [[f64; 4]; 3],
    ev: f64,
    populations: Vec<f64>,
    rate_lo: Vec<f64>,
    rate_hi: Vec<f64>,
    heat_lo: Vec<f64>,
    heat_hi: Vec<f64>,
}

fn exact_sha(x: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(x);
    format!("{:x}", h.finalize())
}

fn fixture(data: &[u8], expected_sha: Option<&str>) -> Result<Snapshot, String> {
    if let Some(expected) = expected_sha {
        if exact_sha(data) != expected {
            return Err("ORIGINAL_SOURCE_RAW_SHA_MISMATCH".into());
        }
    }
    if data.len() < 16 || &data[..8] != MAGIC {
        return Err("WRONG_SOURCE_FIXTURE_MAGIC".into());
    }
    let n = usize::try_from(u64::from_le_bytes(data[8..16].try_into().unwrap()))
        .map_err(|_| "CELL_COUNT_OVERFLOW")?;
    if n == 0 || n > ORIGINAL_N {
        return Err("WRONG_SOURCE_CELL_COUNT".into());
    }
    let expected = 16usize
        .checked_add(
            8usize
                .checked_mul(33 + 11 * n)
                .ok_or("SOURCE_FIXTURE_SIZE_OVERFLOW")?,
        )
        .ok_or("SIZE_OVERFLOW")?;
    if data.len() != expected {
        return Err("SOURCE_FIXTURE_LENGTH_MISMATCH".into());
    }
    let mut offset = 16usize;
    let mut read = || {
        let out = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        out
    };
    let j = std::array::from_fn(|_| read());
    let e = std::array::from_fn(|_| read());
    let alpha = std::array::from_fn(|_| std::array::from_fn(|_| read()));
    let excess = std::array::from_fn(|_| std::array::from_fn(|_| read()));
    let ev = read();
    let populations = (0..3 * n).map(|_| read()).collect::<Vec<_>>();
    let rate_lo = (0..3 * n).map(|_| read()).collect::<Vec<_>>();
    let rate_hi = (0..3 * n).map(|_| read()).collect::<Vec<_>>();
    let heat_lo = (0..n).map(|_| read()).collect::<Vec<_>>();
    let heat_hi = (0..n).map(|_| read()).collect::<Vec<_>>();
    if offset != expected {
        return Err("SOURCE_BINARY_UNCONSUMED".into());
    }
    if j.iter().chain(&e).any(|x| !x.is_finite() || *x < 0.0)
        || alpha.iter().flatten().any(|x| !x.is_finite() || *x < 0.0)
        || excess.iter().flatten().any(|x| !x.is_finite() || *x < 0.0)
        || !ev.is_finite()
        || ev <= 0.0
    {
        return Err("SOURCE_PHOTON_PARAMETER_INVALID".into());
    }
    if populations.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err("SOURCE_POPULATIONS_NOT_STRICT_POSITIVE".into());
    }
    for i in 0..3 * n {
        if !rate_lo[i].is_finite() || !rate_hi[i].is_finite() || rate_lo[i] > rate_hi[i] {
            return Err("ORIGINAL_PHOTON_INTERVAL_INVERTED".into());
        }
    }
    for i in 0..n {
        if !heat_lo[i].is_finite() || !heat_hi[i].is_finite() || heat_lo[i] > heat_hi[i] {
            return Err("ORIGINAL_HEAT_INTERVAL_INVERTED".into());
        }
    }
    Ok(Snapshot {
        n,
        j,
        e,
        alpha,
        excess,
        ev,
        populations,
        rate_lo,
        rate_hi,
        heat_lo,
        heat_hi,
    })
}
struct Coeff {
    totals: [f64; 3],
    den: [f64; 4],
    local: [[f64; 3]; 4],
    kappa: [f64; 3],
    q: [f64; 3],
    b: [[f64; 3]; 3],
    emitted: f64,
    escaped: f64,
    heii_log10: f64,
}

fn log10sum(logs: &[f64]) -> f64 {
    let m = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !m.is_finite() {
        return f64::NEG_INFINITY;
    }
    m + logs.iter().map(|x| 10f64.powf(*x - m)).sum::<f64>().log10()
}

fn coeff(src: &Snapshot) -> Result<Coeff, String> {
    let mut totals_dd = [DD::default(); 3];
    for i in 0..src.n {
        for (s, t) in totals_dd.iter_mut().enumerate() {
            *t = t.add(DD::new(src.populations[3 * i + s]));
        }
    }
    let totals = totals_dd.map(DD::value);
    let mut den = [0.; 4];
    let mut local = [[0.; 3]; 4];
    let mut kappa = [DD::default(); 3];
    let mut q = [DD::default(); 3];
    let mut b = [[DD::default(); 3]; 3];
    let mut emitted = DD::default();
    let mut escaped = DD::default();
    let mut heii_logs = Vec::new();
    for g in 0..4 {
        emitted = emitted.add(DD::new(src.j[g]));
        let mut d = DD::new(src.e[g]);
        for s in 0..3 {
            d = d.add(DD::new(src.alpha[g][s]).mul(totals_dd[s]));
        }
        den[g] = d.value();
        if !(den[g] > 0.0 && den[g].is_finite()) {
            return Err("ORIGINAL_PHOTON_GROUP_DENOMINATOR_FAIL".into());
        }
        escaped = escaped.add(DD::new(src.j[g]).mul(DD::new(src.e[g])).div(d)?);
        for s in 0..3 {
            // j*alpha is representable for HI/HeI; for HeII the genuine
            // positive original e-389 coefficient underflows in f64.
            let ls = DD::new(src.j[g]).mul(DD::new(src.alpha[g][s])).div(d)?;
            local[g][s] = ls.value();
            kappa[s] = kappa[s].add(ls);
            q[s] = q[s].add(ls.mul(DD::new(src.excess[s][g] * src.ev)));
            for t in 0..3 {
                b[s][t] = b[s][t].add(ls.mul(DD::new(src.alpha[g][t])).div(d)?);
            }
        }
        if src.j[g] > 0.0 && src.alpha[g][2] > 0.0 && totals[2] > 0.0 {
            heii_logs.push(
                src.j[g].log10() + src.alpha[g][2].log10() + totals[2].log10() - den[g].log10(),
            );
        }
    }
    let heii_log10 = log10sum(&heii_logs);
    Ok(Coeff {
        totals,
        den,
        local,
        kappa: kappa.map(DD::value),
        q: q.map(DD::value),
        b: b.map(|row| row.map(DD::value)),
        emitted: emitted.value(),
        escaped: escaped.value(),
        heii_log10,
    })
}
fn source_fields(src: &Snapshot, c: &Coeff) -> (Vec<f64>, Vec<f64>) {
    let mut p = vec![0.0; 3 * src.n];
    let mut heat = vec![0.0; src.n];
    for i in 0..src.n {
        let mut h = 0.0;
        for s in 0..3 {
            let x = src.populations[3 * i + s];
            p[3 * i + s] = x * c.kappa[s];
            h += x * c.q[s];
        }
        heat[i] = h;
    }
    (p, heat)
}
fn check_source(src: &Snapshot, c: &Coeff, p: &[f64], heat: &[f64]) -> ([usize; 3], usize, f64) {
    let mut included = [0usize; 3];
    let mut included_heat = 0;
    let mut sum = DD::default();
    for i in 0..src.n {
        for s in 0..3 {
            let k = 3 * i + s;
            if src.rate_lo[k] <= p[k] && p[k] <= src.rate_hi[k] {
                included[s] += 1;
            }
            sum = sum.add(DD::new(p[k]));
        }
        if src.heat_lo[i] <= heat[i] && heat[i] <= src.heat_hi[i] {
            included_heat += 1;
        }
    }
    let budget = (sum.value() + c.escaped - c.emitted).abs() / c.emitted.abs().max(1e-300);
    (included, included_heat, budget)
}
fn moment(v: &[f64]) -> [f64; 3] {
    let mut sum = [DD::default(); 3];
    for chunk in v.chunks_exact(3) {
        for s in 0..3 {
            sum[s] = sum[s].add(DD::new(chunk[s]));
        }
    }
    sum.map(DD::value)
}
fn direction(n: &[f64]) -> Vec<f64> {
    n.iter()
        .enumerate()
        .map(|(k, &x)| {
            let index = k / 3;
            let s = k % 3;
            x * (((index + 1) as f64) * [0.101, 0.203, 0.307][s]).cos()
        })
        .collect()
}
fn jvp_group(src: &Snapshot, c: &Coeff, v: &[f64]) -> Vec<f64> {
    let vm = moment(v);
    let mut out = vec![0.0; 3 * src.n];
    for g in 0..4 {
        let common = (0..3).map(|s| src.alpha[g][s] * vm[s]).sum::<f64>() / c.den[g];
        for i in 0..src.n {
            for s in 0..3 {
                let k = 3 * i + s;
                out[k] += c.local[g][s] * (v[k] - src.populations[k] * common);
            }
        }
    }
    out
}
fn jvp_rank3(src: &Snapshot, c: &Coeff, v: &[f64]) -> Vec<f64> {
    let vm = moment(v);
    let q: [f64; 3] = std::array::from_fn(|s| (0..3).map(|t| c.b[s][t] * vm[t]).sum::<f64>());
    let mut out = vec![0.; 3 * src.n];
    for i in 0..src.n {
        for s in 0..3 {
            let k = 3 * i + s;
            out[k] = c.kappa[s] * v[k] - src.populations[k] * q[s];
        }
    }
    out
}
fn native_jvp_error(base: &[f64], fast: &[f64]) -> f64 {
    let scale = base.iter().copied().map(f64::abs).fold(1e-300, f64::max);
    base.iter()
        .zip(fast)
        .map(|(x, y)| (x - y).abs())
        .fold(0., f64::max)
        / scale
}
fn finite_difference(src: &Snapshot, v: &[f64], epsilon: f64) -> Result<Vec<f64>, String> {
    let mut plus = src.clone();
    let mut minus = src.clone();
    for i in 0..v.len() {
        plus.populations[i] += epsilon * v[i];
        minus.populations[i] -= epsilon * v[i];
        if plus.populations[i] <= 0.0 || minus.populations[i] <= 0.0 {
            return Err("FINITE_DIFFERENCE_LEFT_PHYSICAL_CONE".into());
        }
    }
    let (p, _) = source_fields(&plus, &coeff(&plus)?);
    let (m, _) = source_fields(&minus, &coeff(&minus)?);
    Ok(p.iter()
        .zip(&m)
        .map(|(x, y)| (x - y) / (2.0 * epsilon))
        .collect())
}
fn rank3_w_inverse(src: &Snapshot, c: &Coeff) -> Result<(f64, f64), String> {
    let n = src.n * 3;
    let max_kappa = c.kappa.iter().copied().fold(0.0, f64::max);
    if !(max_kappa.is_finite() && max_kappa > 0.0) {
        return Err("NO_NATIVE_NONZERO_PHOTO_SOURCE".into());
    }
    // Fixed diagnostic linearization only, NOT the original proper-time h or
    // the full 26-event W matrix.
    let theta = 0.2 / max_kappa;
    let diag = (0..n)
        .map(|k| 1.0 + theta * c.kappa[k % 3])
        .collect::<Vec<_>>();
    let mut u = vec![0.; n * 3];
    let mut v = vec![0.; n * 3];
    for i in 0..src.n {
        for s in 0..3 {
            let row = i * 3 + s;
            for t in 0..3 {
                u[row * 3 + t] = -theta * src.populations[row] * c.b[s][t];
                v[row * 3 + t] = if s == t { 1.0 } else { 0.0 };
            }
        }
    }
    let mut pc = LowRankBorderedPreconditioner::new(n, 0, 3, 0, 0)
        .map_err(|e| format!("NATIVE_PRECONDITIONER_CONSTRUCTION:{e}"))?;
    let clock = Instant::now();
    pc.refactor(n, 0, 3, &diag, &[], &[], &[], &u, &v)
        .map_err(|e| format!("NATIVE_PRECONDITIONER_FACTORIZATION:{e}"))?;
    let factor_seconds = clock.elapsed().as_secs_f64();
    let mut ws =
        LowRankApplyWorkspace::new(n, 0, 3).map_err(|e| format!("NATIVE_WORKSPACE:{e}"))?;
    let rhs = (0..n)
        .map(|k| ((k + 3) as f64 * 0.01437).sin())
        .collect::<Vec<_>>();
    let mut x = vec![0.0; n];
    pc.apply_with_workspace(&rhs, &mut x, &mut ws)
        .map_err(|e| format!("NATIVE_RANK3_W_APPLY:{e}"))?;
    let j = jvp_rank3(src, c, &x);
    let mut worst = 0.0_f64;
    for i in 0..n {
        let reconstructed = x[i] + theta * j[i];
        worst = worst.max((reconstructed - rhs[i]).abs());
    }
    Ok((worst, factor_seconds))
}
fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    (v[(v.len() - 1) / 2] + v[v.len() / 2]) * 0.5
}
fn benchmark(src: &Snapshot, c: &Coeff, direction: &[f64]) -> (f64, f64) {
    let mut a = Vec::new();
    let mut b = Vec::new();
    for k in 0..19 {
        let baseline_first = k % 2 == 0;
        if baseline_first {
            let t = Instant::now();
            black_box(jvp_group(src, c, direction));
            a.push(t.elapsed().as_secs_f64());
            let t = Instant::now();
            black_box(jvp_rank3(src, c, direction));
            b.push(t.elapsed().as_secs_f64());
        } else {
            let t = Instant::now();
            black_box(jvp_rank3(src, c, direction));
            b.push(t.elapsed().as_secs_f64());
            let t = Instant::now();
            black_box(jvp_group(src, c, direction));
            a.push(t.elapsed().as_secs_f64());
        }
    }
    (median(&mut a), median(&mut b))
}

fn study(src: &Snapshot, mode: &str) -> Result<serde_json::Value, String> {
    let timer = Instant::now();
    let c = coeff(src)?;
    let setup_seconds = timer.elapsed().as_secs_f64();
    let (rates, heat) = source_fields(src, &c);
    let (inside, heat_inside, budget) = check_source(src, &c, &rates, &heat);
    if inside.iter().any(|&cells| cells != src.n) || heat_inside != src.n {
        return Err(format!(
            "ORIGINAL_OWNER_INTERVAL_EXCLUSION:rate={inside:?} heat={heat_inside} of {}",
            src.n
        ));
    }
    if src.n == ORIGINAL_N && (!c.heii_log10.is_finite() || c.heii_log10 >= 0.0) {
        return Err("ORIGINAL_POSITIVE_HEII_EXTENDED_TRACE_NOT_PRESERVED_AS_LOG".into());
    }
    let v = direction(&src.populations);
    let group = jvp_group(src, &c, &v);
    let fast = jvp_rank3(src, &c, &v);
    let relative_jvp = native_jvp_error(&group, &fast);
    let fd = finite_difference(src, &v, 2e-4)?;
    let relative_fd = native_jvp_error(&fd, &fast);
    let (w_residual, w_seconds) = rank3_w_inverse(src, &c)?;
    let (group_s, fast_s) = benchmark(src, &c, &v);
    if relative_jvp >= 1e-10
        || relative_fd >= 1e-6
        || !budget.is_finite()
        || budget >= 1e-12
        || w_residual >= 1e-8
    {
        return Err(format!(
            "SCIENTIFIC_SOURCE_DIAGNOSTIC_FAILED:diff={relative_jvp} fd={relative_fd} ledger={budget} W={w_residual}"
        ));
    }
    Ok(json!({
      "schema":"VIGILODE_RUST_REI_ORIGINAL26_F05_T160_NATIVE_PHOTON_RANK3_V1",
      "mode":mode,"source_fixture_raw_sha256":SOURCE_RAW_SHA,
      "real_original_cells":src.n,"source_photon_groups":4,
      "neutral_moments":3,"photon_partial_nonlocal_rank_upper":3,
      "source_stage":"ORIGINAL_26EVENT_ACCEPTED_T160_PHOTON_SOURCE_PARTIAL_ONLY",
      "source_owner_interval_HI_HeI_HeII_inside":[inside[0],inside[1],inside[2]],
      "source_owner_heat_cells_inside":heat_inside,
      "owner_interval_all_46080_included":inside.iter().all(|x|*x==src.n)&&heat_inside==src.n,
      "photon_ledger_relative_residual":budget,
      "source_emitted_per_s":c.emitted,
      "source_escaped_per_s":c.escaped,
      "source_total_neutral_counts":c.totals,
      "native_f64_kappa":c.kappa,
      "native_f64_heii_rate_underflowed":c.kappa[2]==0.0,
      "native_heii_source_integrated_log10_nonzero":c.heii_log10,
      "no_claim_exact_zero_heii":true,
      "source_direct_vs_rank3_jvp_max_relative":relative_jvp,
      "source_finite_difference_vs_rank3_max_relative":relative_fd,
      "native_existing_LowRankBorderedPreconditioner_W_residual_linf":w_residual,
      "native_lowrank_W_factor_seconds":w_seconds,
      "native_source_cache_setup_seconds":setup_seconds,
      "native_groupwise_jvp_median_seconds":group_s,
      "native_rank3_jvp_median_seconds":fast_s,
      "native_repeated_jvp_speedup_excluding_setup":group_s/fast_s,
      "timing":"single_host_research_only; fixed_source_19_alternating_calls; no_full_IVP",
      "original_complete_26event_ode":"NOT_RUN",
      "original_14_ledgers":"NOT_RUN_IN_RUST",
      "tick161":"NOT_ACCEPTED","production":"HOLD","canonical_delta":0,
      "vigilode_official_dag":"7/23","rei_central_dag":"28/88"
    }))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_sha = SOURCE_RAW_SHA;
    let mut b = Vec::new();
    io::stdin().read_to_end(&mut b)?;
    let src = fixture(&b, Some(raw_sha))?;
    if src.n != ORIGINAL_N {
        return Err("NOT_REAL_ORIGINAL_46080_SOURCE".into());
    }
    let report = study(&src, "ORIGINAL_F05_REAL_NATIVE_RUST_SOURCE_PARTIAL")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Snapshot {
        let n = 3;
        let populations = vec![1., 2., 3., 2., 3., 4., 3., 4., 5.];
        Snapshot {
            n,
            j: [2., 3., 1., 0.],
            e: [1., 2., 4., 1.],
            alpha: [[1., 0., 0.], [0.5, 0.3, 0.1], [0.1, 0.2, 0.3], [0., 0., 0.]],
            excess: [[2., 3., 4., 0.], [1., 2., 3., 0.], [2., 4., 6., 0.]],
            ev: 1e-12,
            populations,
            rate_lo: vec![-1e6; 3 * n],
            rate_hi: vec![1e6; 3 * n],
            heat_lo: vec![-1e6; n],
            heat_hi: vec![1e6; n],
        }
    }
    #[test]
    fn native_rank3_matches_groupwise_and_real_difference() {
        let p = sample();
        let c = coeff(&p).unwrap();
        let v = direction(&p.populations);
        let a = jvp_group(&p, &c, &v);
        let b = jvp_rank3(&p, &c, &v);
        assert!(native_jvp_error(&a, &b) < 1e-13);
        let fd = finite_difference(&p, &v, 1e-5).unwrap();
        assert!(native_jvp_error(&fd, &b) < 1e-8);
        let (r, _) = rank3_w_inverse(&p, &c).unwrap();
        assert!(r < 1e-12, "{r}");
    }
    #[test]
    fn source_native_positive_and_ledger() {
        let p = sample();
        let c = coeff(&p).unwrap();
        let (r, h) = source_fields(&p, &c);
        let (n, hn, budget) = check_source(&p, &c, &r, &h);
        assert_eq!(n, [3, 3, 3]);
        assert_eq!(hn, 3);
        assert!(budget < 1e-13);
        assert!(r.iter().all(|x| *x >= 0.0));
    }
    #[test]
    fn original_fixture_checksum_and_shape_fail_closed() {
        assert!(fixture(&[0u8; 16], None).is_err());
        let mut data = Vec::new();
        data.extend(MAGIC);
        data.extend(1u64.to_le_bytes());
        assert!(fixture(&data, None).is_err());
        assert!(fixture(&data, Some(SOURCE_RAW_SHA)).is_err());
    }
    #[test]
    fn source_denominator_and_invalid_state_fail_closed() {
        let mut p = sample();
        p.e = [0.; 4];
        p.alpha = [[0.; 3]; 4];
        assert!(coeff(&p).is_err());
        let mut p = sample();
        p.populations[0] = f64::NAN;
        assert!(
            coeff(&p).is_err(),
            "invalid state must not form coefficients"
        );
    }
}
