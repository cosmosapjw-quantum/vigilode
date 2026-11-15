//! Native original REI F05 gas-U first-law source-root interval ledger.
//! Directly consumes the independent certified 46k x 9 thermal root tubes.
//! Never substitutes a source/owner thermal-root solver or final 14-ledger gate.
use rodas5p_core::directed::{exp_interval, Interval as I};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs};
type R<T> = Result<T, String>;
const N: usize = 46_080;
const NR: usize = 9;
const COLS: usize = 32;
const SRC_SHA: &str = "73ed05ef73ebc2b098d70aa66970b9e6ceb2bcb567c13fa80b2efc4017ebce3a";
const ROOT_SHA: &str = "5b7aef979237dc4aae89d4248e733496949a1707c2587af815eec59c021b9777";
fn sha(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}
fn n(lo: f64, hi: f64) -> R<I> {
    I::new(lo, hi).map_err(|e| e.to_string())
}
fn p(v: f64) -> R<I> {
    I::point(v).map_err(|e| e.to_string())
}
fn add(a: I, b: I) -> R<I> {
    a.add(b).map_err(|e| e.to_string())
}
fn sub(a: I, b: I) -> R<I> {
    a.sub(b).map_err(|e| e.to_string())
}
fn mul(a: I, b: I) -> R<I> {
    a.mul(b).map_err(|e| e.to_string())
}
fn rd(b: &[u8], pos: usize) -> R<f64> {
    if pos + 8 > b.len() {
        return Err("THERMAL_LEDGER_TRUNCATED_SOURCE".into());
    }
    let x = f64::from_le_bytes(b[pos..pos + 8].try_into().unwrap());
    if !x.is_finite() {
        return Err("THERMAL_LEDGER_NONFINITE_INPUT".into());
    }
    Ok(x)
}
fn iv(b: &[u8], pos: usize) -> R<I> {
    n(rd(b, pos)?, rd(b, pos + 8)?)
}
fn run(src: &[u8], roots: &[u8]) -> R<serde_json::Value> {
    if src.len() != 24 + NR * (32 + N * COLS * 8)
        || &src[..8] != b"REIF05T2"
        || roots.len() != 24 + NR * N * 41
        || &roots[..8] != b"REIF05R2"
        || sha(src) != SRC_SHA
        || sha(roots) != ROOT_SHA
    {
        return Err("ORIGINAL_REI_THERMAL_LEDGER_IDENTITY_OR_SCHEMA".into());
    }
    let mut detail = Vec::new();
    for k in 0..NR {
        let head = 24 + k * (32 + N * COLS * 8);
        let dt = rd(src, head + 16)?;
        let weight = rd(src, head + 24)?;
        let mut cumulative = n(0.0, 0.0)?;
        let mut matches = 0usize;
        let mut energy_strict_negative = 0usize;
        for i in 0..N {
            let so = head + 32 + i * COLS * 8;
            let ro = 24 + (k * N + i) * 41;
            if roots[ro] != 1 {
                return Err(format!("UNVERIFIED_THERMAL_ROOT_{k}_{i}"));
            }
            let lo = rd(roots, ro + 1)?;
            let hi = rd(roots, ro + 9)?;
            let rhs = iv(roots, ro + 25)?;
            let t = n(
                exp_interval(lo).map_err(|e| e.to_string())?.lo,
                exp_interval(hi).map_err(|e| e.to_string())?.hi,
            )?;
            let ecoef = iv(src, so + 4 * 8)?;
            let u0 = iv(src, so + 6 * 8)?;
            let stage_const = iv(src, so + 8 * 8)?;
            // Exact source-owner total gas energy U = A exp(lnT).
            let delta_u = sub(mul(ecoef, t)?, u0)?;
            // The owner-root identity is U-U0 = h(C+w R).
            let rhs_integral = mul(p(dt)?, add(stage_const, mul(p(weight)?, rhs)?)?)?;
            let residual = sub(delta_u, rhs_integral)?;
            if !residual.contains_zero() {
                return Err(format!(
                    "ORIGINAL_GAS_FIRST_LAW_NOT_INCLUDED root={k} cell={i}"
                ));
            }
            if delta_u.hi < 0.0 {
                energy_strict_negative += 1;
            }
            cumulative = add(cumulative, delta_u)?;
            matches += 1;
        }
        let map_name = ["full", "half1", "half2"][k / 3];
        let root_name = ["predictor", "stage", "final"][k % 3];
        detail.push(json!({"map":map_name,
            "thermal_root":root_name,
            "source_cells_checked":matches,
            "signed_total_delta_U_erg":[cumulative.lo,cumulative.hi],
            "cells_with_strictly_negative_delta_U":energy_strict_negative,
            "uniform_source_root_first_law_zero_included":"PASS",
            "whole_IVP_first_law_coupling":"NOT_ESTABLISHED"}));
    }
    Ok(
        json!({"schema":"REI_F05_NATIVE_THERMAL_SOURCE_FIRST_LAW_CROSS_LEDGER_V1",
       "original_source_sha256":SRC_SHA,"native_thermal_root_sha256":ROOT_SHA,
       "source_root_energy_identity_rows_checked":N*NR,
       "reports":detail,"native_14_original_final_set_ledgers":"NOT_RUN",
       "owner_14_original_final_set_ledgers":"PASS_SEPARATELY",
       "official_tick161":"NOT_ACCEPTED","canonical_delta":0,"production":"HOLD"}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = env::args().collect::<Vec<_>>();
    if a.len() != 3 {
        return Err(
            "USAGE: rei_original46k_thermal_energy_ledger ORIGINAL_SOURCE.raw NATIVE_ROOTS.raw"
                .into(),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&run(&fs::read(&a[1])?, &fs::read(&a[2])?)?)?
    );
    Ok(())
}
