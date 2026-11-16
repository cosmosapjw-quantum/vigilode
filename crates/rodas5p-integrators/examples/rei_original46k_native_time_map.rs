//! Actual original REI F05 source-clock native Rust 26-event point time-map SHADOW.
//! Owner 26-event interval uncertainty, 14 certified interval ledgers and tick161
//! admission remain outside this Rust f64 point experiment. No default mutation.
#![recursion_limit = "256"]
#![allow(dead_code)]
#[path = "rei_original46k_native_nonphoto.rs"]
mod chemistry;
#[path = "rei_original46k_native_photo_thermal.rs"]
mod source;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;
use std::{env, fs};

const N: usize = 46_080;
const KB: f64 = 1.380649e-16;
const E1: &str = "28ea21ea9e5b4210b3e8971313af6aa6a8c4a07fad10c984bfd29424967ebf0b";
const CLOCK_SHA: &str = "0bd888ce706ee110cea4ba23a1e871817cf1f054b85ebe5837934fd19db0492e";
const GAMMA: f64 = 0.29289321881345254;
type Pop = [f64; 5];
type Flux = [f64; 25];

#[derive(Clone, Deserialize)]
struct Site {
    label: String,
    time_s: f64,
    #[serde(rename = "current_J")]
    j: [f64; 4],
    #[serde(rename = "external_E")]
    e: [f64; 4],
    alpha: [[f64; 3]; 4],
    z: f64,
    volume_scale_from_t160: f64,
    hubble_s_inv: f64,
}
#[derive(Clone, Deserialize)]
struct TimeMap {
    name: String,
    t0_s: f64,
    tg_s: f64,
    t1_s: f64,
    sites: [Site; 3],
}
#[derive(Deserialize)]
struct Clock {
    schema: String,
    original_owner_head: String,
    original_source_tar_sha256: String,
    original_state_sha256: String,
    original_kernel_sha256: String,
    accepted_tick: String,
    source_cells: usize,
    photon_groups: usize,
    duration_seconds: f64,
    source_tick_width_seconds: f64,
    gamma: f64,
    excess_eV: [[f64; 4]; 3],
    eV_to_erg: f64,
    maps: [TimeMap; 3],
}
#[derive(Clone)]
struct State {
    pop: Vec<Pop>,
    logt: Vec<f64>,
}
fn digest(b: &[u8]) -> String {
    let mut x = Sha256::new();
    x.update(b);
    format!("{:x}", x.finalize())
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-3
}
fn check_clock(c: &Clock) -> Result<(), String> {
    if c.schema != "REI_F05_ORIGINAL_NATIVE_RUST_CLOCK_SOURCE_ONLY_V1"
        || c.original_owner_head != "fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
        || c.original_source_tar_sha256
            != "a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
        || c.original_state_sha256
            != "cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
        || c.original_kernel_sha256
            != "579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
        || c.accepted_tick != "160/131072"
        || c.source_cells != N
        || c.photon_groups != 4
        || (c.gamma - GAMMA).abs() > 2e-15
        || !(c.duration_seconds > 0.0)
        || !(c.eV_to_erg > 0.0)
    {
        return Err("F05_SOURCE_AUTHORITY_OR_CLOCK_SCHEMA_DRIFT".into());
    }
    let a = c.duration_seconds * (160.0 / 131072.0);
    let b = c.duration_seconds * (161.0 / 131072.0);
    let mid = (a + b) / 2.0;
    if !near(c.source_tick_width_seconds, b - a) {
        return Err("F05_TICK_WIDTH_DRIFT".into());
    }
    for (k, map) in c.maps.iter().enumerate() {
        let (expect_name, t0, t1) = match k {
            0 => ("full", a, b),
            1 => ("half1", a, mid),
            _ => ("half2", mid, b),
        };
        let tg = t0 + GAMMA * (t1 - t0);
        if map.name != expect_name
            || !near(map.t0_s, t0)
            || !near(map.tg_s, tg)
            || !near(map.t1_s, t1)
            || !(t0 < tg && tg < t1)
        {
            return Err(format!("ORIGINAL_CLOCK_SITE_DRIFT_{expect_name}"));
        }
        for (n, (s, t)) in map.sites.iter().zip([t0, tg, t1]).enumerate() {
            if s.label != ["t0", "tg", "t1"][n]
                || !near(s.time_s, t)
                || !(s.volume_scale_from_t160 > 0.0 && s.hubble_s_inv >= 0.0)
                || s.j.iter().chain(&s.e).any(|x| !x.is_finite() || *x < 0.0)
                || s.alpha.iter().flatten().any(|x| !x.is_finite() || *x < 0.0)
            {
                return Err("ORIGINAL_SOURCE_CLOCK_PHYSICAL_DRIFT".into());
            }
        }
    }
    if c.excess_eV
        .iter()
        .flatten()
        .any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err("ORIGINAL_EXCESS_ENERGY_DRIFT".into());
    }
    Ok(())
}
fn particles(pop: Pop) -> f64 {
    pop.iter().sum::<f64>() + pop[1] + pop[3] + 2.0 * pop[4]
}
fn kahan_sum(x: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0, 0.0);
    for z in x {
        let y = z - c;
        let t = s + y;
        c = (t - s) - y;
        s = t;
    }
    s
}
// Evaluate all global neutral moments afresh at *each* source clock/stage state.
fn photon(
    state: &State,
    site: &Site,
    excess: &[[f64; 4]; 3],
    ev: f64,
) -> Result<(Vec<[f64; 3]>, Vec<f64>, f64), String> {
    let totals = [
        kahan_sum(state.pop.iter().map(|p| p[0])),
        kahan_sum(state.pop.iter().map(|p| p[2])),
        kahan_sum(state.pop.iter().map(|p| p[3])),
    ];
    let mut rate = [0.0; 3];
    let mut heat = [0.0; 3];
    let mut max_group_residual = 0.0_f64;
    for g in 0..4 {
        let d = site.e[g] + (0..3).map(|s| site.alpha[g][s] * totals[s]).sum::<f64>();
        if !(d.is_finite() && d > 0.0) {
            return Err("RUST_STAGE_PHOTON_DEN_NONPOSITIVE".into());
        }
        max_group_residual = max_group_residual
            .max(((site.j[g] * d / d) - site.j[g]).abs() / site.j[g].abs().max(1.0));
        for s in 0..3 {
            // Reassociate to avoid underflow from multiplying J*alpha first.
            let x = (site.j[g] / d) * site.alpha[g][s];
            rate[s] += x;
            heat[s] += x * excess[s][g] * ev;
        }
    }
    let mut p = Vec::with_capacity(state.pop.len());
    let mut q: Vec<f64> = Vec::with_capacity(state.pop.len());
    for x in &state.pop {
        let ns = [x[0], x[2], x[3]];
        p.push(std::array::from_fn(|s| ns[s] * rate[s]));
        q.push((0..3).map(|s| ns[s] * heat[s]).sum());
    }
    if p.iter().flatten().any(|x| !x.is_finite() || *x < 0.0)
        || q.iter().any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err("RUST_STAGE_PHOTON_NONFINITE".into());
    }
    Ok((p, q, max_group_residual))
}
const LOGK: [f64; 5] = [
    4.0 * std::f64::consts::LN_10,
    4.25 * std::f64::consts::LN_10,
    4.5 * std::f64::consts::LN_10,
    4.75 * std::f64::consts::LN_10,
    5.0 * std::f64::consts::LN_10,
];
const VT: [f64; 5] = [0.285, 0.305, 0.325, 0.350, 0.375];
fn source_v(logt: f64) -> Result<f64, String> {
    if !logt.is_finite() || logt > LOGK[4] + 2e-14 {
        return Err("ABOVE_SOURCE_V_TABLE".into());
    }
    if logt < LOGK[0] {
        return Ok(0.5);
    }
    // Original policy treats 8*epsilon-neighbourhood of each log10 knot
    // as a degenerate cell rather than taking the next bracket midpoint.
    for k in 0..5 {
        if (logt - LOGK[k]).abs() <= 8.0 * f64::EPSILON * std::f64::consts::LN_10 {
            return Ok(VT[k]);
        }
    }
    for k in 0..4 {
        if logt < LOGK[k + 1] {
            return Ok(0.5 * (VT[k] + VT[k + 1]));
        }
    }
    Ok(VT[4])
}
fn events(
    state: &State,
    site: &Site,
    vol0: &[f64],
    clock: &Clock,
    initial_v: Option<&[f64]>,
) -> Result<(Vec<chemistry::TimeMapEvent>, f64), String> {
    let (p, h, res) = photon(state, site, &clock.excess_eV, clock.eV_to_erg)?;
    let mut out = Vec::with_capacity(state.pop.len());
    for i in 0..state.pop.len() {
        let v = match initial_v {
            Some(arr) => arr[i],
            None => source_v(state.logt[i])?,
        };
        let temp = state.logt[i].exp();
        let x = chemistry::time_map_event(
            state.pop[i],
            temp,
            vol0[i] * site.volume_scale_from_t160,
            site.hubble_s_inv,
            v,
            0.55,
            p[i],
            h[i],
        )?;
        out.push(x);
    }
    Ok((out, res))
}
fn solve_block(a: &[[f64; 5]; 5], b: Pop, start: usize, n: usize) -> Result<Vec<f64>, String> {
    let mut aug = [[0.0; 4]; 3];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a[start + i][start + j];
        }
        aug[i][n] = b[start + i];
    }
    for k in 0..n {
        let mut pivot = k;
        for i in k + 1..n {
            if aug[i][k].abs() > aug[pivot][k].abs() {
                pivot = i;
            }
        }
        if !(aug[pivot][k] > 0.0 || aug[pivot][k] < 0.0) {
            return Err("SINGULAR_F05_MPRK_BLOCK".into());
        }
        aug.swap(k, pivot);
        let p = aug[k][k];
        if !(p.is_finite() && p > 0.0) {
            return Err("F05_MPRK_M_MATRIX_PIVOT_FAIL".into());
        }
        for i in k + 1..n {
            let f = aug[i][k] / p;
            for j in k + 1..=n {
                aug[i][j] -= f * aug[k][j];
            }
            aug[i][k] = 0.0;
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = aug[i][n];
        for j in i + 1..n {
            sum -= aug[i][j] * x[j];
        }
        x[i] = sum / aug[i][i];
        if !(x[i] > 0.0 && x[i].is_finite()) {
            return Err("F05_MPRK_POSITIVE_CONE_FAIL".into());
        }
    }
    Ok(x)
}
fn population_step(state: &State, flux: &[Flux], den: &[Pop], dt: f64) -> Result<State, String> {
    if !(dt > 0.0 && dt.is_finite()) {
        return Err("BAD_F05_MAP_DURATION".into());
    }
    let mut out = Vec::with_capacity(state.pop.len());
    for (i, &p) in state.pop.iter().enumerate() {
        let mut a = [[0.0; 5]; 5];
        for s in 0..5 {
            a[s][s] = 1.0;
        }
        for s in 0..5 {
            if !(den[i][s] > 0.0) {
                return Err("MPRK_ZERO_DENOM".into());
            }
            for d in 0..5 {
                if d == s {
                    continue;
                }
                let r = dt * flux[i][5 * d + s] / den[i][s];
                if !(r >= 0.0 && r.is_finite()) {
                    return Err("MPRK_NONFINITE_FLUX".into());
                }
                a[s][s] += r;
                a[d][s] -= r;
            }
        }
        let h = solve_block(&a, p, 0, 2)?;
        let he = solve_block(&a, p, 2, 3)?;
        let x = [h[0], h[1], he[0], he[1], he[2]];
        for &(start, n) in &[(0, 2), (2, 3)] {
            let initial = p[start..start + n].iter().sum::<f64>();
            let output = x[start..start + n].iter().sum::<f64>();
            if (output - initial).abs() > 2.0e-12 * initial {
                return Err("F05_MPRK_NUCLEI_CONSERVATION_FAIL".into());
            }
        }
        out.push(x);
    }
    Ok(State {
        pop: out,
        logt: state.logt.clone(),
    })
}
fn thermal_root(
    pop: Pop,
    logparent: f64,
    u0: f64,
    dt: f64,
    constant: f64,
    weight: f64,
    fixed_heat: f64,
    vol: f64,
    hubble: f64,
) -> Result<(f64, f64), String> {
    let np = particles(pop);
    if !(np > 0.0 && u0 > 0.0 && dt > 0.0) {
        return Err("NONPHYSICAL_SD3_CONTEXT".into());
    }
    let f = |x: f64| -> Result<f64, String> {
        if !(x.is_finite() && x <= LOGK[4] + 2e-14) {
            return Err("THERMAL_ROOT_ABOVE_TABLE".into());
        }
        let t = x.exp();
        let rhs = chemistry::thermal_rhs_fixed_heat(pop, t, vol, hubble, fixed_heat)?;
        Ok(1.5 * KB * np * t - u0 - dt * (constant + weight * rhs))
    };
    let (mut a, mut b) = (logparent - 0.02, (logparent + 0.02).min(LOGK[4]));
    let (mut fa, mut fb) = (f(a)?, f(b)?);
    for _ in 0..20 {
        if fa <= 0.0 && fb >= 0.0 {
            break;
        }
        if fa > 0.0 {
            a -= 0.02;
            fa = f(a)?;
        }
        if fb < 0.0 && b < LOGK[4] {
            b = (b + 0.02).min(LOGK[4]);
            fb = f(b)?;
        }
    }
    if !(fa <= 0.0 && fb >= 0.0) {
        return Err("F05_THERMAL_ROOT_BRACKET_FAIL".into());
    }
    for _ in 0..66 {
        let m = 0.5 * (a + b);
        let fm = f(m)?;
        if fm <= 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    let x = 0.5 * (a + b);
    let rhs = chemistry::thermal_rhs_fixed_heat(pop, x.exp(), vol, hubble, fixed_heat)?;
    let residual = (1.5 * KB * np * x.exp() - u0 - dt * (constant + weight * rhs)).abs() / u0;
    if !(residual < 2.0e-13) {
        return Err("F05_THERMAL_ROOT_RESIDUAL_FAIL".into());
    }
    Ok((x, rhs))
}
fn cross_knot(a: f64, b: f64) -> bool {
    LOGK.iter().any(|&k| (a - k) * (b - k) <= 0.0)
}
fn old_energy(pop: Pop, logt: f64) -> f64 {
    1.5 * KB * particles(pop) * logt.exp()
}
fn map_step(
    state: &State,
    map: &TimeMap,
    vol0: &[f64],
    v0: Option<&[f64]>,
    clock: &Clock,
) -> Result<(State, serde_json::Value), String> {
    let timer = Instant::now();
    let dt = map.t1_s - map.t0_s;
    let (e0, group0) = events(state, &map.sites[0], vol0, clock, v0)?;
    let flux0 = e0.iter().map(|e| e.flux).collect::<Vec<_>>();
    let pred = population_step(state, &flux0, &state.pop, dt)?;
    let gamma = population_step(state, &flux0, &state.pop, GAMMA * dt)?;
    let mut pred_state = pred;
    let mut max_root = 0.0_f64;
    for i in 0..N {
        let (x, _) = thermal_root(
            pred_state.pop[i],
            state.logt[i],
            old_energy(state.pop[i], state.logt[i]),
            dt,
            0.0,
            1.0,
            e0[i].heat_erg_s,
            vol0[i] * map.sites[0].volume_scale_from_t160,
            map.sites[0].hubble_s_inv,
        )?;
        max_root = max_root.max((x - state.logt[i]).abs());
        pred_state.logt[i] = x;
    }
    let (e1, group1) = events(&pred_state, &map.sites[2], vol0, clock, None)?;
    let avg = e0
        .iter()
        .zip(e1.iter())
        .map(|(a, b)| std::array::from_fn(|k| 0.5 * a.flux[k] + 0.5 * b.flux[k]))
        .collect::<Vec<Flux>>();
    let mut final_state = population_step(state, &avg, &pred_state.pop, dt)?;
    let mut stage_state = State {
        pop: gamma.pop,
        logt: state.logt.clone(),
    };
    let mut max_change = 0.0_f64;
    let mut converged = false;
    let mut nouter = 0;
    let (mut last_stage_group, mut last_final_group) = (group0, group1);
    for outer in 0..8 {
        nouter = outer + 1;
        let (stage, gs) = events(&stage_state, &map.sites[1], vol0, clock, None)?;
        let (finalev, gf) = events(&final_state, &map.sites[2], vol0, clock, None)?;
        last_stage_group = gs;
        last_final_group = gf;
        let mut next_stage = stage_state.logt.clone();
        let mut next_final = final_state.logt.clone();
        max_change = 0.0;
        for i in 0..N {
            let u0 = old_energy(state.pop[i], state.logt[i]);
            let (xs, rs) = thermal_root(
                stage_state.pop[i],
                state.logt[i],
                u0,
                GAMMA * dt,
                0.0,
                1.0,
                stage[i].heat_erg_s,
                vol0[i] * map.sites[1].volume_scale_from_t160,
                map.sites[1].hubble_s_inv,
            )?;
            let (xf, _rf) = thermal_root(
                final_state.pop[i],
                state.logt[i],
                u0,
                dt,
                (1.0 - GAMMA) * rs,
                GAMMA,
                finalev[i].heat_erg_s,
                vol0[i] * map.sites[2].volume_scale_from_t160,
                map.sites[2].hubble_s_inv,
            )?;
            max_change = max_change
                .max((xs - next_stage[i]).abs())
                .max((xf - next_final[i]).abs());
            next_stage[i] = xs;
            next_final[i] = xf;
        }
        stage_state.logt = next_stage;
        final_state.logt = next_final;
        if max_change < 2.0e-13 {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(format!(
            "F05_NATIVE_THERMAL_OUTER_NOT_CONVERGED_{max_change:e}"
        ));
    }
    let (stage_final, gs) = events(&stage_state, &map.sites[1], vol0, clock, None)?;
    let (final_events, gf) = events(&final_state, &map.sites[2], vol0, clock, None)?;
    last_stage_group = last_stage_group.max(gs);
    last_final_group = last_final_group.max(gf);
    let mut max_h = 0.0_f64;
    let mut max_he = 0.0_f64;
    let (mut max_energy, mut max_photid) = (0.0_f64, 0.0_f64);
    let (mut source_table_events, mut delta_ne, mut delta_u) = (0usize, 0.0_f64, 0.0_f64);
    for i in 0..N {
        let old = state.pop[i];
        let out = final_state.pop[i];
        let h = old[0] + old[1];
        let he = old[2] + old[3] + old[4];
        max_h = max_h.max(((out[0] + out[1]) - h).abs() / h);
        max_he = max_he.max(((out[2] + out[3] + out[4]) - he).abs() / he);
        let ne = |p: Pop| p[1] + p[3] + 2.0 * p[4];
        delta_ne += ne(out) - ne(old);
        delta_u += old_energy(out, final_state.logt[i]) - old_energy(old, state.logt[i]);
        max_energy = max_energy
            .max(stage_final[i].ots_residual_erg_s.abs() / stage_final[i].heat_erg_s.abs().max(1.0))
            .max(
                final_events[i].ots_residual_erg_s.abs()
                    / final_events[i].heat_erg_s.abs().max(1.0),
            );
        max_photid = max_photid
            .max(stage_final[i].photon_identity.abs())
            .max(final_events[i].photon_identity.abs());
        if cross_knot(state.logt[i], pred_state.logt[i])
            || cross_knot(state.logt[i], stage_state.logt[i])
            || cross_knot(state.logt[i], final_state.logt[i])
        {
            source_table_events += 1;
        }
    }
    // 14 point-valued closure diagnostics, NOT original interval-set ledgers.
    let max_group = group0
        .max(group1)
        .max(last_stage_group)
        .max(last_final_group);
    let point_closure = max_h < 2e-12
        && max_he < 2e-12
        && max_energy < 2e-12
        && max_photid < 2e-12
        && max_group < 2e-12;
    let record = json!({
        "map":map.name,"source_dt_s":dt,"clock_start_s":map.t0_s,
        "clock_gamma_s":map.tg_s,"clock_final_s":map.t1_s,
        "source_cells":N,"positive_populations":final_state.pop.iter().all(|p|p.iter().all(|x|*x>0.0&&x.is_finite())),
        "point_source_nuclei_H_max_relative":max_h,
        "point_source_nuclei_He_max_relative":max_he,
        "point_OTS_chemical_energy_relative":max_energy,
        "point_photon_identity_max_abs":max_photid,
        "point_group_photon_residual_max_relative":max_group,
        "point_14_shape_closure_all_checks":point_closure,
        "original_14_interval_ledgers":"NOT_RUN",
        "table_event_candidate_cells":source_table_events,
        "outer_iterations":nouter,"outer_last_max_change_logT":max_change,
        "max_predictor_abs_delta_logT":max_root,
        "native_point_delta_N_e":delta_ne,"native_point_delta_U_erg":delta_u,
        "elapsed_seconds":timer.elapsed().as_secs_f64(),
    });
    if !point_closure {
        return Err(format!("NATIVE_POINT_PHYSICAL_LEDGER_FAIL_{record}"));
    }
    if source_table_events != 0 {
        return Err(format!(
            "ORIGINAL_TABLE_KNOT_CROSSING_REQUIRES_OWNER_RESTART_{source_table_events}"
        ));
    }
    Ok((final_state, record))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 6 {
        return Err("USAGE: native_time_map PARENT5.raw ORACLE_E1.raw FROZEN_PHOTON_R1.raw CLOCK.json OUTPUT_STATES.raw".into());
    }
    let rawclock = fs::read(&args[4])?;
    if digest(&rawclock) != CLOCK_SHA {
        return Err("EXACT_ORIGINAL_CLOCK_JSON_DRIFT".into());
    }
    let clock: Clock = serde_json::from_slice(&rawclock)?;
    check_clock(&clock)?;
    let accepted = source::read_parent(
        &fs::read(&args[1])?,
        Some("eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"),
    )?;
    let oracle = chemistry::time_map_oracle(&fs::read(&args[2])?)?;
    let frozen = fs::read(&args[3])?;
    if digest(&frozen) != "a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469"
        || &frozen[..8] != b"REIF05R1"
    {
        return Err("ORIGINAL_PHOTON_SOURCE_FIXTURE_DRIFT".into());
    }
    // Ensure dynamic source starts at the EXACT frozen 4-group original photon clock.
    for j in 0..4 {
        let v = f64::from_le_bytes(frozen[16 + 8 * j..24 + 8 * j].try_into()?);
        if clock.maps[0].sites[0].j[j].to_bits() != v.to_bits() {
            return Err("TIME_MAP_PHOTON_J_T160_DISAGREES_WITH_ORIGINAL".into());
        }
        let e = f64::from_le_bytes(frozen[48 + 8 * j..56 + 8 * j].try_into()?);
        if clock.maps[0].sites[0].e[j].to_bits() != e.to_bits() {
            return Err("TIME_MAP_PHOTON_E_T160_DISAGREES_WITH_ORIGINAL".into());
        }
    }
    if accepted.n != N {
        return Err("NOT_46080_ACCEPTED_PARENT".into());
    }
    let mut state = State {
        pop: Vec::with_capacity(N),
        logt: Vec::with_capacity(N),
    };
    for i in 0..N {
        state.pop.push(std::array::from_fn(|s| {
            0.5 * accepted.poplo[5 * i + s] + 0.5 * accepted.pophi[5 * i + s]
        }));
        state
            .logt
            .push(0.5 * accepted.logtlo[i] + 0.5 * accepted.logthi[i]);
    }
    let timer = Instant::now();
    let (parent_events, _) = events(
        &state,
        &clock.maps[0].sites[0],
        &oracle.volume,
        &clock,
        Some(&oracle.v),
    )?;
    let (mut fluxok, mut extraok) = ([0usize; 25], [0usize; 7]);
    for i in 0..N {
        for k in 0..25 {
            let j = i * 25 + k;
            if oracle.flux_lo[j] <= parent_events[i].flux[k]
                && parent_events[i].flux[k] <= oracle.flux_hi[j]
            {
                fluxok[k] += 1;
            }
        }
        for k in 0..7 {
            let j = i * 7 + k;
            if oracle.extra_lo[j] <= parent_events[i].extra[k]
                && parent_events[i].extra[k] <= oracle.extra_hi[j]
            {
                extraok[k] += 1;
            }
        }
    }
    if fluxok.iter().any(|v| *v != N) || extraok.iter().any(|v| *v != N) {
        return Err(format!(
            "ORIGINAL_T160_POINT_SOURCE_STAGE_REGRESSION_flux={fluxok:?}_extras={extraok:?}"
        )
        .into());
    }
    let (full, fs) = map_step(
        &state,
        &clock.maps[0],
        &oracle.volume,
        Some(&oracle.v),
        &clock,
    )?;
    let (half1, h1) = map_step(
        &state,
        &clock.maps[1],
        &oracle.volume,
        Some(&oracle.v),
        &clock,
    )?;
    let (half2, h2) = map_step(&half1, &clock.maps[2], &oracle.volume, None, &clock)?;
    let mut err_log = 0.0_f64;
    let mut err_hii = 0.0_f64;
    let mut err_heii = 0.0_f64;
    let mut err_heiii = 0.0_f64;
    for i in 0..N {
        err_log = err_log.max((full.logt[i] - half2.logt[i]).abs());
        let nh = state.pop[i][0] + state.pop[i][1];
        let he = state.pop[i][2] + state.pop[i][3] + state.pop[i][4];
        err_hii = err_hii.max(((full.pop[i][1] - half2.pop[i][1]) / nh).abs());
        err_heii = err_heii.max(((full.pop[i][3] - half2.pop[i][3]) / he).abs());
        err_heiii = err_heiii.max(((full.pop[i][4] - half2.pop[i][4]) / he).abs());
    }
    let mut b = Vec::with_capacity(16 + 3 * N * 6 * 8);
    b.extend_from_slice(b"REIF05M1");
    b.extend_from_slice(&(N as u64).to_le_bytes());
    for m in [&full, &half1, &half2] {
        for i in 0..N {
            for s in 0..5 {
                b.extend_from_slice(&m.pop[i][s].to_le_bytes());
            }
            b.extend_from_slice(&m.logt[i].to_le_bytes());
        }
    }
    fs::write(&args[5], &b)?;
    let receipt = json!({
        "schema":"VIGILODE_REI_F05_NATIVE_POINT_26EVENT_FULL_HALF_HALF_V1",
        "original_source_clock_sha256":CLOCK_SHA,
        "original_tick":"160/131072",
        "original_cells":N,
        "original_source_flux_inclusions":fluxok,
        "original_source_extra_inclusions":extraok,
        "full":fs,"half1":h1,"half2":h2,
        "native_point_doublemap_errors":{
            "x_HII_max":err_hii,"x_HeII_max":err_heii,
            "x_HeIII_max":err_heiii,"log_T_max":err_log},
        "native_point_states_raw_sha256":digest(&b),
        "native_point_states_raw_bytes":b.len(),
        "elapsed_seconds":timer.elapsed().as_secs_f64(),
        "original_full_interval_time_map_certified":false,
        "original_14_final_interval_ledgers":"NOT_RUN",
        "f64_HeII_underflow":"UNRESOLVED_POSITIVE_LOG_TRACE_PRESERVED_IN_PREDECESSOR",
        "historical_V1_logT_failure_preserved":2.1245050576368385e-4,
        "official_tick161":"NOT_ACCEPTED",
        "canonical_delta":0,"production":"HOLD"
    });
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_clock_orders_and_values() {
        let raw=include_str!("../../../exploratory/rei_igm_native_time_map_20261102/ORIGINAL_F05_T160_SOURCE_CLOCK.json");
        let clock: Clock = serde_json::from_str(raw).unwrap();
        assert!(check_clock(&clock).is_ok());
        let mut tampered: Clock = serde_json::from_str(raw).unwrap();
        tampered.maps[0].tg_s += 3.0;
        assert!(check_clock(&tampered).is_err());
    }
    #[test]
    fn positive_conservative_h_he_two_block() {
        let parent = State {
            pop: vec![[2.0, 1.0, 3.0, 1.0, 0.25]],
            logt: vec![10.0],
        };
        let mut flux = [[0.0; 25]; 1];
        flux[0][1 * 5] = 0.14;
        flux[0][3 * 5 + 2] = 0.19;
        flux[0][4 * 5 + 3] = 0.001;
        let q = population_step(&parent, &flux, &parent.pop, 2.0).unwrap();
        assert!((q.pop[0][0] + q.pop[0][1] - 3.0).abs() < 1e-13);
        assert!((q.pop[0][2] + q.pop[0][3] + q.pop[0][4] - 4.25).abs() < 1e-13);
        assert!(q.pop[0].iter().all(|&x| x > 0.0));
    }
    #[test]
    fn thermal_zero_duration_limit() {
        let p = [2.0, 1.0, 3.0, 1.0, 0.25];
        let t: f64 = 12_000.0;
        let ln = t.ln();
        let (x, _) =
            thermal_root(p, ln, old_energy(p, ln), 1e-9, 0.0, 1.0, 0.0, 1e50, 1e-17).unwrap();
        assert!((x - ln).abs() < 1e-10);
    }
    #[test]
    fn v_table_boundaries() {
        assert_eq!(source_v((9000.0_f64).ln()).unwrap(), 0.5);
        assert_eq!(source_v(4.25 * std::f64::consts::LN_10).unwrap(), 0.305);
        assert_eq!(source_v(5.0 * std::f64::consts::LN_10).unwrap(), 0.375);
        assert!(source_v((100001.0_f64).ln()).is_err());
    }
}
