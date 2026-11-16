//! Original REI F05 frozen-tick160 H/He/OTS/thermal RHS, native Rust opt-in.
//!
//! EXACT original source event_box coefficients and cooling functions, evaluated
//! at the midpoint of the immutable original accepted parent and one declared
//! f=0.55, v=(owner_v_lo+owner_v_hi)/2 source-admissible point.
//! The resulting 46,080 original physical cells are compared against original
//! owner interval flux and energy boxes. No time integration or tick161 admission.
//! Reuses existing native VigilODE source-locked rank-three photon method and
//! original accepted 5-population thermal parent without editing REI owner.
#![allow(dead_code)]
#![recursion_limit = "256"]
#[path = "rei_original46k_native_photo_thermal.rs"]
mod source;

use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::time::Instant;

const N: usize = 46_080;
const E1_SHA: &str = "28ea21ea9e5b4210b3e8971313af6aa6a8c4a07fad10c984bfd29424967ebf0b";
const MAGIC: &[u8; 8] = b"REIF05E1";
const KB: f64 = 1.380649e-16;
const EV: f64 = 1.602176634e-12;
const ELL: f64 = 1.425;
const MCAS: f64 = 0.737;
const PEXC: f64 = 0.96;
const CHI_H: f64 = 13.598434599702;
const CHI_HEI: f64 = 24.587389011;
const CHI_HEII: f64 = 54.417760;
const HEII_LYA: f64 = 0.75 * CHI_HEII;
const SIG_H24: f64 = 1.2391519584513023e-18;
const SIG_HEI24: f64 = 7.43469869411065e-18;
const SIG_H41: f64 = 2.884642817876362e-19;
const SIG_HEI41: f64 = 3.0402144676144673e-18;
const SIG_H54: f64 = 1.2306959247142394e-19;
const SIG_HEI54: f64 = 1.6907806870529807e-18;
const SIG_HEII54: f64 = 1.5872802575386495e-18;
const EXTRA: usize = 7;
const DERIVED: usize = 40;

fn sha(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}

struct Oracle {
    n: usize,
    volume_lo: Vec<f64>,
    volume_hi: Vec<f64>,
    hubble_lo: Vec<f64>,
    hubble_hi: Vec<f64>,
    v_lo: Vec<f64>,
    v_hi: Vec<f64>,
    flux_lo: Vec<f64>,
    flux_hi: Vec<f64>,
    extras_lo: Vec<f64>,
    extras_hi: Vec<f64>,
}
fn oracle(b: &[u8], expected: Option<&str>) -> Result<Oracle, String> {
    if let Some(s) = expected {
        if sha(b) != s {
            return Err("ORIGINAL_F05_EVENT_ORACLE_SHA_DRIFT".into());
        }
    }
    if b.len() < 16 || &b[..8] != MAGIC {
        return Err("ORIGINAL_SOURCE_EVENT_MAGIC_DRIFT".into());
    }
    let n = usize::try_from(u64::from_le_bytes(b[8..16].try_into().unwrap()))
        .map_err(|_| "SOURCE_COUNT_OVERFLOW")?;
    if n == 0 || n > N {
        return Err("SOURCE_EVENT_CELL_COUNT_INVALID".into());
    }
    if b.len() != 16 + 8 * 70 * n {
        return Err("SOURCE_EVENT_BIN_LENGTH_CHANGED".into());
    }
    let mut idx = 16;
    let mut read = || {
        let v = f64::from_le_bytes(b[idx..idx + 8].try_into().unwrap());
        idx += 8;
        v
    };
    let mut vec = |nn: usize| (0..nn).map(|_| read()).collect::<Vec<_>>();
    let x = Oracle {
        n,
        volume_lo: vec(n),
        volume_hi: vec(n),
        hubble_lo: vec(n),
        hubble_hi: vec(n),
        v_lo: vec(n),
        v_hi: vec(n),
        flux_lo: vec(25 * n),
        flux_hi: vec(25 * n),
        extras_lo: vec(EXTRA * n),
        extras_hi: vec(EXTRA * n),
    };
    if idx != b.len() {
        return Err("NONPHOTO_SOURCE_TRAILING_BYTES".into());
    }
    for i in 0..n {
        if !(x.volume_lo[i].is_finite()
            && x.volume_hi[i].is_finite()
            && x.volume_lo[i] > 0.0
            && x.volume_lo[i] <= x.volume_hi[i]
            && x.hubble_lo[i].is_finite()
            && x.hubble_hi[i].is_finite()
            && x.hubble_lo[i] >= 0.0
            && x.hubble_lo[i] <= x.hubble_hi[i]
            && x.v_lo[i].is_finite()
            && x.v_hi[i].is_finite()
            && x.v_lo[i] >= 0.0
            && x.v_lo[i] <= x.v_hi[i]
            && x.v_hi[i] <= 1.0)
        {
            return Err("ORIGINAL_EVENT_FORCING_INTERVAL_INVALID".into());
        }
    }
    for i in 0..25 * n {
        if !(x.flux_lo[i].is_finite() && x.flux_hi[i].is_finite() && x.flux_lo[i] <= x.flux_hi[i]) {
            return Err("ORIGINAL_SOURCE_TRANSITION_INTERVAL_INVALID".into());
        }
    }
    for i in 0..EXTRA * n {
        if !(x.extras_lo[i].is_finite()
            && x.extras_hi[i].is_finite()
            && x.extras_lo[i] <= x.extras_hi[i])
        {
            return Err("ORIGINAL_SOURCE_ENERGY_INTERVAL_INVALID".into());
        }
    }
    Ok(x)
}
fn sigmoid(x: f64) -> f64 {
    if x >= 0. {
        1.0 / (1.0 + (-x).exp())
    } else {
        let z = x.exp();
        z / (1.0 + z)
    }
}
fn rates(t: f64) -> [f64; 10] {
    let lh = 315614.0 / t;
    let le = 570670.0 / t;
    let le3 = 1263030.0 / t;
    let dr = 1.9e-3 * t.powf(-1.5) * (-473638.0 / t).exp() * (1.0 + 0.3 * (-94728.0 / t).exp());
    let sa = sigmoid((t - 1.5e4) / 250.0);
    let ab_h = 2.753e-14 * lh.powf(1.5) / (1.0 + (lh / 2.740).powf(0.407)).powf(2.242);
    let aa2 = 3.0e-14 * le.powf(0.654) + sa * dr;
    let ab2 = 1.26e-14 * le.powf(0.750) + sa * dr;
    let aa3 = 2.0 * 1.269e-13 * le3.powf(1.503) / (1.0 + (le3 / 0.522).powf(0.470)).powf(1.923);
    let ab3 = 2.0 * 2.753e-14 * le3.powf(1.5) / (1.0 + (le3 / 2.740).powf(0.407)).powf(2.242);
    let an2 = 3.4e-13 * (t / 1.0e4).powf(-0.6);
    let bh = 5.835e-11 * t.sqrt() * (-157804.0 / t).exp();
    let be = 2.71e-11 * t.sqrt() * (-285331.0 / t).exp();
    let be2 = 5.707e-12 * t.sqrt() * (-631495.0 / t).exp();
    [ab_h, aa2, ab2, aa3, ab3, an2, bh, be, be2, dr]
}
fn cool_rate(t: f64, p: [f64; 5], ne: f64) -> f64 {
    let lh = 315614.0 / t;
    let le = 570670.0 / t;
    let le3 = 1263030.0 / t;
    let rec_h = 3.435e-30 * t * lh.powf(1.970) / (1.0 + (lh / 2.250).powf(0.376)).powf(3.720);
    let rec_e = KB * t * (1.26e-14 * le.powf(0.750));
    let rec_e3 =
        8.0 * 3.435e-30 * t * le3.powf(1.970) / (1.0 + (le3 / 2.250).powf(0.376)).powf(3.720);
    let sqrt = (t / 1e5).sqrt();
    let exc_h = 7.5e-19 * (-118348.0 / t).exp() / (1.0 + sqrt);
    let exc_e = 5.54e-17 * t.powf(-0.397) * (-473638.0 / t).exp() / (1.0 + sqrt);
    let bh = 5.835e-11 * t.sqrt() * (-157804.0 / t).exp();
    let be = 2.71e-11 * t.sqrt() * (-285331.0 / t).exp();
    let be2 = 5.707e-12 * t.sqrt() * (-631495.0 / t).exp();
    let a = 5.5 - t.ln() / 10f64.ln();
    let ff = 1.42e-27 * t.sqrt() * (1.1 + 0.34 * (-a * a / 3.0).exp());
    ne * p[1] * rec_h
        + ne * p[3] * rec_e
        + ne * p[4] * rec_e3
        + ne * p[0] * exc_h
        + ne * p[3] * exc_e
        + ne * p[0] * 13.598 * EV * bh
        + ne * p[2] * 24.587 * EV * be
        + ne * p[3] * 54.416 * EV * be2
        + ne * (p[1] + p[3] + 4.0 * p[4]) * ff
}
#[derive(Clone, Copy, Debug)]
struct Event {
    flux: [f64; 25],
    extra: [f64; EXTRA],
    net: [f64; 5],
    ne_rate: f64,
    photo_net_rate: f64,
    ots_heat: f64,
    cooling: f64,
    expansion: f64,
    source_h_balance: f64,
    source_he_balance: f64,
    source_energy_residual: f64,
    photon_identity_residual: f64,
    dlogt_dt_per_s: f64,
    thermal_firstlaw_residual_relative: f64,
}

fn point_event(
    pop: [f64; 5],
    t: f64,
    vol: f64,
    hubble: f64,
    v: f64,
    f: f64,
    photon: [f64; 3],
    primary_heat: f64,
) -> Result<Event, String> {
    if !(t.is_finite()
        && t > 0.0
        && t <= 1e5 * (1.0 + 32.0 * f64::EPSILON)
        && vol.is_finite()
        && vol > 0.0
        && hubble.is_finite()
        && hubble >= 0.0
        && v.is_finite()
        && v >= 0.0
        && v <= 1.0
        && f.is_finite()
        && f >= 0.1
        && f <= 1.0
        && primary_heat.is_finite()
        && primary_heat >= 0.0
        && pop.iter().all(|x| x.is_finite() && *x > 0.0)
        && photon.iter().all(|x| x.is_finite() && *x >= 0.0))
    {
        return Err("ORIGINAL_REI_NONPHOTO_SOURCE_POINT_OUT_OF_DOMAIN".into());
    }
    let [nhi, nhii, nhei, nheii, nheiii] = pop;
    let ne = (nhii + nheii + 2.0 * nheiii) / vol;
    let dhi = nhi / vol;
    let dhei = nhei / vol;
    let dheii = nheii / vol;
    let y = dhi * SIG_H24 / (dhi * SIG_H24 + dhei * SIG_HEI24 + 1e-300);
    let z = dhi * SIG_H41 / (dhi * SIG_H41 + dhei * SIG_HEI41 + 1e-300);
    let h54 = dhi * SIG_H54;
    let hei54 = dhei * SIG_HEI54;
    let heii54 = dheii * SIG_HEII54;
    let total = h54 + hei54 + heii54 + 1e-300;
    let y2a = heii54 / total;
    let y2b = hei54 / total;
    let ah = v * ((ELL - MCAS) + MCAS * y) + (1.0 - v) * f * z;
    let ahe = v * MCAS * (1.0 - y) + (1.0 - v) * f * (1.0 - z);
    let c = rates(t);
    let ab_h = c[0];
    let aa2 = c[1];
    let ab2 = c[2];
    let aa3 = c[3];
    let ab3 = c[4];
    let an2 = c[5].min(ab3);
    let cas = (ab3 - an2).max(0.0);
    let rhi = photon[0] + nhi * ne * c[6];
    let rhei = photon[1] + nhei * ne * c[7];
    let rheii = photon[2] + nheii * ne * c[8];
    let rhb = nhii * ne * ab_h;
    let r2g = nheii * ne * (aa2 - ab2).max(0.0);
    let r2b = nheii * ne * ab2;
    let r3g = nheiii * ne * (aa3 - ab3).max(0.0);
    let r3n = nheiii * ne * an2;
    let r3c = nheiii * ne * cas;
    let mut flux = [0.0; 25];
    let mut add = |dest: usize, source: usize, value: f64| {
        flux[5 * dest + source] += value;
    };
    // Exact original event_box list, same source->destination ordering.
    add(1, 0, rhi);
    add(3, 2, rhei);
    add(4, 3, rheii);
    add(0, 1, rhb);
    add(2, 3, r2g);
    add(1, 0, r2g * y);
    add(3, 2, r2g * (1.0 - y));
    add(2, 3, r2b);
    add(1, 0, r2b * PEXC);
    add(3, 4, r3g);
    add(1, 0, r3g * (1.0 - y2a - y2b));
    add(3, 2, r3g * y2b);
    add(4, 3, r3g * y2a);
    add(3, 4, r3n);
    add(1, 0, r3n);
    add(3, 4, r3c);
    add(1, 0, r3c * ah);
    add(3, 2, r3c * ahe);
    let ots_heat =
        r3c * (1.0 - v) * f * (z * (HEII_LYA - CHI_H) + (1.0 - z) * (HEII_LYA - CHI_HEI)) * EV;
    let escape = r3c * (1.0 - v) * (1.0 - f) * HEII_LYA * EV;
    let chemical = (rhb * (-CHI_H)
        + r2g * (-CHI_HEI + y * CHI_H + (1.0 - y) * CHI_HEI)
        + r2b * (-CHI_HEI + PEXC * CHI_H)
        + r3g * (-CHI_HEII + (1.0 - y2a - y2b) * CHI_H + y2b * CHI_HEI + y2a * CHI_HEII)
        + r3n * (-CHI_HEII + CHI_H)
        + r3c
            * (-CHI_HEII
                + v * (((ELL - MCAS) + MCAS * y) * CHI_H + MCAS * (1.0 - y) * CHI_HEI)
                + (1.0 - v) * f * (z * CHI_H + (1.0 - z) * CHI_HEI)))
        * EV;
    let unresolved = -chemical - ots_heat - escape;
    let photon_identity = ah + ahe + v * (2.0 - ELL) + (1.0 - v) * (1.0 - f) - (1.0 + v);
    let total_heat = primary_heat + ots_heat;
    let ptotal = nhi + nhii + nhei + nheii + nheiii + nhii + nheii + 2.0 * nheiii;
    let expansion = 3.0 * hubble * KB * ptotal * t;
    let cooling = cool_rate(t, pop, ne);
    let rhs = total_heat - cooling - expansion;
    let extra = [
        ots_heat,
        unresolved,
        escape,
        chemical,
        total_heat,
        photon_identity,
        rhs,
    ];
    let net = std::array::from_fn(|s| {
        let mut sum = 0.0;
        for k in 0..5 {
            sum += flux[5 * s + k] - flux[5 * k + s];
        }
        sum
    });
    let ne_rate = net[1] + net[3] + 2.0 * net[4];
    let n_e_count = nhii + nheii + 2.0 * nheiii;
    let n_part = nhi + nhii + nhei + nheii + nheiii + n_e_count;
    let u = 1.5 * KB * t * n_part;
    if !(n_part.is_finite() && n_part > 0.0 && u.is_finite() && u > 0.0) {
        return Err("INVALID_ORIGINAL_COUNT_GAS_ENERGY".into());
    }
    let dlogt = rhs / u - ne_rate / n_part;
    let firstlaw = (rhs - (u * dlogt + 1.5 * KB * t * ne_rate)).abs()
        / rhs.abs().max((u * dlogt).abs()).max(1e-300);
    let photo_net_rate = photon.iter().sum();
    let hbalance = net[0] + net[1];
    let hebalance = net[2] + net[3] + net[4];
    let eres = chemical + unresolved + ots_heat + escape;
    if !ne.is_finite()
        || !rhs.is_finite()
        || !cooling.is_finite()
        || !expansion.is_finite()
        || !extra.iter().all(|x| x.is_finite())
        || !flux.iter().all(|x| x.is_finite() && *x >= 0.0)
        || !net.iter().all(|x| x.is_finite())
    {
        return Err("ORIGINAL_26_EVENT_SOURCE_NATIVE_NONFINITE".into());
    }
    Ok(Event {
        flux,
        extra,
        net,
        ne_rate,
        photo_net_rate,
        ots_heat,
        cooling,
        expansion,
        source_h_balance: hbalance,
        source_he_balance: hebalance,
        source_energy_residual: eres,
        photon_identity_residual: photon_identity,
        dlogt_dt_per_s: dlogt,
        thermal_firstlaw_residual_relative: firstlaw,
    })
}

// Opt-in exported point operator for the source-clock research shadow.
// This does not change the existing source-only example or default solver.
pub(crate) struct TimeMapEvent {
    pub(crate) flux: [f64; 25],
    pub(crate) extra: [f64; EXTRA],
    pub(crate) net: [f64; 5],
    pub(crate) heat_erg_s: f64,
    pub(crate) thermal_rhs_erg_s: f64,
    pub(crate) electron_rate_s: f64,
    pub(crate) ots_residual_erg_s: f64,
    pub(crate) photon_identity: f64,
}
pub(crate) fn time_map_event(
    pop: [f64; 5],
    temp: f64,
    volume: f64,
    hubble: f64,
    v: f64,
    f: f64,
    photon: [f64; 3],
    photoheat: f64,
) -> Result<TimeMapEvent, String> {
    let p = point_event(pop, temp, volume, hubble, v, f, photon, photoheat)?;
    Ok(TimeMapEvent {
        flux: p.flux,
        extra: p.extra,
        net: p.net,
        heat_erg_s: p.extra[4],
        thermal_rhs_erg_s: p.extra[6],
        electron_rate_s: p.ne_rate,
        ots_residual_erg_s: p.source_energy_residual,
        photon_identity: p.photon_identity_residual,
    })
}
pub(crate) fn thermal_rhs_fixed_heat(
    pop: [f64; 5],
    temp: f64,
    volume: f64,
    hubble: f64,
    total_heat: f64,
) -> Result<f64, String> {
    if !(pop.iter().all(|v| v.is_finite() && *v > 0.0)
        && temp.is_finite()
        && temp > 0.0
        && temp <= 1e5
        && volume.is_finite()
        && volume > 0.0
        && hubble.is_finite()
        && hubble >= 0.0
        && total_heat.is_finite()
        && total_heat >= 0.0)
    {
        return Err("TIME_MAP_ROOT_CONTEXT_OUT_OF_DOMAIN".into());
    }
    let ne = (pop[1] + pop[3] + 2.0 * pop[4]) / volume;
    let np = pop.iter().sum::<f64>() + pop[1] + pop[3] + 2.0 * pop[4];
    let rhs = total_heat - cool_rate(temp, pop, ne) - 3.0 * hubble * KB * np * temp;
    if !rhs.is_finite() {
        return Err("TIME_MAP_NONFINITE_ROOT_RHS".into());
    }
    Ok(rhs)
}

pub(crate) struct TimeMapOracle {
    pub(crate) volume: Vec<f64>,
    pub(crate) v: Vec<f64>,
    pub(crate) flux_lo: Vec<f64>,
    pub(crate) flux_hi: Vec<f64>,
    pub(crate) extra_lo: Vec<f64>,
    pub(crate) extra_hi: Vec<f64>,
}
pub(crate) fn time_map_oracle(data: &[u8]) -> Result<TimeMapOracle, String> {
    let x = oracle(data, Some(E1_SHA))?;
    if x.n != N {
        return Err("NOT_ORIGINAL_46080_ORACLE".into());
    }
    Ok(TimeMapOracle {
        volume: (0..N)
            .map(|i| 0.5 * x.volume_lo[i] + 0.5 * x.volume_hi[i])
            .collect(),
        v: (0..N).map(|i| 0.5 * x.v_lo[i] + 0.5 * x.v_hi[i]).collect(),
        flux_lo: x.flux_lo,
        flux_hi: x.flux_hi,
        extra_lo: x.extras_lo,
        extra_hi: x.extras_hi,
    })
}

#[derive(Default)]
struct Sum {
    sum: f64,
    correction: f64,
}
impl Sum {
    fn add(&mut self, x: f64) {
        let z = self.sum + x;
        self.correction += if self.sum.abs() >= x.abs() {
            (self.sum - z) + x
        } else {
            (x - z) + self.sum
        };
        self.sum = z;
    }
    fn val(&self) -> f64 {
        self.sum + self.correction
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "USAGE: native_nonphoto ORIG5_PARENT.raw ORIGINAL_EVENT_ORACLE.raw OUTPUT_DERIVED.raw"
                .into(),
        );
    }
    let original = source::read_parent(
        &fs::read(&args[1])?,
        Some("eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"),
    )?;
    let event = oracle(&fs::read(&args[2])?, Some(E1_SHA))?;
    let mut b = Vec::new();
    io::stdin().read_to_end(&mut b)?;
    let native_photo = source::photon::source_locked_photo_stage(&b)?;
    if original.n != N || event.n != N || native_photo.n != N {
        return Err("NOT_FULL_ORIGINAL_46080_SOURCE".into());
    }
    let clock = Instant::now();
    let mut output = Vec::with_capacity(16 + 8 * DERIVED * N);
    output.extend_from_slice(b"REIF05N1");
    output.extend_from_slice(&(N as u64).to_le_bytes());
    let mut flux_ok = [0usize; 25];
    let mut energy_ok = [0usize; EXTRA];
    let mut hmax = 0.0_f64;
    let mut hemax = 0.0_f64;
    let mut hrelmax = 0.0_f64;
    let mut herelmax = 0.0_f64;
    let mut erelmax = 0.0_f64;
    let mut pidmax = 0.0_f64;
    let mut max_firstlaw = 0.0_f64;
    let mut min_dlogt = f64::INFINITY;
    let mut max_dlogt = f64::NEG_INFINITY;
    let mut extra_tot = [
        Sum::default(),
        Sum::default(),
        Sum::default(),
        Sum::default(),
        Sum::default(),
        Sum::default(),
        Sum::default(),
    ];
    let mut d_ne = Sum::default();
    let mut cool = Sum::default();
    let mut expansion = Sum::default();
    let mut noneq_nodes = 0usize;
    let mut miss_records = Vec::new();
    for i in 0..N {
        let pop = std::array::from_fn(|s| {
            let k = 5 * i + s;
            0.5 * (original.poplo[k] + original.pophi[k])
        });
        let ln_t = (original.logtlo[i] + original.logthi[i]) * 0.5;
        let t = ln_t.exp();
        let vol = (event.volume_lo[i] + event.volume_hi[i]) * 0.5;
        let hub = (event.hubble_lo[i] + event.hubble_hi[i]) * 0.5;
        let v = (event.v_lo[i] + event.v_hi[i]) * 0.5;
        let f = 0.55;
        for (s, t) in [0usize, 2, 3].iter().enumerate() {
            if pop[*t].to_bits() != native_photo.neutral[3 * i + s].to_bits() {
                return Err(format!("SOURCE_PARENT_MIDPOINT_NOT_IDENTICAL_NODE_{i}").into());
            }
        }
        let photo = [
            native_photo.photo[3 * i],
            native_photo.photo[3 * i + 1],
            native_photo.photo[3 * i + 2],
        ];
        let p = point_event(
            pop,
            t,
            vol,
            hub,
            v,
            f,
            photo,
            native_photo.primary_heat_erg_per_s[i],
        )?;
        for k in 0..25 {
            let ix = i * 25 + k;
            if event.flux_lo[ix] <= p.flux[k] && p.flux[k] <= event.flux_hi[ix] {
                flux_ok[k] += 1;
            } else if miss_records.len() < 12 {
                miss_records.push(format!(
                    "flux:node={i}:channel={k}:value={}:lo={}:hi={}",
                    p.flux[k], event.flux_lo[ix], event.flux_hi[ix]
                ));
            }
        }
        for k in 0..EXTRA {
            let ix = i * EXTRA + k;
            if event.extras_lo[ix] <= p.extra[k] && p.extra[k] <= event.extras_hi[ix] {
                energy_ok[k] += 1;
            } else if miss_records.len() < 12 {
                miss_records.push(format!(
                    "extra:node={i}:channel={k}:value={}:lo={}:hi={}",
                    p.extra[k], event.extras_lo[ix], event.extras_hi[ix]
                ));
            }
            extra_tot[k].add(p.extra[k]);
        }
        hmax = hmax.max(p.source_h_balance.abs());
        hemax = hemax.max(p.source_he_balance.abs());
        let hscale = (p.net[0].abs() + p.net[1].abs()).max(1e-300);
        let hescale = (p.net[2].abs() + p.net[3].abs() + p.net[4].abs()).max(1e-300);
        hrelmax = hrelmax.max(p.source_h_balance.abs() / hscale);
        herelmax = herelmax.max(p.source_he_balance.abs() / hescale);
        pidmax = pidmax.max(p.photon_identity_residual.abs());
        let escale = p.extra[3].abs().max(p.extra[1].abs()).max(1e-300);
        erelmax = erelmax.max(p.source_energy_residual.abs() / escale);
        max_firstlaw = max_firstlaw.max(p.thermal_firstlaw_residual_relative);
        min_dlogt = min_dlogt.min(p.dlogt_dt_per_s);
        max_dlogt = max_dlogt.max(p.dlogt_dt_per_s);
        d_ne.add(p.ne_rate);
        cool.add(p.cooling);
        expansion.add(p.expansion);
        if p.ne_rate < 0.0 {
            noneq_nodes += 1
        }
        for x in p.net {
            output.extend_from_slice(&x.to_le_bytes());
        }
        for x in p.extra {
            output.extend_from_slice(&x.to_le_bytes());
        }
        output.extend_from_slice(&p.ne_rate.to_le_bytes());
        output.extend_from_slice(&p.cooling.to_le_bytes());
        output.extend_from_slice(&p.dlogt_dt_per_s.to_le_bytes());
        for x in p.flux {
            output.extend_from_slice(&x.to_le_bytes());
        }
    }
    if output.len() != 16 + 8 * DERIVED * N {
        return Err("NATIVE_ORIGINAL_EVENT_OUTPUT_WRONG_LENGTH".into());
    }
    let report = json!({
        "schema":"VIGILODE_REI_F05_REAL_46080_NATIVE_FULL_EVENT_SOURCE_POINT_V1",
        "original_26event_source_owner":"fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120",
        "actual_source_nodes":N,"source_time_tick":"160/131072",
        "original_event_fixture_raw_sha256":E1_SHA,
        "original_accepted_parent_raw_sha256":"eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064",
        "original_photon_fixture_raw_sha256":"a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469",
        "original_heii_source_log10_positive_extended":native_photo.source_heii_trace_log10,
        "native_binary64_heii_trace_underflow_is_not_physical_zero":true,
        "f_hummer_photon_escape_source_interval":[0.1,1.0],
        "selected_source_point_f":0.55,
        "selected_source_point_v":"(original_policy_vlo+original_policy_vhi)/2_PER_NODE",
        "actual_25_original_flux_transition_checks":flux_ok,
        "actual_7_original_energy_component_checks":energy_ok,
        "all_owner_interval_checks_pass":flux_ok.iter().all(|x|*x==N)&&energy_ok.iter().all(|x|*x==N),
        "owner_interval_exclusion_first_examples":miss_records,
        "H_nuclei_max_absolute_residual_per_second":hmax,
        "He_nuclei_max_absolute_residual_per_second":hemax,
        "H_nuclei_max_relative_rate_residual":hrelmax,
        "He_nuclei_max_relative_rate_residual":herelmax,
        "OTS_energy_closure_max_relative_residual":erelmax,
        "thermal_firstlaw_max_relative_residual":max_firstlaw,
        "full_source_dlogt_min_per_s":min_dlogt,
        "full_source_dlogt_max_per_s":max_dlogt,
        "original_photon_identity_max_abs_residual":pidmax,
        "native_total_electron_rate_per_s":d_ne.val(),
        "native_net_electron_decrease_cells":noneq_nodes,
        "native_resolved_OTS_heat_total_erg_s":extra_tot[0].val(),
        "native_unresolved_OTS_energy_total_erg_s":extra_tot[1].val(),
        "native_escaped_OTS_energy_total_erg_s":extra_tot[2].val(),
        "native_chemical_energy_total_erg_s":extra_tot[3].val(),
        "native_resolved_photo_plus_OTS_heat_total_erg_s":extra_tot[4].val(),
        "native_thermal_rhs_total_erg_s":extra_tot[6].val(),
        "native_legacy_original_cooling_total_erg_s":cool.val(),
        "native_hubble_expansion_cooling_total_erg_s":expansion.val(),
        "original_extra_order":["OTS_heat","unresolved","escaped","chemical","photo_plus_OTS_heat","photon_identity","thermal_rhs"],
        "native_binary_format":"REIF05N1 u64N then per node: net5, oracle_extra7, ne_rate, original_cooling, original_source_dlnT_dt, original_5x5_destination_source_flux25; all f64 le",
        "derived_binary_bytes":output.len(),
        "derived_binary_sha256":sha(&output),
        "native_source_rate_seconds":clock.elapsed().as_secs_f64(),
        "full_original_26event_continuum_ivp":"NOT_RUN",
        "original_14_final_time_map_ledgers":"NOT_RUN",
        "original_tick161":"NOT_ACCEPTED","canonical_delta":0,
        "vigilode_dag":"7/23","rei_central_dag":"28/88","production":"HOLD"
    });
    fs::write(&args[3], &output)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report["all_owner_interval_checks_pass"]
        .as_bool()
        .unwrap_or(false)
    {
        return Err("ORIGINAL_NATIVE_NONPHOTO_SOURCE_INTERVAL_FAIL".into());
    }
    if pidmax > 1e-12
        || erelmax > 1e-12
        || max_firstlaw > 1e-12
        || hrelmax > 1e-12
        || herelmax > 1e-12
    {
        return Err("ORIGINAL_NONPHOTO_ENERGY_OR_PHOTON_LEDGER_RESIDUAL".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_equations_nuclear_charge_and_energy() {
        let p = [2.0e10, 1e10, 1e9, 4e8, 1e8];
        let r = point_event(p, 1e4, 1e24, 2e-17, 0.3, 0.55, [5e5, 3e4, 2e3], 1e-9).unwrap();
        assert!(r.source_h_balance.abs() < 1e-5);
        assert!(r.source_he_balance.abs() < 1e-5);
        assert!(r.source_energy_residual.abs() < 1e-10);
        assert!(r.photon_identity_residual.abs() < 1e-12);
        assert!(r.extra.iter().all(|x| x.is_finite()));
    }
    #[test]
    fn rate_coefficients_and_domain_guards() {
        let r = rates(1e4);
        assert!(r.iter().all(|x| x.is_finite() && *x > 0.0));
        let p = [2e10, 1e10, 1e9, 4e8, 1e8];
        assert!(point_event(p, -1.0, 1e24, 2e-17, 0.5, 0.55, [1., 1., 1.], 1e-5).is_err());
        assert!(point_event(p, 1e4, -1.0, 2e-17, 0.5, 0.55, [1., 1., 1.], 1e-5).is_err());
        assert!(point_event(p, 1e4, 1e24, 2e-17, 2.0, 0.55, [1., 1., 1.], 1e-5).is_err());
        assert!(point_event(p, 1e4, 1e24, 2e-17, 0.5, 2.0, [1., 1., 1.], 1e-5).is_err());
    }
    #[test]
    fn bad_oracle_magic_and_source_identity_fail() {
        assert!(oracle(&[0u8; 16], None).is_err());
        assert!(oracle(&[0u8; 16], Some(E1_SHA)).is_err());
    }
}
