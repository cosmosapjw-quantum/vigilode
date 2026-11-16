//! Original REI 46,080 × three-map MPRK22(1) interval-corrector certificate.
//! These are real original source-owner boxes, NOT a replacement time-map,
//! thermal/OTS uncertainty interval IVP, production admission or tick161.
use rodas5p_core::directed::{
    add_down, add_up, div_down, sub_down, sub_up, sum_up, Interval as IV,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, time::Instant};

const N: usize = 46080;
const COLUMNS: usize = 70;
const MAPS: usize = 3;
const MAGIC: &[u8; 8] = b"REIF05C2";
const OUTPUT_MAGIC: &[u8; 8] = b"REIF05I2";
const SOURCE_SHA: &str = "95b2cdbfeb5777c2b893910f1663c271d0ead673629180ece068bf3e0a6fb001";
const OWNER_CSV_SHA: &str = "11df439390209bd48742ca9f4303c8743c9e14900289805e13d1ed489450a5dd";
type R<T> = Result<T, String>;
const ZERO: IV = IV { lo: 0.0, hi: 0.0 };
fn err<T>(r: rodas5p_core::CoreResult<T>) -> R<T> {
    r.map_err(|e| e.to_string())
}
fn point(x: f64) -> R<IV> {
    err(IV::point(x))
}
fn boxx(lo: f64, hi: f64) -> R<IV> {
    err(IV::new(lo, hi))
}
fn add(a: IV, b: IV) -> R<IV> {
    err(a.add(b))
}
fn sub(a: IV, b: IV) -> R<IV> {
    err(a.sub(b))
}
fn mul(a: IV, b: IV) -> R<IV> {
    err(a.mul(b))
}
fn div(a: IV, b: IV) -> R<IV> {
    err(a.div(b))
}
fn sha(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}
fn rd(b: &[u8], start: usize) -> R<f64> {
    if start + 8 > b.len() {
        return Err("MPRK_SOURCE_BINARY_TRUNCATED".into());
    }
    let x = f64::from_le_bytes(b[start..start + 8].try_into().unwrap());
    if !x.is_finite() {
        return Err("MPRK_SOURCE_NONFINITE_F64".into());
    }
    Ok(x)
}
fn solve(a: [[f64; 3]; 3], b: [f64; 3], n: usize) -> R<[f64; 3]> {
    if n != 2 && n != 3 {
        return Err("MPRK_UNSUPPORTED_BLOCK".into());
    }
    let mut m = [[0.0_f64; 4]; 3];
    for i in 0..n {
        for j in 0..n {
            m[i][j] = a[i][j];
        }
        m[i][n] = b[i];
    }
    for k in 0..n {
        let pivot = m[k][k];
        if !(pivot.is_finite() && pivot > 0.0) {
            return Err("ORIGINAL_F05_MPRK_POSITIVE_PIVOT_MISSING".into());
        }
        for i in k + 1..n {
            let fac = m[i][k] / pivot;
            for j in k + 1..=n {
                m[i][j] -= fac * m[k][j];
            }
            m[i][k] = 0.0;
        }
    }
    let mut x = [0.0_f64; 3];
    for i in (0..n).rev() {
        let mut v = m[i][n];
        for j in i + 1..n {
            v -= m[i][j] * x[j];
        }
        x[i] = v / m[i][i];
        if !x[i].is_finite() {
            return Err("NONFINITE_TRIAL_MPRK_INCREMENT".into());
        }
    }
    Ok(x)
}
struct Cell {
    p: [IV; 5],
    den: [IV; 5],
    flux: [IV; 25],
    had_original_zero_widening: bool,
}
fn decode_cell(bytes: &[u8], offset: usize) -> R<Cell> {
    let mut a = [0.0; COLUMNS];
    for (k, x) in a.iter_mut().enumerate() {
        *x = rd(bytes, offset + 8 * k)?;
    }
    let mut p = [ZERO; 5];
    let mut den = [ZERO; 5];
    let mut flux = [ZERO; 25];
    let mut clipped = false;
    for i in 0..5 {
        p[i] = boxx(a[i], a[i + 5])?;
        den[i] = boxx(a[i + 60], a[i + 65])?;
        if p[i].lo <= 0.0 || den[i].lo <= 0.0 {
            return Err("ORIGINAL_F05_MPRK_PARENT_OR_PREDICTOR_NONPOSITIVE".into());
        }
    }
    for k in 0..25 {
        let lo = a[10 + k];
        let hi = a[35 + k];
        if lo < -f64::from_bits(1) || hi < 0.0 || lo > hi {
            return Err("ORIGINAL_F05_NEGATIVE_PHYSICAL_EVENT_RATE".into());
        }
        if lo < 0.0 {
            clipped = true;
        }
        flux[k] = boxx(lo.max(0.0), hi)?;
    }
    for s in 0..5 {
        for d in 0..5 {
            if (s < 2) != (d < 2) {
                // The frozen original 18-event graph has no H<->He material
                // population transfers. Its generic interval writer rounds
                // all structural zero entries to [-minsub,+minsub].
                // Restore the exactly-zero *source graph*, but only when
                // the recorded owner bounds match that representation.
                let k = 5 * d + s;
                if a[10 + k] < -f64::from_bits(1) || a[35 + k] > f64::from_bits(1) {
                    return Err("ORIGINAL_INTERELEMENT_GRAPH_OR_SOURCE_DRIFT".into());
                }
                flux[k] = ZERO;
            }
        }
    }
    Ok(Cell {
        p,
        den,
        flux,
        had_original_zero_widening: clipped,
    })
}
fn cert_block(p: &[IV; 5], a: &[[IV; 5]; 5], start: usize, n: usize) -> R<([IV; 3], f64, bool)> {
    let mut mat = [[0.0; 3]; 3];
    let mut rhs = [0.0; 3];
    for i in 0..n {
        let row = start + i;
        for j in 0..n {
            let col = start + j;
            mat[i][j] = 0.5 * a[row][col].lo + 0.5 * a[row][col].hi;
        }
        for j in 0..n {
            let q = start + j;
            let x = 0.5 * p[q].lo + 0.5 * p[q].hi;
            rhs[i] -= (mat[i][j] - if i == j { 1.0 } else { 0.0 }) * x;
        }
    }
    let trial = solve(mat, rhs, n)?;
    let mut rr = [ZERO; 3];
    for i in 0..n {
        let row = start + i;
        // b=-(A-I)p, calculated with the original source input boxes
        let mut b = ZERO;
        let mut az = ZERO;
        for j in 0..n {
            let col = start + j;
            let shift = if i == j { point(1.0)? } else { ZERO };
            b = sub(b, mul(sub(a[row][col], shift)?, p[col])?)?;
            az = add(az, mul(a[row][col], point(trial[j])?)?)?;
        }
        rr[i] = sub(b, az)?;
    }
    // exact M-matrix inverse 1-norm = 1 for EVERY physical member.
    // Directed residual bounds automatically include parent/flux/den rounding.
    let radius = err(sum_up((0..n).map(|i| rr[i].mag())))?;
    if !radius.is_finite() {
        return Err("MPRK_CERTIFICATE_RESIDUAL_OVERFLOW".into());
    }
    let mut raw = [ZERO; 3];
    for i in 0..n {
        let global = start + i;
        let low = err(sub_down(trial[i], radius))?;
        let high = err(add_up(trial[i], radius))?;
        // Independent physics: y_i >= p_i/A_ii >0 and
        // y_i <= sum_s p_s in this conservative nucleus block.
        let guaranteed_ylo = err(div_down(p[global].lo, a[global][global].hi))?;
        let max_nuclei = err(sum_up((0..n).map(|j| p[start + j].hi)))?;
        let cone_dlo = err(sub_down(guaranteed_ylo, p[global].hi))?;
        let cone_dhi = err(sub_up(max_nuclei, p[global].lo))?;
        let lo = low.max(cone_dlo);
        let hi = high.min(cone_dhi);
        if lo > hi {
            return Err("EMPTY_NATIVE_MPRK_RESIDUAL_CONE_INTERSECTION".into());
        }
        raw[i] = boxx(lo, hi)?;
    }
    // The exact directed-transfer conservation identity gives sum delta =0.
    // Propagate it once for each population; use a snapshot (not iterative
    // floating rebranching). Correct outward roundoff on every sum.
    let old = raw;
    for i in 0..n {
        let mut s_lo = 0.0;
        let mut s_hi = 0.0;
        for j in 0..n {
            if i != j {
                s_lo = err(add_down(s_lo, old[j].lo))?;
                s_hi = err(add_up(s_hi, old[j].hi))?;
            }
        }
        let lower = (-s_hi).max(raw[i].lo);
        let upper = (-s_lo).min(raw[i].hi);
        if lower > upper {
            return Err("MPRK_NUCLEI_INTERVAL_CONTRACTION_EMPTY".into());
        }
        raw[i] = boxx(lower, upper)?;
    }
    // Residual radius is not a scalar physical LTE: only a corrector solve.
    let positive = p[start..start + n].iter().all(|x| x.lo > 0.0);
    Ok((raw, radius, positive))
}
fn certify(c: &Cell, dt: f64) -> R<([IV; 5], [f64; 2])> {
    if !(dt.is_finite() && dt > 0.0) {
        return Err("ORIGINAL_MPRK_BAD_DT".into());
    }
    let mut a = [[ZERO; 5]; 5];
    for src in 0..5 {
        let mut outgoing = ZERO;
        for dst in 0..5 {
            if dst == src {
                continue;
            }
            let f = c.flux[5 * dst + src];
            outgoing = add(outgoing, f)?;
            a[dst][src] = sub(ZERO, mul(point(dt)?, div(f, c.den[src])?)?)?;
        }
        a[src][src] = add(point(1.0)?, mul(point(dt)?, div(outgoing, c.den[src])?)?)?;
        if a[src][src].lo < 1.0 {
            return Err("MPRK_DIAGONAL_BELOW_ONE".into());
        }
    }
    let mut d = [ZERO; 5];
    let mut radii = [0.0; 2];
    for (block, (start, n)) in [(0, 2), (2, 3)].into_iter().enumerate() {
        let (x, radius, _positive) = cert_block(&c.p, &a, start, n)?;
        for j in 0..n {
            d[start + j] = x[j];
        }
        radii[block] = radius;
    }
    Ok((d, radii))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("USAGE: rei_original46k_interval_mprk ORIGINAL_CORRECTOR.bin ORIGINAL_SOURCE_MANIFEST.json OUTPUT_INTERVAL.raw".into());
    }
    let bytes = fs::read(&args[1])?;
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if sha(&bytes) != SOURCE_SHA
        || manifest["source_binary_sha256"] != SOURCE_SHA
        || manifest["source_rei_git_head"] != "f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa"
        || manifest["owner_original_14_ledgers_pass"] != true
        || manifest["original_nodes"] != N
        || manifest["owner_original_table_events"] != 0
        || manifest["official_tick161"] != "NOT_ACCEPTED"
    {
        return Err("ORIGINAL_F05_SOURCE_BINARY_OR_PROVIDER_AUTHORITY_DRIFT".into());
    }
    if bytes.len() != 24 + MAPS * (16 + N * COLUMNS * 8)
        || &bytes[..8] != MAGIC
        || u64::from_le_bytes(bytes[8..16].try_into()?) != N as u64
        || u64::from_le_bytes(bytes[16..24].try_into()?) != MAPS as u64
    {
        return Err("ORIGINAL_MPRK_SOURCE_LAYOUT_DRIFT".into());
    }
    let t0 = Instant::now();
    let mut output = Vec::with_capacity(16 + MAPS * N * 10 * 8);
    output.extend_from_slice(OUTPUT_MAGIC);
    output.extend_from_slice(&(N as u64).to_le_bytes());
    let mut per_map = Vec::new();
    let mut zero_widened = 0usize;
    for m in 0..MAPS {
        let head = 24 + m * (16 + N * COLUMNS * 8);
        if &bytes[head..head + 8] != [b"FULL0000", b"HALF1000", b"HALF2000"][m] {
            return Err("MPRK_ORIGINAL_SOURCE_MAP_ORDER_DRIFT".into());
        }
        let dt = rd(&bytes, head + 8)?;
        let expected = [4871848749.168213, 2435924374.5841064, 2435924374.5841064][m];
        if (dt - expected).abs() > 0.001 {
            return Err("ORIGINAL_MPRK_SOURCE_CLOCK_DURATION_DRIFT".into());
        }
        let mut max_rad = [0.0_f64; 2];
        let mut max_width = [0.0_f64; 5];
        // Physical electron count Ne = HII + HeII + 2*HeIII. Outward
        // additions retain source-box correlations only by inclusion,
        // never by assuming independence of cells or photon moments.
        let mut electron_delta = ZERO;
        let mut count = 0usize;
        for i in 0..N {
            let c = decode_cell(&bytes, head + 16 + i * COLUMNS * 8)?;
            if c.had_original_zero_widening {
                zero_widened += 1;
            }
            let (delta, rad) = certify(&c, dt).map_err(|e| format!("map={m} node={i}: {e}"))?;
            let e = add(add(delta[1], delta[3])?, mul(point(2.0)?, delta[4])?)?;
            electron_delta = add(electron_delta, e)?;
            for s in 0..5 {
                max_width[s] = max_width[s].max(delta[s].hi - delta[s].lo);
                output.extend_from_slice(&delta[s].lo.to_le_bytes());
            }
            for s in 0..5 {
                output.extend_from_slice(&delta[s].hi.to_le_bytes());
            }
            for k in 0..2 {
                max_rad[k] = max_rad[k].max(rad[k]);
            }
            count += 1;
        }
        let map_name = ["full", "half1", "half2"][m];
        per_map.push(json!({"map":map_name,"source_cells":count,
            "source_duration_seconds":dt,
            "max_absolute_increment_radius_H":max_rad[0],
            "max_absolute_increment_radius_He":max_rad[1],
            "max_delta_interval_width_by_species":max_width,
            "certified_source_box_total_delta_electrons":[electron_delta.lo,electron_delta.hi],
            "source_box_electron_delta_strictly_negative":electron_delta.hi<0.0,
            "true_owner_source_corrector":"MPRK22(1)_5POP",
            "native_outward_certified_2x2_and_3x3":"PASS"}));
    }
    fs::write(&args[3], &output)?;
    let receipt = json!({
        "schema":"REI_F05_NATIVE_RUST_ORIGINAL_CORRECTOR_INTERVAL_PROOF_V1",
        "input_original_owner_head":"f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa",
        "original_26event_kernel_sha256":"579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644",
        "original_source_binary_sha256":SOURCE_SHA,
        "original_reference_csv_sha256":OWNER_CSV_SHA,
        "source_cells_per_map":N,"map_count":MAPS,
        "native_interval_corrector_map_report":per_map,
        "source_rows_with_outward_zero_flux":zero_widened,
        "directed_interval_backend":"rodas5p_core::directed",
        "global_full_HHe_OTS_thermal_interval_ivp":"NOT_ESTABLISHED",
        "full_source_14_Rust_interval_ledgers":"NOT_RUN",
        "f64_heii_positive_subnormal":"UNRESOLVED",
        "native_interval_increment_raw_bytes":output.len(),
        "native_interval_increment_raw_sha256":sha(&output),
        "timing_seconds":t0.elapsed().as_secs_f64(),
        "official_tick161":"NOT_ACCEPTED","canonical_delta":0,"production":"HOLD"
    });
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_species_exact_conserved_pds() {
        let mut c = Cell {
            p: [point(1.0).unwrap(); 5],
            den: [point(1.0).unwrap(); 5],
            flux: [ZERO; 25],
            had_original_zero_widening: false,
        };
        c.p[0] = point(2.0).unwrap();
        c.p[1] = point(1.0).unwrap();
        c.flux[5] = point(0.125).unwrap(); // source H0->H1
        let (d, r) = certify(&c, 2.0).unwrap();
        let expected = -0.4_f64;
        assert!(d[0].contains(expected));
        assert!(d[1].contains(-expected));
        assert!(r[0].is_finite());
    }
    #[test]
    fn negative_flux_rejected_beyond_source_outward_zero() {
        let mut a = [1.0_f64; COLUMNS];
        for k in 10..60 {
            a[k] = 0.0;
        }
        a[10] = -1e-4;
        let mut bytes = Vec::new();
        for x in a {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        assert!(decode_cell(&bytes, 0).is_err());
    }
    #[test]
    fn original_cross_species_structural_zero_ulp_is_not_transfer() {
        let mut a = [1.0_f64; COLUMNS];
        for k in 0..25 {
            a[10 + k] = -f64::from_bits(1);
            a[35 + k] = f64::from_bits(1);
        }
        let mut bytes = Vec::new();
        for x in a {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        let c = decode_cell(&bytes, 0).unwrap();
        assert_eq!(c.flux[10].as_pair(), [0.0, 0.0]);
        // Genuinely finite cross-element transfer violates frozen chemistry.
        a[35 + 10] = 1.0e-5;
        let mut tampered = Vec::new();
        for x in a {
            tampered.extend_from_slice(&x.to_le_bytes());
        }
        assert!(decode_cell(&tampered, 0).is_err());
    }
    #[test]
    fn conservation_and_m_matrix_protects_zero_rates() {
        let c = Cell {
            p: [point(2.0).unwrap(); 5],
            den: [point(2.0).unwrap(); 5],
            flux: [ZERO; 25],
            had_original_zero_widening: false,
        };
        let (d, r) = certify(&c, 1.0).unwrap();
        assert!(d.iter().all(|q| q.contains(0.0)));
        assert!(r.iter().all(|x| x.is_finite()));
    }
}
