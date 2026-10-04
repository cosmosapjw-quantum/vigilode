//! Export of research node `research/safe_enclosure_exp_floor_20261004`:
//! `directed::exp_interval` on the floor window and three grids, as IEEE
//! bits; `tools/safe_enclosure_exp_floor_check.py` checks them at 60 digits.

use rodas5p_core::directed::exp_interval;
use serde_json::{Value, json};

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn points() -> Vec<(String, f64)> {
    let mut out = vec![(
        "l0064_counterexample".to_string(),
        1071.0 * f64::from_bits(0x3f5d_28b0_1f11_190b) * f64::from_bits(0xc077_2ec0_562e_1c0f),
    )];
    for i in 0..=20_000 {
        out.push(("grid_709_700".into(), -709.0 + 9.0 * i as f64 / 20_000.0));
    }
    for centre in [-707.0, -707.023_458_683_456_7_f64] {
        let (mut lo, mut hi) = (centre, centre);
        out.push(("neighbours".into(), centre));
        for _ in 0..64 {
            lo = lo.next_down();
            hi = hi.next_up();
            out.push(("neighbours".into(), lo));
            out.push(("neighbours".into(), hi));
        }
    }
    // The REV-02 grid (rev02_nonnormal_stepping.rs), verbatim.
    for x in (0..=200)
        .map(|k| -700.0 + 7.0 * k as f64)
        .chain((0..50).map(|k| -1.0 + 2.0 * k as f64 / 49.0))
    {
        out.push(("rev02_grid".into(), x));
    }
    let mut state = 0x5afe_f100_u64;
    for _ in 0..20_000 {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        let unit = (z >> 11) as f64 / (1u64 << 53) as f64;
        let magnitude = (1.0e-300_f64.ln() + unit * (709.0_f64.ln() - 1.0e-300_f64.ln())).exp();
        let x = if z & 1 == 0 { magnitude } else { -magnitude };
        out.push(("random".into(), x));
    }
    out
}

#[test]
#[ignore = "export of research/safe_enclosure_exp_floor_20261004"]
fn export_exp_floor() {
    let rows: Vec<Value> = points()
        .into_iter()
        .filter_map(|(group, x)| {
            exp_interval(x)
                .ok()
                .map(|e| json!([group, hx(x), hx(e.lo), hx(e.hi)]))
        })
        .collect();
    let out = json!({"schema": "vigilode-safe-enclosure-exp-floor-v1", "rows": rows});
    if let Ok(path) = std::env::var("SAFE_EXP_FLOOR_OUTPUT") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

#[test]
fn floor_window_is_enclosed() {
    // e^{-707.01} = 8.927e-308 must not exceed the upper endpoint.
    let e = exp_interval(-707.01).unwrap();
    assert!(e.hi > 8.93e-308);
}
