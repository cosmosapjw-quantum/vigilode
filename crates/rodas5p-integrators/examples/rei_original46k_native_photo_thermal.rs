//! Source-locked original REI F05 accepted tick160 *photo-only* H/He/thermal stage.
//!
//! All photon rates and primary heat are calculated by the pre-existing Rust
//! 46,080-cell native rank-three source method, NOT a Python solver.
//! A separate SHA-locked original accepted 5-population + logT interval
//! fixture provides actual H/He and thermal parent. The stage gives a real
//! physical photo-only H/He/electron + ideal gas thermal RHS at all 46k sites.
//!
//! NOT the complete original 26-event H/He/OTS/thermal MPRK/SDIRK time map,
//! and not an accepted tick161. No source-owner runtime or default mutation.
#![allow(dead_code)]
#![recursion_limit = "256"]
#[path = "rei_original46k_native_rank3.rs"]
mod photon;

use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::time::Instant;

const PARENT_RAW_SHA: &str = "eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064";
const PARENT_MAGIC: &[u8; 8] = b"REIF05P2";
const N: usize = 46_080;
const KB: f64 = 1.380649e-16;
const ERG_PER_EV: f64 = 1.602176634e-12;
const BINDING_EV: [f64; 3] = [13.598, 24.587, 54.416];
const DERIVED_COLUMNS: usize = 10;

#[derive(Clone)]
struct OriginalAcceptedParent {
    n: usize,
    kb: f64,
    poplo: Vec<f64>,
    pophi: Vec<f64>,
    logtlo: Vec<f64>,
    logthi: Vec<f64>,
}
fn sha(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}
fn read_parent(data: &[u8], expected: Option<&str>) -> Result<OriginalAcceptedParent, String> {
    if let Some(x) = expected {
        if sha(data) != x {
            return Err("ACCEPTED_REI_F05_PARENT_RAW_SHA_MISMATCH".into());
        }
    }
    if data.len() < 24 || &data[..8] != PARENT_MAGIC {
        return Err("ACCEPTED_REI_PARENT_MAGIC_INVALID".into());
    }
    let n = usize::try_from(u64::from_le_bytes(data[8..16].try_into().unwrap()))
        .map_err(|_| "ORIGINAL_PARENT_N_OVERFLOW")?;
    if n == 0 || n > N {
        return Err("ACCEPTED_REI_PARENT_COUNT_BAD".into());
    }
    let expected_bytes = 24 + 8 * 12 * n;
    if data.len() != expected_bytes {
        return Err("ACCEPTED_REI_PARENT_BYTES_BAD".into());
    }
    let mut at = 16;
    let mut read = || {
        let x = f64::from_le_bytes(data[at..at + 8].try_into().unwrap());
        at += 8;
        x
    };
    let kb = read();
    if kb.to_bits() != KB.to_bits() {
        return Err("ORIGINAL_SOURCE_KB_CHANGED".into());
    }
    let poplo = (0..5 * n).map(|_| read()).collect::<Vec<_>>();
    let pophi = (0..5 * n).map(|_| read()).collect::<Vec<_>>();
    let logtlo = (0..n).map(|_| read()).collect::<Vec<_>>();
    let logthi = (0..n).map(|_| read()).collect::<Vec<_>>();
    if at != expected_bytes {
        return Err("ORIGINAL_SOURCE_PARENT_UNCONSUMED".into());
    }
    for i in 0..5 * n {
        if !(poplo[i].is_finite() && pophi[i].is_finite() && poplo[i] > 0.0 && pophi[i] >= poplo[i])
        {
            return Err("ORIGINAL_PARENT_POPULATION_NOT_POSITIVE_INTERVAL".into());
        }
    }
    for i in 0..n {
        if !(logtlo[i].is_finite()
            && logthi[i].is_finite()
            && logtlo[i] <= logthi[i]
            && logtlo[i] > 0.0)
        {
            return Err("ORIGINAL_PARENT_LOGTEMPERATURE_NOT_VALID".into());
        }
    }
    Ok(OriginalAcceptedParent {
        n,
        kb,
        poplo,
        pophi,
        logtlo,
        logthi,
    })
}

#[derive(Clone, Copy, Debug)]
struct PhotoStageRow {
    // All 5 original H/He species have photo-only derivatives in cell-count/s.
    dn: [f64; 5],
    d_ne: f64,                // electrons/second/cell
    q_erg_s: f64,             // original primary PHOTO excess energy, NOT binding heat
    d_lnt_photo_s: f64,       // photo-only logT time derivative
    u_erg: f64,               // original accepted parent ideal-gas thermal energy/cell
    photo_binding_erg_s: f64, // binding reservoir, not heat
    t_k: f64,
    h_count: f64,
    he_count: f64,
    electrons: f64,
    energy_identity_residual: f64,
    nuclei_h_residual: f64,
    nuclei_he_residual: f64,
    charge_residual: f64,
}
impl PhotoStageRow {
    fn data(self) -> [f64; DERIVED_COLUMNS] {
        [
            self.dn[0],
            self.dn[1],
            self.dn[2],
            self.dn[3],
            self.dn[4],
            self.d_ne,
            self.q_erg_s,
            self.d_lnt_photo_s,
            self.u_erg,
            self.photo_binding_erg_s,
        ]
    }
}

fn photo_stage(
    p: [f64; 5],
    t: f64,
    photo: [f64; 3],
    q: f64,
    kb: f64,
) -> Result<PhotoStageRow, String> {
    if p.iter().any(|x| !x.is_finite() || *x <= 0.0)
        || photo.iter().any(|x| !x.is_finite() || *x < 0.0)
        || !(t.is_finite() && t > 0.0 && q.is_finite() && q >= 0.0)
        || kb.to_bits() != KB.to_bits()
    {
        return Err("ORIGINAL_PHYSICAL_STAGE_INVALID_DOMAIN".into());
    }
    let h = p[0] + p[1];
    let he = p[2] + p[3] + p[4];
    let ne = p[1] + p[3] + 2.0 * p[4];
    let particles = h + he + ne;
    let u = 1.5 * kb * t * particles;
    if !(h > 0.0
        && he > 0.0
        && ne > 0.0
        && particles > 0.0
        && u > 0.0
        && h.is_finite()
        && he.is_finite()
        && particles.is_finite()
        && u.is_finite())
    {
        return Err("ORIGINAL_SOURCE_GAS_EOS_NONFINITE".into());
    }
    let [ph, phei, pheii] = photo;
    let dn = [-ph, ph, -phei, phei - pheii, pheii];
    let dne = ph + phei + pheii;
    let d_lnt = q / u - dne / particles;
    let binding = ERG_PER_EV
        * BINDING_EV
            .iter()
            .zip(photo)
            .map(|(e, v)| e * v)
            .sum::<f64>();
    if !(d_lnt.is_finite() && binding.is_finite() && binding >= 0.0) {
        return Err("NONFINITE_ORIGINAL_SOURCE_PHOTO_STAGE".into());
    }
    let restored = q - (u * d_lnt + 1.5 * kb * t * dne);
    let scale = q.abs().max((u * d_lnt).abs()).max(1e-300);
    let n_h = dn[0] + dn[1];
    let n_he = dn[2] + dn[3] + dn[4];
    let charge = dn[1] + dn[3] + 2.0 * dn[4] - dne;
    Ok(PhotoStageRow {
        dn,
        d_ne: dne,
        q_erg_s: q,
        d_lnt_photo_s: d_lnt,
        u_erg: u,
        photo_binding_erg_s: binding,
        t_k: t,
        h_count: h,
        he_count: he,
        electrons: ne,
        energy_identity_residual: restored.abs() / scale,
        nuclei_h_residual: n_h,
        nuclei_he_residual: n_he,
        charge_residual: charge,
    })
}

#[derive(Default)]
struct Sum {
    sum: f64,
    comp: f64,
}
impl Sum {
    fn add(&mut self, x: f64) {
        let t = self.sum + x;
        self.comp += if self.sum.abs() >= x.abs() {
            (self.sum - t) + x
        } else {
            (x - t) + self.sum
        };
        self.sum = t;
    }
    fn val(&self) -> f64 {
        self.sum + self.comp
    }
}

fn run(
    parent: &OriginalAcceptedParent,
    photo: &photon::NativeOriginalPhotoStage,
) -> Result<(serde_json::Value, Vec<u8>), String> {
    if parent.n != N || photo.n != N {
        return Err("ORIGINAL_REAL_CELLS_MUST_BE_46080".into());
    }
    let start = Instant::now();
    let mut bytes = Vec::with_capacity(16 + 8 * N * DERIVED_COLUMNS);
    bytes.extend_from_slice(b"REIF05D1");
    bytes.extend_from_slice(&(N as u64).to_le_bytes());
    let mut n_h = Sum::default();
    let mut n_he = Sum::default();
    let mut nel = Sum::default();
    let mut u = Sum::default();
    let mut dnel = Sum::default();
    let mut q = Sum::default();
    let mut binding = Sum::default();
    let mut n_bad = 0usize;
    let mut e_bad = 0usize;
    let mut source_identity_bad = 0usize;
    let mut no_heat = 0usize;
    let mut photo_gamma_positive = 0usize;
    let mut max_res_energy = 0.0_f64;
    let mut max_res_charge = 0.0_f64;
    let mut max_res_h = 0.0_f64;
    let mut max_res_he = 0.0_f64;
    let mut max_abs_lnt = 0.0_f64;
    let mut min_lnt = f64::INFINITY;
    let mut max_lnt = f64::NEG_INFINITY;
    let mut min_t = f64::INFINITY;
    let mut max_t = f64::NEG_INFINITY;
    for i in 0..N {
        let mut p = [0.0; 5];
        for s in 0..5 {
            let k = 5 * i + s;
            p[s] = (parent.poplo[k] + parent.pophi[k]) * 0.5;
            if !p[s].is_finite() || p[s] <= 0.0 {
                return Err("PARENT_ACCEPTED_POP_MIDPOINT_BAD".into());
            }
        }
        for (j, s) in [0usize, 2, 3].iter().enumerate() {
            if p[*s].to_bits() != photo.neutral[3 * i + j].to_bits() {
                source_identity_bad += 1;
            }
        }
        let tm = (parent.logtlo[i] + parent.logthi[i]) * 0.5;
        let temperature = tm.exp();
        let events = [
            photo.photo[3 * i],
            photo.photo[3 * i + 1],
            photo.photo[3 * i + 2],
        ];
        let heat = photo.primary_heat_erg_per_s[i];
        let row = photo_stage(p, temperature, events, heat, parent.kb)?;
        for x in row.data() {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        n_h.add(row.h_count);
        n_he.add(row.he_count);
        nel.add(row.electrons);
        u.add(row.u_erg);
        dnel.add(row.d_ne);
        q.add(row.q_erg_s);
        binding.add(row.photo_binding_erg_s);
        if row.q_erg_s == 0.0 {
            no_heat += 1;
        }
        if row.d_ne > 0.0 {
            photo_gamma_positive += 1;
        }
        max_res_energy = max_res_energy.max(row.energy_identity_residual);
        let scale = row.d_ne.abs().max(1e-300);
        max_res_h = max_res_h.max(row.nuclei_h_residual.abs() / scale);
        max_res_he = max_res_he.max(row.nuclei_he_residual.abs() / scale);
        max_res_charge = max_res_charge.max(row.charge_residual.abs() / scale);
        if row.energy_identity_residual > 1e-10 {
            e_bad += 1;
        }
        if row.nuclei_h_residual.abs() > 64.0 * f64::EPSILON * scale
            || row.nuclei_he_residual.abs() > 64.0 * f64::EPSILON * scale
            || row.charge_residual.abs() > 64.0 * f64::EPSILON * scale
        {
            n_bad += 1;
        }
        max_abs_lnt = max_abs_lnt.max(row.d_lnt_photo_s.abs());
        min_lnt = min_lnt.min(row.d_lnt_photo_s);
        max_lnt = max_lnt.max(row.d_lnt_photo_s);
        min_t = min_t.min(row.t_k);
        max_t = max_t.max(row.t_k);
    }
    if source_identity_bad > 0 || n_bad > 0 || e_bad > 0 {
        return Err(format!(
            "PHOTO_ONLY_NATIVE_STAGE_INVARIANT_FAIL:source={source_identity_bad} nuclei={n_bad} thermal={e_bad}"
        ));
    }
    if bytes.len() != 16 + 8 * N * DERIVED_COLUMNS {
        return Err("DERIVED_ORIGINAL_STAGE_WRONG_BINARY_BYTES".into());
    }
    let hash = sha(&bytes);
    let result = json!({
       "schema":"VIGILODE_REI_ORIGINAL26_T160_NATIVE_PHOTO_HHE_THERMAL_STAGE_V1",
       "accepted_original_nodes":N,
       "source_original_photon_partial_raw_sha256":"a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469",
       "original_accepted_parent5_raw_sha256":PARENT_RAW_SHA,
       "stage_data_raw_sha256":hash,
       "stage_data_raw_bytes":bytes.len(),
       "stage_data_layout":"REIF05D1 n_u64 then 10*original_nodes f64 per node: dHI,dHII,dHeI,dHeII,dHeIII,dNe,Qerg_s,dlnTphoto_s,Uerg,bindingEerg_s",
       "actual_source_input":"original REI F05 accepted tick160, 5 H/He populations+T interval and original four photon groups",
       "source_operator":"native Rust VigilODE rank3 JVP photon-source group allocation",
       "source_photon_ledger_relative_residual":photo.source_budget_relative,
       "source_heii_total_log10_positive_extended":photo.source_heii_trace_log10,
       "native_heii_photo_f64_underflow_caveat":"POSITIVE_LONGDOUBLE_TRACE_NOT_EXACT_PHYSICAL_ZERO",
       "photo_only_h_nuclei_conserved_at_all_nodes":n_bad==0,
       "photo_only_he_nuclei_conserved_at_all_nodes":n_bad==0,
       "photo_electrons_charge_identity_at_all_nodes":n_bad==0,
       "photo_only_thermal_first_law_at_all_nodes":e_bad==0,
       "max_nuclei_H_rate_relative_residual":max_res_h,
       "max_nuclei_He_rate_relative_residual":max_res_he,
       "max_electron_charge_rate_relative_residual":max_res_charge,
       "max_energy_identity_relative_residual":max_res_energy,
       "original_source_neutral_midpoint_byte_mismatches":source_identity_bad,
       "photo_only_electron_total_rate_per_s":dnel.val(),
       "photo_only_primary_heating_total_erg_s":q.val(),
       "photo_only_binding_gain_total_erg_s":binding.val(),
       "accepted_parent_H_total":n_h.val(),
       "accepted_parent_He_total":n_he.val(),
       "accepted_parent_electron_total":nel.val(),
       "accepted_parent_gas_thermal_total_erg":u.val(),
       "accepted_parent_min_temperature_K":min_t,
       "accepted_parent_max_temperature_K":max_t,
       "photo_only_dlnTdt_min_per_s":min_lnt,
       "photo_only_dlnTdt_max_per_s":max_lnt,
       "photo_only_dlnTdt_max_abs_per_s":max_abs_lnt,
       "native_photo_electron_positive_nodes":photo_gamma_positive,
       "native_primary_heat_f64_zero_nodes":no_heat,
       "native_full_HHe_OTS_recombination_cooling_expansion":"NOT_INCLUDED",
       "original_26event_MPRK22_Alexander_SDIRK2_time_map":"NOT_RUN",
       "original_14_full_time_map_ledgers":"NOT_RUN",
       "original_26event_ivp_accuracy_or_speedup":"NOT_ESTABLISHED",
       "native_stage_computation_seconds":start.elapsed().as_secs_f64(),
       "original_tick161":"NOT_ACCEPTED",
       "vigilode_official_dag":"7/23",
       "rei_central_dag":"28/88",
       "canonical_delta":0,
       "production":"HOLD"
    });
    Ok((result, bytes))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(
            "USAGE: rei_original46k_native_photo_thermal PARENT_RAW_FILE OUT_STAGE_BINARY".into(),
        );
    }
    let parent = read_parent(&fs::read(&args[1])?, Some(PARENT_RAW_SHA))?;
    let mut raw_source = Vec::new();
    io::stdin().read_to_end(&mut raw_source)?;
    let photo = photon::source_locked_photo_stage(&raw_source)?;
    let (report, staged) = run(&parent, &photo)?;
    fs::write(&args[2], &staged)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn test_photo(p: [f64; 3], q: f64) -> PhotoStageRow {
        photo_stage([1.0e3, 2.0e3, 1.0e2, 3.0e2, 1.0e2], 1.0e4, p, q, KB).unwrap()
    }
    #[test]
    fn closed_photo_hhe_charge_thermal_source() {
        let x = test_photo([1.0, 0.5, 0.1], 3e-12);
        assert!((x.dn[0] + x.dn[1]).abs() < 1e-15);
        assert!((x.dn[2] + x.dn[3] + x.dn[4]).abs() < 1e-15);
        assert!((x.dn[1] + x.dn[3] + 2.0 * x.dn[4] - x.d_ne).abs() < 1e-15);
        assert!(x.energy_identity_residual < 1e-12);
        assert!(x.photo_binding_erg_s > 0.);
    }
    #[test]
    fn no_photo_source_has_zero_photo_only_dln_t() {
        let x = test_photo([0.0; 3], 0.0);
        assert_eq!(x.d_lnt_photo_s, 0.0);
        assert_eq!(x.d_ne, 0.0);
        assert_eq!(x.photo_binding_erg_s, 0.0);
    }
    #[test]
    fn source_domain_guards_refuse_negative_and_wrong_magic() {
        assert!(photo_stage([1., 2., 3., 4., 5.], 1e4, [1., -1., 0.], 1., KB).is_err());
        assert!(photo_stage([1., 2., 3., 4., 5.], 1e4, [1., 1., 0.], -1., KB).is_err());
        assert!(photo_stage([1., 2., 3., 4., 5.], -1., [1., 1., 0.], 1., KB).is_err());
        assert!(read_parent(&[0u8; 24], None).is_err());
        assert!(read_parent(&[0u8; 24], Some(PARENT_RAW_SHA)).is_err());
    }
}
