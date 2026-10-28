//! Deterministic bit-exact inputs and interval outputs for an independent
//! Fraction-from-bits SYM02 oracle. Small scalar fixtures, no timing claims.
use rodas5p_core::{
    CoreError, DenseMatrix, RODAS5P_COEFFICIENT_SNAPSHOT_SHA256, directed::Interval,
    rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, FastInteriorControl, OdeProblem,
    affine_event_certificate::{
        AFFINE_EVENT_CLOCK_ID, AffineEventBinding, AffineEventCertificate, AffineEventQuery,
        AffineEventSpec, AffineScalarModel, RepresentedAffineSegment,
    },
    integrate_rodas5p_fast_dense_observed,
};
use serde_json::{Value, json};
use std::sync::Arc;

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn iv(x: Interval) -> Value {
    json!([bits(x.lo), bits(x.hi)])
}
fn spec(name: &str, lambda: [f64; 2], b: [f64; 2], threshold: f64) -> AffineEventSpec {
    AffineEventSpec::new(
        AffineEventBinding::new(
            "declared-scalar-affine-family",
            name,
            AFFINE_EVENT_CLOCK_ID,
            "dimensionless-state;unit-time",
            "x-threshold",
            0,
        )
        .unwrap(),
        AffineScalarModel::new(
            Interval::new(lambda[0], lambda[1]).unwrap(),
            Interval::new(b[0], b[1]).unwrap(),
        )
        .unwrap(),
        threshold,
    )
    .unwrap()
}
fn query_times(t: [f64; 2]) -> Vec<f64> {
    let mut result = vec![t[0]];
    for q in [0.125, 0.25, 0.5, 0.75, 0.875] {
        let time = t[0] + (t[1] - t[0]) * q;
        if t[0] < time && time < t[1] && result.last().unwrap().to_bits() != time.to_bits() {
            result.push(time);
        }
    }
    result.push(t[1]);
    result
}
fn query_json(q: AffineEventQuery) -> Value {
    json!({"time":bits(q.time),"actual_center":bits(q.actual_center),
        "represented_polynomial":iv(q.represented_polynomial),
        "evaluation_rounding_upper":bits(q.evaluation_rounding_upper),
        "error_upper":bits(q.error_upper),"state_enclosure":iv(q.state_enclosure),
        "symbol":format!("{:?}",q.symbol)})
}
#[allow(clippy::too_many_arguments)]
fn record(
    name: &str,
    c: &AffineEventCertificate,
    s: &AffineEventSpec,
    t: [f64; 2],
    y: [f64; 2],
    h: [[f64; 8]; 3],
    u: [f64; 8],
    queries: Vec<Value>,
) -> Value {
    json!({"case":name,"origin":format!("{:?}",c.origin()),"segment_index":c.segment_index(),
        "source_id":s.binding().source_id(),"model_id":s.binding().model_id(),
        "clock_id":s.binding().clock_id(),"unit_id":s.binding().unit_id(),
        "event_id":s.binding().event_id(),"epoch":s.binding().epoch(),
        "times":t.map(bits),"states":y.map(bits),"dense_h":h.map(|r|r.map(bits)),
        "stages":u.map(bits),"lambda":iv(s.model().lambda()),"b":iv(s.model().forcing()),
        "threshold":bits(s.threshold()),"initial_error":bits(c.initial_error()),
        "clock_width":iv(c.clock_width()),"power":c.power_coefficients().map(iv),
        "residual_bernstein":c.residual_bernstein().map(iv),
        "defect_upper":bits(c.defect_upper()),"endpoint_error_upper":bits(c.endpoint_error_upper()),
        "whole_slab_error_upper":bits(c.whole_slab_error_upper()),
        "polynomial_range":iv(c.polynomial_range()),"state_tube":iv(c.state_tube()),
        "flow_range":iv(c.flow_range()),"endpoint_symbols":c.endpoint_symbols().map(|v|format!("{v:?}")),
        "slab_symbol":format!("{:?}",c.slab_symbol()),"flow_direction":format!("{:?}",c.flow_direction()),
        "event_status":format!("{:?}",c.event_status()),"queries":queries})
}
// This is an explicitly identified source-expression reproduction for
// declared packets. Native records below call the actual native evaluator.
fn reproduced_center(t: [f64; 2], y: [f64; 2], h: [[f64; 8]; 3], u: [f64; 8], time: f64) -> f64 {
    if time == t[0] {
        return y[0];
    }
    if time == t[1] {
        return y[1];
    }
    let theta = (time - t[0]) / (t[1] - t[0]);
    if theta == 0.0 {
        return y[0];
    }
    let mut d = [0.0; 3];
    for r in 0..3 {
        for j in 0..8 {
            d[r] += h[r][j] * u[j];
        }
    }
    let complement = 1.0 - theta;
    complement * y[0] + theta * (y[1] + complement * (d[0] + theta * (d[1] + theta * d[2])))
}
#[allow(clippy::too_many_arguments)]
fn declared(
    name: &str,
    t: [f64; 2],
    y: [f64; 2],
    h: [[f64; 8]; 3],
    u: [f64; 8],
    s: &AffineEventSpec,
    e0: f64,
) -> Result<Value, CoreError> {
    let c = AffineEventCertificate::certify(RepresentedAffineSegment::new(t, y, h, u, 0)?, s, e0)?;
    let queries = query_times(t)
        .into_iter()
        .map(|time| {
            c.query(s, time, reproduced_center(t, y, h, u, time))
                .map(query_json)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = record(name, &c, s, t, y, h, u, queries);
    output["center_origin"] = json!("identified-source-expression-reproduction;declared-packet");
    Ok(output)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    let mut identity_h = [[0.0; 8]; 3];
    for (r, row) in identity_h.iter_mut().enumerate() {
        row[r] = 1.0;
    }
    records.push(declared(
        "quartic",
        [0.0, 0.125],
        [1.0, 113.0 / 128.0],
        identity_h,
        [-1.0 / 128.0, 0.0, 1.0 / 256.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &spec("declared-quartic", [1.0; 2], [0.0; 2], 15.0 / 16.0),
        0.0,
    )?);
    let mut cancellation_h = [[0.0; 8]; 3];
    cancellation_h[0][..3].copy_from_slice(&[1.0e16, 1.0, -1.0e16]);
    records.push(declared(
        "cancellation-and-real-clock",
        [-1.0e16, 1.0],
        [0.0, 0.0],
        cancellation_h,
        [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &spec("declared-cancellation", [1.0e-16; 2], [0.0; 2], 0.125),
        0.0,
    )?);
    let tiny = f64::from_bits(1);
    records.push(declared(
        "subnormal",
        [0.0, 1.0],
        [tiny, tiny],
        [[0.0; 8]; 3],
        [0.0; 8],
        &spec("declared-subnormal", [1.0; 2], [tiny; 2], 0.0),
        0.0,
    )?);
    records.push(declared(
        "parameter-box-and-incoming-error",
        [0.0, 0.125],
        [1.0, 113.0 / 128.0],
        identity_h,
        [-1.0 / 128.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &spec(
            "declared-parameter-box",
            [0.9, 1.1],
            [-0.01, 0.01],
            15.0 / 16.0,
        ),
        0.01,
    )?);

    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -y[0];
        Ok(())
    });
    let jacobian = Arc::new(|_t: f64, _y: &[f64]| {
        let mut j = DenseMatrix::zeros(1, 1);
        j[(0, 0)] = -1.0;
        Ok(j)
    });
    let problem = OdeProblem::new(
        "SYM02-native-evidence-xprime=-x",
        1,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )?;
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.0625,
        min_step: 1.0e-10,
        max_step: 0.0625,
        max_attempts: 128,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_rodas5p_fast_dense_observed(
        &problem,
        (0.0, 0.125),
        &[1.0],
        &config,
        FastInteriorControl::Off,
    )?;
    if result.partial.is_some() || result.segments.len() > 16 {
        return Err(CoreError::InvalidInput(
            "SYM02 bounded native fixture did not finish within16 steps".into(),
        )
        .into());
    }
    let coefficients = rodas5p_coefficients()?;
    let mut native_h = [[0.0; 8]; 3];
    if coefficients.dense_h.nrows() != 3 || coefficients.dense_h.ncols() != 8 {
        return Err(
            CoreError::InvalidInput("unexpected native coefficient snapshot shape".into()).into(),
        );
    }
    for (r, row) in native_h.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            *x = coefficients.dense_h[(r, j)];
        }
    }
    let s = spec(
        "native-declared-equation-xprime=-x;H-from-same-cached-snapshot",
        [1.0; 2],
        [0.0; 2],
        15.0 / 16.0,
    );
    let mut previous: Option<AffineEventCertificate> = None;
    for k in 0..result.segments.len() {
        let c = if let Some(prior) = previous.as_ref() {
            result
                .segments
                .certify_affine_scalar_segment_with_carry(k, &s, &prior.carry()?)?
        } else {
            result.segments.certify_affine_scalar_segment(k, &s, 0.0)?
        };
        let t = [result.segments.t_old(k), result.segments.t_new(k)];
        let y = [result.segments.state(k)[0], result.segments.state(k + 1)[0]];
        let mut u = [0.0; 8];
        u.copy_from_slice(result.segments.stage_rows(k));
        let queries = query_times(t)
            .into_iter()
            .map(|time| {
                result
                    .segments
                    .query_affine_scalar_certificate(&c, &s, time)
                    .map(query_json)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut output = record("native-accepted-affine", &c, &s, t, y, native_h, u, queries);
        output["center_origin"] = json!("FastDenseSegments::evaluate_into;actual-returned-bits");
        records.push(output);
        previous = Some(c);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
        "schema":"vigilode.sym02.bit_evidence.v1",
        "compile_source_commit":option_env!("SYM02_SOURCE_COMMIT"),
        "native_coefficient_snapshot_sha256":RODAS5P_COEFFICIENT_SNAPSHOT_SHA256,
        "source_revision_premise":"same executable coefficient snapshot and accepted segment storage",
        "scope":"declared scalar affine family;native and synthetic provenance separated",
        "actual_bass30_provider":false,"performance_claim":false,
        "native_steps":result.segments.len(),"records":records}))?
    );
    Ok(())
}
