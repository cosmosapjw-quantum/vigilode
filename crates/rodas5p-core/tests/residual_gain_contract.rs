//! Research node AS05 (`research/as05_residual_gain_core_20261011`): the
//! core residual gain witness against the unchanged re-audit prototype
//! `gain_witness.rs` (included by path as the parity reference), the new
//! negatives and positives, and the RUNS.json export for
//! `tools/as05_residual_gain_check.py`.
//!
//! The W-formation fixture selection uses exact dyadic/rational arithmetic
//! only (module `exact`), never a floating-point comparison or a core
//! interval; the checker repeats it independently with Python `Fraction`.

#[allow(dead_code, clippy::all)]
#[path = "../../../research/reaudit_accuracy_speed_20261010/gain_witness.rs"]
mod gain_witness;

use rodas5p_core::residual_gain::{
    AssemblyUncertainty, DeclaredStructure, FrozenMetric, GainEvidence, LinearTargetId,
    ResidualGainWitness, VerifiedCorrection, WitnessRejection, WitnessRoute,
    certify_by_approximate_inverse_defect, dense_lu_comparator, lu_comparator_flops,
    projected_nu_estimate,
};
use rodas5p_core::{DenseMatrix, LuFactorization, rodas5p_coefficients, sha256_hex};
use serde_json::{Value, json};

const GAIN_WITNESS_SOURCE: &[u8] =
    include_bytes!("../../../research/reaudit_accuracy_speed_20261010/gain_witness.rs");
const GAIN_WITNESS_SHA256: &str =
    "22c977199d3c60849db94227a973a5d83ffea381650043774e8510ad5e745c89";
const EPOCH: Option<u64> = Some(7);
const H_LIST: [f64; 8] = [0.1, 0.3, 0.7, 1.1, 1.3, 1.7, 1.9, 2.3];
const K_LIST: [f64; 2] = [0.0, 1024.0];

/// Exact dyadic arithmetic (sign, magnitude limbs, binary exponent) and
/// correctly rounded quotients; no floating-point comparison decides.
mod exact {
    use std::cmp::Ordering;

    #[derive(Clone, Debug)]
    pub struct Dyadic {
        neg: bool,
        mag: Vec<u32>,
        exp: i64,
    }

    fn trim(v: &mut Vec<u32>) {
        while v.last() == Some(&0) {
            v.pop();
        }
    }

    fn mag_cmp(a: &[u32], b: &[u32]) -> Ordering {
        if a.len() != b.len() {
            return a.len().cmp(&b.len());
        }
        for (x, y) in a.iter().rev().zip(b.iter().rev()) {
            if x != y {
                return x.cmp(y);
            }
        }
        Ordering::Equal
    }

    fn mag_add(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
        let mut carry = 0u64;
        for i in 0..a.len().max(b.len()) {
            let s = carry + *a.get(i).unwrap_or(&0) as u64 + *b.get(i).unwrap_or(&0) as u64;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        trim(&mut out);
        out
    }

    /// `a - b` for `a >= b`.
    fn mag_sub(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow = 0i64;
        for (i, ai) in a.iter().enumerate() {
            let mut d = *ai as i64 - borrow - *b.get(i).unwrap_or(&0) as i64;
            borrow = 0;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            }
            out.push(d as u32);
        }
        assert_eq!(borrow, 0);
        trim(&mut out);
        out
    }

    fn mag_mul(a: &[u32], b: &[u32]) -> Vec<u32> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0u64; a.len() + b.len() + 1];
        for (i, x) in a.iter().enumerate() {
            let mut carry = 0u64;
            for (j, y) in b.iter().enumerate() {
                let t = out[i + j] + (*x as u64) * (*y as u64) + carry;
                out[i + j] = t & 0xffff_ffff;
                carry = t >> 32;
            }
            let mut k = i + b.len();
            while carry > 0 {
                let t = out[k] + carry;
                out[k] = t & 0xffff_ffff;
                carry = t >> 32;
                k += 1;
            }
        }
        let mut v: Vec<u32> = out.into_iter().map(|x| x as u32).collect();
        trim(&mut v);
        v
    }

    fn mag_shl(a: &[u32], bits: u64) -> Vec<u32> {
        if a.is_empty() {
            return Vec::new();
        }
        let words = (bits / 32) as usize;
        let rem = (bits % 32) as u32;
        let mut out = vec![0u32; words];
        let mut carry = 0u32;
        for x in a {
            if rem == 0 {
                out.push(*x);
            } else {
                out.push((x << rem) | carry);
                carry = x >> (32 - rem);
            }
        }
        if carry > 0 {
            out.push(carry);
        }
        trim(&mut out);
        out
    }

    impl Dyadic {
        pub fn zero() -> Self {
            Self {
                neg: false,
                mag: Vec::new(),
                exp: 0,
            }
        }

        pub fn from_f64(x: f64) -> Self {
            assert!(x.is_finite(), "exact: non-finite input");
            if x == 0.0 {
                return Self::zero();
            }
            let bits = x.to_bits();
            let e = ((bits >> 52) & 0x7ff) as i64;
            let f = bits & ((1u64 << 52) - 1);
            let (m, exp) = if e == 0 {
                (f, -1074)
            } else {
                (f | (1u64 << 52), e - 1075)
            };
            let mut mag = vec![m as u32, (m >> 32) as u32];
            trim(&mut mag);
            Self {
                neg: x < 0.0,
                mag,
                exp,
            }
        }

        pub fn is_zero(&self) -> bool {
            self.mag.is_empty()
        }

        pub fn negate(&self) -> Self {
            Self {
                neg: !self.neg && !self.is_zero(),
                mag: self.mag.clone(),
                exp: self.exp,
            }
        }

        pub fn half(&self) -> Self {
            Self {
                neg: self.neg,
                mag: self.mag.clone(),
                exp: self.exp - 1,
            }
        }

        pub fn mul(&self, o: &Self) -> Self {
            let mag = mag_mul(&self.mag, &o.mag);
            if mag.is_empty() {
                return Self::zero();
            }
            Self {
                neg: self.neg != o.neg,
                mag,
                exp: self.exp + o.exp,
            }
        }

        pub fn add(&self, o: &Self) -> Self {
            if self.is_zero() {
                return o.clone();
            }
            if o.is_zero() {
                return self.clone();
            }
            let exp = self.exp.min(o.exp);
            let a = mag_shl(&self.mag, (self.exp - exp) as u64);
            let b = mag_shl(&o.mag, (o.exp - exp) as u64);
            if self.neg == o.neg {
                return Self {
                    neg: self.neg,
                    mag: mag_add(&a, &b),
                    exp,
                };
            }
            match mag_cmp(&a, &b) {
                Ordering::Equal => Self::zero(),
                Ordering::Greater => Self {
                    neg: self.neg,
                    mag: mag_sub(&a, &b),
                    exp,
                },
                Ordering::Less => Self {
                    neg: o.neg,
                    mag: mag_sub(&b, &a),
                    exp,
                },
            }
        }

        pub fn sub(&self, o: &Self) -> Self {
            self.add(&o.negate())
        }

        pub fn signum(&self) -> i32 {
            if self.is_zero() {
                0
            } else if self.neg {
                -1
            } else {
                1
            }
        }

        pub fn cmp(&self, o: &Self) -> Ordering {
            self.sub(o).signum().cmp(&0)
        }

        /// `(m, e)` with the value close to `m 2^e`, `m` a binary64 (an
        /// approximation used only to start the exact rounding search).
        fn approx(&self) -> (f64, i64) {
            let top = self.mag.len().saturating_sub(3);
            let mut m = 0.0;
            for (i, limb) in self.mag[top..].iter().enumerate() {
                m += (*limb as f64) * 2f64.powi(32 * i as i32);
            }
            let m = if self.neg { -m } else { m };
            (m, self.exp + 32 * top as i64)
        }
    }

    fn ldexp(mut m: f64, mut e: i64) -> f64 {
        while e > 600 {
            m *= 2f64.powi(600);
            e -= 600;
        }
        while e < -600 {
            m *= 2f64.powi(-600);
            e += 600;
        }
        m * 2f64.powi(e as i32)
    }

    /// Compare `p / q` with `c` exactly (`q != 0`).
    pub fn ratio_cmp(p: &Dyadic, q: &Dyadic, c: &Dyadic) -> Ordering {
        assert!(!q.is_zero());
        let (p, q) = if q.neg {
            (p.negate(), q.negate())
        } else {
            (p.clone(), q.clone())
        };
        p.cmp(&c.mul(&q))
    }

    /// `p / q` rounded to nearest binary64, ties to even (finite results).
    pub fn round_ratio(p: &Dyadic, q: &Dyadic) -> f64 {
        assert!(!q.is_zero());
        if p.is_zero() {
            return 0.0;
        }
        let (pm, pe) = p.approx();
        let (qm, qe) = q.approx();
        let mut c = ldexp(pm / qm, pe - qe);
        assert!(c.is_finite() && c != 0.0, "exact: quotient out of range");
        for _ in 0..10_000 {
            let here = Dyadic::from_f64(c);
            let lo = here.add(&Dyadic::from_f64(c.next_down())).half();
            let hi = here.add(&Dyadic::from_f64(c.next_up())).half();
            match ratio_cmp(p, q, &lo) {
                Ordering::Less => {
                    c = c.next_down();
                    continue;
                }
                Ordering::Equal => {
                    return if c.to_bits() & 1 == 0 {
                        c
                    } else {
                        c.next_down()
                    };
                }
                Ordering::Greater => {}
            }
            match ratio_cmp(p, q, &hi) {
                Ordering::Greater => {
                    c = c.next_up();
                    continue;
                }
                Ordering::Equal => {
                    return if c.to_bits() & 1 == 0 { c } else { c.next_up() };
                }
                Ordering::Less => return c,
            }
        }
        panic!("exact: rounding search did not converge");
    }

    pub fn d(x: f64) -> Dyadic {
        Dyadic::from_f64(x)
    }
}

use exact::{Dyadic, d, ratio_cmp, round_ratio};

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

fn hex_matrix(m: &[Vec<f64>]) -> Vec<Vec<String>> {
    m.iter().map(|row| hexes(row)).collect()
}

fn pair_hex(p: [f64; 2]) -> Vec<String> {
    vec![hex(p[0]), hex(p[1])]
}

// ------------------------------------------------------------------ bindings

#[derive(Clone, Debug)]
enum Operator {
    Represented(Vec<Vec<f64>>),
    Shifted {
        j: Vec<Vec<f64>>,
        h: f64,
        gamma: f64,
    },
}

#[derive(Clone, Debug)]
struct Binding {
    operator: Operator,
    metric: Vec<f64>,
    epoch: Option<u64>,
    assembly: AssemblyUncertainty,
    blocks: Option<Vec<(usize, usize)>>,
}

impl Binding {
    fn represented(w: Vec<Vec<f64>>, metric: Vec<f64>) -> Self {
        Self {
            operator: Operator::Represented(w),
            metric,
            epoch: EPOCH,
            assembly: AssemblyUncertainty::Exact,
            blocks: None,
        }
    }

    fn shifted(j: Vec<Vec<f64>>, h: f64, gamma: f64, metric: Vec<f64>) -> Self {
        Self {
            operator: Operator::Shifted { j, h, gamma },
            metric,
            epoch: EPOCH,
            assembly: AssemblyUncertainty::Exact,
            blocks: None,
        }
    }

    fn target(&self) -> Result<LinearTargetId, WitnessRejection> {
        let structure = match &self.blocks {
            None => DeclaredStructure::Dense,
            Some(b) => DeclaredStructure::IsolatedBlocks(b.iter().map(|(s, e)| *s..*e).collect()),
        };
        // The operator is validated before the metric (research order).
        match &self.operator {
            Operator::Represented(w) => {
                if !w.iter().flatten().all(|x| x.is_finite()) {
                    return Err(WitnessRejection::NonfiniteMatrix);
                }
                let metric = FrozenMetric::new(&self.metric)?;
                LinearTargetId::represented(w, metric, self.epoch, structure)
            }
            Operator::Shifted { j, h, gamma } => {
                if !j.iter().flatten().all(|x| x.is_finite()) {
                    return Err(WitnessRejection::NonfiniteMatrix);
                }
                let metric = FrozenMetric::new(&self.metric)?;
                LinearTargetId::shifted(j, *h, *gamma, metric, self.epoch, self.assembly, structure)
            }
        }
    }

    fn json(&self) -> Value {
        let operator = match &self.operator {
            Operator::Represented(w) => json!({"kind": "represented", "w_hex": hex_matrix(w)}),
            Operator::Shifted { j, h, gamma } => json!({
                "kind": "shifted", "j_hex": hex_matrix(j), "h_hex": hex(*h), "gamma_hex": hex(*gamma)
            }),
        };
        let assembly = match self.assembly {
            AssemblyUncertainty::Exact => json!({"kind": "exact"}),
            AssemblyUncertainty::EntrywiseAbs(delta) => {
                json!({"kind": "entrywise_abs", "delta_hex": hex(delta)})
            }
            AssemblyUncertainty::Undeclared => json!({"kind": "undeclared"}),
        };
        let structure = match &self.blocks {
            None => json!({"kind": "dense"}),
            Some(b) => json!({"kind": "isolated_blocks",
                              "ranges": b.iter().map(|(s, e)| vec![*s, *e]).collect::<Vec<_>>()}),
        };
        json!({
            "operator": operator,
            "metric_hex": hexes(&self.metric),
            "model_epoch": self.epoch,
            "assembly": assembly,
            "structure": structure,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Route {
    Represented,
    Shifted,
    Lognorm,
    ProjectedNu,
}

impl Route {
    fn name(self) -> &'static str {
        match self {
            Self::Represented => "represented",
            Self::Shifted => "shifted",
            Self::Lognorm => "lognorm",
            Self::ProjectedNu => "projected_nu",
        }
    }
}

fn run(route: Route, witness: &Binding, target: &Binding, rhs: &[f64], x: &[f64]) -> GainEvidence {
    let wt = match witness.target() {
        Ok(t) => t,
        Err(r) => return GainEvidence::Unavailable(r),
    };
    let tt = match target.target() {
        Ok(t) => t,
        Err(r) => return GainEvidence::Unavailable(r),
    };
    let built = match route {
        Route::Represented => ResidualGainWitness::represented(&wt),
        Route::Shifted => ResidualGainWitness::shifted(&wt),
        Route::Lognorm => ResidualGainWitness::lognorm(&wt),
        Route::ProjectedNu => {
            let mut start = vec![0.0; wt.dimension()];
            start[0] = 1.0;
            return projected_nu_estimate(&tt, rhs, x, &start, wt.dimension());
        }
    };
    match built {
        Ok(w) => w.certify(&tt, rhs, x),
        Err(r) => GainEvidence::Unavailable(r),
    }
}

fn verified_json(v: &VerifiedCorrection) -> Value {
    let pairs = |iv: Vec<rodas5p_core::directed::Interval>| -> Vec<Vec<String>> {
        iv.iter().map(|x| pair_hex([x.lo, x.hi])).collect()
    };
    let route = match v.route() {
        WitnessRoute::RepresentedDense2 => "represented_dense2",
        WitnessRoute::RepresentedBlocks => "represented_blocks",
        WitnessRoute::ShiftedEnclosure => "shifted_enclosure",
        WitnessRoute::Lognorm => "lognorm",
    };
    json!({
        "route": route,
        "residual_hex": pairs(v.residual()),
        "correction_hex": v.correction().map(pairs),
        "weighted_correction_hex": v.weighted_correction().map(pairs),
        "weighted_inf_upper_hex": hex(v.weighted_inf_upper()),
        "wrms_upper_hex": hex(v.wrms_upper()),
        "gain_hex": v.residual_gain_inf_upper().map(hex),
        "lognorm": v.lognorm_summary().map(|l| json!({
            "shift_hex": pair_hex(l.shift), "mu_up_hex": hex(l.mu_up), "mu_source": l.mu_source,
            "denominator_lower_hex": hex(l.denominator_lower)
        })),
        "setup_flops": v.setup_flops(),
        "verification_flops": v.verification_flops(),
        "transport_factor_hex": v.transport_factor().map(hex),
        "bound_operator_sha256": v.target().operator_sha256(),
        "bound_metric_hex": hexes(v.target().metric().scales()),
        "bound_model_epoch": v.target().model_epoch(),
    })
}

fn evidence_json(e: &GainEvidence) -> Value {
    match e {
        GainEvidence::Verified(v) => json!({
            "kind": "verified", "rejection": null, "estimate": null, "verified": verified_json(v)
        }),
        GainEvidence::Estimated(est) => json!({
            "kind": "estimated", "rejection": null,
            "estimate": {"source": format!("{:?}", est.source), "value_hex": hex(est.estimate)},
            "verified": null
        }),
        GainEvidence::Unavailable(r) => json!({
            "kind": "unavailable", "rejection": r.name(), "estimate": null, "verified": null
        }),
    }
}

fn comparator_json(target: &Binding, rhs: &[f64], x: &[f64]) -> Value {
    let n = rhs.len();
    match target
        .target()
        .and_then(|t| dense_lu_comparator(&t, rhs, x))
    {
        Ok(est) => json!({
            "source": format!("{:?}", est.source), "estimate_hex": hex(est.estimate),
            "nominal_flops": lu_comparator_flops(n), "rejection": null
        }),
        Err(r) => json!({
            "source": "DenseLuComparator", "estimate_hex": null,
            "nominal_flops": lu_comparator_flops(n), "rejection": r.name()
        }),
    }
}

struct Case {
    id: String,
    group: &'static str,
    route: Route,
    witness: Binding,
    target: Binding,
    rhs: Vec<f64>,
    candidate: Vec<f64>,
    evidence: GainEvidence,
    comparator: bool,
    research: Option<Value>,
    note: Option<Value>,
}

impl Case {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: impl Into<String>,
        group: &'static str,
        route: Route,
        witness: Binding,
        target: Binding,
        rhs: Vec<f64>,
        candidate: Vec<f64>,
    ) -> Self {
        let evidence = run(route, &witness, &target, &rhs, &candidate);
        Self {
            id: id.into(),
            group,
            route,
            witness,
            target,
            rhs,
            candidate,
            evidence,
            comparator: false,
            research: None,
            note: None,
        }
    }

    fn json(&self) -> Value {
        json!({
            "id": self.id,
            "group": self.group,
            "route": self.route.name(),
            "witness": self.witness.json(),
            "target": self.target.json(),
            "rhs_hex": hexes(&self.rhs),
            "candidate_hex": hexes(&self.candidate),
            "evidence": evidence_json(&self.evidence),
            "comparator": if self.comparator {
                comparator_json(&self.target, &self.rhs, &self.candidate)
            } else {
                Value::Null
            },
            "research": self.research.clone().unwrap_or(Value::Null),
            "note": self.note.clone().unwrap_or(Value::Null),
        })
    }
}

// ------------------------------------------------------- research reference

fn f(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().expect("finite number"),
        Value::String(s) if s == "NaN" => f64::NAN,
        Value::String(s) if s == "+Infinity" => f64::INFINITY,
        Value::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        other => panic!("not a number: {other}"),
    }
}

fn fv(v: &Value) -> Vec<f64> {
    v.as_array().expect("array").iter().map(f).collect()
}

fn fm(v: &Value) -> Vec<Vec<f64>> {
    v.as_array().expect("array").iter().map(fv).collect()
}

/// One cell of the unchanged research study.
struct StudyCell {
    id: String,
    w: Vec<Vec<f64>>,
    b: Vec<f64>,
    x: Vec<f64>,
    s: Vec<f64>,
    raw: Value,
}

fn study_cells(key: &str) -> Vec<StudyCell> {
    let study = gain_witness::study();
    study[key]
        .as_array()
        .expect("cells")
        .iter()
        .map(|c| StudyCell {
            id: c["id"].as_str().expect("id").to_string(),
            w: fm(&c["W"]),
            b: fv(&c["b"]),
            x: fv(&c["x"]),
            s: fv(&c["scales"]),
            raw: c.clone(),
        })
        .collect()
}

fn research_json(cell: &Value) -> Value {
    let status = cell["status"].as_str().expect("status");
    if status != "admitted" {
        return json!({"status": status, "error": cell["error"], "stage": cell["rejection_stage"]});
    }
    let cert = &cell["certificate"];
    let pairs = |key: &str| -> Vec<Vec<String>> {
        cert[key]
            .as_array()
            .expect("pairs")
            .iter()
            .map(|p| hexes(&fv(p)))
            .collect()
    };
    json!({
        "status": status, "error": null, "stage": null,
        "residual_hex": pairs("residual_intervals"),
        "correction_hex": pairs("correction_intervals"),
        "weighted_correction_hex": pairs("weighted_correction_intervals"),
        "weighted_inf_upper_hex": hex(f(&cert["weighted_inf_upper"])),
        "wrms_upper_hex": hex(f(&cert["wrms_upper"])),
        "gain_hex": hex(f(&cert["scaled_residual_gain_inf_upper"])),
    })
}

/// The research certificate of a represented 2x2 solve, read from its
/// `Debug` rendering (shortest round-trip decimals, parsed exactly).
struct ResearchEnclosure {
    status: String,
    correction: Vec<[f64; 2]>,
    weighted: Vec<[f64; 2]>,
    wrms_upper: f64,
}

fn debug_numbers(text: &str, key: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(key) {
        rest = &rest[pos + key.len()..];
        let end = rest.find([',', ' ', '}']).unwrap_or(rest.len());
        out.push(rest[..end].parse::<f64>().expect("debug float"));
        rest = &rest[end..];
    }
    out
}

fn research_enclosure(w: [[f64; 2]; 2], b: [f64; 2], x: [f64; 2]) -> ResearchEnclosure {
    let fail = |status: String| ResearchEnclosure {
        status,
        correction: Vec::new(),
        weighted: Vec::new(),
        wrms_upper: f64::NAN,
    };
    let witness = match gain_witness::GainWitness2::new(w, [1.0, 1.0]) {
        Ok(w) => w,
        Err(e) => return fail(format!("rejected:{e:?}")),
    };
    let cert = match witness.certify(b, x) {
        Ok(c) => c,
        Err(e) => return fail(format!("rejected:{e:?}")),
    };
    let text = format!("{cert:?}");
    let lo = debug_numbers(&text, "lo: ");
    let hi = debug_numbers(&text, "hi: ");
    assert_eq!(
        (lo.len(), hi.len()),
        (6, 6),
        "research Debug layout: {text}"
    );
    let pairs: Vec<[f64; 2]> = lo.iter().zip(&hi).map(|(a, b)| [*a, *b]).collect();
    let wrms = cert.wrms_upper();
    assert_eq!(
        debug_numbers(&text, "weighted_inf_upper: ")[0].to_bits(),
        wrms.to_bits()
    );
    ResearchEnclosure {
        status: "admitted".into(),
        correction: pairs[2..4].to_vec(),
        weighted: pairs[4..6].to_vec(),
        wrms_upper: wrms,
    }
}

// ------------------------------------------------------- W-formation family

struct Member {
    id: String,
    h: f64,
    k: f64,
    gamma: f64,
    lambda: f64,
    j: Vec<Vec<f64>>,
    w_rep: Vec<Vec<f64>>,
    rhs: Vec<f64>,
    candidate: Vec<f64>,
    research: ResearchEnclosure,
    discriminates: bool,
}

fn gamma() -> f64 {
    rodas5p_coefficients().expect("coefficients").gamma
}

/// Exact solution of `W u = b` (2x2, exact dyadic entries) as ratios.
fn exact_solve2(w: &[[Dyadic; 2]; 2], b: &[Dyadic; 2]) -> ([Dyadic; 2], Dyadic) {
    let det = w[0][0].mul(&w[1][1]).sub(&w[0][1].mul(&w[1][0]));
    let u0 = w[1][1].mul(&b[0]).sub(&w[0][1].mul(&b[1]));
    let u1 = w[0][0].mul(&b[1]).sub(&w[1][0].mul(&b[0]));
    ([u0, u1], det)
}

fn w_formation_family() -> Vec<Member> {
    let g = gamma();
    let mut out = Vec::new();
    for h in H_LIST {
        for k in K_LIST {
            let hg = h * g;
            let lambda = (1.0 - 2f64.powi(-40)) / hg;
            let j = vec![vec![lambda, k], vec![0.0, lambda]];
            let w_rep: Vec<Vec<f64>> = (0..2)
                .map(|r| {
                    (0..2)
                        .map(|c| f64::from(u8::from(r == c)) - hg * j[r][c])
                        .collect()
                })
                .collect();
            let rhs = vec![1.0, 1.0];
            // Candidate: the exact solution of the represented W, rounded.
            let wd = [
                [d(w_rep[0][0]), d(w_rep[0][1])],
                [d(w_rep[1][0]), d(w_rep[1][1])],
            ];
            let (u, det) = exact_solve2(&wd, &[d(1.0), d(1.0)]);
            let candidate = vec![round_ratio(&u[0], &det), round_ratio(&u[1], &det)];
            let research = research_enclosure(
                [[w_rep[0][0], w_rep[0][1]], [w_rep[1][0], w_rep[1][1]]],
                [1.0, 1.0],
                [candidate[0], candidate[1]],
            );
            // Exact shifted W = I - h gamma J from the exact rationals.
            let hgx = d(h).mul(&d(g));
            let ws = [
                [
                    d(1.0).sub(&hgx.mul(&d(j[0][0]))),
                    d(0.0).sub(&hgx.mul(&d(j[0][1]))),
                ],
                [
                    d(0.0).sub(&hgx.mul(&d(j[1][0]))),
                    d(1.0).sub(&hgx.mul(&d(j[1][1]))),
                ],
            ];
            let (us, dets) = exact_solve2(&ws, &[d(1.0), d(1.0)]);
            // Discriminates iff u*_i - x_i lies outside the research weighted
            // enclosure (unit metric) in some component; a research rejection
            // gives no enclosure and does not discriminate.
            let discriminates = research.status == "admitted"
                && (0..2).any(|i| {
                    let lo = d(candidate[i]).add(&d(research.weighted[i][0]));
                    let hi = d(candidate[i]).add(&d(research.weighted[i][1]));
                    ratio_cmp(&us[i], &dets, &lo).is_lt() || ratio_cmp(&us[i], &dets, &hi).is_gt()
                });
            out.push(Member {
                id: format!("wf_h{h}_k{k}"),
                h,
                k,
                gamma: g,
                lambda,
                j,
                w_rep,
                rhs,
                candidate,
                research,
                discriminates,
            });
        }
    }
    out
}

fn member_json(m: &Member, fixture: bool) -> Value {
    json!({
        "id": m.id, "h_hex": hex(m.h), "k_hex": hex(m.k), "gamma_hex": hex(m.gamma),
        "lambda_hex": hex(m.lambda), "j_hex": hex_matrix(&m.j), "w_rep_hex": hex_matrix(&m.w_rep),
        "rhs_hex": hexes(&m.rhs), "candidate_hex": hexes(&m.candidate),
        "research": {
            "status": m.research.status,
            "correction_hex": m.research.correction.iter().map(|p| pair_hex(*p)).collect::<Vec<_>>(),
            "weighted_correction_hex": m.research.weighted.iter().map(|p| pair_hex(*p)).collect::<Vec<_>>(),
            "wrms_upper_hex": if m.research.status == "admitted" { json!(hex(m.research.wrms_upper)) } else { Value::Null },
        },
        "discriminates": m.discriminates,
        "fixture": fixture,
    })
}

// ------------------------------------------------------------------ fixtures

/// One block: `(W, b, x, s)`.
type Part = (Vec<Vec<f64>>, Vec<f64>, Vec<f64>, Vec<f64>);

fn block_diag(cells: &[Part]) -> StudyCell {
    let n: usize = cells.iter().map(|c| c.1.len()).sum();
    let mut w = vec![vec![0.0; n]; n];
    let (mut b, mut x, mut s) = (Vec::new(), Vec::new(), Vec::new());
    let mut at = 0;
    for (cw, cb, cx, cs) in cells {
        for (i, row) in cw.iter().enumerate() {
            for (k, v) in row.iter().enumerate() {
                w[at + i][at + k] = *v;
            }
        }
        b.extend(cb);
        x.extend(cx);
        s.extend(cs);
        at += cb.len();
    }
    StudyCell {
        id: String::new(),
        w,
        b,
        x,
        s,
        raw: Value::Null,
    }
}

fn blocks_of(sizes: &[usize]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    for s in sizes {
        out.push((at, at + s));
        at += s;
    }
    out
}

/// `-100 tridiag(-1, 2, -1)` plus first-order upwind convection with cell
/// Peclet `pe`: diagonal `-200 - 100 pe`, sub-diagonal `100 + 100 pe`,
/// super-diagonal `100`.
fn lognorm_j(n: usize, pe: f64) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|k| {
                    if i == k {
                        -200.0 - 100.0 * pe
                    } else if k + 1 == i {
                        100.0 + 100.0 * pe
                    } else if k == i + 1 {
                        100.0
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

/// `s_k = 1 + |sin k|`, `k = 1..n`.
fn lognorm_metric(n: usize) -> Vec<f64> {
    (1..=n).map(|k| 1.0 + (k as f64).sin().abs()).collect()
}

/// The binary64 LU solution of `fl(I - fl(h gamma) J) u = b`, then
/// `u_1 += 2^-30` (first coordinate).
fn lognorm_candidate(j: &[Vec<f64>], h: f64, gamma: f64, b: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let hg = h * gamma;
    let n = j.len();
    let w: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|k| f64::from(u8::from(i == k)) - hg * j[i][k])
                .collect()
        })
        .collect();
    let lu = LuFactorization::new(&DenseMatrix::from_vec_rows(w).unwrap()).unwrap();
    let base = lu.solve(b).unwrap();
    let mut x = base.clone();
    x[0] += 2f64.powi(-30);
    (base, x)
}

fn metric_variants(s: &[f64]) -> [(&'static str, Vec<f64>); 2] {
    [
        ("double_half", vec![2.0 * s[0], s[1] / 2.0]),
        ("next_up", vec![s[0].next_up(), s[1]]),
    ]
}

// --------------------------------------------------------------- the cases

struct Study {
    family: Vec<Member>,
    fixtures: Vec<usize>,
    cases: Vec<Case>,
}

fn build() -> Study {
    let mut cases = Vec::new();
    let cells = study_cells("cells");
    let invalid = study_cells("invalid_controls");
    let discovery: Vec<&StudyCell> = cells
        .iter()
        .filter(|c| c.raw["partition"] == "discovery")
        .collect();

    // A. Retained oracle, route (a).
    let mut base_bindings = Vec::new();
    for cell in &cells {
        let b = Binding::represented(cell.w.clone(), cell.s.clone());
        let mut case = Case::new(
            cell.id.clone(),
            "A_cell",
            Route::Represented,
            b.clone(),
            b.clone(),
            cell.b.clone(),
            cell.x.clone(),
        );
        case.research = Some(research_json(&cell.raw));
        base_bindings.push((cell, b, case.evidence.clone()));
        cases.push(case);
    }
    for cell in &invalid {
        let b = Binding::represented(cell.w.clone(), cell.s.clone());
        let mut case = Case::new(
            cell.id.clone(),
            "A_invalid",
            Route::Represented,
            b.clone(),
            b,
            cell.b.clone(),
            cell.x.clone(),
        );
        case.research = Some(research_json(&cell.raw));
        cases.push(case);
    }

    // C. Metric change and stale epoch on the 30 cells; D. transport.
    for (cell, b, evidence) in &base_bindings {
        for (tag, metric) in metric_variants(&cell.s) {
            let mut t = b.clone();
            t.metric = metric.clone();
            cases.push(Case::new(
                format!("{}_metric_{tag}", cell.id),
                "C_metric",
                Route::Represented,
                b.clone(),
                t,
                cell.b.clone(),
                cell.x.clone(),
            ));
        }
        for (tag, epoch) in [("8", Some(8)), ("none", None)] {
            let mut t = b.clone();
            t.epoch = epoch;
            cases.push(Case::new(
                format!("{}_epoch_{tag}", cell.id),
                "C_epoch",
                Route::Represented,
                b.clone(),
                t,
                cell.b.clone(),
                cell.x.clone(),
            ));
        }
        for (tag, metric) in metric_variants(&cell.s) {
            let mut t = b.clone();
            t.metric = metric.clone();
            let transported = match evidence {
                GainEvidence::Verified(v) => match FrozenMetric::new(&metric) {
                    Ok(m) => match v.transport_to(&m) {
                        Ok(tv) => GainEvidence::Verified(tv),
                        Err(r) => GainEvidence::Unavailable(r),
                    },
                    Err(r) => GainEvidence::Unavailable(r),
                },
                other => other.clone(),
            };
            cases.push(Case {
                id: format!("{}_transport_{tag}", cell.id),
                group: "D_transport",
                route: Route::Represented,
                witness: b.clone(),
                target: t,
                rhs: cell.b.clone(),
                candidate: cell.x.clone(),
                evidence: transported,
                comparator: true,
                research: None,
                note: None,
            });
        }
    }

    // W-formation family and its first four discriminating members.
    let family = w_formation_family();
    let fixtures: Vec<usize> = (0..family.len())
        .filter(|i| family[*i].discriminates)
        .take(4)
        .collect();
    for &i in &fixtures {
        let m = &family[i];
        let unit = vec![1.0, 1.0];
        let represented = Binding::represented(m.w_rep.clone(), unit.clone());
        let shifted = Binding::shifted(m.j.clone(), m.h, m.gamma, unit.clone());
        cases.push(Case::new(
            format!("{}_target_mismatch", m.id),
            "C_w_formation",
            Route::Represented,
            represented,
            shifted.clone(),
            m.rhs.clone(),
            m.candidate.clone(),
        ));
        let mut undeclared = shifted.clone();
        undeclared.assembly = AssemblyUncertainty::Undeclared;
        cases.push(Case::new(
            format!("{}_assembly_undeclared", m.id),
            "C_assembly",
            Route::Shifted,
            undeclared.clone(),
            undeclared,
            m.rhs.clone(),
            m.candidate.clone(),
        ));
        let changes: [Change; 6] = [
            ("j00", |b| bump_j(b, 0, 0)),
            ("j01", |b| bump_j(b, 0, 1)),
            ("j10", |b| bump_j(b, 1, 0)),
            ("j11", |b| bump_j(b, 1, 1)),
            ("h", bump_h),
            ("gamma", bump_gamma),
        ];
        for (tag, change) in changes {
            let mut t = shifted.clone();
            change(&mut t);
            cases.push(Case::new(
                format!("{}_operator_{tag}_ulp", m.id),
                "C_operator",
                Route::Shifted,
                shifted.clone(),
                t,
                m.rhs.clone(),
                m.candidate.clone(),
            ));
        }
        let mut case = Case::new(
            format!("{}_exact", m.id),
            "D_shifted_exact",
            Route::Shifted,
            shifted.clone(),
            shifted.clone(),
            m.rhs.clone(),
            m.candidate.clone(),
        );
        case.comparator = true;
        cases.push(case);
        let max_j = m.j.iter().flatten().fold(0.0_f64, |a, x| a.max(x.abs()));
        let mut delta = shifted.clone();
        delta.assembly = AssemblyUncertainty::EntrywiseAbs(2f64.powi(-60) * max_j);
        let mut case = Case::new(
            format!("{}_delta", m.id),
            "D_shifted_delta",
            Route::Shifted,
            delta.clone(),
            delta,
            m.rhs.clone(),
            m.candidate.clone(),
        );
        case.comparator = true;
        cases.push(case);
    }

    // Off-block coupling: blocks {0,1}, {2,3} from discovery cells 1-2.
    let pair = |a: &StudyCell, b: &StudyCell| {
        block_diag(&[
            (a.w.clone(), a.b.clone(), a.x.clone(), a.s.clone()),
            (b.w.clone(), b.b.clone(), b.x.clone(), b.s.clone()),
        ])
    };
    let base4 = pair(discovery[0], discovery[1]);
    let off_block = [
        (0, 2),
        (0, 3),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (3, 0),
        (3, 1),
    ];
    for (r, c) in off_block {
        for (tag, value) in [
            ("min_subnormal", f64::from_bits(1)),
            ("1e-300", 1.0e-300),
            ("one", 1.0),
        ] {
            let mut w = base4.w.clone();
            w[r][c] = value;
            let mut b = Binding::represented(w, base4.s.clone());
            b.blocks = Some(blocks_of(&[2, 2]));
            cases.push(Case::new(
                format!("offblock_r{r}c{c}_{tag}"),
                "C_offblock",
                Route::Represented,
                b.clone(),
                b,
                base4.b.clone(),
                base4.x.clone(),
            ));
        }
    }
    let dense4 = Binding::represented(base4.w.clone(), base4.s.clone());
    cases.push(Case::new(
        "dense4_route_a",
        "C_offblock",
        Route::Represented,
        dense4.clone(),
        dense4,
        base4.b.clone(),
        base4.x.clone(),
    ));

    // Projected nu (finding F105): r = e2.
    for k in [1024.0, 1048576.0] {
        let b = Binding::represented(vec![vec![1.0, k], vec![0.0, 1.0]], vec![1.0, 1.0]);
        cases.push(Case::new(
            format!("nu_k{k}"),
            "C_nu",
            Route::ProjectedNu,
            b.clone(),
            b,
            vec![0.0, 1.0],
            vec![0.0, 0.0],
        ));
    }

    // D. Route (b): block-diagonal from consecutive discovery cells.
    for count in [2usize, 3, 4] {
        let parts: Vec<_> = discovery[..count]
            .iter()
            .map(|c| (c.w.clone(), c.b.clone(), c.x.clone(), c.s.clone()))
            .collect();
        let bd = block_diag(&parts);
        let mut b = Binding::represented(bd.w.clone(), bd.s.clone());
        b.blocks = Some(blocks_of(&vec![2; count]));
        let mut case = Case::new(
            format!("blocks_n{}", 2 * count),
            "D_blocks",
            Route::Represented,
            b.clone(),
            b,
            bd.b.clone(),
            bd.x.clone(),
        );
        case.comparator = true;
        cases.push(case);
    }
    {
        let c = discovery[0];
        let bd = block_diag(&[
            (c.w.clone(), c.b.clone(), c.x.clone(), c.s.clone()),
            (vec![vec![3.0]], vec![1.0], vec![1.0 / 3.0], vec![1.0]),
        ]);
        let mut b = Binding::represented(bd.w.clone(), bd.s.clone());
        b.blocks = Some(blocks_of(&[2, 1]));
        let mut case = Case::new(
            "blocks_n3",
            "D_blocks",
            Route::Represented,
            b.clone(),
            b,
            bd.b.clone(),
            bd.x.clone(),
        );
        case.comparator = true;
        cases.push(case);
    }

    // D. Route (d).
    for n in [3usize, 4, 8] {
        for pe in [0.0, 10.0] {
            for a in [1.0e-3, 1.0e-1] {
                let j = lognorm_j(n, pe);
                let rhs = vec![1.0; n];
                let (base, x) = lognorm_candidate(&j, a, 1.0, &rhs);
                let b = Binding::shifted(j, a, 1.0, lognorm_metric(n));
                let mut case = Case::new(
                    format!("lognorm_n{n}_pe{pe}_a{a:e}"),
                    "D_lognorm",
                    Route::Lognorm,
                    b.clone(),
                    b,
                    rhs,
                    x,
                );
                case.comparator = true;
                case.note = Some(json!({"lu_base_hex": hexes(&base)}));
                cases.push(case);
            }
        }
    }
    for (id, j, h) in [
        ("lognorm_negative_shift", lognorm_j(3, 0.0), -1.0e-3),
        (
            "lognorm_nonpositive_denominator",
            (0..3)
                .map(|i| (0..3).map(|k| if i == k { 10.0 } else { 0.0 }).collect())
                .collect(),
            1.0,
        ),
    ] {
        let rhs = vec![1.0; 3];
        let (base, x) = lognorm_candidate(&j, h, 1.0, &rhs);
        let b = Binding::shifted(j, h, 1.0, lognorm_metric(3));
        let mut case = Case::new(
            id,
            "D_lognorm_negative",
            Route::Lognorm,
            b.clone(),
            b,
            rhs,
            x,
        );
        case.comparator = true;
        case.note = Some(json!({"lu_base_hex": hexes(&base)}));
        cases.push(case);
    }

    Study {
        family,
        fixtures,
        cases,
    }
}

/// A one-ulp operator change.
type Change = (&'static str, fn(&mut Binding));

fn bump_j(b: &mut Binding, r: usize, c: usize) {
    if let Operator::Shifted { j, .. } = &mut b.operator {
        j[r][c] = j[r][c].next_up();
    }
}

fn bump_h(b: &mut Binding) {
    if let Operator::Shifted { h, .. } = &mut b.operator {
        *h = h.next_up();
    }
}

fn bump_gamma(b: &mut Binding) {
    if let Operator::Shifted { gamma, .. } = &mut b.operator {
        *gamma = gamma.next_up();
    }
}

fn git(args: &[&str]) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).expect("utf-8")
}

// -------------------------------------------------------------------- tests

fn case<'a>(study: &'a Study, id: &str) -> &'a Case {
    study
        .cases
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("case {id}"))
}

fn contains(outer: &[f64], inner: [f64; 2]) -> bool {
    outer[0] <= inner[0] && inner[1] <= outer[1]
}

#[test]
fn research_reference_is_the_pinned_file() {
    assert_eq!(sha256_hex(GAIN_WITNESS_SOURCE), GAIN_WITNESS_SHA256);
}

#[test]
fn exact_rounding_matches_correctly_rounded_division() {
    for (p, q) in [
        (1.0, 3.0),
        (2.0, 7.0),
        (-5.0, 9.0),
        (1.0, 1.0e-300),
        (1e300, 3.0),
    ] {
        assert_eq!(round_ratio(&d(p), &d(q)).to_bits(), (p / q).to_bits());
    }
    // A product of three binary64 numbers divided by a product of two.
    let (a, b, c) = (0.1, 0.7, 1.0 / 3.0);
    let p = d(a).mul(&d(b)).mul(&d(c));
    let q = d(b).mul(&d(c));
    assert_eq!(round_ratio(&p, &q), a);
}

#[test]
fn retained_cells_are_verified_inside_the_research_intervals() {
    let study = build();
    let mut verified = 0;
    for c in study.cases.iter().filter(|c| c.group == "A_cell") {
        let GainEvidence::Verified(v) = &c.evidence else {
            panic!("{} not verified: {:?}", c.id, c.evidence)
        };
        verified += 1;
        let research = gain_witness::study();
        let raw = research["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == c.id.as_str())
            .unwrap();
        let cert = &raw["certificate"];
        for (key, core) in [
            ("residual_intervals", v.residual()),
            ("correction_intervals", v.correction().unwrap()),
            (
                "weighted_correction_intervals",
                v.weighted_correction().unwrap(),
            ),
        ] {
            for (outer, inner) in cert[key].as_array().unwrap().iter().zip(core) {
                assert!(contains(&fv(outer), [inner.lo, inner.hi]), "{} {key}", c.id);
            }
        }
        assert!(v.wrms_upper() <= f(&cert["wrms_upper"]));
        assert!(v.residual_gain_inf_upper().unwrap() <= f(&cert["scaled_residual_gain_inf_upper"]));
    }
    assert_eq!(verified, 30);
}

#[test]
fn invalid_controls_map_to_the_research_classes() {
    let study = build();
    let expected = [
        ("invalid_singular", Some("DeterminantContainsZero")),
        ("invalid_nonfinite_w", Some("NonfiniteMatrix")),
        ("invalid_nonfinite_b", Some("NonfiniteRhs")),
        ("invalid_nonfinite_x", Some("NonfiniteCandidate")),
        ("invalid_zero_scale", Some("InvalidMetric")),
        ("invalid_negative_scale", Some("InvalidMetric")),
        ("invalid_infinite_scale", Some("InvalidMetric")),
        ("invalid_nan_scale", Some("InvalidMetric")),
        ("invalid_determinant_overlap", None),
        ("invalid_determinant_overflow", Some("ArithmeticOverflow")),
        ("invalid_residual_overflow", Some("ArithmeticOverflow")),
        ("invalid_scale_gain_overflow", Some("ArithmeticOverflow")),
    ];
    for (id, class) in expected {
        let e = &case(&study, id).evidence;
        match class {
            Some(name) => match e {
                GainEvidence::Unavailable(r) => assert_eq!(r.name(), name, "{id}"),
                other => panic!("{id}: {other:?}"),
            },
            None => assert!(!matches!(e, GainEvidence::Estimated(_)), "{id}"),
        }
    }
}

#[test]
fn every_new_negative_is_refused_with_its_class() {
    let study = build();
    assert_eq!(study.fixtures.len(), 4);
    let classes = [
        ("C_w_formation", "TargetMismatch", 4),
        ("C_metric", "MetricMismatch", 60),
        ("C_epoch", "StaleEpoch", 60),
        ("C_operator", "OperatorMismatch", 24),
    ];
    for (group, class, count) in classes {
        let cases: Vec<_> = study.cases.iter().filter(|c| c.group == group).collect();
        assert_eq!(cases.len(), count, "{group}");
        for c in cases {
            match &c.evidence {
                GainEvidence::Unavailable(r) => assert_eq!(r.name(), class, "{}", c.id),
                other => panic!("{}: {other:?}", c.id),
            }
        }
    }
    for c in study.cases.iter().filter(|c| c.group == "C_offblock") {
        let class = if c.id == "dense4_route_a" {
            "UndeclaredStructure"
        } else {
            "OffBlockCoupling"
        };
        match &c.evidence {
            GainEvidence::Unavailable(r) => assert_eq!(r.name(), class, "{}", c.id),
            other => panic!("{}: {other:?}", c.id),
        }
    }
    for group in ["C_assembly", "C_nu"] {
        for c in study.cases.iter().filter(|c| c.group == group) {
            assert!(matches!(c.evidence, GainEvidence::Estimated(_)), "{}", c.id);
        }
    }
    let neg = &case(&study, "lognorm_negative_shift").evidence;
    assert!(matches!(
        neg,
        GainEvidence::Unavailable(WitnessRejection::NegativeShift)
    ));
    let den = &case(&study, "lognorm_nonpositive_denominator").evidence;
    assert!(matches!(
        den,
        GainEvidence::Unavailable(WitnessRejection::NonpositiveDenominator)
    ));
}

#[test]
fn positives_and_routes() {
    let study = build();
    // Route (d) may refuse a registered positive (a verified mu_up too large
    // for the shift); that is reported, never a certificate.
    for c in study.cases.iter().filter(|c| c.group == "D_lognorm") {
        assert!(
            matches!(
                c.evidence,
                GainEvidence::Verified(_)
                    | GainEvidence::Unavailable(WitnessRejection::NonpositiveDenominator)
            ),
            "{}: {:?}",
            c.id,
            c.evidence
        );
    }
    for group in ["D_shifted_exact", "D_transport", "D_blocks"] {
        for c in study.cases.iter().filter(|c| c.group == group) {
            assert!(
                matches!(c.evidence, GainEvidence::Verified(_)),
                "{}: {:?}",
                c.id,
                c.evidence
            );
        }
    }
    // Transport multiplies the bound by kappa = max s_old / s_new.
    let base = &case(&study, "control_symmetric").evidence;
    let moved = &case(&study, "control_symmetric_transport_double_half").evidence;
    if let (GainEvidence::Verified(a), GainEvidence::Verified(b)) = (base, moved) {
        assert!(b.wrms_upper() >= 2.0 * a.wrms_upper());
        assert_eq!(b.target().metric().scales(), &[2.0, 0.5]);
    } else {
        panic!("transport");
    }
    let t = case(&study, "control_symmetric").witness.target().unwrap();
    assert!(matches!(
        certify_by_approximate_inverse_defect(&t, &[1.0, 1.0], &[0.2, 0.4]),
        GainEvidence::Unavailable(WitnessRejection::RouteNotImplemented)
    ));
    // A represented witness cannot be built from a shifted target and back.
    let m = &study.family[study.fixtures[0]];
    let shifted = Binding::shifted(m.j.clone(), m.h, m.gamma, vec![1.0, 1.0])
        .target()
        .unwrap();
    assert_eq!(
        ResidualGainWitness::represented(&shifted).unwrap_err(),
        WitnessRejection::TargetMismatch
    );
}

/// Writes RUNS.json (registered command 3); set AS05_RUNS to the path.
#[test]
#[ignore]
fn export_runs() {
    // A relative path is taken from the repository root (cargo runs the test
    // binary in the package directory).
    let given = std::path::PathBuf::from(std::env::var("AS05_RUNS").expect("set AS05_RUNS"));
    let path = if given.is_absolute() {
        given
    } else {
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(given)
    };
    let commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let tree_status = git(&["status", "--porcelain"]);
    let study = build();
    let g = gamma();
    let doc = json!({
        "schema": "vigilode-as05-residual-gain-runs-v1",
        "source": {"commit": commit, "tree_status": tree_status},
        "gain_witness_sha256": sha256_hex(GAIN_WITNESS_SOURCE),
        "gamma_hex": hex(g),
        "model_epoch": EPOCH,
        "w_formation_family": study.family.iter().enumerate()
            .map(|(i, m)| member_json(m, study.fixtures.contains(&i)))
            .collect::<Vec<_>>(),
        "fixtures": study.fixtures.iter().map(|i| study.family[*i].id.clone()).collect::<Vec<_>>(),
        "cases": study.cases.iter().map(Case::json).collect::<Vec<_>>(),
    });
    let text = serde_json::to_string(&doc).expect("json") + "\n";
    std::fs::write(&path, text).expect("write RUNS.json");
    println!(
        "wrote {}: {} cases at {commit}",
        path.display(),
        study.cases.len()
    );
}
